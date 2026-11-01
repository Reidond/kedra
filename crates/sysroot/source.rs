//! Unprivileged source planning/assembly; never linked into the privileged helper.
use crate::source_packages::SYSTEM;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::{fmt, path::Path, process::Command};
use sysroot_catalog::language::{Content, Intent, OsReleaseIntent};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Git(String),
    Invalid(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "source I/O: {e}"),
            Self::Git(e) => write!(f, "Git read failed: {e}"),
            Self::Invalid(e) => write!(f, "invalid source: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}
fn text(b: &[u8]) -> Result<&str, Error> {
    std::str::from_utf8(b).map_err(|_| invalid("input must be UTF-8"))
}

#[derive(Debug, Serialize)]
pub struct Target {
    pub id: String,
    pub architecture: String,
    pub image: String,
    pub fedora_release: u32,
    pub candidate_target: bool,
    pub hardware_status: String,
}
#[derive(Debug, Serialize)]
pub struct File {
    /// The committed file that holds the bytes: a `file("./…")` resource, or
    /// the module of an inline literal or a generated manifest.
    pub source_path: String,
    /// The declaring set or target, `system/<module>#<declaration>`.
    pub origin: String,
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_blob: Option<String>,
    pub sha256: String,
    pub mode: String,
    pub home_baseline: bool,
    /// Frozen with the plan; materialization checks them against `sha256`.
    #[serde(skip)]
    bytes: Vec<u8>,
}
#[derive(Debug, Serialize)]
pub struct Plan {
    pub schema_version: u32,
    pub source_revision: String,
    pub input_scope: &'static str,
    pub target: Target,
    pub packages: Vec<String>,
    pub remove_packages: Vec<String>,
    /// The target's image steps (`sysroot derive`), declared in its `.kedra` block.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub native_steps: Vec<sysroot_engine::NativeStep>,
    /// Foundation edits `image/assemble.sh` applies to RPM-owned files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_release: Option<OsReleaseIntent>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub hide_autostart: BTreeMap<String, BTreeSet<String>>,
    pub(crate) package_frontend: crate::source_packages::Frontend,
    pub files: Vec<File>,
}
pub(crate) struct Entry {
    pub(crate) mode: String,
    pub(crate) blob: String,
    pub(crate) path: String,
}

/// The managed home file list `sysroot home` reads, generated from the
/// declarations' `managed` home files.
pub const HOME_FILES: &str = "usr/share/kedra/home-files.toml";
const HOME_BASELINE: &str = "usr/share/sysroot/home/default/";

pub(crate) fn git(repo: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let output = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["--no-replace-objects", "--literal-pathspecs", "-C"])
        .arg(repo)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(output.stdout)
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['\\', ':'])
        && !s.chars().any(char::is_control)
        && s.split('/').all(|p| !matches!(p, "" | "." | ".." | ".git"))
}
fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn private_payload(root: &str, rest: &str) -> bool {
    let path = rest.to_ascii_lowercase();
    let name = path.rsplit('/').next().unwrap_or_default();
    if matches!(
        name,
        "auth.json" | ".credentials.json" | "credentials.json" | "settings.local.json"
    ) {
        return true;
    }
    if root == "etc"
        && (matches!(path.as_str(), "shadow" | "gshadow")
            || path.starts_with("ssh/ssh_host_") && !path.ends_with(".pub"))
    {
        return true;
    }
    root == "home"
        && [
            ".ssh",
            ".codex",
            ".claude",
            ".cache",
            ".local/state",
            ".local/share/keyrings",
            ".config/bitwarden",
            ".config/bitwarden desktop",
        ]
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}
fn entries(repo: &Path, revision: &str) -> Result<Vec<Entry>, Error> {
    // Whole-tree paths, even when the checkout path is a subdirectory.
    let bytes = git(repo, &["ls-tree", "--full-tree", "-r", "-z", revision])?;
    bytes
        .split(|b| *b == 0)
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (metadata, path) = text(item)?
                .split_once('\t')
                .ok_or_else(|| invalid("Git tree record"))?;
            let parts: Vec<_> = metadata.split(' ').collect();
            if parts.len() != 3 {
                return Err(invalid("Git tree metadata"));
            }
            Ok(Entry {
                mode: parts[0].into(),
                blob: parts[2].into(),
                path: path.into(),
            })
        })
        .collect()
}
pub(crate) fn blob(repo: &Path, entry: &Entry) -> Result<Vec<u8>, Error> {
    if !matches!(entry.mode.as_str(), "100644" | "100755") || !safe_path(&entry.path) {
        return Err(invalid(format!(
            "unsupported file mode/path: {} ({})",
            entry.path, entry.mode
        )));
    }
    git(repo, &["cat-file", "blob", &entry.blob])
}

