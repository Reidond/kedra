use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use serde::Serialize;
use sysroot_engine::{Error, Result, RunResult, Store};

#[derive(Args)]
pub struct BuildOptions {
    #[arg(long)]
    store: PathBuf,
    /// Serialized graph produced by the Rust authoring API.
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    root: String,
    /// Resolve identities without opening the store or invoking Docker.
    #[arg(long, conflicts_with = "rebuild")]
    dry_run: bool,
    /// Independently rebuild every required node and compare its existing winner.
    #[arg(long)]
    rebuild: bool,
}

#[derive(Args)]
pub struct StoreOptions {
    #[command(subcommand)]
    command: StoreCommand,
}

#[derive(Subcommand)]
enum StoreCommand {
    /// Initialize a private engine store; never an installed OS store.
    Init(StorePath),
    /// Snapshot an explicitly selected source directory and pin its object.
    AddSource {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        source: PathBuf,
    },
    /// Retain an exact existing Docker image, including its archive evidence.
    AddImage {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        image: String,
    },
    /// Verify an object's immutable bytes and metadata before use.
    Verify(ObjectOptions),
    /// List the complete runtime object and image closure.
    Closure(ObjectOptions),
    /// Export a closure with its runtime images and return the bundle SHA-256.
    Export {
        #[command(flatten)]
        object: ObjectOptions,
        #[arg(long)]
        output: PathBuf,
    },
    /// Admit an independently selected bundle into this private store.
    Import {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        expected_sha256: String,
    },
    /// Retain an object independently of profiles.
    Pin(ObjectOptions),
    /// Remove one explicit object root; referenced/profile objects remain retained.
    Unpin(ObjectOptions),
    /// Remove one explicit image root; runtime references remain retained.
    UnpinImage {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        image: String,
    },
    /// Inspect unreferenced owned objects; deletion requires --delete.
    Gc {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        delete: bool,
    },
    /// Reconcile interrupted owned operations after verifying their authority.
    Recover(StorePath),
}

#[derive(Args)]
struct StorePath {
    #[arg(long)]
    store: PathBuf,
}

#[derive(Args)]
struct ObjectOptions {
    #[command(flatten)]
    path: StorePath,
    #[arg(long)]
    object: String,
}

