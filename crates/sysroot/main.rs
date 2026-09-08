use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
mod agents;
mod home;
mod source;

#[derive(Parser)]
#[command(
    version,
    about = "Kedra source planning and system management (in development)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Review the supported Noctalia settings without writing live configuration.
    Home(home::Options),
    /// Launch an official Codex runtime in the verified Kedra checkout (Linux).
    Codex(agents::Options),
    /// Launch an official Claude runtime in the verified Kedra checkout (Linux).
    Claude(agents::Options),
    /// Verify a signed release record and, optionally, a downloaded installer.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    /// Show capabilities; does not claim the OS is installed.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Inspect committed inputs without modifying the checkout or machine.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    #[command(external_subcommand)]
    Unavailable(Vec<OsString>),
}

#[derive(Subcommand)]
enum ReleaseCommand {
    /// Verify exact signed bytes using an independently trusted public key.
    Verify {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        signature: PathBuf,
        #[arg(long)]
        public_key: PathBuf,
        #[arg(long)]
        artifact: Option<PathBuf>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

fn limited_file(
    path: &std::path::Path,
    limit: usize,
) -> Result<Vec<u8>, sysroot_core::release::Error> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(sysroot_core::release::Error::SizeLimit);
    }
    Ok(bytes)
}

fn verify_release(command: ReleaseCommand) -> Result<(), Box<dyn std::error::Error>> {
    let ReleaseCommand::Verify {
        manifest,
        signature,
        public_key,
        artifact,
        target,
        json,
    } = command;
    let payload = limited_file(&manifest, sysroot_core::release::MAX_DOCUMENT)?;
    let signature = limited_file(&signature, 1024)?;
    let key = limited_file(&public_key, 4096)?;
    let key = std::str::from_utf8(&key).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
    let verified = sysroot_core::release::verify_release(&payload, &signature, key, None)?;
    if target
        .as_ref()
        .is_some_and(|target| *target != verified.release().scope.target)
    {
        return Err(sysroot_core::release::Error::ScopeMismatch.into());
    }
    if let Some(path) = &artifact {
        sysroot_core::release::verify_artifact(&verified, std::fs::File::open(path)?)?;
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "signature_valid": true, "artifact_verified": artifact.is_some(),
                "channel_freshness_verified": false, "deployment_authorized": false,
                "key_fingerprint_sha256": verified.key_fingerprint(),
                "release_sha256": verified.sha256(), "release": verified.release()
            }))?
        );
    } else {
        println!(
            "Verified promoted {} release {}.",
            verified.release().scope.target,
            verified.release().sequence
        );
        println!("Signing key SHA-256: {}", verified.key_fingerprint());
        if let Some(path) = artifact {
            println!("Installer checksum and size verified: {}", path.display());
        }
        println!("Channel freshness and installed-machine authorization are separate checks.");
    }
    Ok(())
}

#[derive(Subcommand)]
enum SourceCommand {
    /// Write a new deterministic tar of committed payloads for an Actions build.
    Archive {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        host: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Resolve packages, payload hashes and host overrides from committed HEAD.
    Plan {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        host: String,
        #[arg(long)]
        json: bool,
    },
}

fn source_plan(repo: PathBuf, host: String, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let plan = source::plan(&repo, &host)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!(
            "Target: {} ({}, Fedora {})",
            plan.target.id, plan.target.architecture, plan.target.fedora_release
        );
        println!("Source: {} ({})", plan.source_revision, plan.input_scope);
        println!(
            "Hardware: {}. This plan is not a tested image or deployment approval.",
            plan.target.hardware_status
        );
        println!("Install packages: {}", plan.packages.join(", "));
        println!("Remove packages: {}", plan.remove_packages.join(", "));
        for file in plan.files {
            println!(
                "{} <- {} [{}]",
                file.destination, file.source_path, file.mode
            );
            if let Some(replaced) = file.replaces {
                println!("  replaces {replaced}");
            }
        }
        println!("Uncommitted edits are excluded and remain untouched.");
    }
    Ok(())
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Some(Commands::Home(options)) => {
            if let Err(error) = home::run(options) {
                eprintln!("sysroot: {error}");
                return ExitCode::FAILURE;
            }
        }
        Some(Commands::Codex(options)) => {
            if let Err(error) = agents::run("codex", options) {
                eprintln!("sysroot: {error}");
                return ExitCode::from(78);
            }
        }
        Some(Commands::Claude(options)) => {
            if let Err(error) = agents::run("claude", options) {
                eprintln!("sysroot: {error}");
                return ExitCode::from(78);
            }
        }
        Some(Commands::Release { command }) => {
            if let Err(error) = verify_release(command) {
                eprintln!("sysroot: {error}");
                return ExitCode::FAILURE;
            }
        }
        None => {
            if let Err(error) = Cli::command().print_help() {
                eprintln!("sysroot: {error}");
                return ExitCode::FAILURE;
            }
            println!();
        }
        Some(Commands::Status { json }) => {
            if json {
                println!("{}", sysroot_core::STATUS_JSON);
            } else {
                println!(
                    "Kedra: source planning, archives and release verification available; OS deployment and live-home management unavailable."
                );
            }
        }
        Some(Commands::Source {
            command: SourceCommand::Plan { repo, host, json },
        }) => {
            if let Err(error) = source_plan(repo, host, json) {
                eprintln!("sysroot: {error}");
                return ExitCode::FAILURE;
            }
        }
        Some(Commands::Source {
            command: SourceCommand::Archive { repo, host, output },
        }) => match source::archive(&repo, &host, &output) {
            Ok(plan) => println!(
                "Created {} from {} for {} ({} payload files).",
                output.display(),
                plan.source_revision,
                plan.target.id,
                plan.files.len()
            ),
            Err(error) => {
                eprintln!("sysroot: {error}");
                return ExitCode::FAILURE;
            }
        },
        Some(Commands::Unavailable(args)) => {
            let command = args
                .first()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if let Some(gates) = sysroot_core::research_gate(command) {
                eprintln!("sysroot: {command} is not implemented; complete research {gates}.");
                return ExitCode::from(78);
            }
            eprintln!("sysroot: unsupported command or arguments; run sysroot --help");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
