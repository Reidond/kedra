//! Bounded, non-capturing assessment of the caller's existing home records.
use super::{activation, linux, text};
use rustix::fs::{self, Mode, OFlags, ResolveFlags};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use sysroot_core::noctalia::State;
use sysroot_helper::storage::Store;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    NotAdopted,
    AcceptedBaselineMatchesInstalled,
    ReconciliationRequired,
    RecoveryRequired,
    Unavailable,
}

#[derive(Serialize)]
pub(super) struct Group {
    status: Status,
    accepted_baseline_revision: Option<String>,
    installed_baseline_revision: Option<String>,
    next_command: Option<&'static str>,
}
impl Group {
    pub(super) fn new(
        accepted: Option<&str>,
        installed: &str,
        matches: bool,
        pending: bool,
        next_command: &'static str,
        recovery_command: &'static str,
    ) -> Self {
        let status = if pending {
            Status::RecoveryRequired
        } else if accepted.is_none() {
            Status::NotAdopted
        } else if matches {
            Status::AcceptedBaselineMatchesInstalled
        } else {
            Status::ReconciliationRequired
        };
        Self {
            status,
            accepted_baseline_revision: accepted.map(str::to_owned),
            installed_baseline_revision: Some(installed.to_owned()),
            next_command: match status {
                Status::RecoveryRequired => Some(recovery_command),
                Status::ReconciliationRequired => Some(next_command),
                _ => None,
            },
        }
    }
    fn unavailable() -> Self {
        Self {
            status: Status::Unavailable,
            accepted_baseline_revision: None,
            installed_baseline_revision: None,
            next_command: None,
        }
    }
}

fn checked_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err("assessment paths must be absolute without traversal".into());
    }
    Ok(())
}

fn directory(path: &Path, owner: u32, private: bool) -> Result<()> {
    let descriptor = fs::openat2(
        fs::CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let stat = fs::fstat(&descriptor)?;
    if stat.st_uid != owner
        || stat.st_mode & 0o022 != 0
        || private && stat.st_mode & 0o7777 != 0o700
    {
        return Err("assessment directory is unsafe".into());
    }
    Ok(())
}

fn missing(error: &(dyn std::error::Error + 'static)) -> bool {
    error.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT)
}

fn profile(owner: u32) -> Result<(PathBuf, PathBuf)> {
    let requested = PathBuf::from(std::env::var_os("HOME").ok_or("HOME unavailable")?);
    checked_path(&requested)?;
    // Permit bootc's /home -> /var/home alias, then check the actual user home.
    let home = requested.canonicalize()?;
    directory(&home, owner, false)?;
    for (name, relative) in [
        ("XDG_CONFIG_HOME", ".config"),
        ("NOCTALIA_CONFIG_HOME", ".config"),
        ("XDG_STATE_HOME", ".local/state"),
        ("NOCTALIA_STATE_HOME", ".local/state"),
        ("NIRI_CONFIG", ".config/niri/config.kdl"),
    ] {
        if let Some(value) = std::env::var_os(name).filter(|v| !v.is_empty())
            && Path::new(&value) != home.join(relative)
            && Path::new(&value) != requested.join(relative)
        {
            return Err("assessment supports the default application profiles only".into());
        }
    }
    Ok((requested, home))
}

fn existing_path(selected: Option<&Path>, owner: u32) -> Result<Option<PathBuf>> {
    let (requested_home, home) = profile(owner)?;
    let path = if let Some(selected) = selected {
        linux::private_parent(selected, owner)?
    } else {
        let mut base = match std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
            Some(value) => PathBuf::from(value),
            None => home.join(".local/state"),
        };
        checked_path(&base)?;
        if let Ok(relative) = base.strip_prefix(&requested_home) {
            base = home.join(relative);
        }
        // A genuinely absent default directory is unadopted. Check its nearest
        // existing ancestor without following symlinks; errors are never absence.
        let mut ancestor = base.clone();
        loop {
            match directory(&ancestor, owner, false) {
                Ok(()) => break,
                Err(error) if missing(error.as_ref()) => {
                    if !ancestor.pop() {
                        return Err("default state parent is unavailable".into());
                    }
                }
                Err(error) => return Err(error),
            }
        }
        if ancestor != base {
            return Ok(None);
        }
        let private = base.join("sysroot");
        match directory(&private, owner, true) {
            Ok(()) => (),
            Err(error) if missing(error.as_ref()) => return Ok(None),
            Err(error) => return Err(error),
        }
        private.join("home")
    };
    match std::fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn noctalia(store: Option<&Store>, instance: &str) -> Result<Group> {
    let baseline = linux::installed_baseline(Path::new("/"), 0)?;
    // Reuse the state model's provenance validation without observing live data.
    State::new(instance.to_owned(), baseline.clone(), baseline.settings)?;
    let state = store
        .map(|store| linux::load_optional(store, instance))
        .transpose()?
        .flatten()
        .map(|(_, state)| state);
    let pending = match store {
        Some(store) => activation::linux::assessment_pending(store, instance, state.as_ref())?,
        None => false,
    };
    Ok(Group::new(
        state
            .as_ref()
            .map(|s| s.accepted_baseline().source_revision.as_str()),
        &baseline.source_revision,
        state
            .as_ref()
            .is_some_and(|s| *s.accepted_baseline() == baseline),
        pending,
        "sysroot home plan",
        "sysroot home recover",
    ))
}

fn groups(selected: Option<&Path>, owner: u32) -> Result<[Group; 2]> {
    if owner == 0 || rustix::process::getuid().as_raw() != owner {
        return Err("assessment requires an ordinary invoking user".into());
    }
    let path = existing_path(selected, owner)?;
    let instance = linux::instance(owner)?;
    let store = path.as_deref().map(Store::open).transpose()?;
    // Store opening may update SQLite sidecars; coordination may create its lock.
    // No logical review records, native files, capture or recovery are changed.
    let _coordination = store.as_ref().map(Store::coordinate).transpose()?;
    let groups = [
        noctalia(store.as_ref(), &instance).unwrap_or_else(|_| Group::unavailable()),
        text::assessment(store.as_ref(), &instance).unwrap_or_else(|_| Group::unavailable()),
    ];
    if store.is_some()
        && groups
            .iter()
            .all(|group| group.status == Status::NotAdopted)
    {
        return Err(
            "existing home store has no recognized adoption; preserve it for recovery".into(),
        );
    }
    Ok(groups)
}

pub(super) fn run(selected: Option<&Path>) -> serde_json::Value {
    let owner = rustix::process::geteuid().as_raw();
    let [noctalia, niri] =
        groups(selected, owner).unwrap_or_else(|_| [Group::unavailable(), Group::unavailable()]);
    let status = [
        Status::Unavailable,
        Status::RecoveryRequired,
        Status::ReconciliationRequired,
        Status::AcceptedBaselineMatchesInstalled,
        Status::NotAdopted,
    ]
    .into_iter()
    .find(|status| noctalia.status == *status || niri.status == *status)
    .unwrap_or(Status::Unavailable);
    serde_json::json!({
        "schema_version": 1,
        "scope": "invoking_user",
        "uid": owner,
        "store": if selected.is_some() { "explicit_home_state" } else { "default_home_state" },
        "status": status,
        "groups": {"noctalia": noctalia, "niri": niri},
        "review_state_changed": false,
        "live_files_changed": false,
        "live_configuration_checked": false,
        "next_command_state": if selected.is_some() { "use_the_same_--state_path" } else { "default_home_state" }
    })
}
