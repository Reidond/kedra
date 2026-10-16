//! Installed catalog packages and the catalog's no-shadowing rule in the
//! sanctioned harness.
use kedra_container_tests::docker::Exec;
use kedra_container_tests::scenario::Context;
use kedra_container_tests::{Error, Result};
use serde_json::Value;
use sysroot_engine::LOGICAL_PREFIX;

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

fn text<'a>(value: &'a Value, message: &str) -> Result<&'a str> {
    value.as_str().ok_or_else(|| Error::Invalid(message.into()))
}

fn output<'a>(inventory: &'a Value, name: &str) -> Result<&'a str> {
    let value = text(&inventory["outputs"][name], "catalog output is missing")?;
    ensure(
        value.strip_prefix("out-").is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }),
        "catalog output identity is invalid",
    )?;
    Ok(value)
}

/// Every exported command is a launcher of its exact output, first on the
/// login and user-manager PATH; private `libexec` exports are on neither; no
/// command shadows the foundation unless declared, so Fedora's jq stays jq.
pub fn installed_catalog(context: &Context<'_>) -> Result<()> {
    std::fs::create_dir_all(&context.artifacts)?;
    let docker = context.docker;
    let container = &context.environment.id;
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| Error::Invalid("catalog case requires a disposable user".into()))?;
    let inventory: Value =
        serde_json::from_slice(&docker.read_file(container, "/usr/share/kedra/catalog.json")?)
            .map_err(|error| Error::Invalid(error.to_string()))?;
    ensure(
        inventory["schema_version"] == 2
            && inventory["namespace"] == "kedra"
            && inventory["format"] == "kedra",
        "installed catalog inventory is unsupported",
    )?;
    let commands = inventory["commands"]
        .as_object()
        .ok_or_else(|| Error::Invalid("command inventory is missing".into()))?;
    let libexec = inventory["libexec"]
        .as_object()
        .ok_or_else(|| Error::Invalid("libexec inventory is missing".into()))?;
    ensure(
        commands.contains_key("bitwarden") && libexec.contains_key("codex"),
        "the shipped targets export bitwarden and the private codex",
    )?;
    let mut located = String::new();
    for (alias, command) in commands.iter().chain(libexec) {
        let package = text(&command["package"], "command package missing")?;
        let path = text(&command["path"], "command path missing")?;
        let launcher = text(&command["launcher"], "command launcher missing")?;
        context
            .environment
            .run(docker, &user.exec(["/usr/bin/test", "-x", launcher]))?;
        let script = String::from_utf8(docker.read_file(container, launcher)?)
            .map_err(|_| Error::Invalid("launcher is not text".into()))?;
        let target = format!("{LOGICAL_PREFIX}/{}/{path}", output(&inventory, package)?);
        ensure(
            script == format!("#!/bin/sh\nexec \"{target}\" \"$@\"\n"),
            "launcher does not exec the exact catalog output",
        )?;
        let shell = context.environment.exec(
            docker,
            &user.exec([
                "/bin/bash",
                "-lc",
                "command -v -- \"$1\"",
                "catalog-path",
                alias.as_str(),
            ]),
        )?;
        let expected = if commands.contains_key(alias) {
            format!("{launcher}\n")
        } else {
            String::new()
        };
        ensure(
            shell.stdout_text() == expected,
            "login shell PATH differs from the exported commands",
        )?;
        located.push_str(&shell.stdout_text());
    }
    for (alias, command) in commands {
        if command["replaces"] == true {
            continue;
        }
        for directory in ["/usr/local/bin", "/usr/local/sbin", "/usr/bin", "/usr/sbin"] {
            let present = docker.exec(
                container,
                &Exec::new(["/usr/bin/test", "-e", &format!("{directory}/{alias}")]),
            )?;
            ensure(
                present.exit != 0,
                "a catalog command shadows a foundation command it does not replace",
            )?;
        }
    }
    let fedora_jq = context
        .environment
        .run(docker, &user.exec(["/bin/bash", "-lc", "command -v jq"]))?;
    ensure(
        fedora_jq.stdout == b"/usr/bin/jq\n",
        "Fedora's jq is not the jq command",
    )?;
    let codex_version = text(&inventory["versions"]["codex"], "codex version missing")?;
    // Codex writes helper files to its home before it parses --version.
    let probe = format!("{}/codex-version", user.home);
    context
        .environment
        .run(docker, &user.exec(["mkdir", "-p", probe.as_str()]))?;
    let mut version = user.exec([
        text(&libexec["codex"]["launcher"], "codex launcher missing")?,
        "--version",
    ]);
    version.env.push(("CODEX_HOME".into(), probe));
    let version = context.environment.run(docker, &version)?;
    ensure(
        version.stdout_text() == format!("codex-cli {codex_version}\n"),
        "private Codex launcher version differs",
    )?;

    docker.run(
        container,
        &Exec::new([
            "install",
            "-d",
            "-m",
            "700",
            "-o",
            &user.name,
            "-g",
            &user.name,
            &user.runtime_dir(),
        ]),
    )?;
    docker.run(
        container,
        &Exec::new(["systemctl", "start", &format!("user@{}.service", user.uid)]),
    )?;
    context.environment.run(
        docker,
        &user.exec(["systemctl", "--user", "start", "dbus.socket"]),
    )?;
    let user_path = context.environment.run(
        docker,
        &user.exec([
            "systemd-run",
            "--user",
            "--quiet",
            "--wait",
            "--pipe",
            "--collect",
            "/bin/bash",
            "--noprofile",
            "--norc",
            "-c",
            "command -v bitwarden; command -v jq; command -v codex || true",
        ]),
    )?;
    ensure(
        user_path.stdout_text()
            == format!(
                "{}\n/usr/bin/jq\n",
                text(
                    &commands["bitwarden"]["launcher"],
                    "bitwarden launcher missing"
                )?
            ),
        "user service environment differs from the exported commands",
    )?;
    std::fs::write(context.artifacts.join("catalog-login-path.txt"), located)?;
    std::fs::write(
        context.artifacts.join("catalog-user-path.txt"),
        &user_path.stdout,
    )?;
    Ok(())
}

