//! Unprivileged source planning/assembly; never linked into the privileged helper.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::{fmt, path::Path, process::Command};
use sysroot_core::layout;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) package_frontend: Option<crate::source_packages::Frontend>,
    pub files: Vec<File>,
}
pub(crate) struct Entry {
    pub(crate) mode: String,
    pub(crate) blob: String,
    pub(crate) path: String,
}

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
    plan_committed(repo, host, revision, "committed HEAD only")
}

/// Read an exact retained source commit without moving HEAD or the index.
#[cfg(target_os = "linux")]
pub(crate) fn plan_revision(repo: &Path, host: &str, revision: &str) -> Result<Plan, Error> {
    if !identifier(host)
        || !matches!(revision.len(), 40 | 64)
        || !revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(
            "an explicit target and full lowercase commit ID are required",
        ));
    }
    let resolved = git(
        repo,
        &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
    )?;
    if text(&resolved)?.trim() != revision {
        return Err(invalid("source revision must identify the commit itself"));
    }
    plan_committed(repo, host, revision.into(), "exact committed revision only")
}

/// Committed input paths of one target, in the layout of one commit.
/// See `sysroot_core::layout`; retained legacy commits keep their old meaning.
struct Inputs {
    legacy: bool,
    target: String,
    packages: [String; 2],
    remove: String,
    overlay: String,
}

impl Inputs {
    fn new(tree: &[Entry], host: &str) -> Result<Self, Error> {
        let legacy = tree.iter().any(|e| {
            layout::LEGACY_ROOTS
                .iter()
                .any(|root| e.path.starts_with(root))
        });
        if legacy && tree.iter().any(|e| e.path.starts_with(layout::DEVELOPMENT)) {
            return Err(invalid(
                "source mixes the root-filesystem layout with legacy hosts/, packages/ or home/",
            ));
        }
        Ok(if legacy {
            Self {
                legacy,
                target: format!("hosts/{host}/host.toml"),
                packages: [
                    "packages/common.list".into(),
                    format!("hosts/{host}/packages.list"),
                ],
                remove: "packages/remove.list".into(),
                overlay: format!("hosts/{host}/"),
            }
        } else {
            let image = layout::IMAGE;
            Self {
                legacy,
                target: format!("{image}targets/{host}/target.toml"),
                packages: [
                    format!("{image}packages.list"),
                    format!("{image}targets/{host}/packages.list"),
                ],
                remove: format!("{image}remove.list"),
                overlay: format!("{image}targets/{host}/"),
            }
        })
    }

    /// Payload root (`etc`, `usr` or `home`) and the path below it, for a path
    /// relative to the shared root or the target overlay. Anything the layout
    /// does not recognize is refused rather than silently left out.
    fn payload<'a>(
        &self,
        shared: bool,
        path: &str,
        relative: &'a str,
    ) -> Result<Option<(&'static str, &'a str)>, Error> {
        let directory = |path: &str| invalid(format!("payload root must be a directory: {path}"));
        if self.legacy {
            if matches!(relative, "etc" | "usr" | "home") {
                return Err(directory(path));
            }
            return Ok(relative.split_once('/').and_then(|(root, rest)| {
                let root = match root {
                    "etc" => "etc",
                    "usr" => "usr",
                    "home" => "home",
                    _ => return None,
                };
                Some((root, rest))
            }));
        }
        if matches!(relative, "etc" | "usr" | "etc/skel") {
            return Err(directory(path));
        }
        let Some((top, rest)) = relative.split_once('/') else {
            // Root files are never payload; an overlay holds only its declarations.
            if shared
                || matches!(
                    relative,
                    "target.toml" | "packages.list" | "README.md" | ".gitkeep"
                )
            {
                return Ok(None);
            }
            return Err(invalid(format!(
                "unexpected file in target overlay: {path}"
            )));
        };
        match top {
            "etc" => Ok(Some(match rest.strip_prefix("skel/") {
                Some(home) => ("home", home),
                None => ("etc", rest),
            })),
            "usr" if rest == "src" || rest.starts_with("src/") => {
                if shared && rest.starts_with("src/kedra/") {
                    Ok(None)
                } else {
                    Err(invalid(format!(
                        "usr/src/ holds only the usr/src/kedra/ development tree: {path}"
                    )))
                }
            }
            "usr" => Ok(Some(("usr", rest))),
            _ if shared && top.starts_with('.') => Ok(None),
            _ => Err(invalid(format!(
                "unexpected directory {top}/ at {path}; only etc/ and usr/ form the image filesystem"
            ))),
        }
    }
}

/// The committed target declaration, refused unless Kedra policy enables it.
fn committed_target(
    repo: &Path,
    tree: &[Entry],
    host: &str,
    target_path: &str,
) -> Result<Target, Error> {
    let entry = tree
        .iter()
        .find(|e| e.path == target_path)
        .ok_or_else(|| invalid(format!("missing committed {target_path}")))?;
    let bytes = blob(repo, entry)?;
    let target: Target =
        toml::from_str(text(&bytes)?).map_err(|e| invalid(format!("{target_path}: {e}")))?;
    if target.id != host
        || target.fedora_release != 44
        || target.image != format!("ghcr.io/reidond/kedra-{host}")
    {
        return Err(invalid(
            "target identity, repository or Fedora release does not match Kedra policy",
        ));
    }
    if !target.candidate_target {
        return Err(invalid(format!("target {host} is disabled")));
    }
    if sysroot_core::targets::enabled(host, &target.architecture)
        .is_none_or(|spec| spec.repository != target.image)
    {
        return Err(invalid(format!(
            "target {host} with architecture {:?} is not enabled",
            target.architecture
        )));
    }
    Ok(target)
}

