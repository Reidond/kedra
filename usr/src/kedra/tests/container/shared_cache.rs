//! Shared cache of content-addressed build artifacts for every checkout of
//! this repository, so that worktrees reuse one QEMU runtime, one VM base disk
//! per image and one engine store instead of building their own. It is the one
//! location outside a checkout's `target/` that AGENTS.md "Artifacts stay in
//! the checkout" allows.
//!
//! The cache is `<primary checkout>/.cache/kedra`, where the primary checkout is
//! the parent of `git rev-parse --git-common-dir`; `KEDRA_SHARED_CACHE` (an
//! absolute directory) overrides it. Layout:
//!
//! - `entries/<kind>/<key>/`: one sealed (read-only) entry: `content/` and its
//!   `receipt.json`. The key is the SHA-256 of the kind and the canonical JSON
//!   of the entry's declared inputs; the receipt records those inputs and the
//!   type, mode, size and SHA-256 of every file.
//! - `locks/`: one file lock per entry key and per shared store.
//! - `work/<kind>-<key>/`: that key's build area, used only under its lock and
//!   deleted after a successful build.
//! - `roots/<checkout id>/`: what each checkout uses; `checkout.json` names the
//!   checkout. `kedra-lab gc` deletes entries and stores no checkout roots.
//! - `stores/`: engine stores every checkout shares; they keep their own
//!   content addressing, locks and recovery.
//!
//! An entry is built once: the builder holds the key's lock exclusively, builds
//! in the work area, seals the result, writes the receipt and publishes it with
//! one rename. Another checkout asking for the same key waits on the lock and
//! then reuses the entry. Every use verifies the entry against its receipt
//! while holding the lock; a corrupt, tampered or unknown entry is refused and
//! left in place, never repaired. GC takes the same lock, so it never deletes
//! an entry that a checkout is building, verifying or rooting.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{Result, invalid};

const SCHEMA: u32 = 1;
const RECEIPT: &str = "receipt.json";
const CONTENT: &str = "content";
const RECEIPT_LIMIT: u64 = 64 * 1024 * 1024;
/// Top-level directories of the cache; anything else there is reported as unknown.
const LAYOUT: [&str; 5] = ["entries", "locks", "work", "roots", "stores"];

