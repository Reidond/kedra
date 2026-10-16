//! Store root filesystems: the trees sandboxed processes run in.
//!
//! A root is input-addressed by its recipe: an exact store image, unpacked here
//! without a container daemon, and optionally one RPM lock role installed over
//! it offline by the root's own rpm inside the sandbox. Roots are reused by
//! identity and never re-hashed on use: they are owner-private like the rest of
//! the store, and their recipe, not their bytes, names them. Ownership is not
//! represented (every entry belongs to the store owner), device nodes are
//! skipped and set-ID bits cleared; every file is owner-readable.
use crate::executor::{self, MOUNT_POINTS, REPOSITORY};
use crate::image_archive::{self, Layer};
use crate::plan::{self, image_id, root_id};
use crate::store::{list_names, owned_node, private, read_json, write_json_new};
use crate::{
    Error, PLATFORM, Result, RootReceipt, RootRecipe, RootRpmFile, RootRpms, RpmFile, Store, tree,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const MAX_ENTRIES: u64 = 2_000_000;
const MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const INSTALL_SECONDS: u64 = 3600;
const OBSERVE_SECONDS: u64 = 300;
const ROW_FORMAT: &str =
    "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n";
/// Runs as the store owner inside the writable staging copy: verify every
/// file's Fedora signature, install with the root's own rpm (a non-root rpm
/// keeps the owner instead of chowning), apply removals, print the inventory.
const INSTALL: &str = r#"set -euo pipefail
key=/etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-$1-primary
test -f "$key"
rpmkeys --import "$key"
shift
removals=()
files=()
for argument; do
    case $argument in
        -*) removals+=("${argument#-}") ;;
        *) files+=("$argument") ;;
    esac
done
for file in "${files[@]}"; do
    checked=$(rpmkeys --checksig -v "$file")
    if ! grep -q 'signature, key fingerprint: [0-9a-f]*: OK$' <<<"$checked" \
        || grep -Eq 'NOT OK|NOKEY|BAD' <<<"$checked"; then
        printf '%s\n' "$checked" >&2
        exit 65
    fi
done
if test "${#files[@]}" -gt 0; then
    rpm -U --nocaps --noplugins "${files[@]}" >&2
fi
if test "${#removals[@]}" -gt 0; then
    rpm -e --noplugins "${removals[@]}" >&2
fi
"#;
/// Lists the inventory of a root that has rpm; a root without it has none.
const INVENTORY: &str = r#"set -eu
if test -x /usr/bin/rpm; then
    rpm -qa --qf "$1"
fi
"#;

/// One locked RPM role for `add_root`, taken from an RPM lock.
pub struct RootLayer {
    pub role: String,
    pub fedora_release: u32,
    pub files: Vec<RpmFile>,
    pub remove: BTreeSet<String>,
    pub packages: Vec<Vec<String>>,
}

pub(crate) fn root_identity(recipe: &RootRecipe) -> Result<String> {
    let mut bytes = b"sysroot-engine-root-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(recipe)?);
    Ok(format!("root-{}", plan::hash(&bytes)))
}

fn image_recipe(image: &str) -> RootRecipe {
    RootRecipe {
        schema: 1,
        platform: PLATFORM.into(),
        image: image.into(),
        rpms: None,
    }
}

impl Store {
    pub(crate) fn root_path(&self, id: &str) -> PathBuf {
        self.path().join("rootfs").join(id)
    }

    pub(crate) fn root_tree(&self, id: &str) -> PathBuf {
        self.root_path(id).join("tree")
    }

    /// Create (or reuse) and pin the root of an exact store image, optionally
    /// with one locked RPM role installed offline over it.
    pub fn add_root(&self, image: &str, layer: Option<RootLayer>) -> Result<RootReceipt> {
        let base = self.image_root(image)?;
        let receipt = match layer {
            None => self.root(&base)?,
            Some(layer) => self.rpm_root(&base, image, layer)?,
        };
        let mut roots = self.roots()?;
        roots.rootfs.insert(receipt.root.clone());
        self.save_roots(&roots)?;
        Ok(receipt)
    }

