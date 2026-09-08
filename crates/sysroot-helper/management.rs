//! Root-only operations using fixed installed trust, state and executable paths.
use crate::protocol::{Request, SignedDocument};
use crate::{storage::Store, trusted_file};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use sysroot_core::deployment::{self, Host, Journal, OperationKind, Phase, StageAction};
use sysroot_core::release::{self, Scope, VerifiedRelease, VerifiedUpdate};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const TRUST: &str = "/usr/lib/sysroot/trust/release-policy.json";
const KEY: &str = "/usr/lib/sysroot/trust/release.pub";
const DIRECTORY: &str = "/var/lib/sysroot";
const STORE: &str = "/var/lib/sysroot/deployment";
const RECORD: &str = "deployment";
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
    machine: String,
    source_revision: String,
    source_manifest_hash: String,
}
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
    let machine = trusted_file::read(Path::new("/etc/machine-id"), 0, 64)?;
    let machine = std::str::from_utf8(&machine)?.trim();
    if machine.len() != 32
        || !machine
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("machine identity is malformed".into());
    }
    Ok(Trust {
        scope: policy.scope,
        key,
        machine: hash(machine.as_bytes()),
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
    Ok(file)
}

fn process(seconds: &str, arguments: &[&str]) -> Command {
    let mut command = Command::new("/usr/bin/timeout");
    command
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("HOME", "/root")
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args([
            "--signal=TERM",
            "--kill-after=10s",
            seconds,
            "/usr/bin/bootc",
        ])
        .args(arguments)
        .stdin(Stdio::null());
    command
}
fn output(arguments: &[&str], limit: usize) -> Result<Vec<u8>> {
    let mut child = process("30s", arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("bootc stdout is unavailable")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err("bootc output exceeded the protocol bound".into());
        }
        if !child.wait()?.success() {
            return Err("bootc observation failed".into());
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
        &output(&["status", "--json"], 262_144)?,
        scope,
    )?)
}
fn verify(document: &SignedDocument, trust: &Trust) -> Result<VerifiedRelease> {
    release::verify_release(
        document.payload.as_bytes(),
        document.signature.as_bytes(),
        &trust.key,
        Some(&trust.scope),
    )
    .map_err(|_| "release signature, scope, promotion or compatibility was refused".into())
}
fn fresh(
    release: &SignedDocument,
    checkpoint: &SignedDocument,
    trust: &Trust,
    journal: Option<&Journal>,
) -> Result<VerifiedUpdate> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    release::verify_update(
        verify(release, trust)?,
        checkpoint.payload.as_bytes(),
        checkpoint.signature.as_bytes(),
        &trust.key,
        &trust.scope,
        journal.map(|state| &state.high_water),
        now,
    )
    .map_err(|_| "checkpoint signature, freshness or replay protection was refused".into())
}
fn load(store: &Store, trust: &Trust) -> Result<(u64, Journal)> {
    let record = store
        .read(RECORD)?
        .ok_or("deployment journal missing; do not reset this store")?;
    let journal: Journal =
        serde_json::from_slice(&record.bytes).map_err(|_| "deployment journal is corrupt")?;
    journal.validate(&trust.machine, &trust.scope)?;
    if serde_json::to_vec(&journal)? != record.bytes {
        return Err("deployment journal is not canonical".into());
    }
    Ok((record.revision, journal))
}
fn save(store: &mut Store, revision: u64, journal: &Journal) -> Result<u64> {
    Ok(store
        .compare_exchange(RECORD, Some(revision), &serde_json::to_vec(journal)?)?
        .revision)
}
fn retain(
    store: &mut Store,
    document: &SignedDocument,
    verified: &VerifiedRelease,
    trust: &Trust,
) -> Result<()> {
    let name = format!("release.{}", verified.sha256());
    let bytes = serde_json::to_vec(document)?;
    if let Some(previous) = store.read(&name)? {
        let previous: SignedDocument =
            serde_json::from_slice(&previous.bytes).map_err(|_| "retained receipt is malformed")?;
        if previous.payload != document.payload
            || verify(&previous, trust)?.sha256() != verified.sha256()
        {
            return Err("retained release receipt changed".into());
        }
    } else {
        store.compare_exchange(&name, None, &bytes)?;
    }
    Ok(())
}
fn matches_installed(verified: &VerifiedRelease, trust: &Trust) -> Result<()> {
    if verified.release().source_revision != trust.source_revision
        || verified.release().home_manifest_sha256 != trust.source_manifest_hash
    {
        return Err(
            "running image source manifest does not match its signed release receipt".into(),
        );
    }
    Ok(())
}
fn reconcile(store: &Store, journal: &mut Journal, host: &Host, trust: &Trust) -> Result<()> {
    if let Some(operation) = &mut journal.operation {
        operation.observe(host);
        if operation.phase == Phase::Booted {
            let record = store
                .read(&format!("release.{}", operation.release_sha256))?
                .ok_or("running deployment receipt is missing")?;
            let document: SignedDocument = serde_json::from_slice(&record.bytes)
                .map_err(|_| "retained release receipt is malformed")?;
            let verified = verify(&document, trust)?;
            if verified.sha256() != operation.release_sha256
                || verified.release().image_digest != host.booted.digest
            {
                return Err("running deployment receipt identity mismatch".into());
            }
            matches_installed(&verified, trust)?;
        }
    }
    Ok(())
}
#[derive(Serialize)]
struct Response<'a> {
    schema_version: u32,
    enrolled: bool,
    scope: &'a Scope,
    host: &'a Host,
    journal: Option<PublicJournal<'a>>,
    reboot_performed: bool,
    activation_performed: bool,
    home_reconciliation_checked: bool,
    home_reconciliation_required: Option<bool>,
}
#[derive(Serialize)]
struct PublicJournal<'a> {
    high_water: &'a release::TrustState,
    rollback_hold: bool,
    operation: &'a Option<deployment::Operation>,
}
fn respond(trust: &Trust, host: &Host, journal: Option<&Journal>) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&Response {
            schema_version: 1,
            enrolled: journal.is_some(),
            scope: &trust.scope,
            host,
            journal: journal.map(|state| PublicJournal {
                high_water: &state.high_water,
                rollback_hold: state.rollback_hold,
                operation: &state.operation
            }),
            reboot_performed: false,
            activation_performed: false,
            home_reconciliation_checked: false,
            home_reconciliation_required: None
        })?
    );
    Ok(())
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
    let _lock = lock()?;
    if output(&["--version"], 4096)?.as_slice() != b"bootc 1.16.10\n" {
        return Err("installed bootc version has not been qualified for this helper".into());
    }
    let before = observe(&trust.scope)?;
    if let Request::Enroll {
        release,
        checkpoint,
        installed_release,
    } = request
    {
        let update = fresh(&release, &checkpoint, &trust, None)?;
        let installed_document = installed_release.as_ref().unwrap_or(&release);
        let installed = verify(installed_document, &trust)?;
        if before.staged.is_some()
            || before.rollback_queued
            || before.booted.digest != installed.release().image_digest
            || trust.source_revision != installed.release().source_revision
            || trust.source_manifest_hash != installed.release().home_manifest_sha256
            || installed.release().sequence > update.release.release().sequence
            || (installed.release().sequence == update.release.release().sequence
                && installed.sha256() != update.release.sha256())
        {
            return Err("enrollment requires the exact running promoted release, its source manifest, and an equal or newer current channel".into());
        }
        let journal = Journal {
            schema_version: 1,
            machine: trust.machine.clone(),
            scope: trust.scope.clone(),
            high_water: update.next_trust_state,
            rollback_hold: false,
            operation: None,
        };
        let bytes = serde_json::to_vec(&journal)?;
        let receipt = serde_json::to_vec(installed_document)?;
        let receipt_name = format!("release.{}", installed.sha256());
        Store::create(
            Path::new(STORE),
            &[(RECORD, &bytes), (&receipt_name, &receipt)],
        )?;
        return respond(&trust, &before, Some(&journal));
    }
    if matches!(request, Request::Status {}) {
        match std::fs::symlink_metadata(STORE) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return respond(&trust, &before, None);
            }
            Err(error) => return Err(error.into()),
            Ok(_) => (),
        }
    }
    let mut store = Store::open(Path::new(STORE))?;
    let (mut revision, mut journal) = load(&store, &trust)?;
    let previous_operation = journal.operation.clone();
    reconcile(&store, &mut journal, &before, &trust)?;
    if previous_operation != journal.operation {
        revision = save(&mut store, revision, &journal)?;
    }
    if matches!(request, Request::Status {}) {
        return respond(&trust, &before, Some(&journal));
    }
    let (kind, target, release_sha256) = match request {
        Request::Stage {
            release,
            checkpoint,
            replace_staged,
            resume,
        } => {
            let update = fresh(&release, &checkpoint, &trust, Some(&journal))?;
            let digest = &update.release.release().image_digest;
            if before.booted.digest == *digest {
                matches_installed(&update.release, &trust)?;
            }
            let action = deployment::stage_action(
                &before,
                digest,
                replace_staged.as_deref(),
                journal.rollback_hold,
                resume,
            )?;
            retain(&mut store, &release, &update.release, &trust)?;
            journal.high_water = update.next_trust_state;
            if resume {
                journal.rollback_hold = false;
            }
            if action != StageAction::Switch {
                journal.operation = Some(deployment::intent(
                    &before,
                    OperationKind::Stage,
                    digest,
                    update.release.sha256(),
                )?);
                journal
                    .operation
                    .as_mut()
                    .ok_or("missing operation")?
                    .observe(&before);
                save(&mut store, revision, &journal)?;
                return respond(&trust, &before, Some(&journal));
            }
            (
                OperationKind::Stage,
                digest.to_owned(),
                update.release.sha256().to_owned(),
            )
        }
        Request::Rollback {
            release,
            replace_staged,
        } => {
            let verified = verify(&release, &trust)?;
            let digest = verified.release().image_digest.clone();
            if before
                .rollback
                .as_ref()
                .is_none_or(|slot| slot.digest != digest || slot.download_only)
            {
                return Err(
                    "requested signed release is not the native retained rollback slot".into(),
                );
            }
            if before
                .staged
                .as_ref()
                .is_some_and(|slot| replace_staged.as_deref() != Some(slot.digest.as_str()))
                || before.staged.is_none() && replace_staged.is_some()
            {
                return Err(
                    "another image is staged; explicitly name it for rollback replacement".into(),
                );
            }
            retain(&mut store, &release, &verified, &trust)?;
            journal.rollback_hold = true;
            if before.rollback_queued {
                journal.operation = Some(deployment::intent(
                    &before,
                    OperationKind::Rollback,
                    &digest,
                    verified.sha256(),
                )?);
                journal
                    .operation
                    .as_mut()
                    .ok_or("missing operation")?
                    .observe(&before);
                save(&mut store, revision, &journal)?;
                return respond(&trust, &before, Some(&journal));
            }
            (
                OperationKind::Rollback,
                digest,
                verified.sha256().to_owned(),
            )
        }
        _ => return Err("unsupported operation in enrolled state".into()),
    };
    // Recheck just before intent publication. The lock coordinates helper callers;
    // a separate administrator running bootc directly is outside that protocol.
    if observe(&trust.scope)? != before {
        return Err("native bootc state changed during preflight".into());
    }
    journal.operation = Some(deployment::intent(
        &before,
        kind.clone(),
        &target,
        &release_sha256,
    )?);
    revision = save(&mut store, revision, &journal)?;
    let reference = format!("{}@{}", trust.scope.repository, target);
    let arguments = match kind {
        OperationKind::Stage => vec![
            "switch",
            "--enforce-container-sigpolicy",
            reference.as_str(),
        ],
        OperationKind::Rollback => vec!["rollback"],
    };
    let native_result = process("30m", &arguments)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status();
    let after = observe(&trust.scope)?;
    journal
        .operation
        .as_mut()
        .ok_or("missing operation")?
        .observe(&after);
    save(&mut store, revision, &journal)?;
    respond(&trust, &after, Some(&journal))?;
    if !native_result.is_ok_and(|status| status.success())
        || journal.operation.as_ref().is_none_or(|operation| {
            ![Phase::AwaitingReboot, Phase::Booted].contains(&operation.phase)
        })
    {
        return Err("native deployment did not complete cleanly; inspect the reported observed state before retrying".into());
    }
    Ok(())
}