/// Docker Official Images' Fedora 44 index: a small exact Fedora 44 foundation
/// with RPM, saved on the harness engine because the image under test has none.
const FEDORA: &str = "docker.io/library/fedora@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80";
const FEDORA_IMAGE: &str =
    "sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80";

/// `sysroot catalog build` on the installed system with a Fedora foundation that
/// has `cat`: an export named `cat` is refused until its package declares
/// `replaces`; a declared replacement of a command Fedora lacks is refused too.
const SHADOWING: &str = r#"set -eu
image=$1
store=$HOME/shadowing-store
catalog=$HOME/shadowing
field() { python3 -c 'import json, sys
print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }
sysroot store init --store "$store" >/dev/null
sysroot store add-image --store "$store" --image "$image" --archive "$HOME/fedora.tar" >/dev/null
root=$(sysroot store add-root --store "$store" --image "$image" | field root)
mkdir -p "$catalog"
printf '{"schema_version":1,"sources":{}}\n' > "$catalog/lock.json"
printf '{"schema_version":1,"namespace":"shadowing","targets":["fixture"],"repositories":["fedora","updates"],"required_packages":["bash"]}\n' > "$catalog/target-policy.json"
printf '{"namespace":"shadowing","packages":["demo"],"builder_roots":["%s"],"runtime_images":["%s"],"source_objects":[]}\n' "$root" "$image" > "$catalog/policy.json"
write() {
    cat > "$catalog/catalog.kedra" <<KEDRA
language 1;
namespace "shadowing";
foundation system { fedora = 44; packages = ["bash"]; }
builder shell { base = system; packages = ["bash"]; }
package demo(builder: Builder, runtime: Foundation) {
    version = "1"; summary = "Shadowing fixture"; license = "MIT";
    source = files { "message" = text "kedra-shadowing\n"; };
    build = shell """
        mkdir -p "\$out/bin"
        printf '#!/bin/sh\nexec cat %s/message\n' "\$out" > "\$out/bin/$1"
        cp "\$src/message" "\$out/message"
        chmod 755 "\$out/bin/$1"
        """;
    export command "$1" = "bin/$1";
    $2
}
target "fixture" { foundation = system; packages = [demo(builder: shell, runtime: system)]; }
KEDRA
}
build() {
    sysroot catalog build --store "$store" --package demo --input-root "$catalog" --entry catalog.kedra \
        --target fixture --lock lock.json --target-policy "$catalog/target-policy.json" \
        --policy "$catalog/policy.json" --builder "$root" --runtime "$image"
}
write cat ''
if build > refused.out 2> refused.err; then
    echo 'an export shadowing the foundation cat was built' >&2
    exit 1
fi
grep -q "would shadow the foundation's cat" refused.err
echo refused-shadowing
write cat 'replaces = ["cat"];'
object=$(build | python3 -c 'import json, sys
print(json.load(sys.stdin)["result"]["outputs"]["package_0"])')
sysroot run --store "$store" --object "$object" --program bin/cat
write kedra-shadowing-free 'replaces = ["kedra-shadowing-free"];'
if build > stale.out 2> stale.err; then
    echo 'a replacement of a command Fedora lacks was built' >&2
    exit 1
fi
grep -q 'the foundation has no kedra-shadowing-free command' stale.err
echo refused-stale-replacement
"#;

pub fn shadowing_refusal(context: &Context<'_>) -> Result<()> {
    std::fs::create_dir_all(&context.artifacts)?;
    let docker = context.docker;
    let container = &context.environment.id;
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| Error::Invalid("shadowing case requires a disposable user".into()))?;
    let platform = match context.target.as_str() {
        "desktop" => "linux/amd64",
        _ => "linux/arm64",
    };
    if docker.image(FEDORA_IMAGE)?.is_none() {
        docker.pull(FEDORA, platform)?;
    }
    let archive = docker.save(FEDORA_IMAGE)?;
    docker.write_file(
        container,
        &format!("{}/fedora.tar", user.home),
        &archive,
        &user.name,
        "0644",
    )?;
    let output = context.environment.exec(
        docker,
        &user
            .exec(["/bin/sh", "-c", SHADOWING, "shadowing", FEDORA_IMAGE])
            .timeout(std::time::Duration::from_secs(600)),
    )?;
    std::fs::write(context.artifacts.join("shadowing.stdout"), &output.stdout)?;
    std::fs::write(context.artifacts.join("shadowing.stderr"), &output.stderr)?;
    ensure(
        output.exit == 0,
        &format!(
            "shadowing workflow exited {}: {}",
            output.exit,
            output.stderr_text()
        ),
    )?;
    let stdout = output.stdout_text();
    for expected in [
        "refused-shadowing",
        "kedra-shadowing",
        "refused-stale-replacement",
    ] {
        ensure(
            stdout.lines().any(|line| line == expected),
            &format!("shadowing output lacks {expected:?}: {stdout}"),
        )?;
    }
    Ok(())
}