    pub fn unpin_root(&self, id: &str) -> Result<()> {
        root_id(id)?;
        let mut roots = self.roots()?;
        roots.rootfs.remove(id);
        self.save_roots(&roots)
    }

    /// The verified receipt of an existing root.
    pub fn root(&self, id: &str) -> Result<RootReceipt> {
        root_id(id)?;
        let path = self.root_path(id);
        if !path.exists() {
            return Err(Error::Invalid(format!(
                "store has no root {id}; create it with `sysroot store add-root`"
            )));
        }
        owned_node(&path, true)?;
        owned_node(&path.join("tree"), true)?;
        private(&path.join("receipt.json"), false)?;
        let receipt: RootReceipt = read_json(&path.join("receipt.json"))?;
        if receipt.schema != 1
            || receipt.root != id
            || root_identity(&receipt.recipe)? != id
            || receipt.recipe.schema != 1
            || receipt.recipe.platform != PLATFORM
            || plan::hash(receipt.rpm_inventory.as_bytes()) != receipt.rpm_sha256
        {
            return Err(Error::Corrupt(format!("root receipt differs for {id}")));
        }
        image_id(&receipt.recipe.image)?;
        Ok(receipt)
    }

    /// The root of an exact store image, unpacked on first use.
    pub(crate) fn image_root(&self, image: &str) -> Result<String> {
        let recipe = image_recipe(image);
        let id = root_identity(&recipe)?;
        if self.root_path(&id).exists() {
            self.root(&id)?;
            return Ok(id);
        }
        self.verify_image(image)?;
        let archive = self.image_path(image).join("image.tar");
        let layers = image_archive::layers(&archive, image, PLATFORM)?;
        self.admit_root(&id, recipe, None, |tree| {
            let mut budget = Budget::default();
            for layer in &layers {
                apply_layer(&archive, layer, tree, &mut budget)?;
            }
            Ok(())
        })?;
        Ok(id)
    }

    fn rpm_root(&self, base: &str, image: &str, layer: RootLayer) -> Result<RootReceipt> {
        let files = layer
            .files
            .iter()
            .map(|rpm| RootRpmFile {
                file: rpm.file_name(),
                sha256: rpm.sha256.clone(),
                object: rpm.object.clone(),
            })
            .collect();
        let recipe = RootRecipe {
            rpms: Some(RootRpms {
                role: layer.role.clone(),
                fedora_release: layer.fedora_release,
                files,
                remove: layer.remove.iter().cloned().collect(),
                packages: layer.packages.clone(),
            }),
            ..image_recipe(image)
        };
        let id = root_identity(&recipe)?;
        if self.root_path(&id).exists() {
            return self.root(&id);
        }
        let rpms: Vec<&RpmFile> = layer.files.iter().collect();
        self.verify_rpms(&rpms)?;
        let source = self.root_tree(base);
        self.admit_root(&id, recipe, Some(&layer), |tree| {
            copy_tree(&source, tree)?;
            let repository = tree
                .parent()
                .ok_or_else(|| Error::Invalid("root stage missing".into()))?
                .join("rpms");
            self.materialize_rpms(&rpms, &repository)?;
            let result = self.install_layer(tree, &repository, &layer);
            tree::remove(&repository)?;
            result
        })?;
        self.root(&id)
    }

