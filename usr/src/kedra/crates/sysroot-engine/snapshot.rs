//! Leased temporary inputs; recovery never infers ownership from age or PID.

use crate::{Error, Result, plan, store};
use rustix::fs::{AtFlags, Mode, OFlags, openat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeSet,
    fs::{self, File, Metadata},
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

const ROOT: &str = ".sysroot-snapshots-v1";
const MARKER: &str = "marker.json";
const LOCK: &str = "lock";
const RECORD_LIMIT: u64 = 4096;

/// The two concrete temporary input formats accepted by recovery.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotPurpose {
    Composition,
    Native,
}

impl SnapshotPurpose {
    fn members(self) -> &'static [&'static str] {
        match self {
            Self::Composition => &[
                "Containerfile",
                "composition.json",
                "foundation.tar",
                "payload.tar",
            ],
            Self::Native => &["Containerfile", "native-driver.py", "native-plan.json"],
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: u32,
    token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lease {
    schema: u32,
    root_token: String,
    nonce: String,
    purpose: SnapshotPurpose,
    device: u64,
    inode: u64,
}

#[derive(Debug, Serialize)]
pub struct SnapshotRefusal {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SnapshotRecovery {
    pub removed: Vec<String>,
    pub active: Vec<String>,
    pub refused: Vec<SnapshotRefusal>,
}

struct Registry {
    path: PathBuf,
    directory: File,
    marker: Marker,
    _lock: File,
}

/// Holds the lease lock until explicit finish or drop. Keep this guard alive
/// while any consumer is reading its paths. Do not pass paths to detached jobs.
pub struct ManagedSnapshot {
    path: PathBuf,
    registry: File,
    directory: File,
    lease_file: File,
    lease: Lease,
    retired: bool,
}

impl ManagedSnapshot {
    pub fn create(parent: &Path, purpose: SnapshotPurpose) -> Result<Self> {
        let registry = Registry::open(parent, true)?
            .ok_or_else(|| invalid("snapshot registry was not created"))?;
        let nonce = store::nonce()?;
        let name = format!("snapshot-{nonce}");
        let lease_name = format!("lease-{nonce}.json");
        rustix::fs::mkdirat(&registry.directory, &name, Mode::from_raw_mode(0o700))?;
        let directory = open_directory(&registry.directory, &name)?;
        let metadata = directory.metadata()?;
        private_directory(&metadata)?;
        let lease = Lease {
            schema: 1,
            root_token: registry.marker.token.clone(),
            nonce,
            purpose,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        let operation: Result<Self> = (|| {
            let mut lease_file = new_file(&registry.directory, &lease_name)?;
            lease_file.lock()?;
            lease_file.write_all(&serde_json::to_vec(&lease)?)?;
            lease_file.sync_all()?;
            directory.sync_all()?;
            registry.directory.sync_all()?;
            Ok(Self {
                path: registry.path.join(&name),
                registry: registry.directory.try_clone()?,
                directory: directory.try_clone()?,
                lease_file,
                lease,
                retired: false,
            })
        })();
        if operation.is_err() {
            // No payload writer has received a path yet. Remove only the lease
            // and exact empty directory created by this still-locked operation.
            let _ = rustix::fs::unlinkat(&registry.directory, &lease_name, AtFlags::empty());
            if same_named(&registry.directory, &name, &directory).is_ok() {
                let _ = rustix::fs::unlinkat(&registry.directory, &name, AtFlags::REMOVEDIR);
            }
            let _ = registry.directory.sync_all();
        }
        operation
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reports cleanup failure instead of relying solely on best-effort Drop.
    pub fn finish(mut self) -> Result<()> {
        self.retire()
    }

    fn retire(&mut self) -> Result<()> {
        if self.retired {
            return Ok(());
        }
        let lock = existing_file(&self.registry, LOCK)?;
        lock.lock()?;
        let name = format!("snapshot-{}", self.lease.nonce);
        retire(
            &self.registry,
            &name,
            &self.directory,
            &self.lease_file,
            &self.lease,
        )?;
        self.retired = true;
        Ok(())
    }
}

impl Drop for ManagedSnapshot {
    fn drop(&mut self) {
        let _ = self.retire();
    }
}

/// Inspect only the versioned managed registry. Active locks are skipped;
/// malformed/unregistered state is reported and never recursively deleted.
pub fn recover_snapshots(parent: &Path) -> Result<SnapshotRecovery> {
    let Some(registry) = Registry::open(parent, false)? else {
        return Ok(SnapshotRecovery::default());
    };
    let mut result = SnapshotRecovery::default();
    let mut nonces = BTreeSet::new();
    for name in names(&registry.directory)? {
        if name == MARKER || name == LOCK {
            continue;
        }
        let nonce = name.strip_prefix("snapshot-").or_else(|| {
            name.strip_prefix("lease-")
                .and_then(|name| name.strip_suffix(".json"))
        });
        if let Some(nonce) = nonce.filter(|nonce| plan::hex(nonce)) {
            nonces.insert(nonce.to_owned());
        } else {
            result.refused.push(SnapshotRefusal {
                name,
                reason: "unregistered snapshot entry".into(),
            });
        }
    }
    for nonce in nonces {
        let name = format!("snapshot-{nonce}");
        let lease_name = format!("lease-{nonce}.json");
        let outcome: Result<bool> = (|| {
            let lease_file = existing_file(&registry.directory, &lease_name)?;
            match lease_file.try_lock() {
                Ok(()) => {}
                Err(fs::TryLockError::WouldBlock) => return Ok(false),
                Err(fs::TryLockError::Error(error)) => return Err(error.into()),
            }
            let lease: Lease = read_record(&lease_file)?;
            if lease.schema != 1
                || lease.nonce != nonce
                || lease.root_token != registry.marker.token
            {
                return Err(invalid("snapshot lease identity differs"));
            }
            let directory = match open_directory(&registry.directory, &name) {
                Ok(directory) => directory,
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                    // The durable lease outlives directory retirement. No path
                    // contents are removed when finishing this interrupted step.
                    rustix::fs::unlinkat(&registry.directory, &lease_name, AtFlags::empty())?;
                    registry.directory.sync_all()?;
                    return Ok(true);
                }
                Err(error) => return Err(error),
            };
            retire(&registry.directory, &name, &directory, &lease_file, &lease)?;
            Ok(true)
        })();
        match outcome {
            Ok(true) => result.removed.push(name),
            Ok(false) => result.active.push(name),
            Err(error) => result.refused.push(SnapshotRefusal {
                name,
                reason: error.to_string(),
            }),
        }
    }
    Ok(result)
}

impl Registry {
    fn open(parent: &Path, create: bool) -> Result<Option<Self>> {
        let parent_directory = open_directory(rustix::fs::CWD, parent)?;
        let parent_metadata = parent_directory.metadata()?;
        if !parent_metadata.is_dir()
            || parent_metadata.uid() != rustix::process::geteuid().as_raw()
            || parent_metadata.mode() & 0o022 != 0
        {
            return Err(invalid(
                "snapshot parent must be owned and not writable by others",
            ));
        }
        let parent_path = fs::canonicalize(parent)?;
        let directory = match open_directory(&parent_directory, ROOT) {
            Ok(directory) => directory,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                if !create {
                    return Ok(None);
                }
                initialize_registry(&parent_directory)?;
                open_directory(&parent_directory, ROOT)?
            }
            Err(error) => return Err(error),
        };
        private_directory(&directory.metadata()?)?;
        let lock = existing_file(&directory, LOCK)?;
        lock.lock()?;
        let marker: Marker = read_record(&existing_file(&directory, MARKER)?)?;
        if marker.schema != 1 || !plan::hex(&marker.token) {
            return Err(invalid("unknown snapshot registry identity"));
        }
        let path = parent_path.join(ROOT);
        let named = fs::symlink_metadata(&path)?;
        if !same(&named, &directory.metadata()?) {
            return Err(invalid("snapshot registry changed while opening"));
        }
        Ok(Some(Self {
            path,
            directory,
            marker,
            _lock: lock,
        }))
    }
}

fn initialize_registry(parent: &File) -> Result<()> {
    let temporary = format!(".sysroot-snapshots-init-{}", store::nonce()?);
    rustix::fs::mkdirat(parent, &temporary, Mode::from_raw_mode(0o700))?;
    let directory = open_directory(parent, &temporary)?;
    let outcome: Result<()> = (|| {
        private_directory(&directory.metadata()?)?;
        new_file(&directory, LOCK)?.sync_all()?;
        let marker = Marker {
            schema: 1,
            token: store::nonce()?,
        };
        let mut file = new_file(&directory, MARKER)?;
        file.write_all(&serde_json::to_vec(&marker)?)?;
        file.sync_all()?;
        directory.sync_all()?;
        publish_registry(parent, &temporary)?;
        parent.sync_all()?;
        Ok(())
    })();
    if outcome.is_err() && same_named(parent, &temporary, &directory).is_ok() {
        // A registry is published only after both metadata files are durable.
        // This still-private initializer has never accepted a payload writer.
        let members = names(&directory)?;
        if members.iter().any(|name| name != LOCK && name != MARKER) {
            return Err(invalid("unpublished registry has an unexpected member"));
        }
        for name in &members {
            existing_file(&directory, name)?;
        }
        for name in members {
            rustix::fs::unlinkat(&directory, &name, AtFlags::empty())?;
        }
        rustix::fs::unlinkat(parent, &temporary, AtFlags::REMOVEDIR)?;
        parent.sync_all()?;
    }
    match outcome {
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        result => result,
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn publish_registry(parent: &File, temporary: &str) -> Result<()> {
    rustix::fs::renameat_with(
        parent,
        temporary,
        parent,
        ROOT,
        rustix::fs::RenameFlags::NOREPLACE,
    )?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn publish_registry(_parent: &File, _temporary: &str) -> Result<()> {
    Err(invalid(
        "atomic snapshot registry publication requires Linux or macOS",
    ))
}

fn retire(
    registry: &File,
    name: &str,
    directory: &File,
    lease_file: &File,
    lease: &Lease,
) -> Result<()> {
    let metadata = directory.metadata()?;
    private_directory(&metadata)?;
    same_named(registry, name, directory)?;
    if metadata.dev() != lease.device || metadata.ino() != lease.inode {
        return Err(invalid("snapshot directory differs from durable lease"));
    }
    let lease_name = format!("lease-{}.json", lease.nonce);
    same_named(registry, &lease_name, lease_file)?;
    let members = names(directory)?;
    for member in &members {
        if !lease.purpose.members().contains(&member.as_str()) {
            return Err(invalid("snapshot has an unexpected member"));
        }
        existing_file(directory, member)?;
    }
    // Validate every member before the first removal. Partial copies are owned
    // too: size/content are deliberately not deletion-authority inputs.
    for member in &members {
        rustix::fs::unlinkat(directory, member, AtFlags::empty())?;
    }
    directory.sync_all()?;
    same_named(registry, name, directory)?;
    rustix::fs::unlinkat(registry, name, AtFlags::REMOVEDIR)?;
    registry.sync_all()?;
    rustix::fs::unlinkat(registry, &lease_name, AtFlags::empty())?;
    registry.sync_all()?;
    Ok(())
}

fn open_directory(fd: impl std::os::fd::AsFd, path: impl AsRef<Path>) -> Result<File> {
    Ok(openat(
        fd,
        path.as_ref(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?
    .into())
}

fn new_file(directory: &File, name: &str) -> Result<File> {
    let file: File = openat(
        directory,
        name,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )?
    .into();
    private_file(&file.metadata()?)?;
    Ok(file)
}

fn existing_file(directory: &File, name: &str) -> Result<File> {
    let file: File = openat(
        directory,
        name,
        OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?
    .into();
    let metadata = file.metadata()?;
    private_file(&metadata)?;
    same_named(directory, name, &file)?;
    Ok(file)
}

fn private_directory(metadata: &Metadata) -> Result<()> {
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(invalid("snapshot directory must be owned and mode 0700"));
    }
    Ok(())
}

fn private_file(metadata: &Metadata) -> Result<()> {
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.nlink() != 1
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(invalid(
            "snapshot member must be owned, single-link and mode 0600",
        ));
    }
    Ok(())
}

fn same(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino() && left.uid() == right.uid()
}

fn same_named(directory: &File, name: &str, file: &File) -> Result<()> {
    let named = rustix::fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)?;
    let opened = rustix::fs::fstat(file)?;
    if named.st_dev != opened.st_dev
        || named.st_ino != opened.st_ino
        || named.st_uid != opened.st_uid
    {
        return Err(invalid("snapshot entry changed while opening"));
    }
    Ok(())
}

fn names(directory: &File) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in rustix::fs::Dir::read_from(directory)? {
        let entry = entry?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        let name =
            std::str::from_utf8(bytes).map_err(|_| invalid("snapshot entry is not UTF-8"))?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

fn read_record<T: DeserializeOwned>(file: &File) -> Result<T> {
    let mut bytes = Vec::new();
    file.take(RECORD_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECORD_LIMIT {
        return Err(invalid("snapshot record exceeds 4096 bytes"));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn invalid(message: impl Into<String>) -> Error {
    Error::RecoveryRequired(format!("managed snapshot: {}", message.into()))
}
