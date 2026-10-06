use crate::{
    Error, LOGICAL_PREFIX, Result,
    plan::{object_id, relative},
};
use rustix::fs::{Mode, OFlags, openat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, Permissions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};
pub(crate) const MAX_TREE: u64 = 1024 * 1024 * 1024;
pub(crate) const MAX_FILE: u64 = 256 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub path: String,
    pub kind: Kind,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Kind {
    Directory,
    File {
        executable: bool,
        bytes: u64,
        sha256: String,
    },
    Symlink {
        target: String,
    },
}
pub(crate) struct Tree {
    pub digest: String,
    pub entries: Vec<Entry>,
    pub references: BTreeSet<String>,
}
struct Walker<'a> {
    dest: Option<&'a Path>,
    entries: Vec<Entry>,
    refs: BTreeSet<String>,
    total: u64,
}
pub(crate) fn inspect(path: &Path, dest: Option<&Path>) -> Result<Tree> {
    let fd: File = openat(
        rustix::fs::CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?
    .into();
    let mut walker = Walker {
        dest,
        entries: Vec::new(),
        refs: BTreeSet::new(),
        total: 0,
    };
    walker.walk(&fd, "")?;
    validate_links(&walker.entries)?;
    let digest = digest(&walker.entries)?;
    Ok(Tree {
        digest,
        entries: walker.entries,
        references: walker.refs,
    })
}
fn digest(entries: &[Entry]) -> Result<String> {
    let bytes = serde_json::to_vec(entries)?;
    let mut hash = Sha256::new();
    hash.update(b"sysroot-engine-tree-v1\0");
    hash.update(bytes);
    Ok(crate::plan::encode_hex(&hash.finalize()))
}
pub(crate) fn single_file(member: &str, bytes: &[u8]) -> Result<Tree> {
    relative(member, false)?;
    if member.contains('/') || bytes.len() as u64 > MAX_FILE {
        return Err(Error::Invalid(
            "source member must be one bounded file".into(),
        ));
    }
    let entries = vec![Entry {
        path: member.into(),
        kind: Kind::File {
            executable: false,
            bytes: bytes.len() as u64,
            sha256: crate::plan::hash(bytes),
        },
    }];
    let mut references = BTreeSet::new();
    scan(member.as_bytes(), &mut references)?;
    scan(bytes, &mut references)?;
    if !references.is_empty() {
        return Err(Error::Invalid(
            "single-file source cannot reference store objects".into(),
        ));
    }
    Ok(Tree {
        digest: digest(&entries)?,
        entries,
        references,
    })
}
impl Walker<'_> {
    fn walk(&mut self, dir: &File, prefix: &str) -> Result<()> {
        let mut names = Vec::new();
        let mut iter = rustix::fs::Dir::read_from(dir)?;
        for entry in &mut iter {
            let entry = entry?;
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let name = std::str::from_utf8(bytes)
                .map_err(|_| Error::Invalid("non-UTF8 tree name".into()))?
                .to_owned();
            if names.len() + self.entries.len() >= 100_000 {
                return Err(Error::Invalid("tree exceeds 100000 entries".into()));
            }
            names.push(name);
        }
        names.sort();
        for name in names {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            relative(&path, false)?;
            if self.entries.len() >= 100_000 {
                return Err(Error::Invalid("tree exceeds 100000 entries".into()));
            }
            scan(path.as_bytes(), &mut self.refs)?;
            let stat =
                rustix::fs::statat(dir, name.as_str(), rustix::fs::AtFlags::SYMLINK_NOFOLLOW)?;
            let ty = rustix::fs::FileType::from_raw_mode(stat.st_mode);
            let kind = match ty {
                rustix::fs::FileType::Directory => {
                    let file: File = openat(
                        dir,
                        name.as_str(),
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )?
                    .into();
                    if let Some(dest) = self.dest {
                        fs::create_dir(dest.join(&path))?;
                        fs::set_permissions(dest.join(&path), Permissions::from_mode(0o755))?;
                    }
                    self.entries.push(Entry {
                        path: path.clone(),
                        kind: Kind::Directory,
                    });
                    self.walk(&file, &path)?;
                    continue;
                }
                rustix::fs::FileType::RegularFile => {
                    let mut file: File = openat(
                        dir,
                        name.as_str(),
                        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )?
                    .into();
                    let meta = file.metadata()?;
                    if !meta.is_file() || meta.nlink() != 1 || meta.len() > MAX_FILE {
                        return Err(Error::Invalid(format!(
                            "unsupported hardlink/file size: {path}"
                        )));
                    }
                    self.total = self
                        .total
                        .checked_add(meta.len())
                        .ok_or_else(|| Error::Invalid("tree size overflow".into()))?;
                    if self.total > MAX_TREE {
                        return Err(Error::Invalid("tree exceeds 1 GiB".into()));
                    }
                    let mut data = Vec::new();
                    Read::by_ref(&mut file)
                        .take(MAX_FILE + 1)
                        .read_to_end(&mut data)?;
                    let after = file.metadata()?;
                    if data.len() as u64 != meta.len()
                        || after.len() != meta.len()
                        || after.dev() != meta.dev()
                        || after.ino() != meta.ino()
                        || after.mtime() != meta.mtime()
                        || after.mtime_nsec() != meta.mtime_nsec()
                        || after.ctime() != meta.ctime()
                        || after.ctime_nsec() != meta.ctime_nsec()
                        || after.mode() != meta.mode()
                        || after.nlink() != meta.nlink()
                    {
                        return Err(Error::Invalid(format!(
                            "file changed during capture: {path}"
                        )));
                    }
                    scan(&data, &mut self.refs)?;
                    let executable = meta.mode() & 0o111 != 0;
                    if let Some(dest) = self.dest {
                        let mut out = File::create_new(dest.join(&path))?;
                        out.write_all(&data)?;
                        out.set_permissions(Permissions::from_mode(if executable {
                            0o755
                        } else {
                            0o644
                        }))?;
                        out.sync_all()?;
                    }
                    Kind::File {
                        executable,
                        bytes: meta.len(),
                        sha256: crate::plan::hash(&data),
                    }
                }
                rustix::fs::FileType::Symlink => {
                    let target = rustix::fs::readlinkat(dir, name.as_str(), Vec::new())?
                        .into_string()
                        .map_err(|_| Error::Invalid("non-UTF8 symlink".into()))?;
                    link_target(&path, &target)?;
                    scan(target.as_bytes(), &mut self.refs)?;
                    if let Some(dest) = self.dest {
                        std::os::unix::fs::symlink(&target, dest.join(&path))?;
                    }
                    Kind::Symlink { target }
                }
                _ => {
                    return Err(Error::Invalid(format!(
                        "unsupported special tree node: {path}"
                    )));
                }
            };
            self.entries.push(Entry { path, kind });
        }
        Ok(())
    }
}
pub(crate) fn link_target(path: &str, target: &str) -> Result<()> {
    if target.is_empty() || target.len() > 4096 || target.bytes().any(|b| b < 32 || b == 127) {
        return Err(Error::Invalid("unsafe symlink target".into()));
    }
    if let Some(rest) = target.strip_prefix(&format!("{LOGICAL_PREFIX}/")) {
        let mut parts = rest.splitn(2, '/');
        object_id(parts.next().unwrap_or_default())?;
        if let Some(path) = parts.next() {
            relative(path, false)?;
        }
        return Ok(());
    }
    if target.starts_with('/') {
        return Err(Error::Invalid(format!("symlink escapes tree: {path}")));
    }
    let mut depth = path.split('/').count() - 1;
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::Invalid(format!("symlink escapes tree: {path}")))?;
            }
            _ => depth += 1,
        }
    }
    Ok(())
}
pub(crate) fn scan(bytes: &[u8], refs: &mut BTreeSet<String>) -> Result<()> {
    let prefix = format!("{LOGICAL_PREFIX}/");
    let needle = prefix.as_bytes();
    let mut start = 0;
    while let Some(offset) = bytes[start..]
        .windows(needle.len())
        .position(|w| w == needle)
    {
        let begin = start + offset + needle.len();
        let tail = &bytes[begin..];
        let end = tail
            .iter()
            .position(|b| !b.is_ascii_alphanumeric() && *b != b'-')
            .unwrap_or(tail.len());
        let id = std::str::from_utf8(&tail[..end])
            .map_err(|_| Error::Invalid("invalid store reference".into()))?;
        object_id(id)?;
        refs.insert(id.into());
        start = begin + end;
    }
    Ok(())
}
pub(crate) fn seal(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.file_type()?;
        if meta.is_dir() {
            seal(&entry.path())?;
        } else if meta.is_file() {
            let mode = entry.metadata()?.mode();
            fs::set_permissions(
                entry.path(),
                Permissions::from_mode(if mode & 0o111 != 0 { 0o555 } else { 0o444 }),
            )?;
            File::open(entry.path())?.sync_all()?;
        }
    }
    fs::set_permissions(path, Permissions::from_mode(0o555))?;
    File::open(path)?.sync_all()?;
    Ok(())
}
pub(crate) fn remove(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(Error::Corrupt(format!(
            "unsafe cleanup directory or symlink: {}",
            path.display()
        )));
    }
    fs::set_permissions(path, Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            remove(&entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::set_permissions(path, Permissions::from_mode(0o700))?;
    fs::remove_dir(path)?;
    Ok(())
}

pub(crate) fn validate_links(entries: &[Entry]) -> Result<()> {
    let kinds: BTreeMap<_, _> = entries
        .iter()
        .map(|entry| (entry.path.as_str(), &entry.kind))
        .collect();
    for entry in entries {
        let Kind::Symlink { target } = &entry.kind else {
            continue;
        };
        if target.starts_with('/') {
            continue;
        }
        let mut components: Vec<&str> = entry
            .path
            .rsplit_once('/')
            .map_or_else(Vec::new, |(parent, _)| parent.split('/').collect());
        let parts: Vec<_> = target.split('/').collect();
        for (index, part) in parts.iter().enumerate() {
            match *part {
                "" | "." => {}
                ".." => {
                    components.pop();
                }
                part => components.push(part),
            }
            if index + 1 < parts.len()
                && matches!(
                    kinds.get(components.join("/").as_str()),
                    Some(Kind::Symlink { .. })
                )
            {
                return Err(Error::Invalid(format!(
                    "unsafe intermediate symlink traversal: {}",
                    entry.path
                )));
            }
        }
    }
    Ok(())
}