#[derive(Args)]
pub struct RunOptions {
    #[command(flatten)]
    object: ObjectOptions,
    /// Program relative to the selected output object.
    #[arg(long)]
    program: String,
    #[arg(long, default_value_t = 60)]
    timeout: u64,
    #[arg(last = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Args)]
pub struct ProfileOptions {
    #[command(subcommand)]
    command: ProfileCommand,
}

#[derive(Args)]
struct ProfilePath {
    #[command(flatten)]
    path: StorePath,
    #[arg(long)]
    name: String,
}

#[derive(Subcommand)]
enum ProfileCommand {
    /// Select a validated output and retain its immutable generation.
    Switch {
        #[command(flatten)]
        profile: ProfilePath,
        #[arg(long)]
        object: String,
        #[arg(long)]
        program: String,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show selected and retained generations.
    List(ProfilePath),
    /// Run the selected generation with additional arguments.
    Run {
        #[command(flatten)]
        profile: ProfilePath,
        #[arg(long, default_value_t = 60)]
        timeout: u64,
        #[arg(last = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Select the previous retained generation; never roll back OS/home/data.
    Rollback(ProfilePath),
}

#[derive(Args)]
pub struct DevelopOptions {
    #[command(flatten)]
    profile: ProfilePath,
    /// Absolute program from the declared runtime image; default is profile program.
    #[arg(long)]
    program: Option<String>,
    #[arg(long, default_value_t = 60)]
    timeout: u64,
    #[arg(last = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

pub enum Command {
    Build(BuildOptions),
    Store(StoreOptions),
    Run(RunOptions),
    Profile(ProfileOptions),
    Develop(DevelopOptions),
}

fn json(value: &impl Serialize) -> Result<ExitCode> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(ExitCode::SUCCESS)
}

fn emit_run(result: RunResult) -> Result<ExitCode> {
    std::io::stdout().lock().write_all(&result.stdout)?;
    std::io::stderr().lock().write_all(&result.stderr)?;
    if result.stdout_truncated || result.stderr_truncated {
        eprintln!("sysroot: runtime output exceeded the engine capture limit");
        return Ok(ExitCode::from(74));
    }
    Ok(ExitCode::from(u8::try_from(result.code).unwrap_or(1)))
}

fn store_command(command: StoreCommand) -> Result<ExitCode> {
    match command {
        StoreCommand::Init(path) => {
            let store = Store::create(&path.store)?;
            json(&serde_json::json!({
                "schema": 1, "store": store.path(),
                "logical_prefix": sysroot_engine::LOGICAL_PREFIX
            }))
        }
        StoreCommand::AddSource { path, source } => {
            json(&Store::open(&path.store)?.import_source(&source)?)
        }
        StoreCommand::AddImage { path, image } => {
            json(&Store::open(&path.store)?.import_image(&image)?)
        }
        StoreCommand::Verify(options) => {
            json(&Store::open(&options.path.store)?.verify(&options.object)?)
        }
        StoreCommand::Closure(options) => {
            json(&Store::open(&options.path.store)?.closure(&options.object)?)
        }
        StoreCommand::Export { object, output } => {
            json(&Store::open(&object.path.store)?.export(&object.object, &output)?)
        }
        StoreCommand::Import {
            path,
            bundle,
            expected_sha256,
        } => json(&Store::open(&path.store)?.import(&bundle, &expected_sha256)?),
        StoreCommand::Pin(options) => {
            Store::open(&options.path.store)?.pin(&options.object)?;
            json(&serde_json::json!({"pinned": options.object}))
        }
        StoreCommand::Unpin(options) => {
            Store::open(&options.path.store)?.unpin(&options.object)?;
            json(&serde_json::json!({"unpinned": options.object}))
        }
        StoreCommand::UnpinImage { path, image } => {
            Store::open(&path.store)?.unpin_image(&image)?;
            json(&serde_json::json!({"unpinned_image": image}))
        }
        StoreCommand::Gc { path, delete } => json(&Store::open(&path.store)?.gc(delete)?),
        StoreCommand::Recover(path) => json(&Store::recover(&path.store)?),
    }
}

fn profile_command(command: ProfileCommand) -> Result<ExitCode> {
    match command {
        ProfileCommand::Switch {
            profile,
            object,
            program,
            args,
        } => json(&Store::open(&profile.path.store)?.profile_switch(
            &profile.name,
            &object,
            &program,
            &args,
        )?),
        ProfileCommand::List(profile) => {
            json(&Store::open(&profile.path.store)?.profile_list(&profile.name)?)
        }
        ProfileCommand::Rollback(profile) => {
            json(&Store::open(&profile.path.store)?.profile_rollback(&profile.name)?)
        }
        ProfileCommand::Run {
            profile,
            timeout,
            args,
        } => emit_run(Store::open(&profile.path.store)?.profile_run(
            &profile.name,
            &args,
            timeout,
        )?),
    }
}

pub fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Build(options) => {
            let graph = sysroot_engine::read_graph(&options.plan)?;
            if options.dry_run {
                return json(&sysroot_engine::plan(&graph, &options.root)?);
            }
            json(&Store::open(&options.store)?.build(&graph, &options.root, options.rebuild)?)
        }
        Command::Store(options) => store_command(options.command),
        Command::Run(options) => emit_run(Store::open(&options.object.path.store)?.run_output(
            &options.object.object,
            &options.program,
            &options.args,
            options.timeout,
        )?),
        Command::Profile(options) => profile_command(options.command),
        Command::Develop(options) => emit_run(Store::open(&options.profile.path.store)?.develop(
            &options.profile.name,
            options.program.as_deref(),
            &options.args,
            options.timeout,
        )?),
    }
}

pub fn failure_code(error: &Error) -> ExitCode {
    match error {
        Error::Process { code, .. } => ExitCode::from(
            u8::try_from(*code)
                .ok()
                .filter(|code| *code != 0)
                .unwrap_or(1),
        ),
        Error::Invalid(_)
        | Error::Corrupt(_)
        | Error::RecoveryRequired(_)
        | Error::Divergent(_) => ExitCode::from(78),
        Error::Io(_) | Error::Json(_) => ExitCode::FAILURE,
    }
}
