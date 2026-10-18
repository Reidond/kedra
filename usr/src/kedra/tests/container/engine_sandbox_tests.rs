//! The engine's Nix-style commands on the installed system: `sysroot store
//! add-image --archive`, `add-root`, `build`, `run`, `profile` and `develop` as
//! the ordinary user, in the bubblewrap sandbox, with no Docker in the image,
//! plus recovery after the controller of a running build is killed.
use kedra_container_tests::Result;
use kedra_container_tests::scenario::Context;

use crate::native::{archive_identity, ensure, fail, local_image};

/// A small exact runtime image (Ubuntu 24.04), saved on the harness engine
/// because the image under test has no container daemon.
const REFERENCE: &str = "docker.io/library/ubuntu@sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3";
/// Its arm64 configuration digest, which pins the saved content on both Docker
/// image stores ([`archive_identity`]).
const CONFIG: &str = "sha256:8494c74ca40fca88c433937d0d3a9cfd3ffee8e57d0a517ce68788d85c6b4b48";

const NO_DAEMON: &str = r#"set -eu
command -v bwrap
rpm -q bubblewrap
if command -v docker || command -v dockerd || test -e /run/docker.sock || test -e /var/run/docker.sock; then
    echo 'the installed image carries Docker' >&2
    exit 1
fi
"#;

const WORKFLOW: &str = r#"set -eu
image=$1
store=$HOME/engine-store
field() { python3 -c 'import json, sys
value = json.load(sys.stdin)
for key in sys.argv[1].split("."):
    value = value[key]
print(value)' "$1"; }
sysroot store init --store "$store" >/dev/null
sysroot store add-image --store "$store" --image "$image" --archive "$HOME/runtime.tar" >/dev/null
root=$(sysroot store add-root --store "$store" --image "$image" | field root)
test "$(sysroot store add-root --store "$store" --image "$image" | field root)" = "$root"
echo "root $root"
mkdir -p source
cat > source/build.sh <<'BUILD'
set -eu
mkdir -p "$1/bin"
printf '#!/bin/sh\necho "engine-sandbox:$*:$(id -u)"\n' > "$1/bin/hello"
chmod 755 "$1/bin/hello"
BUILD
cat > source/slow.sh <<'BUILD'
set -eu
sleep 60
mkdir -p "$1"
BUILD
graph() {
    printf '{"schema":1,"nodes":{"app":{"builder_root":"%s","runtime_root":"%s","inputs":{"source":{"kind":"object","value":"%s"}},"argv":[[{"kind":"literal","value":"/bin/sh"}],[{"kind":"input","name":"source","path":"%s"}],[{"kind":"output","path":""}]],"env":{},"runtime_inputs":[],"timeout_seconds":120}}}' "$root" "$root" "$source" "$1"
}
source=$(sysroot store add-source --store "$store" --source source | field object)
graph build.sh > build.json
output=$(sysroot build --store "$store" --plan build.json --root app | field outputs.app)
echo "output $output"
sysroot run --store "$store" --object "$output" --program bin/hello -- run
sysroot profile switch --store "$store" --name sandbox --object "$output" --program bin/hello -- profile >/dev/null
sysroot profile run --store "$store" --name sandbox -- extra
sysroot develop --store "$store" --name sandbox --program /bin/sh -- -c '
    set -eu
    test "$(id -u)" != 0
    test ! -e "$1"
    test ! -e /run/docker.sock
    test ! -e /var/run/docker.sock
    test "$(tail -n +3 /proc/net/dev | grep -vc "^ *lo:")" = 0
    if touch /usr/lib/sysroot/store/forbidden 2>/dev/null; then exit 41; fi
    if touch /forbidden 2>/dev/null; then exit 42; fi
    test -z "${KEDRA_SANDBOX_HOST_SENTINEL:-}"
    test "$HOME" = /build
    echo develop-isolated
' develop "$HOME"
graph slow.sh > slow.json
sysroot build --store "$store" --plan slow.json --root app > slow.out 2>&1 &
controller=$!
# A loaded runner can take a while to reach the sandbox; wait up to two minutes.
for _ in $(seq 1200); do
    if grep -qs '"pid"' "$store/transactions/execution.json"; then break; fi
    sleep 0.1
done
if ! grep -qs '"pid"' "$store/transactions/execution.json"; then
    echo 'the interrupted build never journaled its sandbox process' >&2
    cat slow.out >&2
    exit 1
fi
kill -9 "$controller"
wait "$controller" || true
if sysroot store gc --store "$store" > gc.out 2>&1; then
    echo 'store gc ran over an unfinished sandbox journal' >&2
    exit 1
fi
grep -q 'recovery required' gc.out
sysroot store recover --store "$store" | field recovered | grep -q sandbox-
test ! -e "$store/transactions/execution.json"
echo recovered
sysroot store gc --store "$store" --delete >/dev/null
sysroot run --store "$store" --object "$output" --program bin/hello -- after
"#;

pub fn engine_sandbox(context: &Context<'_>) -> Result<()> {
    std::fs::create_dir_all(&context.artifacts)?;
    let docker = context.docker;
    let container = &context.environment.id;
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| fail("engine sandbox case requires a disposable user"))?;
    context.environment.run(
        docker,
        &kedra_container_tests::docker::Exec::new(["/bin/sh", "-c", NO_DAEMON]),
    )?;
    let archive = docker.save(&local_image(docker, REFERENCE, "linux/arm64")?)?;
    let image = archive_identity(&archive, CONFIG)?;
    docker.write_file(
        container,
        &format!("{}/runtime.tar", user.home),
        &archive,
        &user.name,
        "0644",
    )?;
    let mut exec = user.exec(["/bin/sh", "-c", WORKFLOW, "engine-sandbox", image.as_str()]);
    exec.env
        .push(("KEDRA_SANDBOX_HOST_SENTINEL".into(), "host-only".into()));
    let output = context
        .environment
        .exec(docker, &exec.timeout(std::time::Duration::from_secs(600)))?;
    std::fs::write(
        context.artifacts.join("engine-sandbox.stdout"),
        &output.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("engine-sandbox.stderr"),
        &output.stderr,
    )?;
    ensure(output.exit == 0, || {
        format!(
            "engine sandbox workflow exited {}: {}",
            output.exit,
            output.stderr_text()
        )
    })?;
    let uid = context
        .environment
        .run(docker, &user.exec(["id", "-u"]))?
        .stdout_text();
    let uid = uid.trim();
    let stdout = output.stdout_text();
    for expected in [
        format!("engine-sandbox:run:{uid}"),
        format!("engine-sandbox:profile extra:{uid}"),
        "develop-isolated".into(),
        "recovered".into(),
        format!("engine-sandbox:after:{uid}"),
    ] {
        ensure(stdout.lines().any(|line| line == expected), || {
            format!("engine sandbox output lacks {expected:?}: {stdout}")
        })?;
    }
    Ok(())
}
