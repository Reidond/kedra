//! Persistent root-only OCI layout that lets each policy-enforcing
//! authentication skip image layers an earlier one already transferred.
//!
//! It is a transfer cache, never an authority. Every authentication still runs
//! skopeo's policy-enforcing copy from the registry, so the signature is
//! verified on each run. The copy rewrites the manifest and config from the
//! registry, the helper hashes the manifest it reads back, and containers/image
//! checks the config against that manifest. skopeo 1.22.3 (Fedora 44, vendoring
//! containers/image v5.39.3) skips a layer whose blob path merely exists
//! (`oci/layout/oci_dest.go`, `TryReusingBlobWithOptions`) without rehashing it.
//! No layer is read by the helper or deployed from here: bootc fetches and
//! verifies the exact digest itself. Unexpected state is removed and
//! downloaded again, never followed or repaired in place.
use super::{DIRECTORY, Result, hash};
use rustix::fs::{self, AtFlags, Dir, FileType, Mode, OFlags, ResolveFlags, Stat};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;
use sysroot_core::deployment;

const NAME: &str = "verified-oci";
const LAYOUT: &str = "oci-layout";
const INDEX: &str = "index.json";
/// The helper's own index replacement; a leftover means an interrupted prune.
const REPLACEMENT: &str = "index.json.sysroot-new";
const BLOBS: &str = "blobs";
const ALGORITHM: &str = "sha256";
/// Prefix of containers/image `os.CreateTemp(dir, "oci-put-blob")` files that
/// an interrupted copy can leave; complete blobs are renamed out of them.
const TEMPORARY: &str = "oci-put-blob";
const REF_NAME: &str = "org.opencontainers.image.ref.name";
const INDEX_TYPE: &str = "application/vnd.oci.image.index.v1+json";
const MANIFEST_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
const MAX_JSON: usize = 1_048_576;
const MAX_REFS: usize = 64;
const MAX_ENTRIES: usize = 65_536;

/// The layout cannot be used as it is; the only remedy is to discard it.
#[derive(Debug)]
enum Untrusted {
    Io(std::io::Error),
    Json(serde_json::Error),
    State(&'static str),
}
impl fmt::Display for Untrusted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O: {error}"),
            Self::Json(error) => write!(f, "malformed metadata: {error}"),
            Self::State(reason) => f.write_str(reason),
        }
    }
}
impl From<std::io::Error> for Untrusted {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<rustix::io::Errno> for Untrusted {
    fn from(error: rustix::io::Errno) -> Self {
        Self::Io(error.into())
    }
}
impl From<serde_json::Error> for Untrusted {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
type Checked<T> = std::result::Result<T, Untrusted>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    #[serde(rename = "imageLayoutVersion")]
    version: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Index {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "mediaType", default)]
    media_type: Option<String>,
    #[serde(default)]
    manifests: Option<Vec<Descriptor>>,
    #[serde(default, skip_serializing)]
    annotations: Option<BTreeMap<String, String>>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
    #[serde(default)]
    annotations: BTreeMap<String, String>,
}
/// Only the digests of signed manifest content; other fields are not interpreted here.
#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    config: Blob,
    layers: Vec<Blob>,
}
#[derive(Deserialize)]
struct Blob {
    digest: String,
}

struct Found {
    index: Option<Index>,
    blobs: BTreeSet<String>,
    temporaries: Vec<String>,
}

fn tag(digest: &str) -> Option<String> {
    deployment::digest_valid(digest).then(|| format!("sha256-{}", &digest[7..]))
}

/// The skopeo `oci:` reference that names one digest inside the layout.
pub(super) fn reference(digest: &str) -> Result<String> {
    let tag = tag(digest).ok_or("invalid image digest for the verified image cache")?;
    Ok(format!("oci:{DIRECTORY}/{NAME}:{tag}"))
}

/// Validate the layout before a copy, or replace it with an empty private one.
/// Refs outside `keep`, and blobs only they reference, go first, so stale
/// images never hold space during a transfer that can fail (for example on a
/// full disk). Blobs no ref names stay: they can be complete layers of an
/// interrupted copy that this copy reuses; the prune after a successful copy
/// collects the rest.
pub(super) fn prepare(keep: &BTreeSet<String>) -> Result<()> {
    collect(keep, false)
}

/// After a successful copy keep only the refs in `keep` and the blobs their
/// manifests reference.
pub(super) fn prune(keep: &BTreeSet<String>) -> Result<()> {
    collect(keep, true)
}

