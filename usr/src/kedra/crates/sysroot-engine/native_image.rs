//! Offline image realization of a verified composition and its closed native
//! derivation. Builds run in BuildKit with networking disabled, no pulls and no
//! host mounts; every result is bound to its daemon, labels and parent layers.
use crate::{
    Error, MAX_JSON, NativeImplementation, NativePlan, NativeReceipt, OCI_ARCHITECTURE,
    OCI_PLATFORM, PLATFORM, Result, StepSelector, SystemComposition, SystemFileDisposition,
    VerifiedComposition,
    executor::{self, Connection},
    native,
    native_apply::{Observation, RPM_FORMAT},
    plan, store,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, DirBuilder, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    thread,
    time::{Duration, Instant},
};

const IMPLEMENTATION: &str = "sysroot-native-v4-buildkit";
const OWNER_LABEL: &str = "dev.kedra.sysroot.owner";
const OWNER: &str = "sysroot-derive";
const KIND_LABEL: &str = "dev.kedra.sysroot.kind";
const COMPOSITION_IDENTITY: &str = "dev.kedra.composition.identity";
const COMPOSITION_FOUNDATION: &str = "dev.kedra.composition.foundation";
const COMPOSITION_PAYLOAD: &str = "dev.kedra.composition.payload";
const NATIVE_IDENTITY: &str = "dev.kedra.native.identity";
const NATIVE_PARENT: &str = "dev.kedra.native.parent";
const PROBE_SECONDS: u64 = 300;
const BUILD_SECONDS: u64 = 3600;
const MAX_EXECUTOR: u64 = 256 * 1024 * 1024;
const MAX_IMAGE: u64 = 8 * 1024 * 1024 * 1024;
const STATE_LIMIT: u64 = 8 * 1024 * 1024;
const OVERRIDES: [&str; 6] = [
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "PYTHONHOME",
    "PYTHONPATH",
    "BASH_ENV",
    "ENV",
];
/// Every path component must be a real directory; store paths must be absent and
/// payload destinations ordinary non-executable single-link files.
const PATH_CHECK: &str = "set -eu; for file do p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; if [ -L \"$p\" ]; then echo 'foundation path is a symlink' >&2; exit 1; fi; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; if [ -e \"$p\" ] && [ ! -d \"$p\" ]; then exit 1; fi; fi; done; case \"$file\" in /usr/lib/sysroot/store/*) [ ! -e \"$file\" ] || exit 1;; *) if [ -e \"$file\" ]; then if [ ! -f \"$file\" ] || [ -x \"$file\" ]; then echo 'foundation destination is not ordinary configuration' >&2; exit 1; fi; if [ \"$(/usr/bin/stat -c %h -- \"$file\")\" != 1 ]; then echo 'foundation destination hardlinked' >&2; exit 1; fi; fi;; esac; done";
/// Passthrough files assert existing foundation bytes and mode.
const PASSTHROUGH_CHECK: &str = "set -eu; while [ $# -gt 0 ]; do file=$1; mode=$2; hash=$3; shift 3; p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; [ ! -L \"$p\" ] || exit 1; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; [ -d \"$p\" ] || exit 1; fi; done; [ -f \"$file\" ] || { echo 'foundation passthrough file missing' >&2; exit 1; }; [ \"$(/usr/bin/stat -c %a -- \"$file\")\" = \"$mode\" ] || { echo 'foundation passthrough mode mismatch' >&2; exit 1; }; actual=$(/usr/bin/sha256sum -- \"$file\"); [ \"${actual%% *}\" = \"$hash\" ] || { echo 'foundation passthrough content mismatch' >&2; exit 1; }; done";

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(format!("derive: {}", message.into()))
}

/// Verified static composition image in the local engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposedImage {
    pub identity: String,
    pub image: String,
    pub foundation: String,
    pub engine: String,
    pub outputs: BTreeMap<String, String>,
}

/// Native image with the material re-read from its final filesystem.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedImage {
    pub image: String,
    pub engine: String,
    pub parent_image: String,
    pub material: NativeReceipt,
}

/// The running sysroot binary: the only executor admitted into the build.
struct Executor {
    bytes: Vec<u8>,
    sha256: String,
}
impl Executor {
    fn current() -> Result<Self> {
        if !cfg!(target_os = "linux") {
            return Err(invalid(format!(
                "native derivation runs only on native Linux ({PLATFORM})"
            )));
        }
        let mut bytes = Vec::new();
        File::open("/proc/self/exe")?
            .take(MAX_EXECUTOR + 1)
            .read_to_end(&mut bytes)?;
        if bytes.is_empty() || bytes.len() as u64 > MAX_EXECUTOR {
            return Err(invalid("running sysroot executable is empty or oversized"));
        }
        let sha256 = plan::hash(&bytes);
        Ok(Self { bytes, sha256 })
    }
    fn implementation(&self) -> NativeImplementation {
        NativeImplementation {
            version: IMPLEMENTATION,
            executor_sha256: self.sha256.clone(),
        }
    }
}

