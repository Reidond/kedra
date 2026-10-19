//! Engine stores every checkout shares through the shared cache: the host RPM
//! store (`stores/rpm`) that kedra-lab's host `sysroot` resolves RPM locks and
//! materializes foundation repositories with, and the store of full local
//! builds' engine steps (RPM objects, base images, runtime and builder roots,
//! sources and package outputs). Roots need a Linux filesystem, so that store
//! lives in the Docker volume `kedra-cache-<cache id>-engine`, labeled
//! `kedra.cache=<cache id>`.
//!
//! The engine keeps its own guarantees: content-addressed objects and an
//! exclusive lock held for every `sysroot` command, so builds from several
//! checkouts interleave command by command and reuse what another command
//! admitted; whatever a killed command left unfinished is refused by the next
//! opener until `store recover` reconciles it under the same lock. Two things
//! the engine leaves to its caller are done here:
//!
//! - Creation: `Store::create` is not safe against a concurrent creator, so a
//!   store is created only under the shared cache's exclusive store lock.
//!   Builds hold that lock shared for their whole run.
//! - Retention: every admission pins. Each checkout records the images, roots
//!   and objects its latest build used (`roots/<checkout id>/store-<name>.json`);
//!   a build that finishes while no other build holds the store unpins
//!   everything no record names and runs `store gc --delete`. `kedra-lab gc`
//!   removes a store no checkout roots.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::shared_cache::{Cache, Lock, size};
use crate::{Result, invalid};

/// The host RPM store.
pub const RPM: &str = "rpm";
/// The engine store of full local builds (a Docker volume).
pub const ENGINE: &str = "engine";

/// What a store pins, or what a checkout's record keeps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pins {
    #[serde(default)]
    pub objects: BTreeSet<String>,
    #[serde(default)]
    pub images: BTreeSet<String>,
    #[serde(default)]
    pub rootfs: BTreeSet<String>,
}

impl Pins {
    fn extend(&mut self, other: Pins) {
        self.objects.extend(other.objects);
        self.images.extend(other.images);
        self.rootfs.extend(other.rootfs);
    }
}

/// How kedra-lab reaches one store: a host process or a builder container.
pub trait Access {
    /// The store path as `sysroot` sees it.
    fn store(&self) -> &str;
    /// Standard output of `sysroot <args>`.
    fn sysroot(&self, args: &[&str]) -> Result<Vec<u8>>;
    /// Whether the store has been created.
    fn exists(&self) -> Result<bool>;
    /// The store's own pins (`roots/state.json`).
    fn pins(&self) -> Result<Pins>;
    /// Bytes the store uses.
    fn usage(&self) -> Result<u64>;
}

/// The volume holding a cache's engine store.
pub fn engine_volume(cache: &Cache) -> String {
    format!("kedra-cache-{}-{ENGINE}", cache.id())
}

/// Hold a shared store for one build: create it under the exclusive lock when
/// absent, root it for this checkout (keeping its previous record), reconcile
/// commands a killed build left unfinished, and return the shared lock the
/// build keeps until [`finish`].
pub fn open(cache: &Cache, name: &str, access: &dyn Access) -> Result<Lock> {
    let mut shared = cache.store_lock(name, true)?;
    if !access.exists()? {
        drop(shared);
        create(cache, name, access)?;
        shared = cache.store_lock(name, true)?;
    }
    root(cache, name)?;
    // No command holds the engine's lock between commands, so any transaction
    // left in the store belongs to a killed command; `recover` waits for live ones.
    let recovered: serde_json::Value = serde_json::from_slice(&access.sysroot(&[
        "store",
        "recover",
        "--store",
        access.store(),
    ])?)
    .map_err(|error| invalid(format!("store recover: {error}")))?;
    if recovered["recovered"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        eprintln!(
            "kedra-lab: recovered interrupted work in the shared engine store {name}: {}",
            recovered["recovered"]
        );
    }
    Ok(shared)
}

/// Create the store under the exclusive lock, unless another checkout creates
/// it meanwhile (and then holds it shared for its build).
fn create(cache: &Cache, name: &str, access: &dyn Access) -> Result<()> {
    loop {
        if let Some(_exclusive) = cache.try_store_lock(name)? {
            if !access.exists()? {
                access.sysroot(&["store", "init", "--store", access.store()])?;
                eprintln!("kedra-lab: created the shared engine store {name}");
            }
            // Rooted before the lock is released, so GC never sees it unrooted.
            return root(cache, name);
        }
        if access.exists()? {
            return Ok(());
        }
        crate::cancel::check()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Root a store for this checkout without replacing an existing record.
fn root(cache: &Cache, name: &str) -> Result<()> {
    let mine = cache.store_record(name)?;
    if mine.is_none() {
        cache.root_store(name, &record(&Pins::default()))?;
    }
    Ok(())
}

fn record(pins: &Pins) -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "objects": pins.objects,
        "images": pins.images,
        "rootfs": pins.rootfs,
    })
}

/// Record what this checkout's build used, release the build's shared lock and
/// prune the store if no other build holds it.
pub fn finish(
    cache: &Cache,
    name: &str,
    access: &dyn Access,
    used: &Pins,
    shared: Lock,
) -> Result<()> {
    cache.root_store(name, &record(used))?;
    drop(shared);
    // The build succeeded; a failed prune only leaves the store larger.
    if let Err(error) = prune(cache, name, access) {
        eprintln!("kedra-lab: pruning the shared engine store {name} failed: {error}");
    }
    Ok(())
}

/// Unpin everything no checkout's record keeps and delete what the engine's GC
/// then finds unreferenced, under the exclusive store lock; skipped while a
/// build holds the store.
pub fn prune(cache: &Cache, name: &str, access: &dyn Access) -> Result<()> {
    let Some(_exclusive) = cache.try_store_lock(name)? else {
        eprintln!("kedra-lab: another build holds the shared engine store {name}; not pruned");
        return Ok(());
    };
    let mut keep = Pins::default();
    for value in cache.store_records(name)? {
        if value["schema"] != 1 {
            return Err(invalid(format!(
                "a checkout's record on the shared engine store {name} has an unknown schema; not pruned"
            )));
        }
        keep.extend(
            serde_json::from_value(value)
                .map_err(|error| invalid(format!("store record: {error}")))?,
        );
    }
    let pinned = access.pins()?;
    let before = access.usage()?;
    let store = access.store().to_owned();
    let mut unpinned = 0;
    for (ids, kept, verb, flag) in [
        (&pinned.objects, &keep.objects, "unpin", "--object"),
        (&pinned.images, &keep.images, "unpin-image", "--image"),
        (&pinned.rootfs, &keep.rootfs, "unpin-root", "--root"),
    ] {
        for id in ids.difference(kept) {
            access.sysroot(&["store", verb, "--store", &store, flag, id])?;
            unpinned += 1;
        }
    }
    let deleted: serde_json::Value =
        serde_json::from_slice(&access.sysroot(&["store", "gc", "--store", &store, "--delete"])?)
            .map_err(|error| invalid(format!("store gc: {error}")))?;
    let count = |field: &str| deleted[field].as_array().map_or(0, Vec::len);
    let after = access.usage()?;
    eprintln!(
        "kedra-lab: pruned the shared engine store {name}: unpinned {unpinned}; deleted {} objects, {} images, {} roots; freed {}",
        count("objects"),
        count("images"),
        count("roots"),
        size(before.saturating_sub(after))
    );
    Ok(())
}