    fn install_layer(&self, tree: &Path, repository: &Path, layer: &RootLayer) -> Result<()> {
        let mut argv: Vec<String> = vec![
            "/bin/bash".into(),
            "-c".into(),
            INSTALL.into(),
            "install-rpms".into(),
            layer.fedora_release.to_string(),
        ];
        argv.extend(
            layer
                .files
                .iter()
                .map(|rpm| format!("{REPOSITORY}/{}", rpm.file_name())),
        );
        argv.extend(layer.remove.iter().map(|name| format!("-{name}")));
        let run = executor::sandbox(
            self,
            executor::Sandbox {
                root: tree,
                writable_root: true,
                stage: tree.parent(),
                mounts: &[],
                output: None,
                repository: Some(repository),
                argv: &argv,
                env: &BTreeMap::new(),
                seconds: INSTALL_SECONDS,
            },
        )?;
        executor::checked(run).map(|_| ())
    }

    /// Build a root in staging, observe its RPM inventory in the sandbox, seal
    /// it and publish it under its identity.
    fn admit_root(
        &self,
        id: &str,
        recipe: RootRecipe,
        layer: Option<&RootLayer>,
        populate: impl FnOnce(&Path) -> Result<()>,
    ) -> Result<()> {
        let stage = self.staging()?;
        let result = (|| {
            let tree = stage.join("tree");
            fs::create_dir(&tree)?;
            fs::set_permissions(&tree, Permissions::from_mode(0o755))?;
            populate(&tree)?;
            for point in MOUNT_POINTS {
                mount_point(&tree, point)?;
            }
            let rpm_inventory = self.observe_inventory(&tree, &stage)?;
            if let Some(layer) = layer {
                let expected: String = layer
                    .packages
                    .iter()
                    .map(|row| format!("{}\n", row.join("\t")))
                    .collect();
                if rpm_inventory != expected {
                    return Err(Error::Invalid(format!(
                        "installed RPMs of role {} differ from the RPM lock",
                        layer.role
                    )));
                }
            }
            let (entries, bytes) = seal(&tree)?;
            let receipt = RootReceipt {
                schema: 1,
                root: id.into(),
                rpm_sha256: plan::hash(rpm_inventory.as_bytes()),
                rpm_inventory,
                recipe,
                entries,
                bytes,
            };
            write_json_new(&stage.join("receipt.json"), &receipt)?;
            fs::set_permissions(&stage, Permissions::from_mode(0o700))?;
            fs::rename(&stage, self.root_path(id))?;
            File::open(self.path().join("rootfs"))?.sync_all()?;
            Ok(())
        })();
        if stage.exists() && !self.path().join("transactions/execution.json").exists() {
            tree::remove(&stage)?;
        }
        result
    }

    fn observe_inventory(&self, tree: &Path, stage: &Path) -> Result<String> {
        let argv = [
            "/bin/sh".into(),
            "-c".into(),
            INVENTORY.into(),
            "inventory".into(),
            ROW_FORMAT.into(),
        ];
        let tool = tree.join("bin/sh");
        if fs::symlink_metadata(&tool).is_err()
            && fs::symlink_metadata(tree.join("usr/bin/sh")).is_err()
        {
            return Ok(String::new());
        }
        let run = executor::checked(executor::sandbox(
            self,
            executor::Sandbox {
                root: tree,
                writable_root: false,
                stage: Some(stage),
                mounts: &[],
                output: None,
                repository: None,
                argv: &argv,
                env: &BTreeMap::new(),
                seconds: OBSERVE_SECONDS,
            },
        )?)?;
        inventory(&run.stdout)
    }

    /// Roots retained by pins and by the images the store retains.
    pub(crate) fn retained_roots(&self, images: &BTreeSet<String>) -> Result<BTreeSet<String>> {
        let mut keep = self.roots()?.rootfs;
        for image in images {
            keep.insert(root_identity(&image_recipe(image))?);
        }
        Ok(keep)
    }

    pub(crate) fn list_roots(&self) -> Result<Vec<String>> {
        list_names(&self.path().join("rootfs"))
    }
}