/// Resolve HEAD once; read its blobs without filters, hooks, index or working files.
/// This plan describes committed inputs, not build success or deployment eligibility.
pub fn plan(repo: &Path, host: &str) -> Result<Plan, Error> {
    if !identifier(host) {
        return Err(invalid("host must be a lowercase target identifier"));
    }
    let revision_bytes = git(repo, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let revision = text(&revision_bytes)?.trim().to_owned();
    plan_committed(repo, host, revision, "committed HEAD only")
}

/// Only `system/` declares what enters an image. Root `etc/` and `usr/` are no
/// longer payload, so a committed file there is refused rather than ignored;
/// `crates/`, `usr/src/kedra/` and dot directories are development source.
fn refuse_payload_directories(tree: &[Entry]) -> Result<(), Error> {
    for entry in tree {
        let Some((top, rest)) = entry.path.split_once('/') else {
            continue;
        };
        let development = matches!(top, "system" | "crates")
            || top.starts_with('.')
            || top == "usr" && rest.starts_with("src/kedra/");
        if !development {
            return Err(invalid(format!(
                "unexpected committed path {}; image files and the home baseline are declared in system/*.kedra",
                entry.path
            )));
        }
    }
    Ok(())
}

/// The declared target, refused unless Kedra policy enables it.
fn target(host: &str, intent: &Intent) -> Result<Target, Error> {
    let missing = |field: &str| invalid(format!("target {host} declares no {field}"));
    let architecture = intent
        .architecture
        .clone()
        .ok_or_else(|| missing("architecture"))?;
    let image = intent.image.clone().ok_or_else(|| missing("image"))?;
    let hardware_status = intent.hardware.clone().ok_or_else(|| missing("hardware"))?;
    if image != format!("ghcr.io/reidond/kedra-{host}")
        || sysroot_core::targets::enabled(host, &architecture)
            .is_none_or(|spec| spec.repository != image)
    {
        return Err(invalid(format!(
            "target {host} with architecture {architecture:?} and image {image:?} is not enabled"
        )));
    }
    Ok(Target {
        id: host.into(),
        architecture,
        image,
        fedora_release: 44,
        candidate_target: true,
        hardware_status,
    })
}

fn contains_private_key(bytes: &[u8]) -> bool {
    [
        b"-----BEGIN PRIVATE KEY-----".as_slice(),
        b"-----BEGIN OPENSSH PRIVATE KEY-----",
        b"-----BEGIN RSA PRIVATE KEY-----",
        b"-----BEGIN EC PRIVATE KEY-----",
        b"-----BEGIN ENCRYPTED PRIVATE KEY-----",
    ]
    .iter()
    .any(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
}

/// Committed bytes of declared content: an inline literal belongs to its
/// module, a `file("./…")` resource to its own blob.
struct Contents<'a> {
    repo: &'a Path,
    tree: &'a [Entry],
}
impl Contents<'_> {
    fn file(&self, origin: &str, destination: String, content: &Content) -> Result<File, Error> {
        let module = origin.split('#').next().unwrap_or_default();
        let (source_path, git_blob, bytes, executable) = match content {
            Content::Inline { bytes, executable } => (
                format!("{SYSTEM}{module}"),
                None,
                bytes.as_bytes().to_vec(),
                *executable,
            ),
            Content::File { path, executable } => {
                let path = format!("{SYSTEM}{path}");
                let entry = self
                    .tree
                    .iter()
                    .find(|e| e.path == path)
                    .ok_or_else(|| invalid(format!("missing committed {path}")))?;
                let bytes = blob(self.repo, entry)?;
                (path, Some(entry.blob.clone()), bytes, *executable)
            }
        };
        if contains_private_key(&bytes) {
            return Err(invalid(format!(
                "private-key material cannot enter image payload: {destination}"
            )));
        }
        Ok(File {
            source_path,
            origin: format!("{SYSTEM}{origin}"),
            git_blob,
            sha256: sha256(&bytes),
            mode: if executable { "100755" } else { "100644" }.into(),
            home_baseline: destination.starts_with(HOME_BASELINE),
            destination,
            bytes,
        })
    }
}

/// The generated `sysroot home` list: TOML whose strings and string arrays are
/// written as JSON, which is valid TOML for these values.
fn home_files(intent: &Intent) -> Result<Vec<u8>, Error> {
    fn json(value: &impl Serialize) -> Result<String, Error> {
        serde_json::to_string(value).map_err(|e| invalid(format!("home file list: {e}")))
    }
    let mut text = String::from(
        "# Generated from the managed home files of system/*.kedra; do not edit.\nschema_version = 1\n",
    );
    for (path, home) in intent.home.iter().filter(|(_, home)| home.managed) {
        text.push_str(&format!("\n[[file]]\npath = {}\n", json(path)?));
        for (field, argv) in [("check", &home.check), ("reload", &home.reload)] {
            if let Some(argv) = argv {
                text.push_str(&format!("{field} = {}\n", json(argv)?));
            }
        }
    }
    Ok(text.into_bytes())
}

