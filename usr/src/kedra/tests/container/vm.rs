//! Native QEMU preparation and retained desktop operations.

use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use clap::{Args, Subcommand};

use crate::shared_cache::{Cache, Kind};
use crate::{Result, harness_dir};

#[derive(Args)]
pub struct Instance {
    #[arg(long, default_value = "default")]
    name: String,
    #[arg(long)]
    runtime: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum VmCommand {
    /// Prepare or inspect the private native runtime (no Docker dependency).
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
    /// Prepare an unsigned native VM fixture and a reusable QCOW2 disk.
    Image {
        #[arg(long, default_value = "stable")]
        image: String,
        #[arg(long, default_value = "worktree")]
        overlay: String,
        /// Independently retained identity of the composition context.
        #[arg(long, requires = "derivation_identity")]
        composition_identity: Option<String>,
        /// Independently selected identity of its declared native steps.
        #[arg(long, requires = "composition_identity")]
        derivation_identity: Option<String>,
    },
    /// Build signed ARM64 installer media through the existing verification policy.
    Iso {
        #[arg(long)]
        image: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Create and open a separate native VM for a verified installer ISO.
    Installer {
        #[command(flatten)]
        instance: Instance,
        #[arg(long)]
        iso: PathBuf,
        #[arg(long, default_value_t = 96)]
        disk_gib: u32,
        /// Activate the signed media's read-only boot/signature readiness probe.
        #[arg(long)]
        probe: bool,
        /// Attach an owned marker disk and verify that installation leaves it unchanged.
        #[arg(long)]
        sentinel: bool,
    },
    /// Detach the copied ISO from a stopped installer VM.
    DetachInstaller(Instance),
    /// Open a retained GPU desktop; a new instance needs a prepared image.json.
    Up {
        #[command(flatten)]
        instance: Instance,
        #[arg(long)]
        image: Option<PathBuf>,
        #[arg(long, default_value_t = 4096)]
        memory_mib: u32,
        #[arg(long, default_value_t = 6)]
        cpus: u32,
        #[arg(long, default_value = "2560x1600@2")]
        display: String,
    },
    /// Read the actual QEMU running state.
    Status(Instance),
    /// Stop the VM, preserving its disk, firmware variables and TPM.
    Down {
        #[command(flatten)]
        instance: Instance,
        #[arg(long)]
        force: bool,
    },
    /// Delete one stopped lab instance and its writable disk/state.
    Remove(Instance),
    /// Capture actual guest pixels and their provenance.
    Shot {
        #[command(flatten)]
        instance: Instance,
        #[arg(default_value = "desktop")]
        label: String,
    },
    /// Execute an ordinary-user command in the native desktop session.
    Exec {
        #[command(flatten)]
        instance: Instance,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        argv: Vec<String>,
    },
    /// Save native VM logs and desktop state.
    Logs(Instance),
    /// Require the shared guest UEFI Secure Boot check's pass for the running boot.
    Check(Instance),
    /// Send virtual hardware keys using QEMU qcodes (e.g. meta_l ret).
    Key {
        #[command(flatten)]
        instance: Instance,
        #[arg(required = true)]
        keys: Vec<String>,
    },
    /// Validate and sync supported home configuration without rebuilding the VM.
    Sync {
        #[command(flatten)]
        instance: Instance,
        #[arg(long)]
        shot: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ToolsCommand {
    /// Read-only prerequisite and receipt checks; no GPU pass implied.
    Check {
        /// A runtime outside the shared cache.
        #[arg(long)]
        runtime: Option<PathBuf>,
    },
    /// Build locked native sources in a private prefix, or reuse the shared
    /// cache's runtime for the same inputs, and print its path. Prerequisites
    /// must be installed.
    Prepare {
        /// Build into this directory instead of the shared cache.
        #[arg(long)]
        runtime: Option<PathBuf>,
        #[arg(long, requires = "runtime")]
        workdir: Option<PathBuf>,
        #[arg(long, default_value_t = 8)]
        jobs: u32,
    },
}

fn script(name: &str) -> Command {
    let mut command = Command::new("uv");
    command
        .args(["run", "--script"])
        .arg(harness_dir().join("qemu").join(name))
        // The locked build tools' environment (`uv run --project build-tools`)
        // lives with the rest of the QEMU home, not in the source tree.
        .env(
            "UV_PROJECT_ENVIRONMENT",
            qemu_home().join("build/build-tools-venv"),
        );
    command
}

/// The native QEMU home: instances (`instances/`), shots, logs and the build
/// tools' environment. It is `target/kedra-lab/qemu` of this checkout (AGENTS.md
/// "Artifacts stay in the checkout"; `KEDRA_LAB_ARTIFACTS` moves it with the
/// other artifacts), or `KEDRA_QEMU_HOME` when set. The runtime and prepared
/// disks are entries of the shared cache ([`crate::shared_cache`]).
pub fn qemu_home() -> PathBuf {
    std::env::var_os("KEDRA_QEMU_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::artifact_root().join("qemu"))
}

/// Remove every stopped instance through `vm.py remove`, which refuses a
/// running VM and a directory without its receipt. Returns the removed names.
pub fn remove_instances() -> Result<Vec<String>> {
    let instances = qemu_home().join("instances");
    let entries = match std::fs::read_dir(&instances) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() && !name.starts_with('.') {
            names.push(name);
        }
    }
    names.sort();
    for name in &names {
        let instance = Instance {
            name: name.clone(),
            runtime: None,
        };
        if execute(&mut instance_script(&instance, "remove")?)? != ExitCode::SUCCESS {
            return Err(crate::invalid(format!(
                "native VM instance {name} was not removed; stop it with `kedra-lab vm down --name {name}`"
            )));
        }
    }
    Ok(names)
}

/// Homebrew libraries the runtime links and bundles (check-runtime.py's list).
const RUNTIME_LIBRARIES: [&str; 6] = [
    "glib-2.0",
    "pixman-1",
    "slirp",
    "json-glib-1.0",
    "openssl",
    "libtasn1",
];

/// Declared inputs of the native runtime, its shared cache key: the recipe and
/// pins prepare-runtime.py reads (patches included), the host architecture and
/// the toolchain whose output the runtime bundles (Xcode, the Metal toolchain
/// and the linked Homebrew libraries).
fn runtime_inputs() -> Result<serde_json::Value> {
    let qemu = harness_dir().join("qemu");
    let mut names: Vec<String> = [
        "inputs.json",
        "angle-dependencies.json",
        "prepare-runtime.py",
        "build-tools/pyproject.toml",
        "build-tools/uv.lock",
    ]
    .map(str::to_owned)
    .into();
    for entry in std::fs::read_dir(qemu.join("patches"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        names.push(format!("patches/{name}"));
    }
    let mut recipe = serde_json::Map::new();
    for name in names {
        let digest = crate::builder::file_sha256(&qemu.join(&name))?;
        recipe.insert(name, digest.into());
    }
    let mut libraries = serde_json::Map::new();
    for name in RUNTIME_LIBRARIES {
        libraries.insert(
            name.into(),
            probe("pkg-config", &["--modversion", name]).into(),
        );
    }
    let metal = probe("xcrun", &["metal", "--version"]);
    Ok(serde_json::json!({
        "recipe": recipe,
        "host": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH},
        "toolchain": {
            "xcode": probe("xcodebuild", &["-version"]),
            // Later lines name the user's installation directory.
            "metal": metal.lines().next().unwrap_or_default(),
            "libraries": libraries,
        },
    }))
}

/// Trimmed standard output of a toolchain probe, or `unavailable`.
fn probe(program: &str, args: &[&str]) -> String {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => "unavailable".to_owned(),
    }
}

const NO_RUNTIME: &str = "has no QEMU runtime for this checkout's inputs";

/// The runtime a command uses: an explicit `--runtime`, or the shared cache's
/// runtime for the current inputs. `verify` refuses a missing, tampered or
/// corrupt shared runtime and roots it for this checkout; without it the path
/// is only computed (operations on existing instances read theirs from state).
fn runtime(value: Option<&PathBuf>, verify: bool) -> Result<PathBuf> {
    if let Some(path) = value {
        return Ok(path.clone());
    }
    let cache = Cache::locate()?;
    let inputs = runtime_inputs()?;
    if !verify {
        return Ok(cache.content_path(Kind::QemuRuntime, &inputs));
    }
    match cache.find(Kind::QemuRuntime, &inputs)? {
        Some(entry) => Ok(entry.content()),
        None => Err(crate::invalid(format!(
            "the shared cache {} {NO_RUNTIME}; run `kedra-lab vm tools prepare`",
            cache.root().display()
        ))),
    }
}

/// Build the runtime into the shared cache, or reuse the entry another
/// checkout published for the same inputs. The build tree lives in the entry's
/// work area, so a failed build resumes, and is deleted once the runtime is
/// published.
fn prepare_shared_runtime(jobs: u32) -> Result<ExitCode> {
    let cache = Cache::locate()?;
    let inputs = runtime_inputs()?;
    let entry = cache.ensure(Kind::QemuRuntime, &inputs, |build| {
        let status = execute(
            script("prepare-runtime.py")
                .arg("--runtime")
                .arg(&build.content)
                .arg("--workdir")
                .arg(build.work.join("build"))
                .arg("--jobs")
                .arg(jobs.to_string()),
        )?;
        if status == ExitCode::SUCCESS {
            Ok(())
        } else {
            Err(crate::invalid(
                "prepare-runtime.py failed; run `kedra-lab vm tools prepare` again to resume its build",
            ))
        }
    })?;
    println!("{}", entry.content().display());
    Ok(ExitCode::SUCCESS)
}

fn instance_script(instance: &Instance, operation: &str) -> Result<Command> {
    // Only a new instance uses the given runtime; existing ones read theirs from state.
    let verify = matches!(operation, "up" | "installer");
    let mut command = script("vm.py");
    command
        .arg("--root")
        .arg(qemu_home())
        .arg("--runtime")
        .arg(runtime(instance.runtime.as_ref(), verify)?)
        .arg("--name")
        .arg(&instance.name)
        .arg(operation);
    Ok(command)
}

fn execute(command: &mut Command) -> Result<ExitCode> {
    crate::cancel::check()?;
    #[cfg(unix)]
    command.process_group(0);
    wait_child(&mut command.spawn()?)
}

fn wait_child(child: &mut Child) -> Result<ExitCode> {
    let mut interrupted = None;
    loop {
        if let Some(status) = child.try_wait()? {
            return if interrupted.is_some() {
                Err(crate::Error::Interrupted)
            } else {
                Ok(ExitCode::from(
                    u8::try_from(status.code().unwrap_or(1)).unwrap_or(1),
                ))
            };
        }
        if crate::cancel::requested() && interrupted.is_none() {
            // The unreaped child owns this new group; detached retained VMs are outside it.
            #[cfg(unix)]
            let _ = Command::new("/bin/kill")
                .args(["-TERM", "--", &format!("-{}", child.id())])
                .status();
            #[cfg(not(unix))]
            child.kill()?;
            interrupted = Some(Instant::now());
        }
        if interrupted.is_some_and(|start| start.elapsed() > Duration::from_secs(45)) {
            #[cfg(unix)]
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{}", child.id())])
                .status();
            child.kill()?;
            let _ = child.wait();
            return Err(crate::Error::Interrupted);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn run(command: VmCommand) -> Result<ExitCode> {
    match command {
        VmCommand::Tools {
            command: ToolsCommand::Check { runtime: path },
        } => {
            // A present shared runtime is verified against its cache receipt first.
            let checked = match path {
                Some(path) => path,
                None => match runtime(None, true) {
                    Err(crate::Error::Invalid(message)) if message.contains(NO_RUNTIME) => {
                        eprintln!("kedra-lab: {message}");
                        runtime(None, false)?
                    }
                    other => other?,
                },
            };
            execute(script("check-runtime.py").arg("--runtime").arg(checked))
        }
        VmCommand::Tools {
            command:
                ToolsCommand::Prepare {
                    runtime: None,
                    jobs,
                    ..
                },
        } => prepare_shared_runtime(jobs),
        VmCommand::Tools {
            command:
                ToolsCommand::Prepare {
                    runtime: Some(path),
                    workdir,
                    jobs,
                },
        } => {
            let workdir = match workdir {
                Some(path) => path,
                None => qemu_home().join("build"),
            };
            execute(
                script("prepare-runtime.py")
                    .arg("--runtime")
                    .arg(path)
                    .arg("--workdir")
                    .arg(workdir)
                    .arg("--jobs")
                    .arg(jobs.to_string()),
            )
        }
        VmCommand::Image {
            image,
            overlay,
            composition_identity,
            derivation_identity,
        } => {
            let source = image.parse()?;
            let overlay = overlay.parse()?;
            let composed = matches!(source, crate::image::Source::Composition(_));
            if composed != derivation_identity.is_some() {
                return Err(crate::invalid(
                    "VM composition requires the complete native identity tuple",
                ));
            }
            if composed
                && (overlay != crate::image::Overlay::None
                    || std::env::var_os("KEDRA_LAB_BINARIES").is_some_and(|v| !v.is_empty())
                    || std::env::var("KEDRA_LAB_TARGET")
                        .is_ok_and(|v| !v.is_empty() && v != "qemu-arm64"))
            {
                return Err(crate::invalid(
                    "derived VM requires qemu-arm64 without overlay or binary overrides",
                ));
            }
            if std::env::var("DOCKER_HOST").is_ok_and(|host| !host.starts_with("unix://")) {
                return Err(crate::invalid(
                    "native image preparation requires a local Unix Docker socket",
                ));
            }
            let docker = crate::docker::Docker::connect()?;
            let request = crate::image::Request {
                target: crate::image::target(&docker, Some("qemu-arm64"))?,
                source,
                overlay,
                binaries: None,
                composition_identity,
            };
            let (image, provenance) = match derivation_identity {
                Some(identity) => {
                    let (image, derived, reference) =
                        crate::image::prepare_derived_vm(&docker, &request, &identity)?;
                    (image, Some((derived, reference)))
                }
                None if request.composition_identity.is_none() => {
                    (crate::image::prepare_vm(&docker, &request)?, None)
                }
                _ => {
                    return Err(crate::invalid(
                        "native VM needs the complete identity tuple",
                    ));
                }
            };
            let reference = image.reference();
            let image_id = docker
                .image(&reference)?
                .ok_or_else(|| crate::invalid("native fixture disappeared"))?
                .0;
            let mut command = script("prepare-disk.py");
            command
                .arg("--image")
                .arg(reference)
                .arg("--image-id")
                .arg(image_id)
                .arg("--engine-id")
                .arg(docker.identity()?)
                .arg("--cache")
                .arg(qemu_home());
            if let Some((provenance, fixture_reference)) = provenance {
                command
                    .arg("--native-provenance")
                    .arg("--fixture-reference")
                    .arg(fixture_reference)
                    .stdin(Stdio::piped());
                #[cfg(unix)]
                command.process_group(0);
                let mut child = command.spawn()?;
                let data = serde_json::to_vec(&provenance)
                    .map_err(|error| crate::invalid(error.to_string()))?;
                let result = child
                    .stdin
                    .take()
                    .ok_or_else(|| crate::invalid("disk builder stdin is unavailable"))?
                    .write_all(&data);
                if let Err(error) = result {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.into());
                }
                wait_child(&mut child)
            } else {
                execute(&mut command)
            }
        }
        VmCommand::Iso { image, output } => {
            let mut command = Command::new("uv");
            command
                .args(["run", "--script"])
                .arg(crate::repository_root().join("usr/src/kedra/installer/macos/media.py"))
                .arg("build")
                .arg("--image")
                .arg(image)
                .arg("--output")
                .arg(output);
            execute(&mut command)
        }
        VmCommand::Installer {
            instance,
            iso,
            disk_gib,
            probe,
            sentinel,
        } => {
            let mut command = instance_script(&instance, "installer")?;
            command
                .arg("--iso")
                .arg(iso)
                .arg("--disk-gib")
                .arg(disk_gib.to_string());
            if probe {
                command.arg("--probe");
            }
            if sentinel {
                command.arg("--sentinel");
            }
            execute(&mut command)
        }
        VmCommand::DetachInstaller(instance) => {
            execute(&mut instance_script(&instance, "detach-installer")?)
        }
        VmCommand::Up {
            instance,
            image,
            memory_mib,
            cpus,
            display,
        } => {
            let mut command = instance_script(&instance, "up")?;
            command
                .arg("--memory-mib")
                .arg(memory_mib.to_string())
                .arg("--cpus")
                .arg(cpus.to_string())
                .arg("--display")
                .arg(display);
            if let Some(image) = image {
                command.arg("--image").arg(image);
            }
            execute(&mut command)
        }
        VmCommand::Status(instance) => execute(&mut instance_script(&instance, "status")?),
        VmCommand::Remove(instance) => execute(&mut instance_script(&instance, "remove")?),
        VmCommand::Logs(instance) => execute(&mut instance_script(&instance, "logs")?),
        VmCommand::Check(instance) => execute(&mut instance_script(&instance, "check")?),
        VmCommand::Key { instance, keys } => execute(instance_script(&instance, "key")?.args(keys)),
        VmCommand::Down { instance, force } => {
            let mut command = instance_script(&instance, "down")?;
            if force {
                command.arg("--force");
            }
            execute(&mut command)
        }
        VmCommand::Exec { instance, argv } => {
            execute(instance_script(&instance, "exec")?.arg("--").args(argv))
        }
        VmCommand::Shot { instance, label } => {
            execute(instance_script(&instance, "shot")?.arg(label))
        }
        VmCommand::Sync { instance, shot } => {
            let started = Instant::now();
            let source_started = Instant::now();
            let payload = crate::lab_sync::request("qemu-arm64")?;
            let source_ms = source_started.elapsed().as_millis();
            let mut command = instance_script(&instance, "sync")?;
            command.stdin(Stdio::piped());
            #[cfg(unix)]
            command.process_group(0);
            let transport_started = Instant::now();
            let mut child = command.spawn()?;
            if let Some(mut input) = child.stdin.take() {
                input.write_all(&payload)?;
            }
            let status = wait_child(&mut child)?;
            let transport_ms = transport_started.elapsed().as_millis();
            if status != ExitCode::SUCCESS {
                eprintln!(
                    "kedra-lab: native sync timing {}",
                    serde_json::json!({
                        "source_ms": source_ms,
                        "transport_ms": transport_ms,
                        "capture_ms": null,
                        "total_ms": started.elapsed().as_millis(),
                    })
                );
                return Ok(status);
            }
            let (status, capture_ms) = if let Some(label) = shot {
                let capture_started = Instant::now();
                let status = execute(instance_script(&instance, "shot")?.arg(label))?;
                (status, Some(capture_started.elapsed().as_millis()))
            } else {
                (ExitCode::SUCCESS, None)
            };
            eprintln!(
                "kedra-lab: native sync timing {}",
                serde_json::json!({
                    "source_ms": source_ms,
                    "transport_ms": transport_ms,
                    "capture_ms": capture_ms,
                    "total_ms": started.elapsed().as_millis(),
                })
            );
            Ok(status)
        }
    }
}
