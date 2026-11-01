//! Live home files, the private applied-baseline state, `status` and `capture`.
use super::files::{self, CONFLICT_SUFFIX, Image, MAX_FILE, ManagedFile, Result};
use super::{Command, Options};
use rustix::fs::{self as rfs, Mode, OFlags, ResolveFlags};
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use sysroot_linux::replace_file::Directory;

/// Last baseline merged into each live file.
pub(super) const APPLIED: &str = "applied";
/// Baseline an update could not merge; its conflict file awaits the owner.
pub(super) const PENDING: &str = "pending";

/// 32 lowercase hex characters from the kernel's random source.
pub(super) fn token() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Parent directory and file name of a validated relative path.
pub(super) fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

fn open_directory(base: &OwnedFd, name: &str) -> rustix::io::Result<OwnedFd> {
    rfs::openat2(
        base,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS | ResolveFlags::BENEATH,
    )
}

/// Walk `relative` below `base` through real directories only, creating missing
/// ones when asked. False when a directory is missing and `create` is false.
fn open_components(base: &Path, relative: &str, create: bool) -> Result<bool> {
    let mut current = rfs::openat2(
        rfs::CWD,
        base,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    for component in relative.split('/').filter(|part| !part.is_empty()) {
        current = match open_directory(&current, component) {
            Ok(descriptor) => descriptor,
            Err(rustix::io::Errno::NOENT) if create => {
                match rfs::mkdirat(&current, component, Mode::from_raw_mode(0o755)) {
                    Ok(()) | Err(rustix::io::Errno::EXIST) => (),
                    Err(error) => return Err(format!("{relative}: {error}").into()),
                }
                open_directory(&current, component)?
            }
            Err(rustix::io::Errno::NOENT) => return Ok(false),
            Err(error) => {
                return Err(format!("{relative}: {error}; directories must not be links").into());
            }
        };
    }
    Ok(true)
}

/// The invoking user's home, resolved once (bootc's /home -> /var/home alias).
pub(super) struct Home {
    root: PathBuf,
}

impl Home {
    pub fn open() -> Result<Self> {
        let owner = rustix::process::geteuid().as_raw();
        if owner == 0 || rustix::process::getuid().as_raw() != owner {
            return Err("home files are managed as the ordinary desktop user, never root".into());
        }
        let requested = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable")?);
        if !requested.is_absolute() {
            return Err("HOME must be an absolute path".into());
        }
        let root = requested.canonicalize()?;
        if std::fs::metadata(&root)?.uid() != owner {
            return Err("HOME is not owned by the invoking user".into());
        }
        Ok(Self { root })
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// The real directory holding a managed file, or `None` when it is missing.
    pub fn directory(&self, relative: &str, create: bool) -> Result<Option<Directory>> {
        let (parent, _) = split(relative);
        if !open_components(&self.root, parent, create)? {
            return Ok(None);
        }
        Ok(Some(Directory::open(&self.root.join(parent))?))
    }

    /// An ordinary owned file; links, devices and shared modes are refused.
    pub fn read(&self, relative: &str) -> Result<Option<Vec<u8>>> {
        let Some(directory) = self.directory(relative, false)? else {
            return Ok(None);
        };
        directory
            .read(split(relative).1)
            .map_err(|error| format!("~/{relative}: {error}").into())
    }
}

/// Private per-user record of applied and conflicted baselines (mode 0700).
pub(super) struct State {
    root: PathBuf,
}

/// Owned scratch directory removed on drop; live bytes never leave it.
pub(super) struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) -> Result<()> {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?
            .write_all(bytes)?;
        Ok(())
    }
}

fn private_directory(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(format!(
            "{} must be a directory owned by this user with mode 0700",
            path.display()
        )
        .into());
    }
    Ok(())
}

impl State {
    /// `--state`, else `$XDG_STATE_HOME/kedra/home`, else `~/.local/state/kedra/home`.
    pub fn locate(explicit: Option<&Path>, home: &Home) -> Result<PathBuf> {
        let path = match explicit {
            Some(path) => path.to_path_buf(),
            None => match std::env::var_os("XDG_STATE_HOME").filter(|value| !value.is_empty()) {
                Some(base) => PathBuf::from(base).join("kedra/home"),
                None => home.path(".local/state/kedra/home"),
            },
        };
        if !path.is_absolute() {
            return Err("the home state directory must be an absolute path".into());
        }
        Ok(path)
    }