/// Image files and the home baseline by destination, from the declarations.
fn payload_files(
    repo: &Path,
    tree: &[Entry],
    intent: &Intent,
    entry: &str,
) -> Result<BTreeMap<String, File>, Error> {
    let contents = Contents { repo, tree };
    let mut files = BTreeMap::new();
    for (path, file) in &intent.files {
        let rest = path.trim_start_matches('/');
        let root = rest.split('/').next().unwrap_or_default();
        if private_payload(root, rest.split_once('/').map_or("", |(_, r)| r)) || rest == HOME_FILES
        {
            return Err(invalid(format!(
                "{path} cannot be declared: credential, runtime or generated path"
            )));
        }
        files.insert(
            rest.to_owned(),
            contents.file(&file.origin, rest.to_owned(), &file.content)?,
        );
    }
    for (path, home) in &intent.home {
        if private_payload("home", path) {
            return Err(invalid(format!(
                "credential/runtime home path cannot enter image payload: {path}"
            )));
        }
        let destination = format!("{HOME_BASELINE}{path}");
        files.insert(
            destination.clone(),
            contents.file(&home.origin, destination, &home.content)?,
        );
    }
    let bytes = home_files(intent)?;
    files.insert(
        HOME_FILES.into(),
        File {
            source_path: format!("{SYSTEM}{entry}"),
            origin: format!("{SYSTEM}{entry}#{}", intent.target),
            destination: HOME_FILES.into(),
            git_blob: None,
            sha256: sha256(&bytes),
            mode: "100644".into(),
            home_baseline: false,
            bytes,
        },
    );
    for path in files.keys() {
        if !safe_path(path) {
            return Err(invalid(format!("unsafe payload path {path}")));
        }
    }
    refuse_file_directory_collisions(&files)?;
    Ok(files)
}

fn refuse_file_directory_collisions(files: &BTreeMap<String, File>) -> Result<(), Error> {
    for path in files.keys() {
        for (position, _) in path.match_indices('/') {
            if files.contains_key(&path[..position]) {
                return Err(invalid(format!("file/directory collision at {path}")));
            }
        }
    }
    Ok(())
}

fn plan_committed(
    repo: &Path,
    host: &str,
    revision: String,
    input_scope: &'static str,
) -> Result<Plan, Error> {
    let tree = entries(repo, &revision)?;
    refuse_payload_directories(&tree)?;
    let selection = crate::source_packages::select(repo, &tree, host)?;
    let intent = selection.intent;
    if let Some(package) = intent.packages.intersection(&intent.remove).next() {
        return Err(invalid(format!(
            "package {package} is both installed and removed"
        )));
    }
    let entry = selection
        .frontend
        .entry
        .strip_prefix(SYSTEM)
        .unwrap_or_default()
        .to_owned();
    let files = payload_files(repo, &tree, &intent, &entry)?;
    Ok(Plan {
        schema_version: 1,
        source_revision: revision,
        input_scope,
        target: target(host, &intent)?,
        packages: intent.packages.iter().cloned().collect(),
        remove_packages: intent.remove.iter().cloned().collect(),
        native_steps: intent.native_steps.clone(),
        os_release: intent.os_release.clone(),
        hide_autostart: intent.hide_autostart.clone(),
        package_frontend: selection.frontend,
        files: files.into_values().collect(),
    })
}

fn append<W: Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
    mode: u32,
) -> Result<(), Error> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(mode);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_size(bytes.len() as u64);
    builder.append_data(&mut header, path, bytes)?;
    Ok(())
}

/// Emit the exact bytes a frozen plan validated, without resolving HEAD again,
/// then its provenance manifest.
pub(crate) fn materialize(
    plan: &Plan,
    mut emit: impl FnMut(&str, &[u8], u32) -> Result<(), Error>,
) -> Result<(), Error> {
    for entry in &plan.files {
        let bytes = &entry.bytes;
        if sha256(bytes) != entry.sha256 {
            return Err(invalid("frozen content changed during materialization"));
        }
        let mode = match entry.mode.as_str() {
            "100755" => 0o755,
            "100644" => 0o644,
            _ => return Err(invalid("unsupported frozen source mode")),
        };
        emit(&entry.destination, bytes, mode)?;
    }
    let manifest = serde_json::to_vec_pretty(plan)
        .map_err(|e| invalid(format!("manifest serialization: {e}")))?;
    emit("usr/share/sysroot/source.json", &manifest, 0o644)
}

/// Materialize only validated raw blobs plus a provenance manifest into a new tar.
/// Existing output paths are never overwritten. A failed write leaves an incomplete
/// artifact which must not be consumed; successful callers also check their exit code.
pub fn archive(repo: &Path, host: &str, output: &Path) -> Result<Plan, Error> {
    let plan = plan(repo, host)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut builder = tar::Builder::new(file);
    builder.follow_symlinks(false);
    materialize(&plan, |path, bytes, mode| {
        append(&mut builder, path, bytes, mode)
    })?;
    builder.finish()?;
    builder.into_inner()?.sync_all()?;
    Ok(plan)
}
