use clap::Args;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use sysroot_catalog::language::{self, Intent, Lockfile, TargetPolicy};
use sysroot_engine::{Error, Result};

#[derive(Args, Clone)]
pub(crate) struct Inputs {
    /// Explicit admitted catalog root; modules/imports never leave this directory.
    #[arg(long)]
    pub input_root: PathBuf,
    /// Catalog-relative .kedra entry with one namespace directive.
    #[arg(long)]
    pub entry: String,
    #[arg(long)]
    pub target: String,
    /// Catalog-relative reviewed source lockfile.
    #[arg(long)]
    pub lock: String,
    /// Independently selected target/repository/required-base policy.
    #[arg(long)]
    pub target_policy: PathBuf,
}

pub(crate) struct Loaded {
    pub intent: Intent,
    pub inventory: BTreeMap<String, String>,
    pub resources: BTreeMap<String, Vec<u8>>,
}

pub(crate) fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn ordinary(root: &File, path: &str, limit: usize) -> Result<Vec<u8>> {
    if !language::relative(path) {
        return Err(Error::Invalid("unsafe catalog-relative path".into()));
    }
    let mut parent = root.try_clone()?;
    let parts: Vec<_> = path.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        parent = rustix::fs::openat(
            &parent,
            *part,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )?
        .into();
    }
    let mut file: File = rustix::fs::openat(
        &parent,
        parts[parts.len() - 1],
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC
            | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )?
    .into();
    let before = file.metadata()?;
    if !before.is_file() || before.nlink() != 1 || before.len() > limit as u64 {
        return Err(Error::Invalid(
            "catalog input must be bounded single-link ordinary file".into(),
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    let named = rustix::fs::statat(
        &parent,
        parts[parts.len() - 1],
        rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
    )?;
    if bytes.len() as u64 != before.len()
        || before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.mode() != after.mode()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
        || after.nlink() != 1
        || named.st_ino as u64 != before.ino()
        || named.st_dev as u64 != before.dev()
    {
        return Err(Error::Invalid(
            "catalog input changed while admitted".into(),
        ));
    }
    if ["auth.json", "credentials.json", ".credentials.json"].contains(&parts[parts.len() - 1])
        || parts
            .iter()
            .any(|part| matches!(*part, ".ssh" | ".codex" | ".claude" | ".cache"))
        || [
            b"-----BEGIN PRIVATE KEY-----".as_slice(),
            b"-----BEGIN OPENSSH PRIVATE KEY-----",
            b"-----BEGIN RSA PRIVATE KEY-----",
            b"-----BEGIN EC PRIVATE KEY-----",
            b"-----BEGIN ENCRYPTED PRIVATE KEY-----",
        ]
        .iter()
        .any(|marker| bytes.windows(marker.len()).any(|w| w == *marker))
    {
        return Err(Error::Invalid(
            "private input cannot enter catalog admission".into(),
        ));
    }
    Ok(bytes)
}

pub(crate) fn root(path: &Path) -> Result<File> {
    let absolute = path.canonicalize()?;
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(Error::Invalid("catalog root cannot be a symlink".into()));
    }
    Ok(rustix::fs::openat(
        rustix::fs::CWD,
        absolute,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?
    .into())
}

fn json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(bytes)?)
}

