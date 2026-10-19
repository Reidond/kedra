//! Full local OS image build of the working tree or one commit.
//!
//! Replays the Actions candidate build (release-target.yml) in disposable
//! containers: Linux `sysroot` binaries and `sysroot source archive` in the Rust
//! builder, then the stage's own foundation (usr/src/kedra/image/Containerfile
//! and assemble.sh) on the resolved Fedora bootc base, installed offline from an
//! RPM lock the host `sysroot catalog lock` resolves. The stage's own `sysroot`
//! then builds the target's catalog packages (Codex, Bitwarden) and contributes
//! them with the release's public preflight material, composes that foundation
//! with the committed payload and the contribution, and derives the image steps
//! of the target's `.kedra` block, as the release does. Stages from before the
//! engine pipeline build their complete image in the Containerfile instead. The
//! result is a local, unsigned candidate: never pushed, signed or installed;
//! release identity layers are not added.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use testcontainers::ImageExt;
use testcontainers::core::{AccessMode, Mount};
use testcontainers::runners::SyncRunner;

use crate::builder::{self, Stage, content_key};
use crate::docker::{self, Build, Docker, Exec};
use crate::image::Target;
use crate::shared_cache::{Cache, Kind};
use crate::shared_store;
use crate::{Result, artifact_root, invalid};

const BASE_TAG: &str = "quay.io/fedora/fedora-bootc:44";

