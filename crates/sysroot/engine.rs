use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Subcommand};
use serde::Serialize;
use sysroot_engine::{BinaryCache, Error, Result, RunResult, Store, Substitution, TrustedRecords};

#[derive(Args)]
pub struct BuildOptions {
    #[arg(long)]
    store: PathBuf,
    /// Serialized graph produced by the Rust authoring API.
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    root: String,
    /// Resolve identities without opening the store or running a sandbox.
    #[arg(long, conflicts_with = "rebuild")]
    dry_run: bool,
    /// Independently rebuild every required node and compare its existing winner.
    #[arg(long)]
    rebuild: bool,
    #[command(flatten)]
    cache: SubstituteArgs,
}

/// An OCI repository of realized closures. It is never trusted: records decide.
#[derive(Args)]
pub(crate) struct CacheArgs {
    /// Repository such as ghcr.io/reidond/kedra-cache.
    #[arg(long = "cache", value_name = "REPOSITORY")]
    repository: Option<String>,
    /// Registry credentials (skopeo/containers auth.json).
    #[arg(long, requires = "repository")]
    cache_authfile: Option<PathBuf>,
    /// Plain HTTP for a local test registry; credentials are refused.
    #[arg(long, requires = "repository")]
    cache_plain_http: bool,
}

impl CacheArgs {
    fn open(&self) -> Result<Option<BinaryCache>> {
        self.repository
            .as_deref()
            .map(|repository| {
                BinaryCache::new(
                    repository,
                    self.cache_authfile.clone(),
                    self.cache_plain_http,
                )
            })
            .transpose()
    }
}

/// Substitute outputs from a binary cache, admitting only closures that equal
/// the receipts of a signature-verified image's installed composition receipt.
#[derive(Args)]
pub(crate) struct SubstituteArgs {
    #[command(flatten)]
    cache: CacheArgs,
    /// `/usr/share/sysroot/composition.json` of a verified image (or a copy).
    #[arg(long, requires = "repository")]
    trusted: Option<PathBuf>,
}

impl SubstituteArgs {
    pub(crate) fn open(&self) -> Result<Option<(BinaryCache, TrustedRecords)>> {
        match (self.cache.open()?, &self.trusted) {
            (Some(cache), Some(trusted)) => Ok(Some((cache, TrustedRecords::read(trusted)?))),
            (None, None) => Ok(None),
            _ => Err(Error::Invalid(
                "substitution needs both --cache and --trusted".into(),
            )),
        }
    }
}

/// Build through `Store::build_with`, substituting when `cache` is given.
pub(crate) fn build_graph(
    store: &Store,
    graph: &sysroot_engine::BuildGraph,
    root: &str,
    rebuild: bool,
    cache: &SubstituteArgs,
) -> Result<sysroot_engine::BuildResult> {
    let opened = cache.open()?;
    let substitution = opened
        .as_ref()
        .map(|(cache, records)| Substitution { cache, records });
    store.build_with(graph, root, rebuild, substitution)
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
    /// Retain an exact image with its archive: from the local Docker daemon, or
    /// without a daemon from an archive `docker image save` wrote.
    AddImage {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        image: String,
        #[arg(long)]
        archive: Option<PathBuf>,
    },
    /// Create or reuse and pin the root filesystem of a retained image. With
    /// an RPM lock, its foundation role is installed offline over the image (a
    /// runtime root) and, with a builder role, that role over the result.
    AddRoot {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        image: String,
        /// RPM lock whose role objects the store already holds; `--image` is
        /// its Fedora base.
        #[arg(long)]
        lock: Option<PathBuf>,
        #[arg(long, requires = "lock")]
        builder: Option<String>,
    },
    /// Remove one explicit root filesystem pin.
    UnpinRoot {
        #[command(flatten)]
        path: StorePath,
        #[arg(long)]
        root: String,
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
    /// Push an object's verified closure to a binary cache under its receipt tag.
    Push {
        #[command(flatten)]
        object: ObjectOptions,
        #[command(flatten)]
        cache: CacheArgs,
        /// Source repository URL to annotate (GHCR links the package by it).
        #[arg(long)]
        source: Option<String>,
    },
    /// Admit an object's closure from a binary cache, only if it equals trusted records.
    Substitute {
        #[command(flatten)]
        object: ObjectOptions,
        #[command(flatten)]
        cache: SubstituteArgs,
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
    /// Absolute program from the runtime image's root; default is profile program.
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
        StoreCommand::AddImage {
            path,
            image,
            archive: None,
        } => json(&Store::open(&path.store)?.import_image(&image)?),
        StoreCommand::AddImage {
            path,
            image,
            archive: Some(archive),
        } => json(&Store::open(&path.store)?.import_image_archive(&image, &archive)?),
        StoreCommand::AddRoot {
            path,
            image,
            lock,
            builder,
        } => add_root(&path.store, &image, lock.as_deref(), builder.as_deref()),
        StoreCommand::UnpinRoot { path, root } => {
            Store::open(&path.store)?.unpin_root(&root)?;
            json(&serde_json::json!({"unpinned_root": root}))
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
        StoreCommand::Push {
            object,
            cache,
            source,
        } => {
            let cache = cache
                .open()?
                .ok_or_else(|| Error::Invalid("push needs --cache".into()))?
                .with_source(source)?;
            json(&Store::open(&object.path.store)?.push(&cache, &object.object)?)
        }
        StoreCommand::Substitute { object, cache } => {
            let (cache, records) = cache
                .open()?
                .ok_or_else(|| Error::Invalid("substitute needs --cache and --trusted".into()))?;
            json(&Store::open(&object.path.store)?.substitute(&cache, &records, &object.object)?)
        }
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

fn add_root(
    store: &Path,
    image: &str,
    lock: Option<&Path>,
    builder: Option<&str>,
) -> Result<ExitCode> {
    let mut layers = Vec::new();
    if let Some(lock) = lock {
        let (lock, _) = crate::catalog_lock::read_lock(lock)?;
        if lock.architecture != sysroot_engine::RPM_ARCHITECTURE {
            return Err(Error::Invalid(format!(
                "engine roots are native {} only",
                sysroot_engine::RPM_ARCHITECTURE
            )));
        }
        // Builder roles were resolved over the foundation transaction, so a
        // builder root is the runtime root plus that role.
        for (name, role) in
            std::iter::once(("foundation", None)).chain(builder.map(|b| (b, Some(b))))
        {
            let role_lock = lock.role(role)?;
            layers.push(sysroot_engine::RootLayer {
                role: name.into(),
                fedora_release: lock.fedora_release,
                files: lock.files(role_lock).into_iter().cloned().collect(),
                remove: role_lock.remove.clone(),
                packages: role_lock.packages.clone(),
            });
        }
    }
    let receipt = Store::open(store)?.add_root(image, layers)?;
    json(&serde_json::json!({
        "root": receipt.root, "recipe": receipt.recipe, "rpm_sha256": receipt.rpm_sha256,
        "entries": receipt.entries, "bytes": receipt.bytes,
    }))
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
            json(&build_graph(
                &Store::open(&options.store)?,
                &graph,
                &options.root,
                options.rebuild,
                &options.cache,
            )?)
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
        Error::Unavailable(_) => ExitCode::from(69),
        Error::Io(_) | Error::Json(_) => ExitCode::FAILURE,
    }
}
