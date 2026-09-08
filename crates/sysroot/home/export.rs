//! Export selected typed values through ordinary Git, without copying private state.
use crate::source;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use sysroot_core::noctalia::{self, Baseline, Key, Settings, State, Theme, Value};

#[derive(Debug)]
pub(super) enum Error {
    Io(std::io::Error),
    Source(source::Error),
    State(noctalia::Error),
    Refused(&'static str),
    Conflict(Key),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "export I/O: {error}"),
            Self::Source(error) => error.fmt(f),
            Self::State(error) => error.fmt(f),
            Self::Refused(reason) => f.write_str(reason),
            Self::Conflict(key) => write!(
                f,
                "source changed the selected field {key:?}; review the conflict before export"
            ),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<source::Error> for Error {
    fn from(error: source::Error) -> Self {
        Self::Source(error)
    }
}
impl From<noctalia::Error> for Error {
    fn from(error: noctalia::Error) -> Self {
        Self::State(error)
    }
}
type Result<T> = std::result::Result<T, Error>;

pub(super) struct Prepared {
    pub source_revision: String,
    pub source_path: String,
    pub patch: Vec<u8>,
}
impl Prepared {
    pub(super) fn write_new(&self, path: &Path) -> Result<()> {
        if path.to_str().is_none() {
            return Err(Error::Refused("patch output path must be UTF-8"));
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(&self.patch)?;
        file.sync_all()?;
        Ok(())
    }
}

fn text(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| Error::Refused("source output must be UTF-8"))
}
fn checkout(repo: &Path) -> Result<PathBuf> {
    let root = text(source::git(repo, &["rev-parse", "--show-toplevel"])?)?;
    let root = PathBuf::from(root.trim_end_matches(['\r', '\n']));
    let origin = text(source::git(
        &root,
        &["config", "--local", "--get", "remote.origin.url"],
    )?)?;
    if !matches!(
        origin.trim().to_ascii_lowercase().as_str(),
        "https://github.com/reidond/kedra.git"
            | "https://github.com/reidond/kedra"
            | "git@github.com:reidond/kedra.git"
            | "ssh://git@github.com/reidond/kedra.git"
    ) {
        return Err(Error::Refused(
            "source checkout origin is not Reidond/kedra",
        ));
    }
    Ok(root)
}

fn content(repo: &Path, plan: &source::Plan, baseline: &Baseline) -> Result<(String, String)> {
    let file = plan
        .files
        .iter()
        .find(|file| {
            file.home_baseline
                && file.destination == "usr/share/sysroot/home/default/.config/noctalia/config.toml"
        })
        .ok_or(Error::Refused("source has no effective Noctalia baseline"))?;
    if plan.target.id != baseline.target || file.source_path != baseline.source_path {
        return Err(Error::Refused(
            "source target/override provenance changed; explicit rebind review is required",
        ));
    }
    if file.mode != "100644" {
        return Err(Error::Refused(
            "Noctalia source must remain an ordinary non-executable file",
        ));
    }
    let bytes = source::git(repo, &["cat-file", "blob", &file.git_blob])?;
    if bytes.len() > noctalia::MAX_EXPORT {
        return Err(Error::Refused("Noctalia source exceeds the export bound"));
    }
    Ok((text(bytes)?, file.mode.clone()))
}

fn ancestry(repo: &Path, state: &State, commit: &str) -> Result<()> {
    for anchor in state.source_anchors()? {
        if source::git(repo, &["merge-base", "--is-ancestor", anchor, commit]).is_err() {
            return Err(Error::Refused(
                "source lacks the accepted baseline or a previous receipt; fetch/merge the required history before export",
            ));
        }
    }
    Ok(())
}

fn selected_content(original: &str, state: &State) -> Result<String> {
    let current = noctalia::project(noctalia::APP_VERSION, original)?;
    let selection = state.selection()?;
    if selection.is_empty() {
        return Err(Error::Refused("no selected fields to export"));
    }
    let mut document: toml_edit::DocumentMut = original
        .parse()
        .map_err(|_| Error::Refused("source TOML is malformed"))?;
    for selected in selection {
        let value = current.get(selected.key);
        if value == selected.after {
            continue;
        }
        if value != selected.before {
            return Err(Error::Conflict(selected.key));
        }
        let (table, key) = match selected.key {
            Key::ThemeMode => ("theme", "mode"),
            Key::ButtonBorders => ("shell", "button_borders"),
            Key::InputBorders => ("shell", "input_borders"),
        };
        let item = document
            .get_mut(table)
            .and_then(|item| item.get_mut(key))
            .and_then(toml_edit::Item::as_value_mut)
            .ok_or(Error::Refused(
                "selected source field has an unsupported TOML shape",
            ))?;
        let mut replacement = match selected.after {
            Value::Theme(theme) => toml_edit::Value::from(match theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
                Theme::Auto => "auto",
            }),
            Value::Toggle(value) => toml_edit::Value::from(value),
        };
        *replacement.decor_mut() = item.decor().clone();
        *item = replacement;
    }
    let result = document.to_string();
    if result.len() > noctalia::MAX_EXPORT {
        return Err(Error::Refused("selected source exceeds the export bound"));
    }
    // Independently reproject the rendered document before allowing Git to see it.
    let rendered = noctalia::project(noctalia::APP_VERSION, &result)?;
    for row in state.rows()? {
        let expected = row.selected.unwrap_or(current.get(row.key));
        if rendered.get(row.key) != expected {
            return Err(Error::Refused(
                "rendered source changed an unselected field",
            ));
        }
    }
    Ok(result)
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new(parent: &Path) -> Result<Self> {
        let path = parent.join(format!(
            "export-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Refused("clock is before the Unix epoch"))?
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        #[cfg(unix)]
        let builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = std::fs::DirBuilder::new();
            builder.mode(0o700);
            builder
        };
        #[cfg(not(unix))]
        let builder = std::fs::DirBuilder::new();
        builder.create(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(repo: &Path, args: &[&str], input: Option<&[u8]>, index: &Path) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    #[cfg(windows)]
    let index = {
        let value = index
            .to_str()
            .ok_or(Error::Refused("export scratch path must be UTF-8"))?;
        std::ffi::OsString::from(
            value
                .strip_prefix(r"\\?\")
                .unwrap_or(value)
                .replace('\\', "/"),
        )
    };
    #[cfg(not(windows))]
    let index = index.as_os_str();
    let mut child = command
        .env("GIT_INDEX_FILE", index)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args([
            "--no-replace-objects",
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "-C",
        ])
        .arg(repo)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(bytes) = input {
        let result = child
            .stdin
            .take()
            .ok_or(Error::Refused("Git stdin is unavailable"))?
            .write_all(bytes);
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Refused(
            "Git could not prepare the selected patch; checkout state is unchanged",
        ));
    }
    Ok(output.stdout)
}

pub(super) fn prepare(repo: &Path, scratch_parent: &Path, state: &State) -> Result<Prepared> {
    let repo = checkout(repo)?;
    let baseline = state.accepted_baseline();
    let plan = source::plan(&repo, &baseline.target)?;
    ancestry(&repo, state, &plan.source_revision)?;
    let (original, mode) = content(&repo, &plan, baseline)?;
    let selected = selected_content(&original, state)?;
    let patch = if selected == original {
        Vec::new()
    } else {
        let scratch = Scratch::new(scratch_parent)?;
        let index = scratch.0.join("index");
        git(&repo, &["read-tree", &plan.source_revision], None, &index)?;
        // Only public source plus the selected typed values enter this object store.
        // No private review objects, ancestry, captures or local policies are copied.
        let blob = text(git(
            &repo,
            &["hash-object", "-w", "--stdin"],
            Some(selected.as_bytes()),
            &index,
        )?)?;
        git(
            &repo,
            &[
                "update-index",
                "--cacheinfo",
                &mode,
                blob.trim(),
                &baseline.source_path,
            ],
            None,
            &index,
        )?;
        git(
            &repo,
            &[
                "diff",
                "--cached",
                "--text",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--unified=3",
                &plan.source_revision,
                "--",
                &baseline.source_path,
            ],
            None,
            &index,
        )?
    };
    if patch.len() > noctalia::MAX_EXPORT {
        return Err(Error::Refused("selected patch exceeds the output bound"));
    }
    if text(source::git(&repo, &["rev-parse", "HEAD"])?)?.trim() != plan.source_revision {
        return Err(Error::Refused(
            "source advanced during patch preparation; review and export again",
        ));
    }
    Ok(Prepared {
        source_revision: plan.source_revision,
        source_path: baseline.source_path.clone(),
        patch,
    })
}

/// This verifies a local source commit, not registry publication or deployment.
pub(super) fn verify_commit(repo: &Path, commit: &str, state: &State) -> Result<Settings> {
    let repo = checkout(repo)?;
    let baseline = state.accepted_baseline();
    let plan = source::plan_revision(&repo, &baseline.target, commit)?;
    ancestry(&repo, state, commit)?;
    if source::git(&repo, &["merge-base", "--is-ancestor", commit, "HEAD"]).is_err() {
        return Err(Error::Refused(
            "source commit is not retained in the current checkout ancestry",
        ));
    }
    let (text, _) = content(&repo, &plan, baseline)?;
    let settings = noctalia::project(noctalia::APP_VERSION, &text)?;
    // Validate the complete transition on a copy; caller persists using its read revision.
    let mut candidate = state.clone();
    candidate.record_source_commit(commit, settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests;