/// Sorted seven-column rows without gpg-pubkey, each newline-terminated.
fn inventory(stdout: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(stdout)
        .map_err(|_| Error::Invalid("root RPM inventory is not UTF-8".into()))?;
    let mut rows = BTreeSet::new();
    for line in text.lines() {
        let columns: Vec<_> = line.split('\t').collect();
        if columns.len() != 7 || columns.iter().any(|c| c.is_empty()) {
            return Err(Error::Invalid(
                "invalid seven-column root RPM inventory".into(),
            ));
        }
        if columns[0] != "gpg-pubkey" && !rows.insert(line) {
            return Err(Error::Invalid("duplicate root RPM inventory row".into()));
        }
    }
    Ok(rows.into_iter().map(|row| format!("{row}\n")).collect())
}

fn mount_point(tree: &Path, point: &str) -> Result<()> {
    let mut path = tree.to_path_buf();
    for part in point.split('/') {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(Error::Invalid(format!(
                    "root mount point /{point} is not a directory"
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&path)?;
                fs::set_permissions(&path, Permissions::from_mode(0o755))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Remove every write bit (bubblewrap mounts roots read-only as well) and
/// count what the root holds.
fn seal(path: &Path) -> Result<(u64, u64)> {
    let mut entries = 0;
    let mut bytes = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        entries += 1;
        if meta.is_dir() {
            let (e, b) = seal(&entry.path())?;
            entries += e;
            bytes += b;
        } else if meta.is_file() {
            bytes += meta.len();
            if meta.mode() & 0o222 != 0 {
                fs::set_permissions(entry.path(), Permissions::from_mode(meta.mode() & 0o7555))?;
            }
        }
    }
    let mode = fs::symlink_metadata(path)?.mode();
    fs::set_permissions(path, Permissions::from_mode(mode & 0o7555))?;
    Ok((entries, bytes))
}

/// Copy a sealed root into a writable staging tree, keeping symlinks, modes and
/// hard links; on btrfs and XFS the kernel shares file extents.
fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    let mut links = BTreeMap::new();
    copy_directory(source, target, &mut links)
}

fn copy_directory(
    source: &Path,
    target: &Path,
    links: &mut BTreeMap<(u64, u64), PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        let meta = fs::symlink_metadata(&from)?;
        let kind = meta.file_type();
        if kind.is_dir() {
            fs::create_dir(&to)?;
            copy_directory(&from, &to, links)?;
            fs::set_permissions(&to, Permissions::from_mode(meta.mode() & 0o7777 | 0o700))?;
        } else if kind.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&from)?, &to)?;
        } else if kind.is_file() {
            let key = (meta.dev(), meta.ino());
            if let Some(first) = links.get(&key).filter(|_| meta.nlink() > 1) {
                fs::hard_link(first, &to)?;
                continue;
            }
            fs::copy(&from, &to)?;
            fs::set_permissions(&to, Permissions::from_mode(meta.mode() & 0o7777 | 0o600))?;
            if meta.nlink() > 1 {
                links.insert(key, to);
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct Budget {
    entries: u64,
    bytes: u64,
}

impl Budget {
    fn charge(&mut self, bytes: u64) -> Result<()> {
        self.entries += 1;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.entries > MAX_ENTRIES || self.bytes > MAX_BYTES {
            return Err(Error::Invalid(
                "image root exceeds 2,000,000 entries or 64 GiB".into(),
            ));
        }
        Ok(())
    }
}

/// Hash the uncompressed layer stream while the tar reader consumes it.
struct Hashing<R> {
    inner: R,
    hash: Sha256,
}

impl<R: Read> Read for Hashing<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.hash.update(&buffer[..count]);
        Ok(count)
    }
}

/// The uncompressed tar stream of one layer, through the host's gzip or zstd
/// for compressed media types.
struct LayerStream {
    reader: Box<dyn Read>,
    child: Option<(Child, std::thread::JoinHandle<std::io::Result<u64>>)>,
}