/// Package names one target installs and removes, and the package-language
/// frontend record when the commit selects that language.
struct PackageSets {
    install: BTreeSet<String>,
    remove: BTreeSet<String>,
    frontend: Option<crate::source_packages::Frontend>,
}

/// Package sets from the package language when the commit declares it and from
/// the package lists otherwise. No package may be both installed and removed.
fn package_sets(
    repo: &Path,
    tree: &[Entry],
    host: &str,
    inputs: &Inputs,
) -> Result<PackageSets, Error> {
    let packages = match crate::source_packages::select(repo, tree, host, inputs.legacy)? {
        Some(selection) => PackageSets {
            install: selection.packages,
            remove: selection.remove,
            frontend: Some(selection.frontend),
        },
        None => {
            let mut install = package_list(repo, tree, &inputs.packages[0])?;
            install.extend(package_list(repo, tree, &inputs.packages[1])?);
            PackageSets {
                install,
                remove: package_list(repo, tree, &inputs.remove)?,
                frontend: None,
            }
        }
    };
    if let Some(package) = packages.install.intersection(&packages.remove).next() {
        return Err(invalid(format!(
            "package {package} is both installed and removed"
        )));
    }
    Ok(packages)
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

/// The image file one payload entry contributes, or `None` for a `.gitkeep`
/// placeholder. Paths and contents that must not enter an image are refused.
fn payload_file(repo: &Path, entry: &Entry, root: &str, rest: &str) -> Result<Option<File>, Error> {
    if private_payload(root, rest) {
        return Err(invalid(format!(
            "credential/runtime path cannot enter image payload: {}",
            entry.path
        )));
    }
    if rest.split('/').any(|p| p == ".gitkeep") {
        return Ok(None);
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
    if contains_private_key(&bytes) {
        return Err(invalid(format!(
            "private-key material cannot enter image payload: {}",
            entry.path
        )));
    }
    let destination = if root == "home" {
        format!("usr/share/sysroot/home/default/{rest}")
    } else {
        format!("{root}/{rest}")
    };
    if !safe_path(&destination) {
        return Err(invalid("unsafe payload path"));
    }
    Ok(Some(File {
        source_path: entry.path.clone(),
        destination,
        git_blob: entry.blob.clone(),
        sha256: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        mode: entry.mode.clone(),
        replaces: None,
        home_baseline: root == "home",
    }))
}

/// Payload files by destination: the shared root first, then the target overlay,
/// whose files record the shared file they replace.
fn payload_files(
    repo: &Path,
    tree: &[Entry],
    inputs: &Inputs,
) -> Result<BTreeMap<String, File>, Error> {
    let mut files: BTreeMap<String, File> = BTreeMap::new();
    for prefix in ["", inputs.overlay.as_str()] {
        for entry in tree {
            let Some(relative) = entry.path.strip_prefix(prefix) else {
                continue;
            };
            let Some((root, rest)) = inputs.payload(prefix.is_empty(), &entry.path, relative)?
            else {
                continue;
            };
            let Some(mut file) = payload_file(repo, entry, root, rest)? else {
                continue;
            };
            file.replaces = files
                .get(&file.destination)
                .map(|previous| previous.source_path.clone());
            files.insert(file.destination.clone(), file);
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
    let inputs = Inputs::new(&tree, host)?;
    let target = committed_target(repo, &tree, host, &inputs.target)?;
    let packages = package_sets(repo, &tree, host, &inputs)?;
    let files = payload_files(repo, &tree, &inputs)?;
    Ok(Plan {
        schema_version: 1,
        source_revision: revision,
        input_scope,
        target,
        packages: packages.install.into_iter().collect(),
        remove_packages: packages.remove.into_iter().collect(),
        package_frontend: packages.frontend,
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

/// Read the exact validated blobs of a frozen plan, without resolving HEAD again.
/// The generated manifest retains the existing source archive schema and bytes.
pub(crate) fn materialize(
    repo: &Path,
    plan: &Plan,
    mut emit: impl FnMut(&str, &[u8], u32) -> Result<(), Error>,
) -> Result<(), Error> {
    for entry in &plan.files {
        let bytes = git(repo, &["cat-file", "blob", &entry.git_blob])?;
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if hash != entry.sha256 {
            return Err(invalid("Git object content changed during materialization"));
        }
        let mode = match entry.mode.as_str() {
            "100755" => 0o755,
            "100644" => 0o644,
            _ => return Err(invalid("unsupported frozen source mode")),
        };
        emit(&entry.destination, &bytes, mode)?;
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
    materialize(repo, &plan, |path, bytes, mode| {
        append(&mut builder, path, bytes, mode)
    })?;
    builder.finish()?;
    builder.into_inner()?.sync_all()?;
    Ok(plan)
}
