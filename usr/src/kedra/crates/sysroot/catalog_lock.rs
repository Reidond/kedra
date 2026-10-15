//! `sysroot catalog lock`: resolve Fedora RPMs once, then build offline.
use clap::{Args, Subcommand};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use sysroot_catalog::language::TargetPolicy;
use sysroot_catalog::rpm_lock::{IMAGE_PATH, MAX_LOCK_BYTES, RpmLock};
use sysroot_engine::{Error, Result, RpmRequest, Store};

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Resolve the selected foundation and builder RPM sets once against Fedora
    /// and admit every RPM as a fixed-output store object (network, Docker).
    Resolve(Resolve),
    /// Download locked RPMs the store lacks, verifying each against the lock.
    Fetch {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        lock: PathBuf,
    },
    /// Copy one role's locked RPMs from the store into a new offline repository.
    Repo(Repo),
    /// Copy the RPM lock out of a local image.
    Extract {
        #[arg(long)]
        image: String,
        /// New lock file; an existing file is refused.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Args)]
pub(crate) struct Resolve {
    #[command(flatten)]
    inputs: crate::catalog_language::Inputs,
    /// Exact base image: `<name>@sha256:<digest>` or a local `sha256:` identity.
    #[arg(long)]
    base: String,
    #[arg(long)]
    store: PathBuf,
    /// New lock file; an existing file is refused.
    #[arg(long)]
    output: PathBuf,
    /// Directory for the bounded output of every resolver command.
    #[arg(long)]
    log_dir: Option<PathBuf>,
}

#[derive(Args)]
pub(crate) struct Repo {
    #[arg(long)]
    store: PathBuf,
    #[arg(long)]
    lock: PathBuf,
    /// Materialize the foundation role.
    #[arg(long, conflicts_with = "builder", required_unless_present = "builder")]
    foundation: bool,
    /// Materialize one builder role, by its catalog name.
    #[arg(long)]
    builder: Option<String>,
    /// New directory: `lock.json` and `packages/<file>.rpm`.
    #[arg(long)]
    output: PathBuf,
}

fn emit(value: &serde_json::Value) -> Result<ExitCode> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn read_lock(path: &Path) -> Result<(RpmLock, Vec<u8>)> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_LOCK_BYTES + 1)
        .read_to_end(&mut bytes)?;
    Ok((RpmLock::read(&bytes)?, bytes))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// RPM architecture of a closed release target.
fn architecture(target: &str) -> Result<&'static str> {
    ["aarch64", "x86_64"]
        .into_iter()
        .find(|architecture| sysroot_core::targets::enabled(target, architecture).is_some())
        .ok_or_else(|| Error::Invalid(format!("unknown release target {target:?}")))
}

fn policy(inputs: &crate::catalog_language::Inputs, expected: &str) -> Result<TargetPolicy> {
    let mut bytes = Vec::new();
    File::open(&inputs.target_policy)?
        .take(sysroot_catalog::language::MAX_INPUT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if crate::catalog_language::digest(&bytes) != expected {
        return Err(Error::Invalid(
            "target policy changed during admission".into(),
        ));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn resolve(options: Resolve) -> Result<ExitCode> {
    let architecture = architecture(&options.inputs.target)?;
    if options.output.exists() {
        return Err(Error::Invalid("RPM lock output already exists".into()));
    }
    let loaded = crate::catalog_language::load(&options.inputs)?;
    let policy = policy(&options.inputs, &loaded.inventory["@target-policy"])?;
    let intent = &loaded.intent;
    let intent_sha256 =
        crate::catalog_language::digest(&serde_json::to_vec(&serde_json::to_value(intent)?)?);
    let request = RpmRequest {
        base: options.base,
        architecture: architecture.into(),
        repositories: policy.repositories.into_iter().collect(),
        install: intent.packages.clone(),
        remove: intent.remove.clone(),
        builders: intent.builders.clone(),
    };
    let store = Store::create(&options.store)?;
    let resolution = store.resolve_rpms(&request, options.log_dir.as_deref())?;
    let lock = RpmLock::new(&intent.target, &intent_sha256, &request, resolution)?;
    lock.check_intent(intent, &intent_sha256)?;
    let bytes = lock.to_bytes()?;
    write_new(&options.output, &bytes)?;
    let builders: BTreeMap<_, _> = lock
        .builders
        .iter()
        .map(|(name, role)| (name.clone(), role.install.len()))
        .collect();
    emit(&serde_json::json!({
        "lock": options.output,
        "sha256": crate::catalog_language::digest(&bytes),
        "pins_sha256": lock.pins_sha256()?,
        "rpms": lock.rpms.len(),
        "bytes": lock.rpms.values().map(|rpm| rpm.size).sum::<u64>(),
        "foundation": lock.foundation.install.len(),
        "builders": builders,
    }))
}

fn fetch(store: &Path, lock: &Path) -> Result<ExitCode> {
    let (lock, _) = read_lock(lock)?;
    let store = Store::create(store)?;
    let mut sources = BTreeMap::new();
    for (file, rpm) in &lock.rpms {
        let (_, source) = store.fetch_rpm(rpm)?;
        sources.insert(file.clone(), source);
    }
    let present = sources.values().filter(|s| *s == "store").count();
    emit(&serde_json::json!({
        "pins_sha256": lock.pins_sha256()?,
        "present": present,
        "fetched": sources.len() - present,
        "sources": sources,
    }))
}

fn repo(options: Repo) -> Result<ExitCode> {
    let (lock, bytes) = read_lock(&options.lock)?;
    let role = lock.role(options.builder.as_deref())?;
    let store = Store::open(&options.store)?;
    let files = lock.files(role);
    store.verify_rpms(&files)?;
    fs::create_dir(&options.output)?;
    write_new(&options.output.join("lock.json"), &bytes)?;
    store.materialize_rpms(&files, &options.output.join("packages"))?;
    File::open(&options.output)?.sync_all()?;
    emit(&serde_json::json!({
        "repository": options.output,
        "role": options.builder.as_deref().unwrap_or("foundation"),
        "pins_sha256": lock.pins_sha256()?,
        "rpms": files.len(),
        "bytes": files.iter().map(|rpm| rpm.size).sum::<u64>(),
    }))
}

fn extract(image: &str, output: &Path) -> Result<ExitCode> {
    if output.exists() {
        return Err(Error::Invalid("RPM lock output already exists".into()));
    }
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Invalid("invalid RPM lock output name".into()))?;
    let scratch = output.with_file_name(format!(".{name}.extract"));
    let bytes = sysroot_engine::image_file(image, IMAGE_PATH, MAX_LOCK_BYTES, &scratch)?;
    let lock = RpmLock::read(&bytes)?;
    write_new(output, &bytes)?;
    emit(&serde_json::json!({
        "lock": output,
        "image": image,
        "target": lock.target,
        "base": lock.base,
        "pins_sha256": lock.pins_sha256()?,
        "rpms": lock.rpms.len(),
    }))
}

pub(crate) fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Resolve(options) => resolve(options),
        Command::Fetch { store, lock } => fetch(&store, &lock),
        Command::Repo(options) => repo(options),
        Command::Extract { image, output } => extract(&image, &output),
    }
}
