//! The image's managed home file list (`usr/share/kedra/home-files.toml`), its
//! baselines and the declared check and reload commands.
use serde::Deserialize;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(crate) const DATA_FILE: &str = "usr/share/kedra/home-files.toml";
pub(crate) const BASELINE: &str = "usr/share/sysroot/home/default";
pub(crate) const SOURCE: &str = "usr/share/sysroot/source.json";
/// Live, baseline and state copies share the replacement primitive's bound.
pub(crate) const MAX_FILE: usize = sysroot_linux::replace_file::MAX_FILE;
const MAX_DATA: usize = 65_536;
const MAX_ENTRIES: usize = 256;
const MAX_ARGUMENTS: usize = 64;
const MAX_OUTPUT: usize = 16_384;
/// Suffix of the file an update writes next to a live file it could not merge.
pub(crate) const CONFLICT_SUFFIX: &str = ".kedra-new";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    #[serde(default, rename = "file")]
    files: Vec<ManagedFile>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedFile {
    pub path: String,
    #[serde(default)]
    pub check: Option<Vec<String>>,
    #[serde(default)]
    pub reload: Option<Vec<String>>,
}

/// A home-relative path without traversal, links through `..` or hidden suffixes.
pub(crate) fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with('/')
        && !path.ends_with(CONFLICT_SUFFIX)
        && !path.contains(['\\', '\0'])
        && !path.chars().any(char::is_control)
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn valid_command(argv: &[String]) -> bool {
    !argv.is_empty()
        && argv.len() <= MAX_ARGUMENTS
        && argv[0].starts_with('/')
        && argv
            .iter()
            .all(|argument| argument.len() <= 4096 && !argument.contains('\0'))
}

fn validate(document: &Document) -> Result<()> {
    if document.schema_version != 1 {
        return Err("managed home file list has an unsupported schema_version".into());
    }
    if document.files.len() > MAX_ENTRIES {
        return Err("managed home file list has too many entries".into());
    }
    let mut seen = BTreeSet::new();
    for file in &document.files {
        if !valid_path(&file.path) || !seen.insert(file.path.as_str()) {
            return Err(format!("invalid or duplicate managed path {:?}", file.path).into());
        }
        if [&file.check, &file.reload]
            .into_iter()
            .flatten()
            .any(|argv| !valid_command(argv))
        {
            return Err(format!(
                "{}: check and reload must be argv with an absolute program",
                file.path
            )
            .into());
        }
    }
    Ok(())
}

/// Image-owned inputs: `/` with root-owned files, or an unpacked image root
/// owned by the caller (tests and the lab's working-tree sync).
pub(crate) struct Image {
    root: PathBuf,
    owner: u32,
}

fn missing(error: &(dyn std::error::Error + 'static)) -> bool {
    error.downcast_ref::<rustix::io::Errno>() == Some(&rustix::io::Errno::NOENT)
        || error
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}

impl Image {
    pub fn open(root: &Path) -> Result<Self> {
        if !root.is_absolute() {
            return Err("image root must be an absolute path".into());
        }
        let root = root.canonicalize()?;
        let owner = if root == Path::new("/") {
            0
        } else {
            rustix::process::geteuid().as_raw()
        };
        Ok(Self { root, owner })
    }

    /// Bytes of one image file, or `None` when it does not exist.
    pub fn read(&self, relative: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        match sysroot_linux::trusted_file::read(&self.root.join(relative), self.owner, limit) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if missing(error.as_ref()) => Ok(None),
            Err(error) => Err(format!("{relative}: {error}").into()),
        }
    }

    pub fn files(&self) -> Result<Vec<ManagedFile>> {
        let bytes = self
            .read(DATA_FILE, MAX_DATA)?
            .ok_or("this image declares no managed home files")?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| "managed home file list is not UTF-8")?;
        let document: Document =
            toml::from_str(text).map_err(|error| format!("{DATA_FILE}: {error}"))?;
        validate(&document)?;
        Ok(document.files)
    }

    pub fn baseline(&self, path: &str) -> Result<Option<Vec<u8>>> {
        self.read(&format!("{BASELINE}/{path}"), MAX_FILE)
    }
}

/// The declared entries for the requested paths, in data-file order; all when empty.
pub(crate) fn select(files: Vec<ManagedFile>, paths: &[String]) -> Result<Vec<ManagedFile>> {
    if let Some(path) = paths
        .iter()
        .find(|path| !files.iter().any(|file| &file.path == *path))
    {
        return Err(format!("{path} is not a managed home file").into());
    }
    Ok(files
        .into_iter()
        .filter(|file| paths.is_empty() || paths.contains(&file.path))
        .collect())
}

/// Result of a declared command: success and its bounded combined output.
pub(crate) struct Outcome {
    pub success: bool,
    pub output: String,
}

/// Keep the first bytes of a stream and discard the rest, so a verbose command
/// never blocks on a full pipe.
fn bounded(mut stream: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = (&mut stream)
        .take(MAX_OUTPUT as u64)
        .read_to_end(&mut bytes);
    let _ = std::io::copy(&mut stream, &mut std::io::sink());
    bytes
}

/// Run one declared argv with `{file}` replaced, bounded in time and output.
pub(crate) fn execute(argv: &[String], file: Option<&Path>) -> Result<Outcome> {
    let file = match file {
        Some(path) => Some(path.to_str().ok_or("managed file path is not UTF-8")?),
        None => None,
    };
    let argv: Vec<String> = argv
        .iter()
        .map(|argument| match file {
            Some(file) => argument.replace("{file}", file),
            None => argument.clone(),
        })
        .collect();
    let mut child = Command::new("/usr/bin/timeout")
        .args(["--kill-after=2s", "30s"])
        .args(&argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{}: {error}", argv[0]))?;
    let stderr = child.stderr.take().ok_or("command output is unavailable")?;
    let reader = std::thread::spawn(move || bounded(stderr));
    let mut bytes = bounded(child.stdout.take().ok_or("command output is unavailable")?);
    let status = child.wait()?;
    bytes.extend(reader.join().unwrap_or_default());
    Ok(Outcome {
        success: status.success(),
        output: String::from_utf8_lossy(&bytes).trim().to_owned(),
    })
}
