//! Root-only operations using fixed installed trust, state and executable paths.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::process::{Command, Stdio};
use sysroot_core::deployment::{self, Host};
use sysroot_core::protocol::Request;
use sysroot_core::release::{self, Scope};
use sysroot_linux::trusted_file;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const TRUST: &str = "/usr/lib/sysroot/trust/release-policy.json";
const KEY: &str = "/usr/lib/sysroot/trust/release.pub";
const DIRECTORY: &str = "/var/lib/sysroot";
mod ghcr;
mod installer;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    scope: Scope,
    key_fingerprint: String,
}
#[derive(Deserialize)]
struct Source {
    schema_version: u32,
    source_revision: String,
    target: Target,
}
#[derive(Deserialize)]
struct Target {
    id: String,
    architecture: String,
    image: String,
    fedora_release: u32,
    candidate_target: bool,
}
struct Trust {
    scope: Scope,
    key: String,
    source_revision: String,
    source_manifest_hash: String,
}
/// Trust comes only from the booted image's /usr and the strict policy derived from it.
fn load_trust() -> Result<Trust> {
    let policy: Policy = serde_json::from_slice(&trusted_file::read(Path::new(TRUST), 0, 16_384)?)
        .map_err(|_| "installed release policy is malformed")?;
    if policy.schema_version != 1 {
        return Err("unsupported installed release policy".into());
    }
    policy.scope.validate()?;
    let source_bytes =
        trusted_file::read(Path::new("/usr/share/sysroot/source.json"), 0, 1_048_576)?;
    let source: Source = serde_json::from_slice(&source_bytes)
        .map_err(|_| "installed source manifest is malformed")?;
    if source.schema_version != 1
        || !source.target.candidate_target
        || source.target.id != policy.scope.target
        || source.target.architecture != policy.scope.architecture
        || source.target.image != policy.scope.repository
        || source.target.fedora_release != policy.scope.fedora_release
    {
        return Err("installed source and release policy disagree about the target".into());
    }
    let key = String::from_utf8(trusted_file::read(Path::new(KEY), 0, 4096)?)?;
    if release::public_key_fingerprint(&key)? != policy.key_fingerprint {
        return Err("installed release key differs from its policy fingerprint".into());
    }
    // bootc enforces this file; it must be exactly the policy the image's trust implies.
    let requirement = serde_json::json!({"type":"sigstoreSigned","keyPath":KEY,
        "signedIdentity":{"type":"exactRepository","dockerRepository":policy.scope.repository}});
    let native_policy = serde_json::json!({"default":[{"type":"reject"}],"transports":{
        "docker":{(policy.scope.repository.clone()):[requirement.clone()]},"containers-storage":{"": [requirement]}}});
    let mut expected = serde_json::to_vec_pretty(&native_policy)?;
    expected.push(b'\n');
    if trusted_file::read(Path::new("/etc/containers/policy.json"), 0, 65_536)? != expected {
        return Err(
            "container signature policy differs from the strict installed target policy".into(),
        );
    }
    Ok(Trust {
        scope: policy.scope,
        key,
        source_revision: source.source_revision,
        source_manifest_hash: hash(&source_bytes),
    })
}

fn lock() -> Result<File> {
    let directory = rustix::fs::openat2(
        rustix::fs::CWD,
        DIRECTORY,
        rustix::fs::OFlags::PATH | rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
        rustix::fs::ResolveFlags::NO_SYMLINKS | rustix::fs::ResolveFlags::NO_MAGICLINKS,
    )?;
    let metadata = rustix::fs::fstat(&directory)?;
    if metadata.st_uid != 0 || metadata.st_mode & 0o777 != 0o700 {
        return Err("management directory must be root-owned and mode 0700".into());
    }
    let path = Path::new(DIRECTORY).join("management.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.mode() & 0o777 != 0o600
    {
        return Err("unsafe management lock".into());
    }
    file.try_lock()
        .map_err(|_| "another sysroot management operation is running")?;
    // A surviving trusted child retains the lock if this helper is interrupted.
    rustix::io::fcntl_setfd(&file, rustix::io::FdFlags::empty())?;
    // containers/image prefers $HOME/.config/containers/policy.json over
    // /etc/containers/policy.json, and --enforce-container-sigpolicy checks only
    // the default entry of whichever file it finds. Children run with this
    // root-only directory as HOME, so no per-user container configuration may exist.
    match std::fs::symlink_metadata(Path::new(DIRECTORY).join(".config")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(file),
        Err(error) => Err(error.into()),
        Ok(_) => Err("unexpected /var/lib/sysroot/.config; container configuration must come only from /etc/containers".into()),
    }
}

/// A fixed image-owned program under a timeout, with a cleared environment.
fn process(program: &str, seconds: &str, arguments: &[&str]) -> Command {
    let mut command = Command::new("/usr/bin/timeout");
    command
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("HOME", DIRECTORY)
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args(["--signal=TERM", "--kill-after=10s", seconds, program])
        .args(arguments)
        .stdin(Stdio::null());
    command
}
/// Bounded stdout of a fixed program; its diagnostics go to the caller's terminal.
fn output(program: &str, arguments: &[&str], limit: usize) -> Result<Vec<u8>> {
    let mut child = process(program, "60s", arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let result = (|| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("child stdout is unavailable")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err("child output exceeded its bound".into());
        }
        if !child.wait()?.success() {
            return Err(format!("{program} failed; nothing was changed").into());
        }
        Ok(bytes)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}
fn observe(scope: &Scope) -> Result<Host> {
    Ok(deployment::observe(
        &output("/usr/bin/bootc", &["status", "--json"], 262_144)?,
        scope,
    )?)
}
/// Called only by the image-owned binary after parsing its bounded stdin protocol.
pub fn run(request: Request) -> Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err("the installed helper requires explicit administrator authorization".into());
    }
    let trust = load_trust()?;
    if matches!(request, Request::VerifyInstaller {}) {
        return installer::verify(&trust);
    }
    ghcr::run(request, &trust)
}
