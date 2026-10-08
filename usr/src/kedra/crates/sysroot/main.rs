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
mod catalog_process;
mod deployment;
mod doctor;
#[cfg(unix)]
mod engine;
mod home;
#[cfg(unix)]
mod home_artifact;
mod installer_artifact;
mod release_channel;
mod release_history;
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
    /// Review, select and reconcile supported Noctalia settings and niri text.
    Home(home::Options),
    /// One-time setup of this installed machine, such as TPM disk unlock (Linux).
    Setup(setup::Options),
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
    /// Authenticate predecessor ordering, including expired metadata; never authorize an update.
    History(release_history::Options),
    /// Verify a downloaded channel bundle and unpack exact public update files.
    Unpack(release_channel::Options),
    /// Check signed channel freshness/replay state without enrolling or staging.
    Channel {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        signature: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        checkpoint_signature: PathBuf,
        #[arg(long)]
        public_key: PathBuf,
        #[arg(long)]
        target: String,
        /// Independently expected image repository, without a tag or digest.
        #[arg(long)]
        repository: String,
        /// Previously retained state; caller input never becomes machine authority.
        #[arg(long)]
        previous_state: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Identify a P-256 public key; this does not establish trust in its owner.
    Key {
        #[arg(long)]
        public_key: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Assemble downloaded parts in the given order and verify the complete ISO.
    Assemble {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        signature: PathBuf,
        #[arg(long)]
        public_key: PathBuf,
        /// Existing output directory; the signed installer filename is used.
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long)]
        target: String,
        #[arg(long)]
        json: bool,
        /// Downloaded parts in release order (never expanded or sorted internally).
        #[arg(required = true, num_args = 1..=64)]
        parts: Vec<PathBuf>,
    },
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
    if let ReleaseCommand::History(options) = command {
        return release_history::run(options);
    }
    if let ReleaseCommand::Unpack(options) = command {
        return release_channel::unpack(options);
    }
    if let ReleaseCommand::Channel {
        manifest,
        signature,
        checkpoint,
        checkpoint_signature,
        public_key,
        target,
        repository,
        previous_state,
        json,
    } = command
    {
        use sysroot_core::release::{MAX_DOCUMENT, Scope, TrustState};
        // Protocol-1 release files exist only for desktop x86_64 (Scope::legacy).
        let scope = Scope {
            target,
            architecture: "x86_64".into(),
            fedora_release: 44,
            repository,
        };
        let payload = limited_file(&manifest, MAX_DOCUMENT)?;
        let signature = limited_file(&signature, 1024)?;
        let key = limited_file(&public_key, 4096)?;
        let key =
            std::str::from_utf8(&key).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
        let verified =
            sysroot_core::release::verify_release(&payload, &signature, key, Some(&scope))?;
        let payload = limited_file(&checkpoint, MAX_DOCUMENT)?;
        let signature = limited_file(&checkpoint_signature, 1024)?;
        let previous: Option<TrustState> = previous_state
            .map(|path| -> Result<TrustState, Box<dyn std::error::Error>> {
                Ok(serde_json::from_slice(&limited_file(&path, MAX_DOCUMENT)?)?)
            })
            .transpose()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let update = sysroot_core::release::verify_update(
            verified,
            &payload,
            &signature,
            key,
            &scope,
            previous.as_ref(),
            now,
        )?;
        if json {
            println!(
                "{}",
                serde_json::json!({"signature_valid":true,"channel_freshness_verified":true,
                    "replay_checked":previous.is_some(),"verified_at":now,
                    "release_sha256":update.release.sha256(),
                    "key_fingerprint_sha256":update.release.key_fingerprint(),
                    "image_reference":update.release.release().image_reference(),
                    "next_trust_state":update.next_trust_state,"deployment_authorized":false})
            );
        } else {
            println!(
                "Fresh signed channel: {}",
                update.release.release().image_reference()
            );
            println!(
                "Checkpoint generation: {}",
                update.next_trust_state.generation
            );
            if previous.is_none() {
                println!("No previous state supplied; earlier accepted history was not checked.");
            }
            println!(
                "Read-only verification; the installed helper independently authorizes staging."
            );
        }
        return Ok(());
    }
    if let ReleaseCommand::Key { public_key, json } = command {
        let bytes = limited_file(&public_key, 4096)?;
        let key =
            std::str::from_utf8(&bytes).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
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
        return Ok(());
    }
    if let ReleaseCommand::Assemble {
        manifest,
        signature,
        public_key,
        output_dir,
        target,
        json,
        parts,
    } = command
    {
        let payload = limited_file(&manifest, sysroot_core::release::MAX_DOCUMENT)?;
        let signature = limited_file(&signature, 1024)?;
        let key = limited_file(&public_key, 4096)?;
        let key =
            std::str::from_utf8(&key).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
        let verified = sysroot_core::release::verify_release(&payload, &signature, key, None)?;
        if target != verified.release().scope.target {
            return Err(sysroot_core::release::Error::ScopeMismatch.into());
        }
        let output = installer_artifact::assemble(&verified, &output_dir, &parts)?;
        if json {
            println!(
                "{}",
                serde_json::json!({"signature_valid":true,"artifact_verified":true,
                "output":output,"release_sha256":verified.sha256(),
                "key_fingerprint_sha256":verified.key_fingerprint(),
                "channel_freshness_verified":false,"deployment_authorized":false})
            );
        } else {
            println!("Verified installer: {}", output.display());
            println!("Signing key SHA-256: {}", verified.key_fingerprint());
        }
        return Ok(());
    }
    let ReleaseCommand::Verify {
        manifest,
        signature,
        public_key,
        artifact,
        target,
        json,
    } = command
    else {
        return Err("unsupported release operation".into());
    };
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

fn print_status(json: bool) {
    if json {
        println!("{}", sysroot_core::STATUS_JSON);
    } else {
        println!(
            "Kedra capabilities: the private Unix build/store/profile engine, source and release tools, Linux deployment, Noctalia/niri home workflows and TPM disk unlock setup are implemented. Engine execution requires native aarch64 Linux Docker and retained image evidence. Installed state is not checked here. Use sysroot update status, sysroot update status --home, sysroot doctor and sysroot setup tpm-unlock --dry-run for installed checks."
        );
    }
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
        Some(Commands::Release { command }) => {
            exit_status(verify_release(command), ExitCode::FAILURE)
        }
        None => print_help(),
        Some(Commands::Status { json }) => {
            print_status(json);
            ExitCode::SUCCESS
        }
        Some(Commands::Source { command }) => run_source(command),
        Some(Commands::Unavailable(args)) => reject_unavailable(&args),
    }
}