    /// The state directory, created when asked; `None` when absent otherwise.
    pub fn open(path: &Path, create: bool) -> Result<Option<Self>> {
        if create {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)?;
        } else if !path.try_exists()? {
            return Ok(None);
        }
        let root = path.canonicalize()?;
        private_directory(&root)?;
        Ok(Some(Self { root }))
    }

    pub fn read(&self, kind: &str, path: &str) -> Result<Option<Vec<u8>>> {
        let owner = rustix::process::geteuid().as_raw();
        let file = self.root.join(kind).join(path);
        match sysroot_linux::trusted_file::read(&file, owner, MAX_FILE) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error)
                if error.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT) =>
            {
                Ok(None)
            }
            Err(error) => Err(format!("{}: {error}", file.display()).into()),
        }
    }

    /// Replace one record atomically; a crash leaves the old record or the new one.
    pub fn write(&self, kind: &str, path: &str, bytes: &[u8]) -> Result<()> {
        let file = self.root.join(kind).join(path);
        let parent = file.parent().ok_or("state record has no parent")?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let temporary = parent.join(format!(".{}.tmp", token()?));
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        output.write_all(bytes)?;
        output.sync_all()?;
        std::fs::rename(&temporary, &file)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }

    pub fn remove(&self, kind: &str, path: &str) -> Result<()> {
        match std::fs::remove_file(self.root.join(kind).join(path)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// Serialize updates; the lock is released when the returned file closes.
    pub fn lock(&self) -> Result<File> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(self.root.join("lock"))?;
        file.try_lock()
            .map_err(|_| "another sysroot home update is running")?;
        Ok(file)
    }

    pub fn scratch(&self) -> Result<Scratch> {
        let path = self.root.join(format!("scratch-{}", token()?));
        std::fs::DirBuilder::new().mode(0o700).create(&path)?;
        Ok(Scratch(path))
    }
}

