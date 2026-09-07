//! Read-only image planning from one immutable Git commit, never the live home.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{fmt, path::Path, process::Command};

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    pub source_path: String,
    pub destination: String,
    pub git_blob: String,
    pub sha256: String,
    pub mode: String,
    pub replaces: Option<String>,
    pub home_baseline: bool,
}
#[derive(Debug, Serialize)]
pub struct Plan {
    pub schema_version: u32,
    pub source_revision: String,
    pub input_scope: &'static str,
    pub target: Target,
    pub packages: Vec<String>,
    pub remove_packages: Vec<String>,
    pub files: Vec<File>,
}
struct Entry {
    mode: String,
    blob: String,
    path: String,
}

fn git(repo: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
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
fn entries(repo: &Path, revision: &str) -> Result<Vec<Entry>, Error> {
    let bytes = git(repo, &["ls-tree", "-r", "-z", revision])?;
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
fn blob(repo: &Path, entry: &Entry) -> Result<Vec<u8>, Error> {
    if !matches!(entry.mode.as_str(), "100644" | "100755") || !safe_path(&entry.path) {
        return Err(invalid(format!(
            "unsupported file mode/path: {} ({})",
            entry.path, entry.mode
        )));
    }
    git(repo, &["cat-file", "blob", &entry.blob])
}
fn package_list(repo: &Path, tree: &[Entry], path: &str) -> Result<BTreeSet<String>, Error> {
    let entry = tree
        .iter()
        .find(|e| e.path == path)
        .ok_or_else(|| invalid(format!("missing committed {path}")))?;
    let bytes = blob(repo, entry)?;
    let mut result = BTreeSet::new();
    for line in text(&bytes)?.lines() {
        let name = line.split('#').next().unwrap_or_default().trim();
        if name.is_empty() {
            continue;
        }
        if !name.as_bytes()[0].is_ascii_alphanumeric()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+._-".contains(&b))
        {
            return Err(invalid(format!(
                "{path}: expected an RPM name, found {name:?}"
            )));
        }
        result.insert(name.into());
    }
    Ok(result)
}

/// Resolve HEAD once; read its blobs without filters, hooks, index or working files.
/// This plan describes committed inputs, not build success or deployment eligibility.
pub fn plan(repo: &Path, host: &str) -> Result<Plan, Error> {
    if !identifier(host) {
        return Err(invalid("host must be a lowercase target identifier"));
    }
    let revision_bytes = git(repo, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let revision = text(&revision_bytes)?.trim().to_owned();
    let tree = entries(repo, &revision)?;
    let target_path = format!("hosts/{host}/host.toml");
    let entry = tree
        .iter()
        .find(|e| e.path == target_path)
        .ok_or_else(|| invalid(format!("missing committed {target_path}")))?;
    let bytes = blob(repo, entry)?;
    let target: Target =
        toml::from_str(text(&bytes)?).map_err(|e| invalid(format!("{target_path}: {e}")))?;
    if target.id != host
        || target.architecture != "x86_64"
        || target.fedora_release != 44
        || target.image != format!("ghcr.io/reidond/kedra-{host}")
    {
        return Err(invalid(
            "target identity, architecture, repository or Fedora release does not match Kedra policy",
        ));
    }
    if !target.candidate_target {
        return Err(invalid(format!("target {host} is disabled")));
    }
    let mut packages = package_list(repo, &tree, "packages/common.list")?;
    packages.extend(package_list(
        repo,
        &tree,
        &format!("hosts/{host}/packages.list"),
    )?);
    let remove = package_list(repo, &tree, "packages/remove.list")?;
    if let Some(package) = packages.intersection(&remove).next() {
        return Err(invalid(format!(
            "package {package} is both installed and removed"
        )));
    }
    let mut files: BTreeMap<String, File> = BTreeMap::new();
    for prefix in [String::new(), format!("hosts/{host}/")] {
        for entry in &tree {
            let Some(relative) = entry.path.strip_prefix(&prefix) else {
                continue;
            };
            if matches!(relative, "etc" | "usr" | "home") {
                return Err(invalid(format!(
                    "payload root must be a directory: {}",
                    entry.path
                )));
            }
            let Some((root, rest)) = relative.split_once('/') else {
                continue;
            };
            if !matches!(root, "etc" | "usr" | "home") {
                continue;
            }
            if rest.split('/').any(|p| p == ".gitkeep") {
                continue;
            }
            if root == "usr" && (rest == "etc" || rest.starts_with("etc/")) {
                return Err(invalid(
                    "usr/etc is bootc-owned; use etc/ or application defaults",
                ));
            }
            if root == "usr" && (rest == "share/sysroot" || rest.starts_with("share/sysroot/")) {
                return Err(invalid(
                    "usr/share/sysroot is reserved for generated manifests and baselines",
                ));
            }
            let bytes = blob(repo, entry)?;
            let destination = if root == "home" {
                format!("usr/share/sysroot/home/default/{rest}")
            } else {
                relative.to_owned()
            };
            if !safe_path(&destination) {
                return Err(invalid("unsafe payload path"));
            }
            let replaces = files
                .get(&destination)
                .map(|previous| previous.source_path.clone());
            files.insert(
                destination.clone(),
                File {
                    source_path: entry.path.clone(),
                    destination,
                    git_blob: entry.blob.clone(),
                    sha256: Sha256::digest(&bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                    mode: entry.mode.clone(),
                    replaces,
                    home_baseline: root == "home",
                },
            );
        }
    }
    for path in files.keys() {
        for (position, _) in path.match_indices('/') {
            if files.contains_key(&path[..position]) {
                return Err(invalid(format!("file/directory collision at {path}")));
            }
        }
    }
    Ok(Plan {
        schema_version: 1,
        source_revision: revision,
        input_scope: "committed HEAD only",
        target,
        packages: packages.into_iter().collect(),
        remove_packages: remove.into_iter().collect(),
        files: files.into_values().collect(),
    })
}
