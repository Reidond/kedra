//! Native QEMU preparation and retained desktop operations.

use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use clap::{Args, Subcommand};
use sha2::{Digest, Sha256};

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
    },
    /// Build signed ARM64 installer media through the existing verification policy.
    Iso {
        #[arg(long)]
        image: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        base_image: Option<String>,
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
        #[arg(long)]
        runtime: Option<PathBuf>,
    },
    /// Build locked native sources in a private prefix. Prerequisites must be installed.
    Prepare {
        #[arg(long)]
        runtime: Option<PathBuf>,
        #[arg(long)]
        workdir: Option<PathBuf>,
        #[arg(long, default_value_t = 8)]
        jobs: u32,
    },
}

fn script(name: &str) -> Command {
    let mut command = Command::new("uv");
    command
        .args(["run", "--script"])
        .arg(harness_dir().join("qemu").join(name));
    command
}

fn configured_qemu_home() -> Option<PathBuf> {
    std::env::var_os("KEDRA_QEMU_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn legacy_qemu_home() -> Option<PathBuf> {
    std::env::var_os("KEDRA_LAB_ARTIFACTS")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .map(|path| path.join("qemu"))
}

fn user_home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| crate::invalid("HOME must name an absolute directory for native QEMU"))
}

fn checkout_key() -> Result<String> {
    let checkout = crate::repository_root().canonicalize()?;
    let digest = Sha256::digest(checkout.as_os_str().as_encoded_bytes());
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3]
    ))
}

fn qemu_cache_home() -> Result<PathBuf> {
    if let Some(path) = configured_qemu_home().or_else(legacy_qemu_home) {
        return Ok(path);
    }
    Ok(user_home()?.join("Library/Caches/kedra/qemu"))
}

fn qemu_state_home() -> Result<PathBuf> {
    if let Some(path) = configured_qemu_home().or_else(legacy_qemu_home) {
        return Ok(path);
    }
    Ok(user_home()?
        .join(".local/share/kedra/lab")
        .join(checkout_key()?))
}

fn runtime(value: Option<&PathBuf>) -> Result<PathBuf> {
    match value {
        Some(path) => Ok(path.clone()),
        None => Ok(qemu_cache_home()?.join("runtime")),
    }
}

fn instance_script(instance: &Instance, operation: &str) -> Result<Command> {
    let mut command = script("vm.py");
    command
        .arg("--root")
        .arg(qemu_state_home()?)
        .arg("--runtime")
        .arg(runtime(instance.runtime.as_ref())?)
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
        } => execute(
            script("check-runtime.py")
                .arg("--runtime")
                .arg(runtime(path.as_ref())?),
        ),
        VmCommand::Tools {
            command:
                ToolsCommand::Prepare {
                    runtime: path,
                    workdir,
                    jobs,
                },
        } => {
            let workdir = match workdir {
                Some(path) => path,
                None => qemu_cache_home()?.join("build"),
            };
            execute(
                script("prepare-runtime.py")
                    .arg("--runtime")
                    .arg(runtime(path.as_ref())?)
                    .arg("--workdir")
                    .arg(workdir)
                    .arg("--jobs")
                    .arg(jobs.to_string()),
            )
        }
        VmCommand::Image { image, overlay } => {
            if std::env::var("DOCKER_HOST").is_ok_and(|host| !host.starts_with("unix://")) {
                return Err(crate::invalid(
                    "native image preparation requires a local Unix Docker socket",
                ));
            }
            let docker = crate::docker::Docker::connect()?;
            let request = crate::image::Request {
                target: crate::image::target(&docker, Some("qemu-arm64"))?,
                source: image.parse()?,
                overlay: overlay.parse()?,
                binaries: None,
            };
            let image = crate::image::prepare_vm(&docker, &request)?;
            let reference = image.reference();
            let image_id = docker
                .image(&reference)?
                .ok_or_else(|| crate::invalid("native fixture disappeared"))?
                .0;
            execute(
                script("prepare-disk.py")
                    .arg("--image")
                    .arg(reference)
                    .arg("--image-id")
                    .arg(image_id)
                    .arg("--engine-id")
                    .arg(docker.identity()?)
                    .arg("--cache")
                    .arg(qemu_cache_home()?),
            )
        }
        VmCommand::Iso {
            image,
            output,
            base_image,
        } => {
            let mut command = Command::new("uv");
            command
                .args(["run", "--script"])
                .arg(crate::repository_root().join("usr/src/kedra/installer/macos/media.py"))
                .arg("build")
                .arg("--image")
                .arg(image)
                .arg("--output")
                .arg(output);
            if let Some(base_image) = base_image {
                command.arg("--base-image").arg(base_image);
            }
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