/// Run Git with no inherited Git environment; returns its exit code and stdout.
pub(super) fn git(directory: &Path, arguments: &[&str]) -> Result<(i32, Vec<u8>)> {
    let mut command = Process::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let output = command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-c", "core.quotePath=false", "-c", "diff.noprefix=false"])
        .args([
            "-c",
            "diff.mnemonicPrefix=false",
            "-c",
            "diff.relative=false",
        ])
        .args(arguments)
        .current_dir(directory)
        .stdin(Stdio::null())
        .output()?;
    let code = output.status.code().unwrap_or(-1);
    if !(0..=127).contains(&code) {
        return Err(format!(
            "git {} failed: {}",
            arguments.first().unwrap_or(&""),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok((code, output.stdout))
}

/// Everything known about one managed file before an operation.
pub(super) struct Observed {
    pub live: Option<Vec<u8>>,
    pub baseline: Option<Vec<u8>>,
    pub applied: Option<Vec<u8>>,
    pub pending: Option<Vec<u8>>,
    pub conflict_file: bool,
}

pub(super) fn conflict_path(path: &str) -> String {
    format!("{path}{CONFLICT_SUFFIX}")
}

pub(super) fn observe(
    image: &Image,
    home: &Home,
    state: Option<&State>,
    path: &str,
) -> Result<Observed> {
    let record = |kind| state.map(|state| state.read(kind, path)).transpose();
    Ok(Observed {
        live: home.read(path)?,
        baseline: image.baseline(path)?,
        applied: record(APPLIED)?.flatten(),
        pending: record(PENDING)?.flatten(),
        conflict_file: home.read(&conflict_path(path))?.is_some(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Applied {
    /// The recorded baseline is the installed one.
    Current,
    /// The installed baseline changed since the last update.
    Outdated,
    /// No update has recorded a baseline for this file.
    None,
}

#[derive(Serialize)]
struct FileStatus {
    path: String,
    live: bool,
    baseline: bool,
    live_matches_baseline: bool,
    applied: Applied,
    conflict: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

impl FileStatus {
    fn new(path: &str, observed: &Observed) -> Self {
        let applied = match (&observed.applied, &observed.baseline) {
            (None, _) => Applied::None,
            (Some(applied), baseline) if Some(applied) == baseline.as_ref() => Applied::Current,
            _ => Applied::Outdated,
        };
        Self {
            path: path.to_owned(),
            live: observed.live.is_some(),
            baseline: observed.baseline.is_some(),
            live_matches_baseline: observed.live.is_some() && observed.live == observed.baseline,
            applied,
            conflict: observed.pending.is_some(),
            diff: None,
        }
    }

    fn summary(&self) -> String {
        let contents = match (self.live, self.baseline) {
            (false, _) => "live file is absent",
            (true, false) => "the image has no baseline",
            (true, true) if self.live_matches_baseline => "matches the installed baseline",
            _ => "differs from the installed baseline",
        };
        let applied = match self.applied {
            Applied::Current => "installed baseline applied",
            Applied::Outdated => "baseline changed since the last update; run sysroot home update",
            Applied::None => "no baseline recorded; run sysroot home update",
        };
        let mut summary = format!("{}: {contents}; {applied}", self.path);
        if self.conflict {
            summary.push_str(&format!(
                "; conflict: merge ~/{} into the live file, delete it, then run sysroot home update",
                conflict_path(&self.path)
            ));
        }
        summary
    }
}

/// Unified diff from the installed baseline to the live file, via Git.
fn diff(state: &State, path: &str, baseline: &[u8], live: &[u8]) -> Result<String> {
    let scratch = state.scratch()?;
    let (before, after) = (format!("baseline/{path}"), format!("live/{path}"));
    scratch.write(&before, baseline)?;
    scratch.write(&after, live)?;
    let (_, output) = git(
        scratch.path(),
        &[
            "diff",
            "--no-index",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--",
            &before,
            &after,
        ],
    )?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}

fn file_status(
    image: &Image,
    home: &Home,
    state: &State,
    file: &ManagedFile,
) -> Result<FileStatus> {
    let observed = observe(image, home, Some(state), &file.path)?;
    let mut status = FileStatus::new(&file.path, &observed);
    if let (Some(live), Some(baseline)) = (&observed.live, &observed.baseline)
        && live != baseline
    {
        status.diff = Some(diff(state, &file.path, baseline, live)?);
    }
    Ok(status)
}

fn status(image: &Image, home: &Home, state: &State, paths: &[String], json: bool) -> Result<()> {
    let files = files::select(image.files()?, paths)?;
    let rows = files
        .iter()
        .map(|file| file_status(image, home, state, file))
        .collect::<Result<Vec<_>>>()?;
    if json {
        let report = serde_json::json!({"schema_version": 1, "files": rows});
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    for row in rows {
        println!("{}", row.summary());
        if let Some(diff) = row.diff {
            print!("{diff}");
        }
    }
    Ok(())
}

/// Read-only summary for `sysroot update status --home`; never creates state.
pub(super) fn assess(state: Option<&Path>) -> serde_json::Value {
    assessment(state).unwrap_or_else(|error| {
        serde_json::json!({
            "schema_version": 2, "scope": "invoking_user", "status": "unavailable",
            "detail": error.to_string(), "live_files_changed": false
        })
    })
}

fn assessment(explicit: Option<&Path>) -> Result<serde_json::Value> {
    let home = Home::open()?;
    let image = Image::open(Path::new("/"))?;
    let path = State::locate(explicit, &home)?;
    let state = State::open(&path, false)?;
    if explicit.is_some() && state.is_none() {
        return Err("the selected home state directory does not exist".into());
    }
    let rows = image
        .files()?
        .iter()
        .map(|file| {
            observe(&image, &home, state.as_ref(), &file.path)
                .map(|observed| FileStatus::new(&file.path, &observed))
        })
        .collect::<Result<Vec<_>>>()?;
    let (status, next) = if rows.iter().any(|row| row.conflict) {
        ("conflict", Some("sysroot home status"))
    } else if rows.iter().any(|row| row.applied != Applied::Current) {
        ("update_available", Some("sysroot home update"))
    } else {
        ("up_to_date", None)
    };
    Ok(serde_json::json!({
        "schema_version": 2, "scope": "invoking_user", "status": status,
        "files": rows, "next_command": next, "live_files_changed": false
    }))
}

/// The committed file the image built a managed home file from, as its source
/// manifest records it: a `file("./…")` resource under `system/`.
fn installed_source(image: &Image, path: &str) -> Result<String> {
    let bytes = image
        .read(files::SOURCE, 4_194_304)?
        .ok_or("the image has no source manifest")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "the image source manifest is malformed")?;
    let destination = format!("{}/{path}", files::BASELINE);
    let source = manifest["files"]
        .as_array()
        .and_then(|files| {
            files
                .iter()
                .find(|file| file["destination"].as_str() == Some(destination.as_str()))
        })
        .and_then(|file| file["source_path"].as_str())
        .ok_or_else(|| format!("the image source manifest has no source for ~/{path}"))?;
    if !source.starts_with("system/") || source.ends_with(".kedra") || !files::valid_path(source) {
        return Err(format!(
            "~/{path} comes from {source}, not a capturable file(\"./…\") resource"
        )
        .into());
    }
    Ok(source.to_owned())
}

/// The checkout's top level, refused unless it holds Kedra's system declarations.
fn checkout(repo: &Path) -> Result<PathBuf> {
    let (code, output) = git(repo, &["rev-parse", "--show-toplevel"])?;
    let top = String::from_utf8(output).map_err(|_| "checkout path is not UTF-8")?;
    let top = PathBuf::from(top.trim()).canonicalize()?;
    if code != 0 || !top.join("system/catalog.kedra").is_file() {
        return Err(format!("{} is not a Kedra checkout", repo.display()).into());
    }
    Ok(top)
}

/// Atomically replace one file in a real directory, keeping its mode and label.
pub(super) fn replace(
    directory: &Directory,
    name: &str,
    expected: Option<&[u8]>,
    bytes: &[u8],
) -> Result<()> {
    let receipt = directory.prepare(name, &token()?, expected, bytes)?;
    directory.publish(&receipt)?;
    directory.finish(&receipt)
}

fn capture(image: &Image, home: &Home, path: &str, repo: &Path, json: bool) -> Result<()> {
    files::select(image.files()?, &[path.to_owned()])?;
    let live = home
        .read(path)?
        .ok_or_else(|| format!("~/{path} does not exist"))?;
    let relative = installed_source(image, path)?;
    let top = checkout(repo)?;
    let (parent, name) = split(&relative);
    open_components(&top, parent, true)?;
    let directory = Directory::open(&top.join(parent))?;
    let current = directory
        .read(name)
        .map_err(|error| format!("{relative}: {error}"))?;
    let written = current.as_deref() != Some(live.as_slice());
    if written {
        replace(&directory, name, current.as_deref(), &live)?;
    }
    if json {
        let report = serde_json::json!({
            "schema_version": 1, "path": path, "checkout": top, "source_path": relative,
            "written": written, "staged": false, "committed": false
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if written {
        println!("Copied ~/{path} to {relative}.");
        println!(
            "Pick the lines to keep with: git -C {} add -p -- {relative}",
            top.display()
        );
    } else {
        println!("{relative} already matches ~/{path}; nothing was written.");
    }
    Ok(())
}

pub(super) fn run(options: Options) -> Result<()> {
    let home = Home::open()?;
    let image = Image::open(&options.root)?;
    let path = State::locate(options.state.as_deref(), &home)?;
    match options.command {
        Command::Status { json, paths } => {
            let state = State::open(&path, true)?.ok_or("home state is unavailable")?;
            status(&image, &home, &state, &paths, json)
        }
        Command::Capture { path, repo, json } => capture(&image, &home, &path, &repo, json),
        Command::Update { json, paths } => {
            let state = State::open(&path, true)?.ok_or("home state is unavailable")?;
            super::update::run(&image, &home, &state, &paths, json)
        }
    }
}
