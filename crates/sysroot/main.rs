use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

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
enum SourceCommand {
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
    let plan = sysroot_core::source::plan(&repo, &host)?;
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
                    "Kedra: source planning available; OS deployment and live-home management unavailable."
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