pub(crate) fn load(inputs: &Inputs) -> Result<Loaded> {
    let directory = root(&inputs.input_root)?;
    let policy_parent = inputs
        .target_policy
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let policy_name = inputs
        .target_policy
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Invalid("invalid policy filename".into()))?;
    let policy_bytes = ordinary(&root(policy_parent)?, policy_name, language::MAX_INPUT)?;
    let policy: TargetPolicy = json(&policy_bytes)?;
    let lock_bytes = ordinary(&directory, &inputs.lock, language::MAX_INPUT)?;
    let lock: Lockfile = json(&lock_bytes)?;
    let mut inventory = BTreeMap::from([(inputs.lock.clone(), digest(&lock_bytes))]);
    let mut modules = BTreeMap::new();
    let mut pending = vec![inputs.entry.clone()];
    let mut total = policy_bytes.len() + lock_bytes.len();
    let started = std::time::Instant::now();
    while let Some(path) = pending.pop() {
        if modules.contains_key(&path) {
            continue;
        }
        if modules.len() >= language::MAX_MODULES || started.elapsed().as_secs() >= 60 {
            return Err(Error::Invalid(
                "frontend module/deadline limit exceeded".into(),
            ));
        }
        let bytes = ordinary(&directory, &path, language::MAX_INPUT)?;
        total += bytes.len();
        if total > language::MAX_TOTAL {
            return Err(Error::Invalid("frontend aggregate size exceeded".into()));
        }
        let contents =
            String::from_utf8(bytes).map_err(|_| Error::Invalid("module must be UTF-8".into()))?;
        let module =
            language::parse(&path, &contents).map_err(|e| Error::Invalid(e.to_string()))?;
        for import in &module.imports {
            pending.push(
                language::import_path(&path, &import.path)
                    .ok_or_else(|| Error::Invalid("unsafe import path".into()))?,
            );
        }
        inventory.insert(path.clone(), digest(contents.as_bytes()));
        modules.insert(path, contents);
    }
    let intent = language::compile(&modules, &inputs.entry, &inputs.target, lock, &policy)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let mut names = BTreeSet::new();
    for recipe in intent.recipes.values() {
        let mut contents = vec![&recipe.build];
        if let language::SourceIntent::Files(files) = &recipe.source {
            contents.extend(files.values());
        }
        contents.extend(recipe.files.values());
        contents.extend(recipe.replace_files.values());
        contents.extend(recipe.patches.iter().map(|p| &p.content));
        for content in contents {
            if let language::Content::File { path, .. } = content {
                names.insert(path.clone());
            }
        }
    }
    let mut resources = BTreeMap::new();
    for path in names {
        let bytes = ordinary(&directory, &path, language::MAX_INPUT)?;
        total += bytes.len();
        if total > language::MAX_TOTAL {
            return Err(Error::Invalid(
                "frontend aggregate resource size exceeded".into(),
            ));
        }
        inventory.insert(path.clone(), digest(&bytes));
        resources.insert(path, bytes);
    }
    inventory.insert("@target-policy".into(), digest(&policy_bytes));
    if started.elapsed().as_secs() >= 60 {
        return Err(Error::Invalid("frontend deadline exceeded".into()));
    }
    Ok(Loaded {
        intent,
        inventory,
        resources,
    })
}

pub(crate) fn fmt(paths: Vec<PathBuf>, check: bool) -> Result<()> {
    if paths.is_empty() || paths.len() > language::MAX_MODULES {
        return Err(Error::Invalid("select1..128 files to format".into()));
    }
    for path in paths {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Invalid("invalid formatter filename".into()))?;
        let directory = root(parent)?;
        let bytes = ordinary(&directory, name, language::MAX_INPUT)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| Error::Invalid("module must be UTF-8".into()))?;
        let formatted = language::format(name, text).map_err(|e| Error::Invalid(e.to_string()))?;
        if formatted.as_bytes() == bytes {
            continue;
        }
        if check {
            return Err(Error::Invalid("selected module needs formatting".into()));
        }
        // Cooperating formatters serialize on the original inode. Publish only a
        // complete file; interruption before rename preserves the original.
        let file: File = rustix::fs::openat(
            &directory,
            name,
            rustix::fs::OFlags::RDWR | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )?
        .into();
        file.lock()?;
        if ordinary(&directory, name, language::MAX_INPUT)? != bytes {
            return Err(Error::Invalid("formatter source changed".into()));
        }
        let metadata = file.metadata()?;
        if metadata.mode() & 0o7000 != 0 {
            return Err(Error::Invalid("formatter source has unsafe mode".into()));
        }
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Invalid("invalid clock".into()))?
            .as_nanos();
        let temporary = format!(".kedra-format-{}-{nonce}", std::process::id());
        let mut output: File = rustix::fs::openat(
            &directory,
            temporary.as_str(),
            rustix::fs::OFlags::WRONLY
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::from_raw_mode((metadata.mode() & 0o777) as _),
        )?
        .into();
        let result = (|| -> Result<()> {
            output.write_all(formatted.as_bytes())?;
            output.set_permissions(fs::Permissions::from_mode(metadata.mode() & 0o777))?;
            output.sync_all()?;
            if ordinary(&directory, name, language::MAX_INPUT)? != bytes {
                return Err(Error::Invalid(
                    "formatter source changed before publication".into(),
                ));
            }
            rustix::fs::renameat(&directory, temporary.as_str(), &directory, name)?;
            directory.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ =
                rustix::fs::unlinkat(&directory, temporary.as_str(), rustix::fs::AtFlags::empty());
        }
        result?;
    }
    Ok(())
}