/// The index is replaced atomically before any blob is deleted, so an
/// interrupted run leaves only unreferenced blobs for the next prune.
fn collect(keep: &BTreeSet<String>, unreferenced: bool) -> Result<()> {
    let parent = parent()?;
    let outcome = (|| -> Checked<()> {
        let layout = open(&parent)?;
        let found = scan(&layout)?;
        remove(&layout, &found.temporaries)?;
        let Some(index) = found.index else {
            // Before a copy: a new layout, or a first copy interrupted before its index.
            if unreferenced {
                return Err(Untrusted::State("layout has no index after a copy"));
            }
            return Ok(());
        };
        let (kept, dropped): (Vec<_>, Vec<_>) = index
            .manifests
            .unwrap_or_default()
            .into_iter()
            .partition(|descriptor| keep.contains(&descriptor.digest));
        if !unreferenced && dropped.is_empty() {
            return Ok(());
        }
        let blobs = subdirectory(&subdirectory(&layout, BLOBS)?, ALGORITHM)?;
        let referenced = references(&blobs, &kept)?;
        // Before a copy only blobs of dropped refs go; after it every unreferenced one.
        let stale = if unreferenced {
            None
        } else {
            Some(references(&blobs, &dropped)?)
        };
        write_index(
            &layout,
            &Index {
                schema_version: 2,
                media_type: Some(INDEX_TYPE.to_owned()),
                manifests: Some(kept),
                annotations: None,
            },
        )?;
        // Only names the scan found as regular files in blobs/sha256.
        for name in found.blobs.difference(&referenced) {
            if stale.as_ref().is_none_or(|stale| stale.contains(name)) {
                fs::unlinkat(&blobs, name.as_str(), AtFlags::empty())?;
            }
        }
        fs::fsync(&blobs)?;
        Ok(())
    })();
    match outcome {
        Ok(()) => Ok(()),
        Err(reason) => replace(&parent, &reason),
    }
}

/// Blob names of the given index entries: each manifest, its config and layers.
fn references(blobs: &OwnedFd, descriptors: &[Descriptor]) -> Checked<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for descriptor in descriptors {
        // validate() accepted only `sha256:` and 64 lowercase hex digits.
        let name = &descriptor.digest[7..];
        let bytes = read(blobs, name)?;
        if hash(&bytes) != name || bytes.len() as u64 != descriptor.size {
            return Err(Untrusted::State("cached manifest differs from its digest"));
        }
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        if manifest.schema_version != 2 {
            return Err(Untrusted::State("cached manifest schema is unsupported"));
        }
        names.insert(name.to_owned());
        for blob in std::iter::once(&manifest.config).chain(&manifest.layers) {
            if !deployment::digest_valid(&blob.digest) {
                return Err(Untrusted::State("cached manifest names an invalid blob"));
            }
            names.insert(blob.digest[7..].to_owned());
        }
    }
    Ok(names)
}

/// Discard the layout after it returned data inconsistent with the copy that
/// just verified it. The next authentication downloads the whole image.
pub(super) fn discard(reason: &'static str) -> Result<()> {
    replace(&parent()?, &Untrusted::State(reason))
}

/// Remove the layout without following links and recreate it empty.
fn replace(parent: &OwnedFd, reason: &Untrusted) -> Result<()> {
    eprintln!(
        "sysroot-helper: discarding the verified image cache ({reason}); \
         the whole image is downloaded again"
    );
    match fs::statat(parent, NAME, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::Directory => {
            // The parent is the validated root-only management directory, and
            // remove_dir_all removes symlinks inside it without following them.
            std::fs::remove_dir_all(Path::new(DIRECTORY).join(NAME))?;
        }
        Ok(_) => fs::unlinkat(parent, NAME, AtFlags::empty())?,
        Err(rustix::io::Errno::NOENT) => (),
        Err(error) => return Err(error.into()),
    }
    fs::fsync(parent)?;
    open(parent).map_err(|error| format!("cannot recreate the verified image cache: {error}"))?;
    Ok(())
}

