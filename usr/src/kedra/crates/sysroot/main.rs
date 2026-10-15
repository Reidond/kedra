use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
mod agents;
#[cfg(unix)]
mod catalog;
#[cfg(unix)]
mod catalog_language;
#[cfg(unix)]
mod catalog_lock;
#[cfg(unix)]
mod catalog_process;
mod deployment;
#[cfg(unix)]
mod derive;
mod doctor;
#[cfg(unix)]
mod engine;
mod home;
mod setup;
mod source;
mod source_packages;
#[cfg(unix)]
mod system;

#[derive(Parser)]
#[command(
    version,
    about = "Kedra source planning and system management (in development)"
)]
struct Cli {
    #[cfg(unix)]
    #[arg(long, hide = true)]
    frontend_process: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[cfg(unix)]
    #[command(hide = true)]
    FrontendInput,
    /// Resolve reviewed package catalogs and author installed contributions.
    #[cfg(unix)]
    Catalog(catalog::Options),
    /// Plan or export declarative configuration over a retained Fedora foundation.
    #[cfg(unix)]
    System(system::Options),
    /// Derive the native OS image from a verified composition, offline.
    #[cfg(unix)]
    Derive(derive::Options),
    /// Build an independent declared package graph in a private Linux namespace.
    #[cfg(unix)]
    Build(engine::BuildOptions),
    /// Manage private immutable development objects, runtime closures and recovery.
    #[cfg(unix)]
    Store(engine::StoreOptions),
    /// Execute a package in its declared runtime foundation and closure.
    #[cfg(unix)]
    Run(engine::RunOptions),
    /// Select and retain development environment generations.
    #[cfg(unix)]
    Profile(engine::ProfileOptions),
    /// Execute a command in a profile's declared environment with private scratch.
    #[cfg(unix)]
    Develop(engine::DevelopOptions),
    /// Inspect installed desktop/session health without changing the machine.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Manage a signed installed release through the independently verifying helper.
    Update(deployment::Options),
    /// Show, capture and update the home files the image declares as managed.
    Home(home::Options),
    /// One-time setup of this installed machine, such as TPM disk unlock (Linux).
    Setup(setup::Options),
    /// Launch an official Codex runtime in the verified Kedra checkout (Linux).
    Codex(agents::Options),
    /// Launch an official Claude runtime in the verified Kedra checkout (Linux).
    Claude(agents::Options),
    /// Identify release signing material without establishing trust.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
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
    /// Identify a P-256 public key; this does not establish trust in its owner.
    Key {
        #[arg(long)]
        public_key: PathBuf,
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

fn run_release(command: ReleaseCommand) -> Result<(), Box<dyn std::error::Error>> {
    let ReleaseCommand::Key { public_key, json } = command;
    let bytes = limited_file(&public_key, 4096)?;
    let key = std::str::from_utf8(&bytes).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
    let fingerprint = sysroot_core::release::public_key_fingerprint(key)?;
    if json {
        println!(
            "{}",
            serde_json::json!({"algorithm":"ecdsa-p256-sha256","key_fingerprint_sha256":fingerprint,"trust_established":false})
        );
    } else {
        println!("P-256 public key SHA-256: {fingerprint}");
        println!("Compare this fingerprint through an independently trusted channel.");
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

fn source_archive(
    repo: PathBuf,
    host: String,
    output: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let plan = source::archive(&repo, &host, &output)?;
    println!(
        "Created {} from {} for {} ({} payload files).",
        output.display(),
        plan.source_revision,
        plan.target.id,
        plan.files.len()
    );
    Ok(())
}

fn run_source(command: SourceCommand) -> ExitCode {
    let result = match command {
        SourceCommand::Plan { repo, host, json } => source_plan(repo, host, json),
        SourceCommand::Archive { repo, host, output } => source_archive(repo, host, output),
    };
    exit_status(result, ExitCode::FAILURE)
}

fn exit_status<E: std::fmt::Display>(result: Result<(), E>, failure: ExitCode) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sysroot: {error}");
            failure
        }
    }
}

#[cfg(unix)]
fn engine_exit_status(result: sysroot_engine::Result<ExitCode>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("sysroot: {error}");
            engine::failure_code(&error)
        }
    }
}

