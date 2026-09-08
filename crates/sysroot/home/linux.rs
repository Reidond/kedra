use super::{Command, Key, Options};
use rustix::fs::{self, FileType, Mode, OFlags, ResolveFlags};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as Process, Stdio};
use sysroot_core::noctalia::{self, Baseline, Settings, State};
use sysroot_helper::storage::Store;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RECORD: &str = "noctalia";
const DESTINATION: &str = "usr/share/sysroot/home/default/.config/noctalia/config.toml";

impl From<Key> for noctalia::Key {
    fn from(key: Key) -> Self {
        match key {
            Key::Theme => Self::ThemeMode,
            Key::ButtonBorders => Self::ButtonBorders,
            Key::InputBorders => Self::InputBorders,
        }
    }
}

/// Pin a regular inode without opening a device or following a path symlink.
/// Raw unprojected bytes never enter the review store or diagnostics.
fn read_regular(path: &Path, owner: u32, limit: usize) -> Result<Vec<u8>> {
    let descriptor = fs::openat2(
        fs::CWD,
        path,
        OFlags::PATH | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let before = fs::fstat(&descriptor)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_uid != owner
        || before.st_nlink != 1
        || before.st_mode & 0o022 != 0
        || before.st_size < 0
        || before.st_size as u64 > limit as u64
    {
        return Err(
            "input is not a bounded regular file with safe ownership and permissions".into(),
        );
    }
    // This proc path references only the inode already checked and held above.
    let mut file = File::open(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = fs::fstat(&descriptor)?;
    let named = std::fs::symlink_metadata(path)?;
    if bytes.len() > limit
        || bytes.len() as u64 != before.st_size as u64
        || before.st_dev != named.dev()
        || before.st_ino != named.ino()
        || before.st_mode != after.st_mode
        || before.st_uid != after.st_uid
        || before.st_gid != after.st_gid
        || before.st_size != after.st_size
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
        || after.st_nlink != 1
    {
        return Err("input changed during capture; retry after the writer finishes".into());
    }
    Ok(bytes)
}

fn private_parent(path: &Path, owner: u32) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path.file_name().is_none()
    {
        return Err("state path must be absolute and have an existing private parent".into());
    }
    // bootc normally exposes /home through /var/home. Resolve that directory
    // alias, then require the actual parent to be protected and user-owned.
    // The store itself still must not be a symlink (enforced by Store).
    let parent = path
        .parent()
        .ok_or("state path has no parent")?
        .canonicalize()?;
    let descriptor = fs::openat2(
        fs::CWD,
        &parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let stat = fs::fstat(&descriptor)?;
    if stat.st_uid != owner || stat.st_mode & 0o022 != 0 {
        return Err("state parent must be owned by this user and not writable by others".into());
    }
    Ok(parent.join(path.file_name().ok_or("state path has no name")?))
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    source_revision: String,
    target: Target,
    files: Vec<Payload>,
}
#[derive(Deserialize)]
struct Target {
    id: String,
}
#[derive(Deserialize)]
struct Payload {
    source_path: String,
    destination: String,
    sha256: String,
    home_baseline: bool,
}

fn installed_baseline(root: &Path, owner: u32) -> Result<Baseline> {
    let manifest: Manifest = serde_json::from_slice(&read_regular(
        &root.join("usr/share/sysroot/source.json"),
        owner,
        1024 * 1024,
    )?)
    .map_err(|_| "installed source manifest is malformed")?;
    if manifest.schema_version != 1 {
        return Err("installed source manifest version is unsupported".into());
    }
    let candidates: Vec<_> = manifest
        .files
        .iter()
        .filter(|f| f.destination == DESTINATION && f.home_baseline)
        .collect();
    if candidates.len() != 1 {
        return Err("installed Noctalia baseline has no unique source provenance".into());
    }
    let payload = candidates[0];
    let bytes = read_regular(&root.join(DESTINATION), owner, noctalia::MAX_EXPORT)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != payload.sha256 {
        return Err("installed Noctalia baseline differs from its source manifest".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "installed baseline is not UTF-8")?;
    Ok(Baseline {
        target: manifest.target.id,
        source_path: payload.source_path.clone(),
        source_revision: manifest.source_revision,
        settings: noctalia::project(noctalia::APP_VERSION, text)?,
    })
}

fn output(arguments: &[&str], limit: usize) -> Result<Vec<u8>> {
    let mut child = Process::new("/usr/bin/timeout")
        .args([
            "--signal=TERM",
            "--kill-after=2s",
            "10s",
            "/usr/bin/noctalia",
        ])
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let result = (|| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("Noctalia output is unavailable")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err("Noctalia output exceeded the capture bound".into());
        }
        if !child.wait()?.success() {
            return Err("Noctalia export failed; no review state was changed".into());
        }
        Ok(bytes)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn live() -> Result<Settings> {
    let bytes = output(&["--version"], 256)?;
    let version = std::str::from_utf8(&bytes).map_err(|_| "Noctalia version is malformed")?;
    // Exact native Fedora build output recorded by the passing R07 probe.
    if version.trim() != "noctalia v5.0.1 (v5.0.1)" {
        return Err("installed Noctalia version is not qualified for home review".into());
    }
    let bytes = output(&["config", "export", "full"], noctalia::MAX_EXPORT)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "Noctalia export is not UTF-8")?;
    Ok(noctalia::project(noctalia::APP_VERSION, text)?)
}

fn load(store: &Store, instance: &str) -> Result<(u64, State)> {
    let record = store
        .read(RECORD)?
        .ok_or("Noctalia review record is missing; do not reset this store")?;
    Ok((record.revision, State::from_bytes(&record.bytes, instance)?))
}

fn change(
    store: &mut Store,
    instance: &str,
    observed: Settings,
    command: &Command,
) -> Result<State> {
    let (revision, mut state) = load(store, instance)?;
    let before = state.to_bytes()?;
    state.capture(observed)?;
    match command {
        Command::Stage { key } => state.stage((*key).into())?,
        Command::Unstage { key } => state.unstage((*key).into())?,
        Command::KeepLocal { key } => state.ignore_exact((*key).into())?,
        Command::AppOwn { key } => state.own((*key).into())?,
        Command::ClearLocal { key } => state.clear_local_policy((*key).into())?,
        Command::Status | Command::Selection => (),
        Command::Init => return Err("existing review store must not be reinitialized".into()),
    }
    let bytes = state.to_bytes()?;
    if bytes != before {
        store.compare_exchange(RECORD, Some(revision), &bytes)?;
    }
    Ok(state)
}

pub(super) fn run(options: Options) -> Result<()> {
    let owner = rustix::process::geteuid().as_raw();
    if owner == 0 {
        return Err("home review runs as the ordinary desktop user, never root".into());
    }
    let path = private_parent(&options.state, owner)?;
    let machine = read_regular(Path::new("/etc/machine-id"), 0, 64)?;
    let machine_text = std::str::from_utf8(&machine)
        .map_err(|_| "machine identity is malformed")?
        .trim();
    if machine_text.len() != 32
        || !machine_text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(
            "machine identity is not initialized; home review cannot be bound safely".into(),
        );
    }
    let instance = format!(
        "{:x}",
        Sha256::digest([machine, owner.to_le_bytes().to_vec()].concat())
    )[..32]
        .to_owned();
    let state = if matches!(options.command, Command::Init) {
        let baseline = installed_baseline(Path::new("/"), 0)?;
        let state = State::new(instance.clone(), baseline, live()?)?;
        Store::create(&path, &[(RECORD, &state.to_bytes()?)])?;
        state
    } else {
        let mut store = Store::open(&path)?;
        change(&mut store, &instance, live()?, &options.command)?
    };
    let response = if matches!(options.command, Command::Selection) {
        serde_json::json!({"application": "noctalia", "selection": state.selection()?, "activation_performed": false, "source_written": false})
    } else {
        serde_json::json!({"application": "noctalia", "fields": state.rows()?, "activation_performed": false, "source_written": false})
    };
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

#[cfg(test)]
mod tests;