/// Owner-private directory for locks, journals, bindings and snapshots.
struct State {
    path: PathBuf,
}
impl State {
    fn open(path: &Path) -> Result<Self> {
        match fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                DirBuilder::new().mode(0o700).create(path)?;
            }
            Err(error) => return Err(error.into()),
            Ok(_) => {}
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir()
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != rustix::process::geteuid().as_raw()
        {
            return Err(invalid("state must be an owner-private directory"));
        }
        Ok(Self {
            path: fs::canonicalize(path)?,
        })
    }
    fn lock(&self, key: &str) -> Result<File> {
        let lock = private_open(&self.path.join(format!("{key}.lock")), true)?;
        lock.lock()?;
        Ok(lock)
    }
    fn scratch(&self, purpose: &str) -> Result<PathBuf> {
        let path = self
            .path
            .join(format!("{purpose}-{}", &store::nonce()?[..24]));
        DirBuilder::new().mode(0o700).create(&path)?;
        Ok(path)
    }
}

fn private_metadata(metadata: &fs::Metadata, links: u64) -> Result<()> {
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.nlink() != links
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(invalid(
            "state must be an owner-only regular file without aliases",
        ));
    }
    Ok(())
}

fn private_open(path: &Path, create: bool) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(create)
        .create(create)
        .truncate(false)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)?;
    let metadata = file.metadata()?;
    private_metadata(&metadata, 1)?;
    let named = fs::symlink_metadata(path)?;
    if named.dev() != metadata.dev() || named.ino() != metadata.ino() {
        return Err(invalid("state changed while opening"));
    }
    Ok(file)
}

