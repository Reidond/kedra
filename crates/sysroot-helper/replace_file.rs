//! Owner-checked, journalable single-file replacement for a stopped app writer.
//! Recovery receipts contain identities/hashes, never native configuration bytes.
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags, ResolveFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const MAX_FILE: usize = 1_048_576;
const SLOT: &str = "candidate";
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && ![".", ".."].contains(&value)
        && !value.contains(['/', '\\', '\0'])
        && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    device: u64,
    inode: u64,
    owner: u32,
    group: u32,
    mode: u32,
    size: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    sha256: String,
    label: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    schema_version: u32,
    parent_device: u64,
    parent_inode: u64,
    pub token: String,
    filename: String,
    original: Option<Identity>,
    candidate: Identity,
}
struct Snapshot {
    identity: Identity,
    bytes: Vec<u8>,
}
pub struct Directory {
    descriptor: OwnedFd,
    owner: u32,
}

fn label(file: &File) -> Result<Option<Vec<u8>>> {
    let mut names = [0u8; 8192];
    let count = fs::flistxattr(file, &mut names[..])?;
    let mut present = false;
    for item in names[..count]
        .split(|b| *b == 0)
        .filter(|part| !part.is_empty())
    {
        if item != b"security.selinux" {
            return Err("extended attributes or ACLs need a separate qualified adapter".into());
        }
        present = true;
    }
    if !present {
        return Ok(None);
    }
    let mut value = [0u8; 4096];
    let count = fs::fgetxattr(file, "security.selinux", &mut value[..])?;
    Ok(Some(value[..count].to_vec()))
}
fn snapshot(directory: &OwnedFd, filename: &str, owner: u32) -> Result<Option<Snapshot>> {
    if !name(filename) {
        return Err("invalid managed filename".into());
    }
    let descriptor = match fs::openat2(
        directory,
        filename,
        OFlags::PATH | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = fs::fstat(&descriptor)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
        || metadata.st_uid != owner
        || metadata.st_nlink != 1
        || metadata.st_mode & 0o7022 != 0
        || metadata.st_size < 0
        || metadata.st_size as u64 > MAX_FILE as u64
    {
        return Err("unsafe managed file ownership, type, links, mode or size".into());
    }
    let mut file = File::open(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))?;
    let before = file.metadata()?;
    let file_label = label(&file)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_FILE as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    let named = fs::statat(directory, filename, AtFlags::SYMLINK_NOFOLLOW)?;
    if bytes.len() > MAX_FILE
        || bytes.len() as u64 != before.len()
        || before.len() != after.len()
        || before.mode() != after.mode()
        || before.uid() != after.uid()
        || before.gid() != after.gid()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || before.dev() != named.st_dev
        || before.ino() != named.st_ino
        || after.nlink() != 1
        || label(&file)? != file_label
    {
        return Err("managed file changed during capture".into());
    }
    Ok(Some(Snapshot {
        identity: Identity {
            device: before.dev(),
            inode: before.ino(),
            owner: before.uid(),
            group: before.gid(),
            mode: before.mode(),
            size: before.len(),
            modified_seconds: before.mtime(),
            modified_nanos: before.mtime_nsec(),
            sha256: digest(&bytes),
            label: file_label,
        },
        bytes,
    }))
}
impl Directory {
    /// Serialize cooperating activations even when callers name different stores.
    pub fn coordinate(&self) -> Result<File> {
        let fd = fs::openat(
            &self.descriptor,
            ".sysroot-activation.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        )?;
        let stat = fs::fstat(&fd)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || stat.st_uid != self.owner
            || stat.st_nlink != 1
            || stat.st_mode & 0o7777 != 0o600
        {
            return Err("application coordination lock is unsafe".into());
        }
        let file = File::from(fd);
        file.try_lock()?;
        Ok(file)
    }
    /// Detect the exact published pair after a crash before the journal update.
    pub fn is_published(&self, receipt: &Receipt) -> Result<bool> {
        self.check_receipt(receipt)?;
        let work = self.work(&receipt.token)?;
        Ok(
            snapshot(&self.descriptor, &receipt.filename, self.owner)?.map(|s| s.identity)
                == Some(receipt.candidate.clone())
                && snapshot(&work, SLOT, self.owner)?.map(|s| s.identity) == receipt.original,
        )
    }

    fn work_exists(&self, receipt: &Receipt) -> Result<bool> {
        match fs::statat(
            &self.descriptor,
            Self::work_name(&receipt.token)?,
            AtFlags::SYMLINK_NOFOLLOW,
        ) {
            Ok(_) => Ok(true),
            Err(rustix::io::Errno::NOENT) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// Only a durable Validated journal authorizes this idempotent cleanup.
    pub fn finish_validated(&self, receipt: &Receipt) -> Result<()> {
        self.check_receipt(receipt)?;
        if !self.work_exists(receipt)? {
            return Ok(());
        }
        let work = self.work(&receipt.token)?;
        let saved = snapshot(&work, SLOT, self.owner)?.map(|s| s.identity);
        if saved.is_some() && saved != receipt.original {
            return Err("private checkpoint changed; explicit recovery is required".into());
        }
        if saved.is_some() {
            fs::unlinkat(&work, SLOT, AtFlags::empty())?;
        }
        fs::fsync(&work)?;
        fs::unlinkat(
            &self.descriptor,
            Self::work_name(&receipt.token)?,
            AtFlags::REMOVEDIR,
        )?;
        fs::fsync(&self.descriptor)?;
        Ok(())
    }

    /// Caller resolves the application's directory; no symlink is followed here.
    pub fn open(path: &Path) -> Result<Self> {
        let descriptor = fs::openat2(
            fs::CWD,
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?;
        let owner = rustix::process::geteuid().as_raw();
        let stat = fs::fstat(&descriptor)?;
        if stat.st_uid != owner || stat.st_mode & 0o022 != 0 {
            return Err("unsafe application directory".into());
        }
        Ok(Self { descriptor, owner })
    }
    pub fn read(&self, filename: &str) -> Result<Option<Vec<u8>>> {
        Ok(snapshot(&self.descriptor, filename, self.owner)?.map(|snapshot| snapshot.bytes))
    }
    fn work_name(token: &str) -> Result<String> {
        if !hex(token, 32) {
            return Err("invalid file transaction token".into());
        }
        Ok(format!(".sysroot-activation-{token}"))
    }
    fn work(&self, token: &str) -> Result<OwnedFd> {
        let descriptor = fs::openat2(
            &self.descriptor,
            Self::work_name(token)?,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?;
        let stat = fs::fstat(&descriptor)?;
        if stat.st_uid != self.owner || stat.st_mode & 0o777 != 0o700 {
            return Err("unsafe private activation directory".into());
        }
        Ok(descriptor)
    }
    fn check_receipt(&self, receipt: &Receipt) -> Result<()> {
        let stat = fs::fstat(&self.descriptor)?;
        if receipt.schema_version != 1
            || stat.st_dev != receipt.parent_device
            || stat.st_ino != receipt.parent_inode
            || !name(&receipt.filename)
            || !hex(&receipt.token, 32)
            || !hex(&receipt.candidate.sha256, 64)
            || receipt.candidate.owner != self.owner
            || receipt
                .original
                .as_ref()
                .is_some_and(|identity| identity.owner != self.owner || !hex(&identity.sha256, 64))
        {
            return Err("file receipt does not identify this owned application directory".into());
        }
        Ok(())
    }
    /// Prepare privately without changing the named live file. Persist the receipt
    /// and review reservation together before calling publish.
    pub fn prepare(
        &self,
        filename: &str,
        token: &str,
        expected: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<Receipt> {
        if bytes.len() > MAX_FILE {
            return Err("managed output exceeds its size bound".into());
        }
        let original = snapshot(&self.descriptor, filename, self.owner)?;
        if original
            .as_ref()
            .is_some_and(|value| value.identity.mode & 0o200 == 0)
        {
            return Err("managed file is not owner-writable".into());
        }
        if original.as_ref().map(|value| value.bytes.as_slice()) != expected {
            return Err("live file changed before preparation".into());
        }
        let work_name = Self::work_name(token)?;
        fs::mkdirat(&self.descriptor, &work_name, Mode::from_raw_mode(0o700))?;
        let work = self.work(token)?;
        let result = (|| -> Result<Receipt> {
            let descriptor = fs::openat(
                &work,
                SLOT,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )?;
            let mut file = File::from(descriptor);
            file.write_all(bytes)?;
            if let Some(original) = &original {
                if fs::fstat(&file)?.st_gid != original.identity.group {
                    fs::fchown(
                        &file,
                        None,
                        Some(fs::Gid::from_raw(original.identity.group)),
                    )?;
                }
                if label(&file)? != original.identity.label {
                    let value =
                        original.identity.label.as_deref().ok_or(
                            "cannot preserve an unlabelled original on a labelled filesystem",
                        )?;
                    fs::fsetxattr(&file, "security.selinux", value, fs::XattrFlags::empty())?;
                }
            }
            let mode = original
                .as_ref()
                .map_or(0o600, |value| value.identity.mode & 0o777);
            fs::fchmod(&file, Mode::from_raw_mode(mode))?;
            file.sync_all()?;
            let candidate =
                snapshot(&work, SLOT, self.owner)?.ok_or("prepared file disappeared")?;
            if let Some(original) = &original
                && (candidate.identity.group != original.identity.group
                    || candidate.identity.label != original.identity.label)
            {
                return Err("file group or SELinux label could not be preserved".into());
            }
            fs::fsync(&work)?;
            fs::fsync(&self.descriptor)?;
            let parent = fs::fstat(&self.descriptor)?;
            Ok(Receipt {
                schema_version: 1,
                parent_device: parent.st_dev,
                parent_inode: parent.st_ino,
                token: token.to_owned(),
                filename: filename.to_owned(),
                original: original.map(|value| value.identity),
                candidate: candidate.identity,
            })
        })();
        if result.is_err() {
            let _ = fs::unlinkat(&work, SLOT, AtFlags::empty());
            let _ = fs::unlinkat(&self.descriptor, &work_name, AtFlags::REMOVEDIR);
        }
        result
    }
    pub fn publish(&self, receipt: &Receipt) -> Result<()> {
        self.check_receipt(receipt)?;
        let work = self.work(&receipt.token)?;
        if snapshot(&self.descriptor, &receipt.filename, self.owner)?.map(|value| value.identity)
            != receipt.original
            || snapshot(&work, SLOT, self.owner)?.map(|value| value.identity)
                != Some(receipt.candidate.clone())
        {
            return Err("application file changed before publication".into());
        }
        let flags = if receipt.original.is_some() {
            RenameFlags::EXCHANGE
        } else {
            RenameFlags::NOREPLACE
        };
        fs::renameat_with(&work, SLOT, &self.descriptor, &receipt.filename, flags)?;
        fs::fsync(&self.descriptor)?;
        fs::fsync(&work)?;
        let previous = snapshot(&work, SLOT, self.owner)?.map(|value| value.identity);
        if previous != receipt.original {
            // A racing rename is retained in the private checkpoint. Restore it
            // only if the live name still contains our exact candidate.
            if receipt.original.is_some()
                && snapshot(&self.descriptor, &receipt.filename, self.owner)?
                    .map(|value| value.identity)
                    == Some(receipt.candidate.clone())
            {
                fs::renameat_with(
                    &work,
                    SLOT,
                    &self.descriptor,
                    &receipt.filename,
                    RenameFlags::EXCHANGE,
                )?;
                fs::fsync(&self.descriptor)?;
                fs::fsync(&work)?;
            }
            return Err("concurrent file change preserved; activation recovery is required".into());
        }
        Ok(())
    }
    /// Abort only when both names still hold the exact recorded versions.
    pub fn abort(&self, receipt: &Receipt) -> Result<()> {
        self.check_receipt(receipt)?;
        if !self.work_exists(receipt)? {
            if snapshot(&self.descriptor, &receipt.filename, self.owner)?.map(|s| s.identity)
                == receipt.original
            {
                return Ok(());
            }
            return Err("recovery checkpoint is missing and the live file changed".into());
        }
        let work = self.work(&receipt.token)?;
        let live =
            snapshot(&self.descriptor, &receipt.filename, self.owner)?.map(|value| value.identity);
        let saved = snapshot(&work, SLOT, self.owner)?.map(|value| value.identity);
        if live == Some(receipt.candidate.clone()) && saved == receipt.original {
            if receipt.original.is_some() {
                fs::renameat_with(
                    &work,
                    SLOT,
                    &self.descriptor,
                    &receipt.filename,
                    RenameFlags::EXCHANGE,
                )?;
            } else {
                fs::renameat_with(
                    &self.descriptor,
                    &receipt.filename,
                    &work,
                    SLOT,
                    RenameFlags::NOREPLACE,
                )?;
            }
        } else if live != receipt.original
            || (saved.is_some() && saved != Some(receipt.candidate.clone()))
        {
            return Err("file changed after activation; preserve current data and review the private checkpoint".into());
        }
        let remaining = snapshot(&work, SLOT, self.owner)?.map(|value| value.identity);
        if remaining.is_some() && remaining != Some(receipt.candidate.clone()) {
            if receipt.original.is_some()
                && snapshot(&self.descriptor, &receipt.filename, self.owner)?
                    .map(|value| value.identity)
                    == receipt.original
            {
                fs::renameat_with(
                    &work,
                    SLOT,
                    &self.descriptor,
                    &receipt.filename,
                    RenameFlags::EXCHANGE,
                )?;
                fs::fsync(&self.descriptor)?;
                fs::fsync(&work)?;
            }
            return Err("recovery observed a concurrent writer; checkpoint retained".into());
        }
        if remaining.is_some() {
            fs::unlinkat(&work, SLOT, AtFlags::empty())?;
        }
        fs::fsync(&work)?;
        fs::unlinkat(
            &self.descriptor,
            Self::work_name(&receipt.token)?,
            AtFlags::REMOVEDIR,
        )?;
        fs::fsync(&self.descriptor)?;
        Ok(())
    }
    /// Call after native read-back succeeds. Retain any changed checkpoint rather
    /// than deleting a concurrent writer's data. Never recursively remove files.
    pub fn finish(&self, receipt: &Receipt) -> Result<()> {
        self.check_receipt(receipt)?;
        let work = self.work(&receipt.token)?;
        let saved = snapshot(&work, SLOT, self.owner)?.map(|value| value.identity);
        if saved != receipt.original {
            return Err("private checkpoint changed; explicit recovery is required".into());
        }
        if saved.is_some() {
            fs::unlinkat(&work, SLOT, AtFlags::empty())?;
        }
        fs::fsync(&work)?;
        fs::unlinkat(
            &self.descriptor,
            Self::work_name(&receipt.token)?,
            AtFlags::REMOVEDIR,
        )?;
        fs::fsync(&self.descriptor)?;
        Ok(())
    }
}