/// What an entry holds; each kind has its own declared inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// The native QEMU/HVF/ANGLE runtime (`kedra-lab vm tools prepare`).
    QemuRuntime,
    /// A prepared VM base disk (`kedra-lab vm image`).
    VmDisk,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::QemuRuntime, Kind::VmDisk];

    pub fn name(self) -> &'static str {
        match self {
            Kind::QemuRuntime => "qemu-runtime",
            Kind::VmDisk => "vm-disk",
        }
    }

    pub fn parse(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// The shared cache of this repository's checkouts.
pub struct Cache {
    root: PathBuf,
}

/// A published, verified entry.
pub struct Entry {
    pub kind: Kind,
    pub key: String,
    path: PathBuf,
}

impl Entry {
    /// The entry directory (`receipt.json` and `content/`).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The artifact itself.
    pub fn content(&self) -> PathBuf {
        self.path.join(CONTENT)
    }
}

/// Where an entry's builder writes: it creates `content` (absent when it is
/// called) and may keep a build tree in `work`, which survives a failed
/// attempt and is deleted after the entry is published.
pub struct Build {
    pub content: PathBuf,
    pub work: PathBuf,
}

/// A held file lock; dropping it releases the lock.
pub struct Lock {
    _file: File,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Node {
    Dir {
        mode: u32,
    },
    File {
        mode: u32,
        size: u64,
        sha256: String,
    },
    Symlink {
        target: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u32,
    kind: String,
    key: String,
    inputs: Value,
    /// Unix seconds of publication.
    created: u64,
    /// Checkout id of the builder.
    creator: String,
    /// Sum of the file sizes.
    bytes: u64,
    files: BTreeMap<String, Node>,
}

/// `roots/<checkout id>/checkout.json`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckoutRecord {
    schema: u32,
    path: PathBuf,
}

/// What `kedra-lab gc` did or, with `dry_run`, would do.
#[derive(Default)]
pub struct GcReport {
    pub lines: Vec<String>,
    /// Bytes on disk of everything deleted (or deletable).
    pub freed: u64,
}

impl GcReport {
    fn note(&mut self, line: String) {
        self.lines.push(line);
    }
}

/// Every checkout's roots, read by GC.
#[derive(Default)]
struct Rooted {
    entries: BTreeMap<(String, String), BTreeSet<String>>,
    stores: BTreeMap<String, BTreeSet<String>>,
}

impl Cache {
    /// Open the shared cache: `KEDRA_SHARED_CACHE`, or `.cache/kedra` in the
    /// primary checkout of this checkout's repository.
    pub fn locate() -> Result<Self> {
        if let Some(path) = std::env::var_os("KEDRA_SHARED_CACHE").filter(|v| !v.is_empty()) {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err(invalid("KEDRA_SHARED_CACHE must be an absolute directory"));
            }
            return Self::open(path);
        }
        let output = Command::new("git")
            .arg("-C")
            .arg(crate::repository_root())
            .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .output()?;
        if !output.status.success() {
            return Err(invalid(format!(
                "git rev-parse --git-common-dir failed in {}: {}; set KEDRA_SHARED_CACHE",
                crate::repository_root().display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let mut text = output.stdout;
        while text.last().is_some_and(|byte| byte.is_ascii_whitespace()) {
            text.pop();
        }
        let common = PathBuf::from(std::ffi::OsString::from_vec(text));
        match (common.file_name(), common.parent()) {
            (Some(name), Some(primary)) if name == ".git" => {
                Self::open(primary.join(".cache").join("kedra"))
            }
            _ => Err(invalid(format!(
                "{} is not the .git directory of a primary checkout; set KEDRA_SHARED_CACHE",
                common.display()
            ))),
        }
    }

    fn open(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            fs::create_dir_all(&root)?;
            fs::set_permissions(&root, Permissions::from_mode(0o700))?;
        }
        let root = root.canonicalize()?;
        for name in LAYOUT {
            fs::create_dir_all(root.join(name))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Docker objects of this cache (the engine store volume) carry
    /// `kedra.cache=<id>`: the first 12 hex digits of the SHA-256 of its path.
    pub fn id(&self) -> String {
        hex(&Sha256::digest(self.root.as_os_str().as_encoded_bytes())[..6])
    }

    fn entry(&self, kind: Kind, key: &str) -> Entry {
        Entry {
            kind,
            key: key.to_owned(),
            path: self.root.join("entries").join(kind.name()).join(key),
        }
    }

    fn lock_path(&self, name: &str) -> PathBuf {
        self.root.join("locks").join(format!("{name}.lock"))
    }

    /// A lock file other tools take to serialize work across checkouts
    /// (prepare-disk.py's nested Podman builds).
    pub fn lock_file(&self, name: &str) -> PathBuf {
        self.lock_path(name)
    }

    /// The entry for `inputs`, built by `build` unless another checkout already
    /// published it; verified and rooted by this checkout either way.
    pub fn ensure(
        &self,
        kind: Kind,
        inputs: &Value,
        build: impl FnOnce(&Build) -> Result<()>,
    ) -> Result<Entry> {
        let key = key(kind, inputs);
        let entry = self.entry(kind, &key);
        let what = format!("shared {} {}", kind.name(), &key[..12]);
        let _lock = acquire(&self.lock_path(&slot(kind, &key)), false, &what)?;
        if entry.path.symlink_metadata().is_ok() {
            verify(&entry, inputs)?;
            self.add_root(&entry)?;
            eprintln!("kedra-lab: reusing {what}: {}", entry.path.display());
            return Ok(entry);
        }
        let work = self.root.join("work").join(slot(kind, &key));
        let stage = work.join("stage");
        // A failed or interrupted attempt's stage; its build tree stays for the retry.
        if stage.symlink_metadata().is_ok() {
            crate::remove_tree(&stage)?;
        }
        fs::create_dir_all(&stage)?;
        let started = Instant::now();
        eprintln!("kedra-lab: building {what} in {}", work.display());
        build(&Build {
            content: stage.join(CONTENT),
            work: work.clone(),
        })?;
        self.publish(&entry, inputs, &stage)?;
        crate::remove_tree(&work)?;
        self.add_root(&entry)?;
        eprintln!(
            "kedra-lab: published {what} after {:.0}s: {}",
            started.elapsed().as_secs_f64(),
            entry.path.display()
        );
        Ok(entry)
    }

    /// The verified entry for `inputs` if one is published (rooted by this
    /// checkout), or none. Waits while another checkout builds it.
    pub fn find(&self, kind: Kind, inputs: &Value) -> Result<Option<Entry>> {
        let key = key(kind, inputs);
        let entry = self.entry(kind, &key);
        let what = format!("shared {} {}", kind.name(), &key[..12]);
        let _lock = acquire(&self.lock_path(&slot(kind, &key)), true, &what)?;
        if entry.path.symlink_metadata().is_err() {
            return Ok(None);
        }
        verify(&entry, inputs)?;
        self.add_root(&entry)?;
        Ok(Some(entry))
    }

    /// The path the entry for `inputs` has once published; not verified.
    pub fn content_path(&self, kind: Kind, inputs: &Value) -> PathBuf {
        self.entry(kind, &key(kind, inputs)).content()
    }

    fn publish(&self, entry: &Entry, inputs: &Value, stage: &Path) -> Result<()> {
        let content = stage.join(CONTENT);
        if !content.symlink_metadata().is_ok_and(|m| m.is_dir()) {
            return Err(invalid(format!(
                "the {} builder left no content directory",
                entry.kind.name()
            )));
        }
        seal(&content)?;
        let files = manifest(&content)?;
        let receipt = Receipt {
            schema: SCHEMA,
            kind: entry.kind.name().to_owned(),
            key: entry.key.clone(),
            inputs: canonical(inputs),
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs())
                .unwrap_or_default(),
            creator: crate::checkout_id().to_owned(),
            bytes: files
                .values()
                .map(|node| match node {
                    Node::File { size, .. } => *size,
                    _ => 0,
                })
                .sum(),
            files,
        };
        let data = serde_json::to_vec_pretty(&receipt).map_err(|e| invalid(e.to_string()))?;
        let path = stage.join(RECEIPT);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o444)
            .open(&path)?;
        file.write_all(&data)?;
        file.sync_all()?;
        File::open(stage)?.sync_all()?;
        if let Some(parent) = entry.path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Moving a directory to another parent needs it writable; seal it after.
        fs::rename(stage, &entry.path)?;
        fs::set_permissions(&entry.path, Permissions::from_mode(0o555))?;
        if let Some(parent) = entry.path.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    }

    fn checkout_roots(&self) -> PathBuf {
        self.root.join("roots").join(crate::checkout_id())
    }

    fn register_checkout(&self) -> Result<PathBuf> {
        let directory = self.checkout_roots();
        fs::create_dir_all(&directory)?;
        let path = crate::repository_root();
        let record = CheckoutRecord {
            schema: SCHEMA,
            path: path.canonicalize().unwrap_or(path),
        };
        write_atomic(
            &directory.join("checkout.json"),
            &serde_json::to_vec_pretty(&record).map_err(|e| invalid(e.to_string()))?,
        )?;
        Ok(directory)
    }

    /// Callers hold the entry's lock.
    fn add_root(&self, entry: &Entry) -> Result<()> {
        let directory = self.register_checkout()?;
        let record =
            serde_json::json!({"schema": SCHEMA, "kind": entry.kind.name(), "key": entry.key});
        write_atomic(
            &directory.join(format!("{}.json", slot(entry.kind, &entry.key))),
            record.to_string().as_bytes(),
        )
    }

    /// Drop this checkout's roots on entries of these kinds; returns the
    /// dropped `<kind>/<key>` names. The entries stay until `kedra-lab gc`.
    pub fn unroot(&self, kinds: &[Kind]) -> Result<Vec<String>> {
        let mut dropped = Vec::new();
        for (name, path) in root_files(&self.checkout_roots())? {
            if let Some((kind, key)) = parse_slot(&name)
                && kinds.contains(&kind)
            {
                fs::remove_file(path)?;
                dropped.push(format!("{}/{key}", kind.name()));
            }
        }
        Ok(dropped)
    }

    /// The lock of a shared store: builds hold it shared for their whole run;
    /// creating, recovering, pruning and removing the store hold it exclusively.
    pub fn store_lock(&self, store: &str, shared: bool) -> Result<Lock> {
        acquire(
            &self.lock_path(&format!("store-{store}")),
            shared,
            &format!("shared engine store {store}"),
        )
    }

    /// The exclusive store lock if no build holds the store.
    pub fn try_store_lock(&self, store: &str) -> Result<Option<Lock>> {
        try_acquire(&self.lock_path(&format!("store-{store}")), false)
    }

    /// Directory of a host engine store.
    pub fn store_path(&self, store: &str) -> PathBuf {
        self.root.join("stores").join(store)
    }

    /// Record what this checkout's latest build used from a shared store; the
    /// record roots the store and is what a prune keeps.
    pub fn root_store(&self, store: &str, record: &Value) -> Result<()> {
        let directory = self.register_checkout()?;
        write_atomic(
            &directory.join(format!("store-{store}.json")),
            &serde_json::to_vec_pretty(record).map_err(|e| invalid(e.to_string()))?,
        )
    }

    /// This checkout's record on a shared store, if it roots it.
    pub fn store_record(&self, store: &str) -> Result<Option<Value>> {
        let path = self.checkout_roots().join(format!("store-{store}.json"));
        match read_limited(&path) {
            Ok(data) => {
                Ok(Some(serde_json::from_slice(&data).map_err(|e| {
                    invalid(format!("{}: {e}", path.display()))
                })?))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Drop this checkout's root on a shared store; true if it had one.
    pub fn unroot_store(&self, store: &str) -> Result<bool> {
        match fs::remove_file(self.checkout_roots().join(format!("store-{store}.json"))) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// Every checkout's record on a shared store, this checkout's included.
    pub fn store_records(&self, store: &str) -> Result<Vec<Value>> {
        let mut records = Vec::new();
        for directory in list(&self.root.join("roots"))? {
            let path = self
                .root
                .join("roots")
                .join(&directory)
                .join(format!("store-{store}.json"));
            match read_limited(&path) {
                Ok(data) => records.push(
                    serde_json::from_slice(&data)
                        .map_err(|e| invalid(format!("{}: {e}", path.display())))?,
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(records)
    }

    /// Whether any checkout roots a shared store.
    pub fn store_rooted(&self, store: &str) -> Result<bool> {
        Ok(self
            .rooted(&mut GcReport::default(), true)?
            .stores
            .contains_key(store))
    }

    /// Delete one entry, even a corrupt one, and every checkout's root on it.
    pub fn remove(&self, kind: Kind, key: &str) -> Result<u64> {
        if !is_hex(key, 64) {
            return Err(invalid("an entry key is 64 lowercase hex digits"));
        }
        let entry = self.entry(kind, key);
        let what = format!("shared {} {}", kind.name(), &key[..12]);
        let _lock = acquire(&self.lock_path(&slot(kind, key)), false, &what)?;
        let mut freed = 0;
        if entry.path.symlink_metadata().is_ok() {
            freed = usage(&entry.path)?;
            crate::remove_tree(&entry.path)?;
        }
        for directory in list(&self.root.join("roots"))? {
            let root = self
                .root
                .join("roots")
                .join(directory)
                .join(format!("{}.json", slot(kind, key)));
            if root.symlink_metadata().is_ok() {
                fs::remove_file(root)?;
            }
        }
        Ok(freed)
    }

    /// Delete what no checkout roots: entries, abandoned build areas and host
    /// stores, after dropping the roots of checkouts that no longer exist.
    /// Unknown or corrupt entries are reported and left in place.
    pub fn gc(&self, dry_run: bool) -> Result<GcReport> {
        let mut report = GcReport::default();
        for name in list(&self.root)? {
            if !LAYOUT.contains(&name.as_str()) {
                report.note(format!(
                    "unknown {}; left in place",
                    self.root.join(name).display()
                ));
            }
        }
        let rooted = self.rooted(&mut report, dry_run)?;
        self.collect_entries(&rooted, dry_run, &mut report)?;
        self.collect_work(dry_run, &mut report)?;
        self.collect_host_stores(&rooted, dry_run, &mut report)?;
        Ok(report)
    }

    /// Every live checkout's roots; drops those of checkouts that no longer
    /// exist unless `dry_run`.
    fn rooted(&self, report: &mut GcReport, dry_run: bool) -> Result<Rooted> {
        let mut rooted = Rooted::default();
        for checkout in list(&self.root.join("roots"))? {
            let directory = self.root.join("roots").join(&checkout);
            match checkout_state(&checkout, &directory) {
                CheckoutState::Live => {}
                CheckoutState::Gone(path) => {
                    report.note(format!(
                        "checkout {} no longer exists; {} its roots",
                        path.display(),
                        if dry_run { "would drop" } else { "dropped" }
                    ));
                    if !dry_run {
                        crate::remove_tree(&directory)?;
                    }
                    continue;
                }
                CheckoutState::Unknown(reason) => report.note(format!(
                    "roots {}: {reason}; its roots are kept",
                    directory.display()
                )),
            }
            for (name, _) in root_files(&directory)? {
                if let Some(store) = name.strip_prefix("store-") {
                    rooted
                        .stores
                        .entry(store.to_owned())
                        .or_default()
                        .insert(checkout.clone());
                } else if let Some((kind, key)) = parse_slot(&name) {
                    rooted
                        .entries
                        .entry((kind.name().to_owned(), key))
                        .or_default()
                        .insert(checkout.clone());
                } else if name != "checkout" {
                    report.note(format!("unknown root {name} in {}", directory.display()));
                }
            }
        }
        Ok(rooted)
    }

    fn collect_entries(&self, rooted: &Rooted, dry_run: bool, report: &mut GcReport) -> Result<()> {
        let entries = self.root.join("entries");
        for kind_name in list(&entries)? {
            let Some(kind) = Kind::parse(&kind_name) else {
                report.note(format!("unknown entry kind {kind_name}; left in place"));
                continue;
            };
            for key in list(&entries.join(&kind_name))? {
                let roots = rooted.entries.get(&(kind_name.clone(), key.clone()));
                self.collect_entry(kind, &key, roots, dry_run, report)?;
            }
        }
        Ok(())
    }

    fn collect_entry(
        &self,
        kind: Kind,
        key: &str,
        roots: Option<&BTreeSet<String>>,
        dry_run: bool,
        report: &mut GcReport,
    ) -> Result<()> {
        let entry = self.entry(kind, key);
        let label = format!("{}/{key}", kind.name());
        if !is_hex(key, 64) {
            report.note(format!("unknown entry {label}; left in place"));
            return Ok(());
        }
        if let Some(roots) = roots {
            let names: Vec<_> = roots.iter().map(String::as_str).collect();
            report.note(format!(
                "keep {label} ({}; rooted by {})",
                size(usage(&entry.path)?),
                names.join(", ")
            ));
            return Ok(());
        }
        let Some(_lock) = try_acquire(&self.lock_path(&slot(kind, key)), false)? else {
            report.note(format!("keep {label} (in use)"));
            return Ok(());
        };
        // A checkout may have rooted it before this lock was taken.
        if self.rooted_now(kind, key)? {
            report.note(format!("keep {label} (rooted meanwhile)"));
            return Ok(());
        }
        if let Err(error) = read_receipt(&entry.path) {
            report.note(format!(
                "{label}: {error}; left in place (`kedra-lab gc --remove {label}` deletes it)"
            ));
            return Ok(());
        }
        let bytes = usage(&entry.path)?;
        report.freed += bytes;
        report.note(format!(
            "{} {label} ({})",
            if dry_run { "would delete" } else { "deleted" },
            size(bytes)
        ));
        if !dry_run {
            crate::remove_tree(&entry.path)?;
        }
        Ok(())
    }

    fn rooted_now(&self, kind: Kind, key: &str) -> Result<bool> {
        let name = format!("{}.json", slot(kind, key));
        for directory in list(&self.root.join("roots"))? {
            if self
                .root
                .join("roots")
                .join(directory)
                .join(&name)
                .symlink_metadata()
                .is_ok()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Build areas whose lock nobody holds belong to failed or interrupted builds.
    fn collect_work(&self, dry_run: bool, report: &mut GcReport) -> Result<()> {
        for name in list(&self.root.join("work"))? {
            let path = self.root.join("work").join(&name);
            if parse_slot(&name).is_none() {
                report.note(format!(
                    "unknown build area {}; left in place",
                    path.display()
                ));
                continue;
            }
            let Some(_lock) = try_acquire(&self.lock_path(&name), false)? else {
                report.note(format!("keep build area {name} (building)"));
                continue;
            };
            let bytes = usage(&path)?;
            report.freed += bytes;
            report.note(format!(
                "{} abandoned build area {name} ({})",
                if dry_run { "would delete" } else { "deleted" },
                size(bytes)
            ));
            if !dry_run {
                crate::remove_tree(&path)?;
            }
        }
        Ok(())
    }

    fn collect_host_stores(
        &self,
        rooted: &Rooted,
        dry_run: bool,
        report: &mut GcReport,
    ) -> Result<()> {
        for store in list(&self.root.join("stores"))? {
            let path = self.store_path(&store);
            if let Some(roots) = rooted.stores.get(&store) {
                let names: Vec<_> = roots.iter().map(String::as_str).collect();
                report.note(format!(
                    "keep store {store} ({}; rooted by {})",
                    size(usage(&path)?),
                    names.join(", ")
                ));
                continue;
            }
            let Some(_lock) = self.try_store_lock(&store)? else {
                report.note(format!("keep store {store} (in use)"));
                continue;
            };
            if self.store_rooted_now(&store)? {
                report.note(format!("keep store {store} (rooted meanwhile)"));
                continue;
            }
            let bytes = usage(&path)?;
            report.freed += bytes;
            report.note(format!(
                "{} store {store} ({})",
                if dry_run { "would delete" } else { "deleted" },
                size(bytes)
            ));
            if !dry_run {
                crate::remove_tree(&path)?;
            }
        }
        Ok(())
    }

    fn store_rooted_now(&self, store: &str) -> Result<bool> {
        let name = format!("store-{store}.json");
        for directory in list(&self.root.join("roots"))? {
            if self
                .root
                .join("roots")
                .join(directory)
                .join(&name)
                .symlink_metadata()
                .is_ok()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

enum CheckoutState {
    Live,
    Gone(PathBuf),
    Unknown(String),
}

/// A roots directory is live when its record names an existing checkout whose
/// id is the directory name.
fn checkout_state(id: &str, directory: &Path) -> CheckoutState {
    if !is_hex(id, 12) {
        return CheckoutState::Unknown("not a checkout id".into());
    }
    let record: CheckoutRecord = match read_limited(&directory.join("checkout.json"))
        .map_err(|e| e.to_string())
        .and_then(|data| serde_json::from_slice(&data).map_err(|e| e.to_string()))
    {
        Ok(record) => record,
        Err(error) => return CheckoutState::Unknown(format!("checkout.json: {error}")),
    };
    if record.schema != SCHEMA {
        return CheckoutState::Unknown("unknown checkout.json schema".into());
    }
    if hex(&Sha256::digest(record.path.as_os_str().as_encoded_bytes())[..6]) != id {
        return CheckoutState::Unknown("checkout.json names another checkout".into());
    }
    if record.path.exists() {
        CheckoutState::Live
    } else {
        CheckoutState::Gone(record.path)
    }
}

/// `<kind>-<key>`: the name of an entry's lock, build area and root.
fn slot(kind: Kind, key: &str) -> String {
    format!("{}-{key}", kind.name())
}

fn parse_slot(name: &str) -> Option<(Kind, String)> {
    let (kind, key) = name.rsplit_once('-')?;
    Some((Kind::parse(kind)?, key.to_owned())).filter(|(_, key)| is_hex(key, 64))
}

/// Root files of one checkout as (stem, path), without `checkout.json`.
fn root_files(directory: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    for name in list(directory)? {
        if let Some(stem) = name.strip_suffix(".json") {
            files.push((stem.to_owned(), directory.join(&name)));
        }
    }
    Ok(files)
}

/// The entry key: SHA-256 of the kind and the canonical JSON of its inputs.
pub fn key(kind: Kind, inputs: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"kedra-shared-cache-v1\0");
    hasher.update(kind.name().as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical(inputs).to_string().as_bytes());
    hex(&hasher.finalize())
}

/// The same value with every object's keys sorted, whatever map order
/// serde_json was built with.
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<_, _> =
                map.iter().map(|(k, v)| (k.clone(), canonical(v))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

fn read_receipt(entry: &Path) -> Result<Receipt> {
    let data = read_limited(&entry.join(RECEIPT))
        .map_err(|error| invalid(format!("unreadable receipt: {error}")))?;
    let receipt: Receipt = serde_json::from_slice(&data)
        .map_err(|error| invalid(format!("unknown receipt: {error}")))?;
    if receipt.schema != SCHEMA {
        return Err(invalid("unknown receipt schema"));
    }
    Ok(receipt)
}

/// Refuse an entry that differs from its receipt or from the requested inputs.
fn verify(entry: &Entry, inputs: &Value) -> Result<()> {
    let refuse = |detail: String| {
        invalid(format!(
            "refusing shared cache entry {}: {detail}. It was not repaired; inspect it, then \
             delete it with `kedra-lab gc --remove {}/{}`",
            entry.path.display(),
            entry.kind.name(),
            entry.key
        ))
    };
    let metadata = entry.path.symlink_metadata()?;
    if !metadata.is_dir() {
        return Err(refuse("not a directory".into()));
    }
    let receipt = read_receipt(&entry.path).map_err(|error| refuse(error.to_string()))?;
    if receipt.kind != entry.kind.name() || receipt.key != entry.key {
        return Err(refuse("the receipt names another entry".into()));
    }
    if key(entry.kind, &receipt.inputs) != entry.key || receipt.inputs != canonical(inputs) {
        return Err(refuse("its recorded inputs differ from its key".into()));
    }
    let mut names = list(&entry.path)?;
    names.sort();
    if names != [CONTENT, RECEIPT] {
        return Err(refuse(format!("unexpected members {names:?}")));
    }
    let observed = manifest(&entry.content()).map_err(|error| refuse(error.to_string()))?;
    if let Some(path) = first_difference(&receipt.files, &observed) {
        return Err(refuse(format!("{path} differs from its receipt")));
    }
    Ok(())
}

fn first_difference(
    expected: &BTreeMap<String, Node>,
    observed: &BTreeMap<String, Node>,
) -> Option<String> {
    expected
        .iter()
        .find(|(path, node)| observed.get(*path) != Some(node))
        .map(|(path, _)| path.clone())
        .or_else(|| {
            observed
                .keys()
                .find(|path| !expected.contains_key(*path))
                .cloned()
        })
}

/// Type, mode, size and SHA-256 of everything under `root`, by relative path.
fn manifest(root: &Path) -> Result<BTreeMap<String, Node>> {
    let mut nodes = BTreeMap::new();
    let mut pending = vec![(root.to_path_buf(), ".".to_owned())];
    while let Some((path, name)) = pending.pop() {
        let metadata = path.symlink_metadata()?;
        let mode = metadata.permissions().mode() & 0o7777;
        let node = if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path)?;
            Node::Symlink {
                target: target
                    .to_str()
                    .ok_or_else(|| invalid(format!("{name}: link target is not UTF-8")))?
                    .to_owned(),
            }
        } else if metadata.is_dir() {
            for child in list(&path)? {
                let relative = if name == "." {
                    child.clone()
                } else {
                    format!("{name}/{child}")
                };
                pending.push((path.join(&child), relative));
            }
            Node::Dir { mode }
        } else if metadata.is_file() {
            Node::File {
                mode,
                size: metadata.len(),
                sha256: file_sha256(&path)?,
            }
        } else {
            return Err(invalid(format!("{name}: not a file, directory or link")));
        };
        nodes.insert(name, node);
    }
    Ok(nodes)
}

/// Remove every write permission, children before their directory.
fn seal(path: &Path) -> Result<()> {
    let metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        for child in list(path)? {
            seal(&path.join(child))?;
        }
    }
    let mode = metadata.permissions().mode() & 0o7777;
    fs::set_permissions(path, Permissions::from_mode(mode & !0o222))?;
    Ok(())
}

/// Bytes on disk under `path` (allocated blocks, links not followed).
pub fn usage(path: &Path) -> Result<u64> {
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let mut total = metadata.blocks() * 512;
    if metadata.is_dir() {
        for child in list(path)? {
            total += usage(&path.join(child))?;
        }
    }
    Ok(total)
}

/// Human-readable size.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = "B";
    for next in UNITS {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = next;
    }
    if unit == "B" {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {unit}")
    }
}

/// Sorted UTF-8 names in a directory; none when it does not exist.
fn list(directory: &Path) -> Result<Vec<String>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let name = entry?.file_name();
        names.push(name.into_string().map_err(|name| {
            invalid(format!("{}: non-UTF-8 name {name:?}", directory.display()))
        })?);
    }
    names.sort();
    Ok(names)
}

fn acquire(path: &Path, shared: bool, what: &str) -> Result<Lock> {
    let mut announced = false;
    loop {
        if let Some(lock) = try_acquire(path, shared)? {
            return Ok(lock);
        }
        if !announced {
            eprintln!(
                "kedra-lab: waiting for {what}: another process holds {}",
                path.display()
            );
            announced = true;
        }
        crate::cancel::check()?;
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn try_acquire(path: &Path, shared: bool) -> Result<Option<Lock>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    let attempt = if shared {
        file.try_lock_shared()
    } else {
        file.try_lock()
    };
    match attempt {
        Ok(()) => Ok(Some(Lock { _file: file })),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let pending = path.with_extension(format!("pending-{}", std::process::id()));
    let mut file = File::create(&pending)?;
    file.write_all(data)?;
    file.sync_all()?;
    fs::rename(&pending, path)?;
    Ok(())
}

fn read_limited(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    File::open(path)?
        .take(RECEIPT_LIMIT + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 > RECEIPT_LIMIT {
        return Err(std::io::Error::other("larger than 64 MiB"));
    }
    Ok(data)
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