fn read_state<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let file = match private_open(path, false) {
        Ok(file) => file,
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(STATE_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > STATE_LIMIT {
        return Err(invalid("state exceeds 8 MiB"));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| invalid(format!("state: {error}")))
}

fn sync_parent(path: &Path) -> Result<()> {
    File::open(
        path.parent()
            .ok_or_else(|| invalid("state has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

/// A killed atomic publication may leave a partial next file or a second link to
/// the completed binding. Only that checked transaction-local path is retired.
fn retire_next(path: &Path) -> Result<()> {
    let next = path.with_extension("next");
    let metadata = match fs::symlink_metadata(&next) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.nlink() == 2 {
        private_metadata(&metadata, 2)?;
        let bound = fs::symlink_metadata(path)?;
        if !bound.is_file() || bound.dev() != metadata.dev() || bound.ino() != metadata.ino() {
            return Err(invalid("pending publication aliases another file"));
        }
    } else {
        private_metadata(&metadata, 1)?;
    }
    fs::remove_file(next)?;
    sync_parent(path)
}

fn write_state(path: &Path, value: &impl Serialize, replace: bool) -> Result<()> {
    retire_next(path)?;
    if replace {
        // Never replace unknown or foreign state; callers validate its schema first.
        match private_open(path, false) {
            Ok(_) => {}
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let next = path.with_extension("next");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(&next)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    if replace {
        fs::rename(&next, path)?;
    } else {
        // No-replace publication. A crash before unlink is recovered by retire_next.
        fs::hard_link(&next, path)?;
        fs::remove_file(&next)?;
    }
    sync_parent(path)
}

#[derive(Deserialize)]
struct Inspect {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Os")]
    os: String,
    #[serde(rename = "Architecture")]
    architecture: String,
    #[serde(rename = "Config", default)]
    config: Option<InspectConfig>,
    #[serde(rename = "RootFS", default)]
    root_fs: Option<RootFs>,
}
#[derive(Default, Deserialize)]
struct InspectConfig {
    #[serde(rename = "Labels", default)]
    labels: Option<BTreeMap<String, String>>,
    #[serde(rename = "Env", default)]
    env: Option<Vec<String>>,
    #[serde(rename = "OnBuild", default)]
    on_build: Option<Vec<String>>,
    #[serde(rename = "Volumes", default)]
    volumes: Option<BTreeMap<String, serde_json::Value>>,
}
#[derive(Deserialize)]
struct RootFs {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "Layers", default)]
    layers: Option<Vec<String>>,
}
impl Inspect {
    fn labels(&self) -> BTreeMap<String, String> {
        self.config
            .as_ref()
            .and_then(|config| config.labels.clone())
            .unwrap_or_default()
    }
    fn layers(&self) -> Result<Vec<String>> {
        let root = self
            .root_fs
            .as_ref()
            .ok_or_else(|| invalid("image has no RootFS metadata"))?;
        let layers = root.layers.clone().unwrap_or_default();
        if root.kind != "layers"
            || layers.is_empty()
            || layers.iter().any(|id| plan::image_id(id).is_err())
        {
            return Err(invalid("image filesystem layer metadata is invalid"));
        }
        Ok(layers)
    }
}

enum Member<'a> {
    Bytes(&'a [u8]),
    File(&'a Path),
}
struct Build<'a> {
    what: &'a str,
    containerfile: &'a [u8],
    members: Vec<(&'a str, Member<'a>, u32)>,
    labels: &'a BTreeMap<String, String>,
    tag: Option<&'a str>,
    output: Option<&'a Path>,
}

/// One exact local Unix Docker daemon, reached only through the Docker CLI.
struct Docker {
    connection: Connection,
    engine: String,
}
impl Docker {
    fn connect() -> Result<Self> {
        let connection = Connection::resolve()?;
        let engine = executor::daemon(&connection)?;
        Ok(Self { connection, engine })
    }
    fn same_engine(&self) -> Result<()> {
        if executor::daemon(&self.connection)? != self.engine {
            return Err(invalid("Docker engine identity changed"));
        }
        Ok(())
    }
    fn run(&self, args: &[&str], seconds: u64) -> Result<crate::RunResult> {
        let mut command = self.connection.command();
        command.args(args);
        executor::command(command, seconds)
    }
    fn checked(&self, args: &[&str], seconds: u64) -> Result<Vec<u8>> {
        Ok(executor::checked(self.run(args, seconds)?)?.stdout)
    }
    fn inspect(&self, reference: &str) -> Result<Option<Inspect>> {
        if reference.is_empty() || reference.starts_with('-') {
            return Err(invalid("unsafe image reference"));
        }
        let result = self.run(&["image", "inspect", reference], PROBE_SECONDS)?;
        if result.code != 0 && String::from_utf8_lossy(&result.stderr).contains("No such image") {
            return Ok(None);
        }
        let result = executor::checked(result)?;
        let mut values: Vec<Inspect> = serde_json::from_slice(&result.stdout)?;
        if values.len() != 1 {
            return Err(invalid("expected one local image"));
        }
        Ok(values.pop())
    }
    fn image_id(&self, reference: &str) -> Result<Option<String>> {
        Ok(self.inspect(reference)?.map(|image| image.id))
    }
    /// Exact native-platform image without inherited ONBUILD or volumes.
    fn native(&self, image: &str) -> Result<Inspect> {
        plan::image_id(image)?;
        let inspected = self
            .inspect(image)?
            .ok_or_else(|| invalid(format!("image {image} is not in the engine")))?;
        let config = inspected.config.as_ref();
        if inspected.id != image
            || inspected.os != "linux"
            || inspected.architecture != OCI_ARCHITECTURE
            || config.is_some_and(|c| c.on_build.as_ref().is_some_and(|v| !v.is_empty()))
            || config.is_some_and(|c| c.volumes.as_ref().is_some_and(|v| !v.is_empty()))
        {
            return Err(invalid(format!(
                "image ID/platform differs from native {OCI_PLATFORM} or declares ONBUILD/volumes"
            )));
        }
        Ok(inspected)
    }
    /// Tag only when absent or already bound to the exact image.
    fn publish(&self, id: &str, tag: &str) -> Result<()> {
        match self.image_id(tag)? {
            Some(actual) if actual != id => {
                return Err(invalid(format!(
                    "{tag} image ID/provenance binding mismatch: it names another image"
                )));
            }
            Some(_) => {}
            None => {
                self.checked(&["image", "tag", id, tag], PROBE_SECONDS)?;
            }
        }
        if self.image_id(tag)?.as_deref() != Some(id) {
            return Err(invalid(format!("{tag} changed during publication")));
        }
        Ok(())
    }
    /// BuildKit needs a named FROM; the name is derived from the exact image ID.
    fn pin(&self, id: &str) -> Result<String> {
        plan::image_id(id)?;
        let tag = format!("sysroot-foundation:{}", &id[7..]);
        self.publish(id, &tag)?;
        Ok(tag)
    }
    /// Remove only a temporary tag owned by this exact transaction.
    fn remove_pending(&self, kind: &str, tag: &str, id: &str, nonce: &str) -> Result<()> {
        if tag != format!("kedra-{kind}-pending:{nonce}") {
            return Err(invalid("invalid temporary tag"));
        }
        let Some(image) = self.inspect(tag)? else {
            return Ok(());
        };
        let labels = image.labels();
        if image.id != id
            || labels.get(OWNER_LABEL).map(String::as_str) != Some(OWNER)
            || labels.get(KIND_LABEL).map(String::as_str) != Some(kind)
            || labels.get(&format!("dev.kedra.{kind}.transaction")) != Some(&nonce.to_owned())
        {
            return Err(invalid("temporary tag is not owned by this transaction"));
        }
        self.checked(&["image", "rm", "--no-prune", tag], PROBE_SECONDS)?;
        Ok(())
    }
    fn load(&self, path: &Path, bytes: u64) -> Result<()> {
        if bytes > MAX_IMAGE || fs::metadata(path)?.len() != bytes {
            return Err(invalid("foundation archive size changed or exceeds 8 GiB"));
        }
        let mut command = self.connection.command();
        command
            .args(["image", "load", "--quiet", "--input"])
            .arg(path);
        executor::checked(executor::command(command, BUILD_SECONDS)?)?;
        Ok(())
    }
    /// Fixed probe in a read-only, unprivileged-capability, network-less container.
    fn probe(&self, image: &str, argv: &[&str]) -> Result<Vec<u8>> {
        plan::image_id(image)?;
        let name = format!("sysroot-derive-probe-{}", &store::nonce()?[..24]);
        let owner = format!("{OWNER_LABEL}={OWNER}");
        let kind = format!("{KIND_LABEL}=probe");
        let mut args = vec![
            "container",
            "create",
            "--pull",
            "never",
            "--name",
            &name,
            "--label",
            &owner,
            "--label",
            &kind,
            "--platform",
            OCI_PLATFORM,
            "--network",
            "none",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--user",
            "0:0",
            "--workdir",
            "/",
            "--entrypoint",
            argv[0],
            image,
        ];
        args.extend_from_slice(&argv[1..]);
        self.checked(&args, PROBE_SECONDS)?;
        let result = self.run(&["container", "start", "--attach", &name], PROBE_SECONDS);
        let removed = self.checked(&["container", "rm", "--force", &name], PROBE_SECONDS);
        let output = executor::checked(result?)?;
        removed?;
        Ok(output.stdout)
    }
    fn build(&self, build: &Build<'_>) -> Result<()> {
        eprintln!("sysroot derive: building {}", build.what);
        let mut command = self.connection.command();
        command
            .env("DOCKER_BUILDKIT", "1")
            .env("BUILDX_NO_DEFAULT_ATTESTATIONS", "1")
            .args([
                "build",
                "--progress=plain",
                "--platform",
                OCI_PLATFORM,
                "--network",
                "none",
                "--pull=false",
                "--provenance=false",
                "--sbom=false",
                "--file",
                "Containerfile",
            ]);
        for (key, value) in build.labels {
            command.arg("--label").arg(format!("{key}={value}"));
        }
        if let Some(tag) = build.tag {
            command.args(["--load", "--tag", tag]);
        }
        if let Some(output) = build.output {
            let output = output.to_str().ok_or_else(|| invalid("non-UTF-8 output"))?;
            if output.contains(',') {
                return Err(invalid("unsafe build output path"));
            }
            command.args([
                "--no-cache",
                "--output",
                &format!("type=local,dest={output}"),
            ]);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let child = command
            .spawn()
            .map_err(|e| invalid(format!("cannot start Docker build: {e}")))?;
        stream_build(child, build)
    }
}

fn context_archive(writer: impl Write, build: &Build<'_>) -> Result<()> {
    let mut archive = tar::Builder::new(writer);
    let mut append = |name: &str, size: u64, mode: u32, data: &mut dyn Read| -> Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        header.set_mode(mode);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        archive.append_data(&mut header, name, data)?;
        Ok(())
    };
    append(
        "Containerfile",
        build.containerfile.len() as u64,
        0o644,
        &mut &*build.containerfile,
    )?;
    for (name, member, mode) in &build.members {
        match member {
            Member::Bytes(bytes) => append(name, bytes.len() as u64, *mode, &mut &**bytes)?,
            Member::File(path) => {
                let mut file = File::open(path)?;
                let size = file.metadata()?.len();
                append(name, size, *mode, &mut file)?;
            }
        }
    }
    archive.finish()?;
    Ok(())
}

/// Stream the context on stdin and keep the progress tail for failures.
fn stream_build(mut child: Child, build: &Build<'_>) -> Result<()> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| invalid("build stdin missing"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| invalid("build stderr missing"))?;
    let deadline = Instant::now() + Duration::from_secs(BUILD_SECONDS);
    thread::scope(|scope| {
        let writer = scope.spawn(move || context_archive(stdin, build));
        let reader = scope.spawn(move || {
            let mut tail = VecDeque::new();
            for line in BufReader::new(stderr)
                .lines()
                .map_while(std::result::Result::ok)
            {
                eprintln!("  | {line}");
                if tail.len() == 80 {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
            tail.into_iter().collect::<Vec<_>>().join("\n")
        });
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err(invalid(format!("{} exceeded its deadline", build.what)));
            }
            thread::sleep(Duration::from_millis(100));
        };
        let written = writer
            .join()
            .map_err(|_| invalid("context writer panicked"))?;
        let tail = reader
            .join()
            .map_err(|_| invalid("progress reader panicked"))?;
        if !status.success() {
            return Err(Error::Process {
                code: status.code().unwrap_or(128),
                message: format!("{} failed:\n{tail}", build.what),
            });
        }
        written
    })
}

fn engine_key(engine: &str) -> String {
    plan::hash(engine.as_bytes())[..16].to_owned()
}

fn sorted_inventory(bytes: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("RPM inventory is not UTF-8"))?;
    let mut lines: Vec<_> = text
        .lines()
        .filter(|line| !line.starts_with("gpg-pubkey\t"))
        .collect();
    lines.sort_unstable();
    Ok(format!("{}\n", lines.join("\n")))
}

fn labels(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

/// Docker loads mutable names from archive envelopes; only our checked FROM name
/// and the exact content ID may accompany the verified image.
fn check_archive_names(path: &Path, image: &str, foundation_tag: &str) -> Result<()> {
    let mut archive = tar::Archive::new(File::open(path)?);
    let allowed = |name: &str| name == image || name == foundation_tag;
    for entry in archive.entries_with_seek()? {
        let mut entry = entry?;
        let name = entry.path_bytes();
        let docker = name.as_ref() == b"manifest.json";
        if !docker && name.as_ref() != b"index.json" {
            continue;
        }
        if entry.size() > 8 * 1024 * 1024 {
            return Err(invalid("foundation naming metadata exceeds 8 MiB"));
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let names_allowed = if docker {
            docker_names(&value, &allowed)?
        } else {
            oci_names(&value, &allowed)?
        };
        if !names_allowed {
            return Err(invalid(
                "foundation archive contains an unowned mutable image name",
            ));
        }
    }
    Ok(())
}

fn docker_names(value: &serde_json::Value, allowed: &dyn Fn(&str) -> bool) -> Result<bool> {
    let records = value
        .as_array()
        .ok_or_else(|| invalid("foundation Docker manifest is not an array"))?;
    Ok(records.iter().all(|record| match record.get("RepoTags") {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::Array(tags)) => {
            tags.iter().all(|tag| tag.as_str().is_some_and(allowed))
        }
        _ => false,
    }))
}

fn oci_names(value: &serde_json::Value, allowed: &dyn Fn(&str) -> bool) -> Result<bool> {
    let records = std::iter::once(value).chain(value["manifests"].as_array().into_iter().flatten());
    for record in records {
        let Some(annotations) = record.get("annotations") else {
            continue;
        };
        let annotations = annotations
            .as_object()
            .ok_or_else(|| invalid("invalid foundation annotations"))?;
        for key in [
            "io.containerd.image.name",
            "org.opencontainers.image.ref.name",
        ] {
            if annotations
                .get(key)
                .is_some_and(|name| !name.as_str().is_some_and(allowed))
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Load the verified foundation and re-check the composition's observations.
fn load_foundation(docker: &Docker, verified: &VerifiedComposition) -> Result<()> {
    let composition = verified.composition();
    let foundation = &composition.plan.foundation;
    let image = &foundation.receipt.image;
    if docker
        .image_id(&composition.foundation_tag)?
        .is_some_and(|id| id != *image)
    {
        return Err(invalid("foundation tag points at another image"));
    }
    check_archive_names(
        &verified.foundation_path(),
        image,
        &composition.foundation_tag,
    )?;
    docker.load(&verified.foundation_path(), foundation.receipt.bytes)?;
    docker.native(image)?;
    if docker.probe(image, &["/usr/bin/cat", "/usr/lib/os-release"])?
        != foundation.os_release.as_bytes()
        || sorted_inventory(&docker.probe(image, &["/usr/bin/rpm", "-qa", "--qf", RPM_FORMAT])?)?
            != foundation.rpm_inventory
    {
        return Err(invalid(
            "loaded foundation differs from the composition observation",
        ));
    }
    check_foundation_paths(docker, composition)?;
    if docker.pin(image)? != composition.foundation_tag {
        return Err(invalid(
            "foundation pin differs from verified Containerfile",
        ));
    }
    Ok(())
}

fn check_foundation_paths(docker: &Docker, composition: &SystemComposition) -> Result<()> {
    let image = &composition.plan.foundation.receipt.image;
    let paths: Vec<String> = composition
        .plan
        .files
        .iter()
        .filter(|file| file.disposition == SystemFileDisposition::Payload)
        .map(|file| file.path.clone())
        .chain(std::iter::once(
            "/usr/share/sysroot/composition.json".into(),
        ))
        .chain(
            composition
                .plan
                .objects
                .keys()
                .map(|id| format!("{}/{id}", crate::LOGICAL_PREFIX)),
        )
        .collect();
    for chunk in paths.chunks(128) {
        let mut argv = vec!["/usr/bin/sh", "-c", PATH_CHECK, "composition-path-check"];
        argv.extend(chunk.iter().map(String::as_str));
        docker.probe(image, &argv)?;
    }
    for file in &composition.plan.files {
        if file.disposition == SystemFileDisposition::Foundation {
            let mode = format!("{:o}", file.mode);
            docker.probe(
                image,
                &[
                    "/usr/bin/sh",
                    "-c",
                    PASSTHROUGH_CHECK,
                    "composition-foundation-check",
                    &file.path,
                    &mode,
                    &file.sha256,
                ],
            )?;
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    schema: u32,
    identity: String,
    engine: String,
    labels: BTreeMap<String, String>,
    nonce: String,
    tag: String,
    image: Option<String>,
}

/// One journaled, owned image publication: pending tag, durable binding, final tag.
struct Publication<'a> {
    docker: &'a Docker,
    state: &'a State,
    kind: &'static str,
    identity: &'a str,
    labels: BTreeMap<String, String>,
    lineage: &'a str,
    key: String,
}
impl Publication<'_> {
    fn tag(&self) -> String {
        format!("kedra-{}:{}", self.kind, self.identity)
    }
    fn transaction_labels(&self, nonce: &str) -> BTreeMap<String, String> {
        let mut labels = self.labels.clone();
        labels.insert(format!("dev.kedra.{}.transaction", self.kind), nonce.into());
        labels
    }
    /// Labels bind provenance; the parent's layers must be a strict prefix.
    fn check_image(&self, image: &str, labels: &BTreeMap<String, String>) -> Result<()> {
        let inspected = self.docker.native(image)?;
        let actual = inspected.labels();
        let parent = self.docker.native(self.lineage)?.layers()?;
        let layers = inspected.layers()?;
        if layers.len() <= parent.len() || !layers.starts_with(&parent) {
            return Err(invalid(format!(
                "{} image filesystem does not extend its immutable parent",
                self.kind
            )));
        }
        if !labels
            .iter()
            .all(|(key, value)| actual.get(key) == Some(value))
        {
            return Err(invalid(format!(
                "{} image identity/provenance binding mismatch",
                self.kind
            )));
        }
        Ok(())
    }
    fn check_pending(&self, journal: &Transaction) -> Result<()> {
        if journal.schema != 1
            || journal.identity != self.identity
            || journal.engine != self.docker.engine
            || journal.labels != self.labels
            || !plan::hex(&journal.nonce)
            || journal.tag != format!("kedra-{}-pending:{}", self.kind, journal.nonce)
        {
            return Err(invalid(format!(
                "{} transaction identity/provenance mismatch",
                self.kind
            )));
        }
        if let Some(actual) = &journal.image {
            self.check_image(actual, &self.labels)?;
            if self
                .docker
                .image_id(&journal.tag)?
                .is_some_and(|id| id != *actual)
            {
                return Err(invalid(format!("{} transaction tag changed", self.kind)));
            }
        }
        Ok(())
    }
    fn complete(
        &self,
        image: &str,
        journal: Option<Transaction>,
        journal_path: &Path,
    ) -> Result<()> {
        self.docker.publish(image, &self.tag())?;
        if let Some(journal) = journal {
            self.docker
                .remove_pending(self.kind, &journal.tag, image, &journal.nonce)?;
            fs::remove_file(journal_path)?;
            sync_parent(journal_path)?;
        }
        Ok(())
    }
    /// Reuse a retained binding after `accept`, or build, verify and bind anew.
    fn run<T: Serialize + DeserializeOwned>(
        &self,
        image_of: impl Fn(&T) -> &str,
        accept: impl Fn(&T) -> Result<()>,
        build: impl Fn(&str, &BTreeMap<String, String>) -> Result<()>,
        finish: impl Fn(&str) -> Result<T>,
    ) -> Result<T> {
        let binding_path = self.state.path.join(format!("{}-binding.json", self.key));
        let journal_path = self
            .state
            .path
            .join(format!("{}-transaction.json", self.key));
        retire_next(&binding_path)?;
        retire_next(&journal_path)?;
        let bound: Option<T> = read_state(&binding_path)?;
        let pending: Option<Transaction> = read_state(&journal_path)?;
        if let Some(journal) = &pending {
            self.check_pending(journal)?;
        }
        if let Some(receipt) = bound {
            let image = image_of(&receipt).to_owned();
            accept(&receipt)?;
            self.check_image(&image, &self.labels)?;
            if pending
                .as_ref()
                .is_some_and(|journal| journal.image.as_deref() != Some(image.as_str()))
            {
                return Err(invalid(format!(
                    "{} binding differs from interrupted transaction",
                    self.kind
                )));
            }
            self.complete(&image, pending, &journal_path)?;
            return Ok(receipt);
        }
        // A final tag without a durable binding is never adopted, even if labels match.
        if self.docker.image_id(&self.tag())?.is_some() {
            return Err(invalid(format!(
                "{} cache tag has no independently retained binding",
                self.kind
            )));
        }
        let mut journal = match pending {
            Some(journal) => journal,
            None => {
                let nonce = store::nonce()?;
                let journal = Transaction {
                    schema: 1,
                    identity: self.identity.to_owned(),
                    engine: self.docker.engine.clone(),
                    labels: self.labels.clone(),
                    tag: format!("kedra-{}-pending:{nonce}", self.kind),
                    nonce,
                    image: None,
                };
                write_state(&journal_path, &journal, false)?;
                journal
            }
        };
        let transaction = self.transaction_labels(&journal.nonce);
        if journal.image.is_none() {
            // Before a result was durably recorded, the nonce tag authorizes cleanup
            // only. Rebuild rather than treating unbound labels as output provenance.
            if let Some(actual) = self.docker.image_id(&journal.tag)? {
                self.check_image(&actual, &transaction)?;
                self.docker
                    .remove_pending(self.kind, &journal.tag, &actual, &journal.nonce)?;
            }
            build(&journal.tag, &transaction)?;
            let actual = self
                .docker
                .image_id(&journal.tag)?
                .ok_or_else(|| invalid(format!("{} build produced no image", self.kind)))?;
            self.check_image(&actual, &transaction)?;
            journal.image = Some(actual);
            write_state(&journal_path, &journal, true)?;
        }
        let image = journal
            .image
            .clone()
            .ok_or_else(|| invalid(format!("{} transaction has no image", self.kind)))?;
        self.check_image(&image, &transaction)?;
        let receipt = finish(&image)?;
        write_state(&binding_path, &receipt, false)?;
        self.complete(&image, Some(journal), &journal_path)?;
        Ok(receipt)
    }
}

/// Build (or reuse) the static composition image of a verified context.
fn realize(
    docker: &Docker,
    state: &State,
    verified: &VerifiedComposition,
) -> Result<ComposedImage> {
    let composition = verified.composition();
    let identity = composition.identity.as_str();
    let foundation = composition.plan.foundation.receipt.image.as_str();
    let payload = &composition
        .artifacts
        .get("payload.tar")
        .ok_or_else(|| invalid("composition has no payload artifact"))?
        .sha256;
    let outputs = &composition.plan.definition.outputs;
    let key = format!("composition-{}-{identity}", engine_key(&docker.engine));
    let _lock = state.lock(&key)?;
    load_foundation(docker, verified)?;
    let containerfile = fs::read(verified.containerfile_path())?;
    let payload_path = verified.payload_path();
    let expected = ComposedImage {
        identity: identity.to_owned(),
        image: String::new(),
        foundation: foundation.to_owned(),
        engine: docker.engine.clone(),
        outputs: outputs.clone(),
    };
    let publication = Publication {
        docker,
        state,
        kind: "composition",
        identity,
        labels: labels(&[
            (OWNER_LABEL, OWNER),
            (KIND_LABEL, "composition"),
            (COMPOSITION_IDENTITY, identity),
            (COMPOSITION_FOUNDATION, foundation),
            (COMPOSITION_PAYLOAD, payload),
        ]),
        lineage: foundation,
        key,
    };
    publication.run(
        |receipt: &ComposedImage| receipt.image.as_str(),
        |receipt| {
            let unbound = ComposedImage {
                image: String::new(),
                ..receipt.clone()
            };
            if unbound != expected {
                return Err(invalid(
                    "composition cache image ID/provenance binding mismatch",
                ));
            }
            Ok(())
        },
        |tag, labels| {
            docker.build(&Build {
                what: "verified static composition (network disabled)",
                containerfile: &containerfile,
                members: vec![("payload.tar", Member::File(&payload_path), 0o644)],
                labels,
                tag: Some(tag),
                output: None,
            })
        },
        |image| {
            Ok(ComposedImage {
                image: image.to_owned(),
                ..expected.clone()
            })
        },
    )
}

/// Build only the verified static composition image (no native step).
pub fn realize_composition(
    context: &Path,
    expected_identity: &str,
    state: &Path,
) -> Result<ComposedImage> {
    let state = State::open(state)?;
    let docker = Docker::connect()?;
    let verified = VerifiedComposition::open(context, expected_identity, &state.path)?;
    let result = realize(&docker, &state, &verified);
    let finished = verified.finish();
    let image = result?;
    finished?;
    Ok(image)
}

/// Data-only plan; it builds nothing and runs no image tool. The steps are the
/// ones `steps` selects from the verified composition.
pub fn plan_derivation(
    context: &Path,
    expected_identity: &str,
    steps: StepSelector<'_>,
    state: &Path,
) -> Result<NativePlan> {
    let executor = Executor::current()?;
    let state = State::open(state)?;
    let verified = VerifiedComposition::open(context, expected_identity, &state.path)?;
    let planned =
        native::native_definition(verified.composition(), steps).and_then(|definition| {
            native::plan_native(&verified, &definition, &executor.implementation())
        });
    let finished = verified.finish();
    let planned = planned?;
    finished?;
    Ok(planned)
}

/// Build the native image offline and bind its re-read material.
pub fn derive_native(
    context: &Path,
    expected_identity: &str,
    steps: StepSelector<'_>,
    expected_derivation: &str,
    state: &Path,
) -> Result<DerivedImage> {
    let executor = Executor::current()?;
    if !plan::hex(expected_derivation) {
        return Err(invalid(
            "expected derivation must be 64 lowercase hexadecimal digits",
        ));
    }
    let state = State::open(state)?;
    let docker = Docker::connect()?;
    let verified = VerifiedComposition::open(context, expected_identity, &state.path)?;
    let derived = Derivation {
        docker: &docker,
        state: &state,
        verified: &verified,
        executor: &executor,
    }
    .run(steps, expected_derivation);
    let finished = verified.finish();
    let derived = derived?;
    finished?;
    Ok(derived)
}

struct Derivation<'a> {
    docker: &'a Docker,
    state: &'a State,
    verified: &'a VerifiedComposition,
    executor: &'a Executor,
}
impl Derivation<'_> {
    fn run(&self, steps: StepSelector<'_>, expected: &str) -> Result<DerivedImage> {
        let definition = native::native_definition(self.verified.composition(), steps)?;
        let plan =
            native::plan_native(self.verified, &definition, &self.executor.implementation())?;
        if plan.identity != expected {
            return Err(invalid(
                "native derivation identity differs from independent selection",
            ));
        }
        let parent = realize(self.docker, self.state, self.verified)?;
        self.environment(&parent.image)?;
        self.environment(&plan.foundation_image)?;
        self.docker.same_engine()?;
        if parent.engine != self.docker.engine {
            return Err(invalid("native derivation daemon changed"));
        }
        let key = format!(
            "native-{}-{}",
            engine_key(&self.docker.engine),
            plan.identity
        );
        let _lock = self.state.lock(&key)?;
        let plan_json = serde_json::to_vec(&plan)?;
        let publication = Publication {
            docker: self.docker,
            state: self.state,
            kind: "native",
            identity: &plan.identity,
            labels: labels(&[
                (OWNER_LABEL, OWNER),
                (KIND_LABEL, "native"),
                (NATIVE_IDENTITY, &plan.identity),
                (NATIVE_PARENT, &parent.image),
            ]),
            lineage: &parent.image,
            key,
        };
        publication.run(
            |receipt: &DerivedImage| receipt.image.as_str(),
            |receipt| {
                if receipt.engine != self.docker.engine || receipt.parent_image != parent.image {
                    return Err(invalid("native cache image/daemon binding mismatch"));
                }
                if self.material(&receipt.image, &plan, &plan_json)? != receipt.material {
                    return Err(invalid(
                        "native cache receipt differs from final image or transaction",
                    ));
                }
                Ok(())
            },
            |tag, labels| {
                self.docker.pin(&parent.image)?;
                let recipe = native::render_native_recipe(&plan, &parent.image)?;
                self.docker.build(&Build {
                    what: "closed native OS derivation (network disabled)",
                    containerfile: recipe.as_bytes(),
                    members: self.members(&plan_json),
                    labels,
                    tag: Some(tag),
                    output: None,
                })
            },
            |image| {
                Ok(DerivedImage {
                    image: image.to_owned(),
                    engine: self.docker.engine.clone(),
                    parent_image: parent.image.clone(),
                    material: self.material(image, &plan, &plan_json)?,
                })
            },
        )
    }

    fn members<'b>(&'b self, plan_json: &'b [u8]) -> Vec<(&'b str, Member<'b>, u32)> {
        vec![
            ("native-plan.json", Member::Bytes(plan_json), 0o644),
            ("sysroot", Member::Bytes(&self.executor.bytes), 0o755),
        ]
    }

    /// An inherited loader or shell override would change what the executor runs.
    fn environment(&self, image: &str) -> Result<()> {
        let inspected = self.docker.native(image)?;
        let env = inspected
            .config
            .and_then(|config| config.env)
            .unwrap_or_default();
        for variable in env {
            if let Some((key, value)) = variable.split_once('=')
                && !value.is_empty()
                && OVERRIDES.contains(&key)
            {
                return Err(invalid(
                    "native parent inherits an executable environment override",
                ));
            }
        }
        Ok(())
    }

    /// Re-read the final image in a throwaway offline stage that exports only its report.
    fn material(&self, image: &str, plan: &NativePlan, plan_json: &[u8]) -> Result<NativeReceipt> {
        self.docker.pin(image)?;
        let recipe = native::render_observation_recipe(image)?;
        let scratch = self.state.scratch("observation")?;
        let output = scratch.join("out");
        let observed = self
            .docker
            .build(&Build {
                what: "native image readback (network disabled)",
                containerfile: recipe.as_bytes(),
                members: self.members(plan_json),
                labels: &BTreeMap::new(),
                tag: None,
                output: Some(&output),
            })
            .and_then(|()| read_observation(&output));
        fs::remove_dir_all(&scratch)?;
        let observation = observed?;
        native::validate_native_receipt(plan, &observation.receipt)?;
        if observation.rpm_inventory != self.verified.composition().plan.foundation.rpm_inventory {
            return Err(invalid("native final artifact or RPM inventory differs"));
        }
        Ok(observation.receipt)
    }
}

fn read_observation(directory: &Path) -> Result<Observation> {
    let path = directory.join(crate::NATIVE_OBSERVATION.trim_start_matches('/'));
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > 2 * MAX_JSON {
        return Err(invalid("native observation is not a bounded ordinary file"));
    }
    let mut bytes = Vec::new();
    File::open(&path)?
        .take(2 * MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}