fn open_layer(archive: &Path, layer: &Layer) -> Result<LayerStream> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(archive)?;
    file.seek(SeekFrom::Start(layer.offset))?;
    let blob = file.take(layer.bytes);
    let program = match layer.media_type.as_str() {
        "application/vnd.oci.image.layer.v1.tar"
        | "application/vnd.docker.image.rootfs.diff.tar" => {
            return Ok(LayerStream {
                reader: Box::new(blob),
                child: None,
            });
        }
        "application/vnd.oci.image.layer.v1.tar+gzip"
        | "application/vnd.docker.image.rootfs.diff.tar.gzip" => "/usr/bin/gzip",
        "application/vnd.oci.image.layer.v1.tar+zstd" => "/usr/bin/zstd",
        other => {
            return Err(Error::Invalid(format!(
                "unsupported layer media type {other}"
            )));
        }
    };
    let mut child = Command::new(program)
        .env_clear()
        .args(["-d", "-c"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error::Invalid(format!("cannot start {program}: {e}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| Error::Invalid("missing decompressor input".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Invalid("missing decompressor output".into()))?;
    let feeder = std::thread::spawn(move || {
        let mut blob = blob;
        let copied = std::io::copy(&mut blob, &mut stdin);
        drop(stdin);
        copied
    });
    Ok(LayerStream {
        reader: Box::new(stdout),
        child: Some((child, feeder)),
    })
}

fn apply_layer(archive: &Path, layer: &Layer, tree: &Path, budget: &mut Budget) -> Result<()> {
    let stream = open_layer(archive, layer)?;
    let mut tar = tar::Archive::new(Hashing {
        inner: stream.reader,
        hash: Sha256::new(),
    });
    let mut added = BTreeSet::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        apply_entry(&mut entry, tree, &mut added, budget)?;
    }
    let mut rest = tar.into_inner();
    std::io::copy(&mut rest, &mut std::io::sink())?;
    let digest = format!("sha256:{}", plan::encode_hex(&rest.hash.finalize()));
    if let Some((mut child, feeder)) = stream.child {
        let fed = feeder
            .join()
            .map_err(|_| Error::Invalid("layer feeder failed".into()))?;
        if !child.wait()?.success() || fed? != layer.bytes {
            return Err(Error::Corrupt("image layer does not decompress".into()));
        }
    }
    if digest != layer.diff_id {
        return Err(Error::Corrupt(format!(
            "uncompressed layer differs from {}",
            layer.diff_id
        )));
    }
    Ok(())
}

/// Validated relative entry path without empty, `.` or `..` components.
fn entry_path(raw: &Path) -> Result<Option<String>> {
    let text = raw
        .to_str()
        .ok_or_else(|| Error::Invalid("non-UTF8 layer path".into()))?;
    let mut parts = Vec::new();
    for part in text.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(Error::Invalid(format!("layer path escapes root: {text}"))),
            part => parts.push(part),
        }
    }
    if parts.iter().any(|p| p.bytes().any(|b| b == 0)) {
        return Err(Error::Invalid("layer path contains NUL".into()));
    }
    Ok((!parts.is_empty()).then(|| parts.join("/")))
}