#[cfg(unix)]
fn run_engine(command: engine::Command) -> ExitCode {
    engine_exit_status(engine::run(command))
}

// Some(code) ends the run before dispatch: the frontend worker was refused or
// could not apply its limits, or catalog planning ran in an isolated worker.
#[cfg(unix)]
fn isolated_frontend_exit(cli: &Cli) -> Option<ExitCode> {
    if cli.frontend_process {
        return confine_frontend_worker(cli.command.as_ref());
    }
    if let Some(Commands::Catalog(options)) = &cli.command
        && options.planning()
    {
        return Some(engine_exit_status(catalog_process::planning()));
    }
    None
}

#[cfg(unix)]
fn confine_frontend_worker(command: Option<&Commands>) -> Option<ExitCode> {
    let data_only = match command {
        Some(Commands::Catalog(options)) => options.planning(),
        Some(Commands::FrontendInput) => true,
        _ => false,
    };
    if !data_only {
        eprintln!("sysroot: frontend worker accepts only data-only commands");
        return Some(ExitCode::from(78));
    }
    if let Err(error) = catalog_process::worker_limits() {
        eprintln!("sysroot: {error}");
        return Some(ExitCode::FAILURE);
    }
    None
}

fn run_doctor(json: bool) -> ExitCode {
    match doctor::run(json) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("sysroot: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() -> ExitCode {
    if let Err(error) = Cli::command().print_help() {
        eprintln!("sysroot: {error}");
        return ExitCode::FAILURE;
    }
    println!();
    ExitCode::SUCCESS
}

fn reject_unavailable(args: &[OsString]) -> ExitCode {
    let command = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if let Some(gates) = sysroot_core::research_gate(command) {
        eprintln!("sysroot: {command} is not implemented; complete research {gates}.");
        return ExitCode::from(78);
    }
    eprintln!("sysroot: unsupported command or arguments; run sysroot --help");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    #[cfg(unix)]
    if let Some(code) = isolated_frontend_exit(&cli) {
        return code;
    }
    match cli.command {
        #[cfg(unix)]
        Some(Commands::FrontendInput) => exit_status(catalog_language::worker(), ExitCode::FAILURE),
        #[cfg(unix)]
        Some(Commands::Catalog(options)) => engine_exit_status(catalog::run(options)),
        #[cfg(unix)]
        Some(Commands::Derive(options)) => {
            engine_exit_status(derive::run(options).map(|()| ExitCode::SUCCESS))
        }
        #[cfg(unix)]
        Some(Commands::System(options)) => {
            engine_exit_status(system::run(options).map(|()| ExitCode::SUCCESS))
        }
        #[cfg(unix)]
        Some(Commands::Build(options)) => run_engine(engine::Command::Build(options)),
        #[cfg(unix)]
        Some(Commands::Store(options)) => run_engine(engine::Command::Store(options)),
        #[cfg(unix)]
        Some(Commands::Run(options)) => run_engine(engine::Command::Run(options)),
        #[cfg(unix)]
        Some(Commands::Profile(options)) => run_engine(engine::Command::Profile(options)),
        #[cfg(unix)]
        Some(Commands::Develop(options)) => run_engine(engine::Command::Develop(options)),
        Some(Commands::Doctor { json }) => run_doctor(json),
        Some(Commands::Home(options)) => exit_status(home::run(options), ExitCode::FAILURE),
        Some(Commands::Update(options)) => {
            exit_status(deployment::run(options), ExitCode::from(78))
        }
        Some(Commands::Setup(options)) => exit_status(setup::run(options), ExitCode::from(78)),
        Some(Commands::Codex(options)) => {
            exit_status(agents::run("codex", options), ExitCode::from(78))
        }
        Some(Commands::Claude(options)) => {
            exit_status(agents::run("claude", options), ExitCode::from(78))
        }
        Some(Commands::Release { command }) => exit_status(run_release(command), ExitCode::FAILURE),
        None => print_help(),
        Some(Commands::Source { command }) => run_source(command),
        Some(Commands::Unavailable(args)) => reject_unavailable(&args),
    }
}