fn parent() -> Result<OwnedFd> {
    let parent = fs::openat2(
        fs::CWD,
        DIRECTORY,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let stat = fs::fstat(&parent)?;
    if stat.st_uid != 0 || stat.st_mode & 0o777 != 0o700 {
        return Err("management directory must be root-owned and mode 0700".into());
    }
    Ok(parent)
}

/// The layout root itself: a root-owned 0700 directory, created if absent.
fn open(parent: &OwnedFd) -> Checked<OwnedFd> {
    let created = match fs::mkdirat(parent, NAME, Mode::from_raw_mode(0o700)) {
        Ok(()) => true,
        Err(rustix::io::Errno::EXIST) => false,
        Err(error) => return Err(error.into()),
    };
    let layout = fs::openat(
        parent,
        NAME,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    if created {
        fs::fchmod(&layout, Mode::from_raw_mode(0o700))?;
        fs::fsync(parent)?;
    }
    let stat = fs::fstat(&layout)?;
    if stat.st_uid != 0 || stat.st_mode & 0o7777 != 0o700 {
        return Err(Untrusted::State(
            "layout is not a root-owned mode 0700 directory",
        ));
    }
    Ok(layout)
}

/// Every entry is known, root-owned, not writable by others and never a link.
fn scan(layout: &OwnedFd) -> Checked<Found> {
    let mut found = Found {
        index: None,
        blobs: BTreeSet::new(),
        temporaries: Vec::new(),
    };
    let mut blobs = false;
    for name in entries(layout)? {
        match name.as_str() {
            LAYOUT => {
                let file: LayoutFile = serde_json::from_slice(&read(layout, LAYOUT)?)?;
                if file.version != "1.0.0" {
                    return Err(Untrusted::State("unsupported OCI layout version"));
                }
            }
            INDEX => found.index = Some(serde_json::from_slice(&read(layout, INDEX)?)?),
            BLOBS => blobs = true,
            other if other == REPLACEMENT || temporary(other) => {
                regular(layout, other)?;
                found.temporaries.push(other.to_owned());
            }
            _ => return Err(Untrusted::State("unexpected entry in the layout")),
        }
    }
    if blobs {
        let directory = subdirectory(layout, BLOBS)?;
        let algorithms = entries(&directory)?;
        if algorithms.iter().any(|name| name != ALGORITHM) {
            return Err(Untrusted::State("unexpected blob algorithm directory"));
        }
        if !algorithms.is_empty() {
            let algorithm = subdirectory(&directory, ALGORITHM)?;
            for name in entries(&algorithm)? {
                if !deployment::digest_valid(&format!("sha256:{name}")) {
                    return Err(Untrusted::State("unexpected blob name"));
                }
                regular(&algorithm, &name)?;
                found.blobs.insert(name);
            }
        }
    }
    if let Some(index) = &found.index {
        validate(index)?;
    }
    Ok(found)
}

/// Only entries this helper's own copies create: one ref per digest, named by it.
fn validate(index: &Index) -> Checked<()> {
    let manifests = index.manifests.as_deref().unwrap_or_default();
    if index.schema_version != 2
        || index.media_type.as_deref().is_some_and(|t| t != INDEX_TYPE)
        || index.annotations.as_ref().is_some_and(|a| !a.is_empty())
        || manifests.len() > MAX_REFS
    {
        return Err(Untrusted::State("unexpected layout index"));
    }
    let mut digests = BTreeSet::new();
    for descriptor in manifests {
        let expected = tag(&descriptor.digest)
            .ok_or(Untrusted::State("layout index names an invalid digest"))?;
        if descriptor.media_type != MANIFEST_TYPE
            || descriptor.size == 0
            || descriptor.size > MAX_JSON as u64
            || descriptor.annotations.len() != 1
            || descriptor.annotations.get(REF_NAME) != Some(&expected)
            || !digests.insert(descriptor.digest.as_str())
        {
            return Err(Untrusted::State("unexpected layout index entry"));
        }
    }
    Ok(())
}

fn temporary(name: &str) -> bool {
    name.strip_prefix(TEMPORARY).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.len() <= 20 && suffix.bytes().all(|b| b.is_ascii_digit())
    })
}

fn entries(directory: &OwnedFd) -> Checked<Vec<String>> {
    let mut names = Vec::new();
    for entry in Dir::read_from(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if names.len() == MAX_ENTRIES {
            return Err(Untrusted::State("too many layout entries"));
        }
        names.push(
            std::str::from_utf8(name)
                .map_err(|_| Untrusted::State("non-UTF-8 layout entry"))?
                .to_owned(),
        );
    }
    Ok(names)
}

fn safe(stat: &Stat, kind: FileType) -> bool {
    FileType::from_raw_mode(stat.st_mode) == kind
        && stat.st_uid == 0
        && stat.st_mode & 0o022 == 0
        && (kind == FileType::Directory || stat.st_nlink == 1)
}

fn regular(directory: &OwnedFd, name: &str) -> Checked<Stat> {
    let stat = fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)?;
    if !safe(&stat, FileType::RegularFile) {
        return Err(Untrusted::State("unsafe layout file"));
    }
    Ok(stat)
}

fn subdirectory(directory: &OwnedFd, name: &str) -> Checked<OwnedFd> {
    let child = fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    if !safe(&fs::fstat(&child)?, FileType::Directory) {
        return Err(Untrusted::State("unsafe layout directory"));
    }
    Ok(child)
}

fn read(directory: &OwnedFd, name: &str) -> Checked<Vec<u8>> {
    let descriptor = fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let stat = fs::fstat(&descriptor)?;
    if !safe(&stat, FileType::RegularFile)
        || stat.st_size < 0
        || stat.st_size as u64 > MAX_JSON as u64
    {
        return Err(Untrusted::State("unsafe or oversized layout metadata"));
    }
    let mut bytes = Vec::new();
    File::from(descriptor)
        .take(MAX_JSON as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_JSON {
        return Err(Untrusted::State("oversized layout metadata"));
    }
    Ok(bytes)
}

fn remove(layout: &OwnedFd, names: &[String]) -> Checked<()> {
    for name in names {
        fs::unlinkat(layout, name.as_str(), AtFlags::empty())?;
    }
    Ok(())
}

fn write_index(layout: &OwnedFd, index: &Index) -> Checked<()> {
    let bytes = serde_json::to_vec(index)?;
    let descriptor = fs::openat(
        layout,
        REPLACEMENT,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o644),
    )?;
    fs::fchmod(&descriptor, Mode::from_raw_mode(0o644))?;
    let mut file = File::from(descriptor);
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::renameat(layout, REPLACEMENT, layout, INDEX)?;
    fs::fsync(layout)?;
    Ok(())
}