/// Parent directories must be real directories inside the tree; missing ones
/// are created. A symlink parent would let a layer write outside the root.
fn parent_directory(tree: &Path, path: &str) -> Result<PathBuf> {
    let mut current = tree.to_path_buf();
    if let Some((parent, _)) = path.rsplit_once('/') {
        for part in parent.split('/') {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => {
                    return Err(Error::Invalid(format!(
                        "layer path traverses a non-directory: {path}"
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&current)?;
                    fs::set_permissions(&current, Permissions::from_mode(0o755))?;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(current)
}

/// An existing entry reached only through real directories of the tree.
fn existing_file(tree: &Path, path: &str) -> Result<PathBuf> {
    let mut current = tree.to_path_buf();
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    for part in parent.split('/').filter(|p| !p.is_empty()) {
        current.push(part);
        if !fs::symlink_metadata(&current)?.is_dir() {
            return Err(Error::Invalid(format!(
                "hard link target traverses a non-directory: {path}"
            )));
        }
    }
    Ok(current.join(name))
}

fn remove_existing(path: &Path, keep_directory: bool) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => {
            if !keep_directory {
                tree::remove(path)?;
            }
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn apply_entry<R: Read>(
    entry: &mut tar::Entry<'_, R>,
    tree: &Path,
    added: &mut BTreeSet<String>,
    budget: &mut Budget,
) -> Result<()> {
    let Some(path) = entry_path(&entry.path()?)? else {
        return Ok(());
    };
    budget.charge(entry.size())?;
    let parent = parent_directory(tree, &path)?;
    let name = path.rsplit('/').next().unwrap_or(&path);
    if name == ".wh..wh..opq" {
        let directory = path.rsplit_once('/').map_or("", |(p, _)| p);
        return clear_opaque(&parent, directory, added);
    }
    if let Some(hidden) = name.strip_prefix(".wh.") {
        // A whiteout hides lower layers only: an entry this layer added stays,
        // whichever order the archive lists them in.
        entry_path(Path::new(hidden))?;
        let directory = path.rsplit_once('/').map_or("", |(p, _)| p);
        let hidden_path = if directory.is_empty() {
            hidden.to_owned()
        } else {
            format!("{directory}/{hidden}")
        };
        if added.contains(&hidden_path) {
            return Ok(());
        }
        return remove_existing(&parent.join(hidden), false);
    }
    let target = parent.join(name);
    let mode = entry.header().mode()? & 0o1777;
    let kind = entry.header().entry_type();
    if kind.is_dir() {
        remove_existing(&target, true)?;
        if !target.exists() {
            fs::create_dir(&target)?;
        }
        fs::set_permissions(&target, Permissions::from_mode(mode | 0o700))?;
    } else if kind.is_file()
        || kind == tar::EntryType::GNUSparse
        || kind == tar::EntryType::Continuous
    {
        remove_existing(&target, false)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)?;
        std::io::copy(entry, &mut file)?;
        file.set_permissions(Permissions::from_mode(mode | 0o600))?;
        if let Ok(mtime) = entry.header().mtime() {
            file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime))?;
        }
    } else if kind.is_symlink() {
        let link = entry
            .link_name()?
            .ok_or_else(|| Error::Invalid("symlink without target".into()))?;
        remove_existing(&target, false)?;
        std::os::unix::fs::symlink(link, &target)?;
    } else if kind.is_hard_link() {
        let link = entry
            .link_name()?
            .ok_or_else(|| Error::Invalid("hard link without target".into()))?;
        let source =
            entry_path(&link)?.ok_or_else(|| Error::Invalid("hard link to the root".into()))?;
        let source = existing_file(tree, &source)?;
        // Layers hard-link regular files and, in packaged firmware, symlinks;
        // linking never follows the symlink.
        if fs::symlink_metadata(&source)?.is_dir() {
            return Err(Error::Invalid(format!(
                "hard link target is a directory: {path}"
            )));
        }
        remove_existing(&target, false)?;
        fs::hard_link(&source, &target)?;
    } else if matches!(
        kind,
        tar::EntryType::Char | tar::EntryType::Block | tar::EntryType::Fifo
    ) {
        // Roots omit special nodes, but the node still replaces the lower entry.
        remove_existing(&target, false)?;
    } else {
        return Err(Error::Invalid(format!("unsupported layer entry: {path}")));
    }
    added.insert(path);
    Ok(())
}

/// An opaque marker hides everything lower layers put in its directory;
/// entries this layer already added stay.
fn clear_opaque(directory: &Path, relative: &str, added: &BTreeSet<String>) -> Result<()> {
    for name in list_names(directory)? {
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if !added.contains(&path) {
            remove_existing(&directory.join(&name), false)?;
        }
    }
    Ok(())
}