/// Run the host `sysroot` with explicit arguments; its standard output.
fn host_sysroot(sysroot: &Path, args: &[&std::ffi::OsStr]) -> Result<Vec<u8>> {
    let output = Command::new(sysroot).args(args).output()?;
    if !output.status.success() {
        return Err(invalid(format!(
            "sysroot {} failed: {}",
            args.iter()
                .take(2)
                .map(|a| a.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

/// The shared host RPM store, reached through the host `sysroot`.
struct HostStore<'a> {
    sysroot: &'a Path,
    path: String,
}

impl shared_store::Access for HostStore<'_> {
    fn store(&self) -> &str {
        &self.path
    }

    fn sysroot(&self, args: &[&str]) -> Result<Vec<u8>> {
        let args: Vec<&std::ffi::OsStr> = args.iter().map(|arg| arg.as_ref()).collect();
        host_sysroot(self.sysroot, &args)
    }

    fn exists(&self) -> Result<bool> {
        Ok(Path::new(&self.path).join("marker.json").is_file())
    }

    fn pins(&self) -> Result<shared_store::Pins> {
        let path = Path::new(&self.path).join("roots/state.json");
        serde_json::from_slice(&fs::read(&path)?)
            .map_err(|error| invalid(format!("{}: {error}", path.display())))
    }

    fn usage(&self) -> Result<u64> {
        crate::shared_cache::usage(Path::new(&self.path))
    }
}

/// The objects of every RPM a lock pins.
fn lock_objects(lock: &Path) -> Result<std::collections::BTreeSet<String>> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(lock)?)
        .map_err(|error| invalid(format!("{}: {error}", lock.display())))?;
    value["rpms"]
        .as_object()
        .ok_or_else(|| invalid(format!("{} has no RPMs", lock.display())))?
        .values()
        .map(|rpm| {
            rpm["object"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid("an RPM pin has no object"))
        })
        .collect()
}

/// Declared inputs of an RPM lock resolution, its shared cache key: the target,
/// the Fedora base digest, the stage's lock inputs (`image/packages/`, the
/// target policy) and the resolver that runs: the host archiver, named by the
/// key of all its source inputs (every Rust source, `Cargo.lock`, the toolchain
/// and the host platform; [`builder::prepare_archiver`]), not by the stage's
/// sources, which a `build:<revision>` stage takes from another commit.
fn lock_inputs(
    snapshot: &Path,
    target: &Target,
    base: &str,
    resolver: &Path,
) -> Result<serde_json::Value> {
    let resolver = resolver
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("the host archiver has no source key"))?;
    let mut files = Vec::new();
    collect_files(&snapshot.join("usr/src/kedra/image/packages"), &mut files)?;
    files.push(snapshot.join("usr/src/kedra/image/package-policy.json"));
    files.sort();
    let mut digests = serde_json::Map::new();
    for path in files {
        let relative = path.strip_prefix(snapshot).unwrap_or(&path);
        let digest = if path.is_file() {
            builder::file_sha256(&path)?
        } else {
            "absent".to_owned()
        };
        digests.insert(relative.to_string_lossy().into_owned(), digest.into());
    }
    Ok(serde_json::json!({
        "target": target.id,
        "base": base,
        "files": digests,
        "resolver": resolver,
    }))
}

/// The stage's RPM lock from the shared cache (resolved once per declared
/// inputs, by whichever checkout needs it first), then the foundation's offline
/// repository as context files under `rpms/`. The RPMs live in the shared host
/// RPM store, so every checkout fetches them once.
fn locked_rpms(
    cache: &Path,
    snapshot: &Path,
    target: &Target,
    base: &str,
    context: &Path,
) -> Result<(Vec<(String, PathBuf)>, PathBuf)> {
    let sysroot = builder::prepare_archiver(cache)?;
    let shared = Cache::locate()?;
    let host = HostStore {
        sysroot: &sysroot,
        path: shared
            .store_path(shared_store::RPM)
            .to_str()
            .ok_or_else(|| invalid("the shared cache path is not UTF-8"))?
            .to_owned(),
    };
    let held = shared_store::open(&shared, shared_store::RPM, &host)?;
    let store = PathBuf::from(&host.path);
    let image = snapshot.join("usr/src/kedra/image");
    let inputs = lock_inputs(snapshot, target, base, &sysroot)?;
    let entry = shared.ensure(Kind::RpmLock, &inputs, |build| {
        fs::create_dir(&build.content)?;
        let lock = build.content.join("lock.json");
        eprintln!(
            "kedra-lab: resolving the RPM lock for {} on {base}",
            target.id
        );
        host_sysroot(
            &sysroot,
            &[
                "catalog".as_ref(),
                "lock".as_ref(),
                "resolve".as_ref(),
                "--input-root".as_ref(),
                image.join("packages").as_os_str(),
                "--entry".as_ref(),
                "catalog.kedra".as_ref(),
                "--lock".as_ref(),
                "packages.lock.json".as_ref(),
                "--target".as_ref(),
                target.id.as_ref(),
                "--target-policy".as_ref(),
                image.join("package-policy.json").as_os_str(),
                "--base".as_ref(),
                base.as_ref(),
                "--store".as_ref(),
                store.as_os_str(),
                "--output".as_ref(),
                lock.as_os_str(),
            ],
        )?;
        Ok(())
    })?;
    let lock = entry.content().join("lock.json");
    let repository = context.join("rpms");
    if repository.exists() {
        fs::remove_dir_all(&repository)?;
    }
    // A lock this checkout resolved earlier may name RPMs a prune removed since.
    host_sysroot(
        &sysroot,
        &[
            "catalog".as_ref(),
            "lock".as_ref(),
            "fetch".as_ref(),
            "--store".as_ref(),
            store.as_os_str(),
            "--lock".as_ref(),
            lock.as_os_str(),
        ],
    )?;
    host_sysroot(
        &sysroot,
        &[
            "catalog".as_ref(),
            "lock".as_ref(),
            "repo".as_ref(),
            "--store".as_ref(),
            store.as_os_str(),
            "--lock".as_ref(),
            lock.as_os_str(),
            "--foundation".as_ref(),
            "--output".as_ref(),
            repository.as_os_str(),
        ],
    )?;
    let mut files = vec![("rpms/lock.json".to_owned(), repository.join("lock.json"))];
    let mut packages: Vec<_> = fs::read_dir(repository.join("packages"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<_>>()?;
    packages.sort();
    for name in packages {
        files.push((
            format!("rpms/packages/{name}"),
            repository.join("packages").join(&name),
        ));
    }
    let used = shared_store::Pins {
        objects: lock_objects(&lock)?,
        ..Default::default()
    };
    shared_store::finish(&shared, shared_store::RPM, &host, &used, held)?;
    Ok((files, lock))
}

/// Whether the stage's committed source declares image steps for `sysroot
/// derive`, read from the payload's generated source manifest.
fn declares_native_steps(payload: &Path) -> Result<bool> {
    let mut archive = tar::Archive::new(fs::File::open(payload)?);
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.as_os_str() != "usr/share/sysroot/source.json" {
            continue;
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(1024 * 1024).read_to_end(&mut bytes)?;
        let manifest: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        return Ok(manifest["native_steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty()));
    }
    Err(invalid(
        "the stage payload has no generated source manifest",
    ))
}

/// The JSON a `sysroot` command printed in the engine container.
fn engine_json(
    docker: &Docker,
    container: &str,
    argv: &[&str],
    minutes: u64,
) -> Result<serde_json::Value> {
    let mut exec = Exec::new(argv.iter().copied()).timeout(Duration::from_secs(minutes * 60));
    exec.user = Some("kedra".into());
    exec.env = vec![
        ("HOME".into(), "/home/kedra".into()),
        ("PATH".into(), "/usr/local/bin:/usr/bin:/bin".into()),
        ("DOCKER_HOST".into(), "unix:///var/run/docker.sock".into()),
    ];
    let output = docker.run(container, &exec)?;
    serde_json::from_slice(&output.stdout).map_err(|error| invalid(error.to_string()))
}

fn engine_string(value: &serde_json::Value, pointer: &str) -> Result<String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("engine result lacks {pointer}")))
}

/// The release's catalog steps, as the ordinary user in the engine container:
/// the public preflight material image/release/material.py records for the
/// release (including the run's single pin envelope), the lock's RPMs fetched
/// into the container's store, then the stage's own
/// image/catalog/build-packages.sh, the script the release build runs, in the
/// shared engine store. Prints `{"definition": path, "pins": {...}}` with the
/// images, roots and objects the build used, or `{}` when the target selects
/// no package.
const CATALOG: &str = r#"set -euo pipefail
target=$1 foundation=$2 base=$3 revision=$4 store=$5
image=/snapshot/usr/src/kedra/image
work=$HOME/catalog
lock=$HOME/rpm-lock.json
context=$HOME/context
pins=$HOME/catalog-pins.json
if test "$(sysroot catalog pins --input-root "$image/packages" --entry catalog.kedra --target "$target" \
    --lock packages.lock.json --target-policy "$image/package-policy.json" | jq '.packages | length')" = 0; then
    echo '{}'
    exit 0
fi
if ! test -x "$image/catalog/build-packages.sh"; then
    echo "this stage has no usr/src/kedra/image/catalog/build-packages.sh; build a newer stage" >&2
    exit 1
fi
sysroot source plan --repo /snapshot --host "$target" --json > "$HOME/source-plan.json"
uv run --quiet --no-project python - "$HOME/source-plan.json" "$base" "$context" "$lock" "$target" "$pins" \
    > "$HOME/resolved-inputs.json" <<'PY'
import pathlib, sys
sys.dont_write_bytecode = True
sys.path.insert(0, '/snapshot/usr/src/kedra/image/release')
import material as m
plan, base, context, lock, target, pins = sys.argv[1:]
context = pathlib.Path(context)
inputs = m.resolved_inputs(pathlib.Path('/snapshot'), m.document(m.read(pathlib.Path(plan))), base, context,
                           context, m.read(pathlib.Path(lock), m.LOCK_LIMIT), target, catalog={'pins': pathlib.Path(pins)})
sys.stdout.buffer.write(m.canonical(inputs))
PY
sysroot catalog lock fetch --store "$store" --lock "$lock" > /dev/null
base_id=$(docker image inspect --format '{{.Id}}' "$base")
"$image/catalog/build-packages.sh" --sysroot "$(command -v sysroot)" --store "$store" --target "$target" \
    --lock "$lock" --base-image "$base_id" --foundation "$foundation" --source-revision "$revision" \
    --inputs "$HOME/resolved-inputs.json" --pins "$pins" --work "$work" >&2
objects=$(jq -n -c --slurpfile lock "$lock" --slurpfile sources <(cat "$work"/*-source.json) \
    --slurpfile builds <(cat "$work"/*-build.json) \
    '[$lock[0].rpms[].object] + [$sources[].object] + [$builds[].result.outputs[]]')
jq -n --arg definition "$work/contribution/system.json" --argjson objects "$objects" \
    --slurpfile base "$work/base.json" --slurpfile roots <(cat "$work"/runtime.json "$work"/builder-*.json) \
    '{definition:$definition,pins:{objects:$objects,images:[$base[0].image],rootfs:[$roots[].root]}}'
"#;

/// Build and contribute the stage's catalog packages in the engine container;
/// the definition path in the container (none when nothing is selected) and
/// what the build used from the shared engine store.
fn catalog_definition(
    docker: &Docker,
    container: &str,
    target: &Target,
    build: &EngineBuild<'_>,
) -> Result<(Option<String>, shared_store::Pins)> {
    // The release material names the run's binaries and RPM lock; the catalog
    // author must be the binary that contributes.
    for (path, file, mode) in [
        ("/home/kedra/rpm-lock.json", build.lock, "0644"),
        ("/home/kedra/context/sysroot", build.sysroot, "0755"),
        ("/home/kedra/context/sysroot-helper", build.helper, "0755"),
    ] {
        docker.write_file(container, path, &fs::read(file)?, "kedra", mode)?;
    }
    let result = engine_json(
        docker,
        container,
        &[
            "bash",
            "-c",
            CATALOG,
            "catalog",
            &target.id,
            build.foundation,
            build.base,
            build.revision,
            ENGINE_STORE,
        ],
        120,
    )?;
    let pins = match result.get("pins") {
        Some(pins) => serde_json::from_value(pins.clone())
            .map_err(|error| invalid(format!("catalog pins: {error}")))?,
        None => shared_store::Pins::default(),
    };
    Ok((result["definition"].as_str().map(str::to_owned), pins))
}

/// Where the shared engine store's volume is mounted in the engine container,
/// outside the builder's home so `useradd -m` still creates and owns that.
const ENGINE_MOUNT: &str = "/var/cache/kedra-engine";
const ENGINE_STORE: &str = "/var/cache/kedra-engine/store";

/// The shared engine store, reached through `sysroot` in the engine container.
struct EngineStore<'a> {
    docker: &'a Docker,
    container: &'a str,
}

impl EngineStore<'_> {
    fn exec(&self, argv: &[&str], minutes: u64) -> Result<docker::Output> {
        let mut exec = Exec::new(argv.iter().copied()).timeout(Duration::from_secs(minutes * 60));
        exec.user = Some("kedra".into());
        exec.env = vec![
            ("HOME".into(), "/home/kedra".into()),
            ("PATH".into(), "/usr/local/bin:/usr/bin:/bin".into()),
            ("DOCKER_HOST".into(), "unix:///var/run/docker.sock".into()),
        ];
        self.docker.exec(self.container, &exec)
    }
}

impl shared_store::Access for EngineStore<'_> {
    fn store(&self) -> &str {
        ENGINE_STORE
    }

    fn sysroot(&self, args: &[&str]) -> Result<Vec<u8>> {
        let mut argv = vec!["sysroot"];
        argv.extend(args);
        let output = self.exec(&argv, 60)?;
        if output.exit != 0 {
            return Err(crate::Error::Command {
                what: format!("{argv:?}"),
                exit: output.exit,
                stderr: output.stderr_text(),
            });
        }
        Ok(output.stdout)
    }

    fn exists(&self) -> Result<bool> {
        Ok(self
            .exec(&["test", "-f", &format!("{ENGINE_STORE}/marker.json")], 1)?
            .exit
            == 0)
    }

    fn pins(&self) -> Result<shared_store::Pins> {
        let output = self.exec(&["cat", &format!("{ENGINE_STORE}/roots/state.json")], 1)?;
        serde_json::from_slice(&output.stdout)
            .map_err(|error| invalid(format!("engine store roots: {error}")))
    }

    fn usage(&self) -> Result<u64> {
        let output = self.exec(&["du", "-sk", ENGINE_STORE], 10)?;
        let text = output.stdout_text();
        text.split_whitespace()
            .next()
            .and_then(|kib| kib.parse::<u64>().ok())
            .map(|kib| kib * 1024)
            .ok_or_else(|| invalid(format!("du of the engine store: {text}")))
    }
}

/// Key of a derived image: its foundation's key, plus what the engine steps read
/// that neither the payload nor the foundation context carries (the catalog
/// tooling, release material code and this file's catalog steps).
fn engine_key(foundation: &str, snapshot: &Path) -> Result<String> {
    let mut files = Vec::new();
    for directory in ["usr/src/kedra/image/catalog", "usr/src/kedra/image/release"] {
        collect_files(&snapshot.join(directory), &mut files)?;
    }
    files.sort();
    let mut parts = vec![foundation.as_bytes().to_vec(), CATALOG.as_bytes().to_vec()];
    for path in files {
        let relative = path.strip_prefix(snapshot).unwrap_or(&path);
        parts.push(relative.to_string_lossy().into_owned().into_bytes());
        parts.push(builder::file_sha256(&path)?.into_bytes());
    }
    let parts: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Ok(content_key(&parts))
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// The stage's inputs to its engine steps.
struct EngineBuild<'a> {
    sysroot: &'a Path,
    helper: &'a Path,
    snapshot: &'a Path,
    lock: &'a Path,
    foundation: &'a str,
    base: &'a str,
    revision: &'a str,
    reference: &'a str,
}

/// Compose the foundation with the snapshot's committed payload and derive the
/// target's declared image steps with the stage's own Linux `sysroot`, in a
/// disposable builder container that reaches this lab's Docker engine. The
/// derived image is tagged `reference`; the store and contexts die with the
/// container. Ordinary-user engine stores refuse root, so the commands run as
/// a user in the socket's group. Composition observes the foundation in the
/// engine's bubblewrap sandbox, which needs unprivileged user namespaces, a
/// fresh `/proc` and `/dev` inside this container.
fn engine_image(docker: &Docker, target: &Target, build: &EngineBuild<'_>) -> Result<()> {
    let EngineBuild {
        sysroot,
        snapshot,
        foundation,
        reference,
        ..
    } = *build;
    eprintln!(
        "kedra-lab: composing and deriving {} through the engine",
        target.id
    );
    let shared = Cache::locate()?;
    let volume = docker.shared_volume(&shared_store::engine_volume(&shared), &shared.id())?;
    let container = builder::builder_image(docker, &target.platform())?
        .with_cmd(["sleep", "infinity"])
        .with_label(docker::OWNER_LABEL, docker::OWNER)
        .with_label(docker::CHECKOUT_LABEL, crate::checkout_id())
        .with_label(docker::KIND_LABEL, "builder")
        .with_security_opt("seccomp=unconfined")
        .with_security_opt("apparmor=unconfined")
        // The API form of `--security-opt systempaths=unconfined`, which the
        // Docker CLI translates and the engine API refuses as an option.
        .with_host_config_modifier(|config| {
            config.masked_paths = Some(Vec::new());
            config.readonly_paths = Some(Vec::new());
        })
        .with_mount(Mount::bind_mount(
            "/var/run/docker.sock",
            "/var/run/docker.sock",
        ))
        .with_mount(
            Mount::bind_mount(snapshot.to_string_lossy(), "/snapshot")
                .with_access_mode(AccessMode::ReadOnly),
        )
        .with_mount(Mount::volume_mount(volume, ENGINE_MOUNT))
        .with_startup_timeout(Duration::from_secs(120))
        .start()?;
    let id = container.id().to_owned();
    docker.write_file(
        &id,
        "/usr/local/bin/sysroot",
        &fs::read(sysroot)?,
        "root",
        "0755",
    )?;
    docker.run(
        &id,
        &Exec::new([
            "sh".to_owned(),
            "-ec".to_owned(),
            format!(
                "gid=$(stat -c %g /var/run/docker.sock); group=$(getent group \"$gid\" | cut -d: -f1); \
                 if [ -z \"$group\" ]; then groupadd -g \"$gid\" docker-host; group=docker-host; fi; \
                 useradd -m -u 1000 -G \"$group\" kedra; chown kedra:kedra {ENGINE_MOUNT}"
            ),
        ]),
    )?;
    let store = ENGINE_STORE;
    let access = EngineStore {
        docker,
        container: &id,
    };
    let held = shared_store::open(&shared, shared_store::ENGINE, &access)?;
    engine_json(
        docker,
        &id,
        &[
            "sysroot",
            "store",
            "add-image",
            "--store",
            store,
            "--image",
            foundation,
        ],
        60,
    )?;
    let (definition, used) = catalog_definition(docker, &id, target, build)?;
    let context = "/home/kedra/composition";
    let mut compose = vec![
        "sysroot",
        "system",
        "compose",
        "--repo",
        "/snapshot",
        "--target",
        &target.id,
        "--store",
        store,
        "--foundation",
        foundation,
        "--output-dir",
        context,
    ];
    if let Some(definition) = &definition {
        compose.extend(["--definition", definition.as_str()]);
    }
    let composed = engine_json(docker, &id, &compose, 60)?;
    let identity = engine_string(&composed, "/identity")?;
    let state = "/home/kedra/derive";
    let common = [
        "--context",
        context,
        "--expected-identity",
        &identity,
        "--state",
        state,
    ];
    let mut plan = vec!["sysroot", "derive", "plan"];
    plan.extend(common);
    let derivation = engine_string(&engine_json(docker, &id, &plan, 10)?, "/identity")?;
    let mut build = vec!["sysroot", "derive", "build"];
    build.extend(common);
    build.extend(["--expected-derivation", derivation.as_str()]);
    let derived = engine_string(&engine_json(docker, &id, &build, 90)?, "/image")?;
    let mut tag = Exec::new(["docker", "image", "tag", &derived, reference]);
    tag.user = Some("kedra".into());
    tag.env = vec![("DOCKER_HOST".into(), "unix:///var/run/docker.sock".into())];
    docker.run(&id, &tag)?;
    // The foundation is this build's own; the next prune collects it.
    shared_store::Access::sysroot(
        &access,
        &[
            "store",
            "unpin-image",
            "--store",
            store,
            "--image",
            foundation,
        ],
    )?;
    shared_store::finish(&shared, shared_store::ENGINE, &access, &used, held)?;
    container.rm()?;
    eprintln!("kedra-lab: derived {derived} (native identity {derivation})");
    Ok(())
}

/// Build the stage's OS image and return its local reference. `binaries`
/// (`KEDRA_LAB_BINARIES`) are the stage's own Linux `sysroot` and helper, built
/// elsewhere; without them the lab builder compiles them in its shared Cargo volume.
pub fn local(
    docker: &Docker,
    target: &Target,
    revision: Option<&str>,
    binaries: Option<&Path>,
) -> Result<String> {
    let stage = match revision {
        Some(revision) => Stage::Revision(revision.to_owned()),
        None => Stage::Worktree,
    };
    let platform = target.platform();
    let cache = artifact_root().join("cache");
    // 1. Binaries and the committed-tree payload, exactly as Actions archives it.
    let outputs = builder::prepare(
        docker,
        &target.id,
        &target.architecture,
        &stage,
        binaries,
        &cache,
    )?;

    // 2. The stage's own build definition and input scripts.
    let snapshot = cache.join(format!("build-snapshot-{}", crate::execution_id()));
    let (snapshot_commit, _) = builder::snapshot(&crate::repository_root(), &stage, &snapshot)?;
    if snapshot_commit != outputs.snapshot_commit {
        return Err(invalid(
            "the stage changed while its binaries and payload were prepared; build again",
        ));
    }
    let engine = declares_native_steps(&outputs.payload)?;
    let context = cache.join(format!("build-context-{}", outputs.snapshot_commit));
    fs::create_dir_all(&context)?;
    for name in ["Containerfile", "assemble.sh"] {
        let source = snapshot.join("usr/src/kedra/image").join(name);
        if !source.is_file() {
            return Err(invalid(format!(
                "this stage has no usr/src/kedra/image/{name}; local builds need the root-filesystem layout"
            )));
        }
        fs::copy(&source, context.join(name))?;
    }
    let installer = snapshot.join("usr/src/kedra/image/catalog/install-rpms.sh");
    let locked = installer.is_file();
    if locked {
        fs::copy(&installer, context.join("install-rpms.sh"))?;
    }
    fs::copy(&outputs.sysroot, context.join("sysroot"))?;
    fs::copy(&outputs.helper, context.join("sysroot-helper"))?;
    fs::copy(&outputs.payload, context.join("payload.tar"))?;

    // 3. Resolve the Fedora bootc base to one platform digest, then build.
    docker.pull(BASE_TAG, &platform)?;
    let (image_id, digests) = docker
        .image(BASE_TAG)?
        .ok_or_else(|| invalid(format!("{BASE_TAG} is missing after pulling")))?;
    let base = digests
        .into_iter()
        .find(|digest| digest.starts_with("quay.io/fedora/fedora-bootc@"))
        .unwrap_or(image_id);
    let mut names = vec!["sysroot", "sysroot-helper", "payload.tar", "assemble.sh"];
    if locked {
        names.push("install-rpms.sh");
    }
    let mut files: Vec<(String, PathBuf)> = names
        .iter()
        .map(|name| ((*name).to_owned(), context.join(name)))
        .collect();
    // 4. Stages that install offline get the run's RPM lock and foundation repository.
    let mut lock = None;
    if locked {
        let (repository, path) = locked_rpms(&cache, &snapshot, target, &base, &context)?;
        files.extend(repository);
        lock = Some(path);
    }
    let mut key_parts: Vec<Vec<u8>> = vec![
        base.as_bytes().to_vec(),
        fs::read(context.join("Containerfile"))?,
    ];
    for (_, path) in &files {
        key_parts.push(builder::file_sha256(path)?.into_bytes());
    }
    let parts: Vec<&[u8]> = key_parts.iter().map(Vec::as_slice).collect();
    let key = docker::scoped_tag(&content_key(&parts));
    let reference = if engine {
        format!(
            "kedra-local-{}:{}",
            target.id,
            docker::scoped_tag(&engine_key(&key, &snapshot)?)
        )
    } else {
        format!("kedra-local-{}:{key}", target.id)
    };
    if engine && docker.image(&reference)?.is_some() {
        eprintln!("kedra-lab: {reference} is cached");
        fs::remove_dir_all(&snapshot)?;
        return Ok(reference);
    }
    let built = if engine {
        format!("kedra-local-foundation-{}:{key}", target.id)
    } else {
        reference.clone()
    };
    docker.build(&Build {
        tag: &built,
        what: "full local OS image; Fedora package installation takes several minutes",
        containerfile: &context.join("Containerfile"),
        files: &files,
        args: &[("BASE_IMAGE", base.clone())],
        platform: &platform,
    })?;
    if engine {
        let (foundation, _) = docker
            .image(&built)?
            .ok_or_else(|| invalid(format!("{built} is missing after its build")))?;
        let lock =
            lock.ok_or_else(|| invalid("engine-pipeline stages install from an RPM lock"))?;
        engine_image(
            docker,
            target,
            &EngineBuild {
                sysroot: &outputs.sysroot,
                helper: &outputs.helper,
                snapshot: &snapshot,
                lock: &lock,
                foundation: &foundation,
                base: &base,
                revision: &outputs.snapshot_commit,
                reference: &reference,
            },
        )?;
    }
    fs::remove_dir_all(&snapshot)?;
    eprintln!(
        "kedra-lab: built {reference} from {} on {base}",
        outputs.snapshot_commit
    );
    Ok(reference)
}
