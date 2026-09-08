use super::{Command, Key, Options, export};
use rustix::fs::{self, Mode, OFlags, ResolveFlags};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as Process, Stdio};
use sysroot_core::noctalia::{self, Baseline, Settings, State};
use sysroot_helper::storage::Store;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RECORD: &str = "noctalia";
const DESTINATION: &str = "usr/share/sysroot/home/default/.config/noctalia/config.toml";

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

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
    sysroot_helper::trusted_file::read(path, owner, limit)
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
    let digest = hash(&bytes);
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
        Command::Status {
            last_capture: false,
        } => (),
        _ => return Err("this operation does not capture live settings".into()),
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
    let instance = hash(&[machine, owner.to_le_bytes().to_vec()].concat())[..32].to_owned();
    let mut source_result = None;
    let state = if matches!(options.command, Command::Init) {
        let baseline = installed_baseline(Path::new("/"), 0)?;
        let state = State::new(instance.clone(), baseline, live()?)?;
        Store::create(&path, &[(RECORD, &state.to_bytes()?)])?;
        state
    } else {
        let mut store = Store::open(&path)?;
        match &options.command {
            Command::Selection | Command::Status { last_capture: true } => {
                load(&store, &instance)?.1
            }
            Command::Export { repo, output } => {
                let (_, state) = load(&store, &instance)?;
                let prepared = export::prepare(repo, &path, &state)?;
                prepared.write_new(output)?;
                source_result = Some(serde_json::json!({
                    "patch_file": output, "base_revision": prepared.source_revision,
                    "source_path": prepared.source_path, "patch_sha256": hash(&prepared.patch),
                    "patch_bytes": prepared.patch.len(), "already_in_source": prepared.patch.is_empty(),
                    "source_objects_may_be_added": !prepared.patch.is_empty(), "commit_created": false
                }));
                state
            }
            Command::RecordSource { repo, commit } => {
                let (revision, mut state) = load(&store, &instance)?;
                let settings = export::verify_commit(repo, commit, &state)?;
                state.record_source_commit(commit, settings)?;
                store.compare_exchange(RECORD, Some(revision), &state.to_bytes()?)?;
                source_result = Some(serde_json::json!({
                    "recorded_commit": commit, "commit_created": false,
                    "registry_publication_verified": false, "deployment_performed": false
                }));
                state
            }
            _ => change(&mut store, &instance, live()?, &options.command)?,
        }
    };
    let mut response = if matches!(options.command, Command::Selection | Command::Export { .. }) {
        serde_json::json!({"schema_version": 1, "application": "noctalia", "accepted_baseline": state.accepted_baseline(),
            "selection": state.selection()?, "activation_performed": false, "checkout_changed": false})
    } else {
        serde_json::json!({"schema_version": 1, "application": "noctalia", "accepted_baseline": state.accepted_baseline(),
            "fields": state.rows()?, "activation_performed": false, "checkout_changed": false})
    };
    if let Some(result) = source_result {
        response["source"] = result;
    }
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

#[cfg(test)]
mod tests;
