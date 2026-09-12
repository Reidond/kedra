//! Direct signed registry updates. Caller input selects intent, never trust or references.
use super::{DIRECTORY, RECORD, Result, STORE, Trust, hash, load_trust, lock, observe, process};
use crate::{protocol::Request, storage::Store, trusted_file};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use sysroot_core::{
    deployment::{self, Host, OperationKind, Phase, StageAction},
    image::{self, Identity, Receipt},
};

const IDENTITY: &str = "/usr/share/sysroot/image-identity.json";
const MATERIAL: &str = "/usr/share/sysroot/resolved-inputs.json";
const POLICY: &str = "/etc/containers/policy.json";
static NEXT: AtomicU64 = AtomicU64::new(0);

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Operation {
    kind: OperationKind,
    target_digest: String,
    previous_booted: String,
    previous_staged: Option<String>,
    phase: Phase,
}
impl Operation {
    fn observe(&mut self, host: &Host) {
        self.phase = if host.booted.digest == self.target_digest
            && host.staged.is_none()
            && !host.rollback_queued
        {
            Phase::Booted
        } else if host
            .staged
            .as_ref()
            .is_some_and(|s| s.digest == self.target_digest && !s.download_only)
            || host.rollback_queued
                && host
                    .rollback
                    .as_ref()
                    .is_some_and(|s| s.digest == self.target_digest)
        {
            Phase::AwaitingReboot
        } else {
            Phase::NotApplied
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    machine: String,
    scope: sysroot_core::release::Scope,
    key_fingerprint: String,
    high_water: Receipt,
    rollback_hold: bool,
    operation: Option<Operation>,
    last_registry_check_at: Option<u64>,
    // Preserve the old ordering domain without pretending its sequence is a run number.
    legacy: Option<deployment::Journal>,
    legacy_rollback: Option<crate::protocol::SignedDocument>,
    identity_health_error: Option<String>,
}

struct Cache(PathBuf);
impl Cache {
    fn new() -> Result<Self> {
        for _ in 0..128 {
            let path = Path::new(DIRECTORY).join(format!(
                "registry-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("cannot reserve private registry cache".into())
    }
}
impl Drop for Cache {
    fn drop(&mut self) {
        // A newly reserved, root-private child of the already validated management directory.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn skopeo(arguments: &[&str], seconds: &str, limit: usize) -> Result<Vec<u8>> {
    let mut child = Command::new("/usr/bin/timeout")
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("HOME", DIRECTORY)
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args([
            "--signal=TERM",
            "--kill-after=10s",
            seconds,
            "/usr/bin/skopeo",
        ])
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let result = (|| -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("registry output unavailable")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err("registry output exceeds bound".into());
        }
        if !child.wait()?.success() {
            return Err("signed registry operation failed; no deployment authorized".into());
        }
        Ok(bytes)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn resolve() -> Result<String> {
    let cache = Cache::new()?;
    let auth = cache.0.join("auth.json");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&auth)?
        .write_all(b"{}")?;
    let reference = format!("docker://{}:{}", image::REPOSITORY, image::CHANNEL);
    let bytes = skopeo(
        &[
            "inspect",
            "--raw",
            "--tls-verify=true",
            "--authfile",
            auth.to_str().ok_or("cache path invalid")?,
            &reference,
        ],
        "60s",
        1_048_576,
    )?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value.get("manifests").is_some() || !value.get("config").is_some_and(|v| v.is_object()) {
        return Err("stable must identify one signed platform manifest, not an index".into());
    }
    Ok(format!("sha256:{}", hash(&bytes)))
}

#[derive(Deserialize)]
struct Config {
    architecture: String,
    os: String,
    config: Runtime,
}
#[derive(Deserialize)]
struct Runtime {
    #[serde(rename = "Labels")]
    labels: BTreeMap<String, String>,
}

fn installed(trust: &Trust) -> Result<(Identity, Vec<u8>)> {
    let bytes = trusted_file::read(Path::new(IDENTITY), 0, image::MAX_IDENTITY)?;
    let identity: Identity = serde_json::from_slice(&bytes)?;
    identity.validate(&trust.scope, now()?)?;
    if identity.source_revision != trust.source_revision
        || identity.source_manifest_sha256 != trust.source_manifest_hash
        || hash(&trusted_file::read(
            Path::new(MATERIAL),
            0,
            4 * 1024 * 1024,
        )?) != identity.resolved_inputs_sha256
    {
        return Err("installed source/material differs from image identity".into());
    }
    Ok((identity, bytes))
}

fn authenticate(digest: &str, trust: &Trust, booted: bool) -> Result<Receipt> {
    if !deployment::digest_valid(digest) {
        return Err("invalid expected image digest".into());
    }
    let cache = Cache::new()?;
    let auth = cache.0.join("auth.json");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&auth)?
        .write_all(b"{}")?;
    let source = format!("docker://{}@{digest}", image::REPOSITORY);
    let target = format!("dir:{}", cache.0.join("image").display());
    // Full native policy enforcement, including missing/wrong signatures, precedes metadata use.
    skopeo(
        &[
            "--policy",
            POLICY,
            "copy",
            "--authfile",
            auth.to_str().ok_or("cache path invalid")?,
            "--src-tls-verify=true",
            "--preserve-digests",
            &source,
            &target,
        ],
        "30m",
        1_048_576,
    )?;
    let manifest = skopeo(&["inspect", "--raw", &target], "30s", 1_048_576)?;
    if format!("sha256:{}", hash(&manifest)) != digest {
        return Err("verified cache manifest digest differs".into());
    }
    let raw = skopeo(&["inspect", "--config", &target], "30s", 1_048_576)?;
    let config: Config = serde_json::from_slice(&raw)?;
    if config.os != "linux" || config.architecture != "amd64" {
        return Err("signed image platform differs".into());
    }
    let label = config
        .config
        .labels
        .get(image::LABEL)
        .ok_or("signed image identity label missing")?;
    if label.len() > image::MAX_IDENTITY {
        return Err("signed identity label exceeds bound".into());
    }
    let identity: Identity = serde_json::from_str(label)?;
    identity.validate(&trust.scope, now()?)?;
    if booted {
        let (local, bytes) = installed(trust)?;
        if identity != local || label.as_bytes() != bytes {
            return Err("booted image label and installed identity file differ".into());
        }
    }
    Ok(Receipt {
        digest: digest.to_owned(),
        identity,
    })
}

fn receipt_name(digest: &str) -> Result<String> {
    if !deployment::digest_valid(digest) {
        return Err("bad receipt digest".into());
    }
    Ok(format!("image.{}", &digest[7..]))
}
fn retain(store: &mut Store, receipt: &Receipt) -> Result<()> {
    let name = receipt_name(&receipt.digest)?;
    let bytes = serde_json::to_vec(receipt)?;
    if let Some(prior) = store.read(&name)? {
        if prior.bytes != bytes {
            return Err("retained image receipt differs".into());
        }
    } else {
        store.compare_exchange(&name, None, &bytes)?;
    }
    Ok(())
}
fn recorded(store: &Store, digest: &str, trust: &Trust) -> Result<Receipt> {
    let record = store
        .read(&receipt_name(digest)?)?
        .ok_or("signed image receipt missing; explicit enrollment/migration is required")?;
    let receipt: Receipt = serde_json::from_slice(&record.bytes)?;
    receipt.validate(&trust.scope, now()?)?;
    if receipt.digest != digest || serde_json::to_vec(&receipt)? != record.bytes {
        return Err("image receipt is inconsistent".into());
    }
    Ok(receipt)
}

fn legacy_rollback(
    store: &Store,
    digest: &str,
    trust: &Trust,
) -> Result<crate::protocol::SignedDocument> {
    let mut found = None;
    for name in store.names_with_prefix("release.")? {
        let record = store
            .read(&name)?
            .ok_or("retained legacy receipt disappeared")?;
        let document: crate::protocol::SignedDocument = serde_json::from_slice(&record.bytes)?;
        let verified = super::verify(&document, trust)?;
        if name != format!("release.{}", verified.sha256()) {
            return Err("legacy receipt name differs from signature".into());
        }
        if verified.release().image_digest == digest {
            if found.is_some() {
                return Err("ambiguous legacy rollback receipts".into());
            }
            found = Some(document);
        }
    }
    found.ok_or_else(|| "native legacy rollback slot has no retained authenticated receipt".into())
}
fn save(store: &mut Store, revision: u64, journal: &Journal) -> Result<u64> {
    Ok(store
        .compare_exchange(RECORD, Some(revision), &serde_json::to_vec(journal)?)?
        .revision)
}
fn load(store: &Store, trust: &Trust) -> Result<(u64, Journal)> {
    let record = store
        .read(RECORD)?
        .ok_or("deployment journal missing; do not reset")?;
    let journal: Journal = serde_json::from_slice(&record.bytes)
        .map_err(|_| "legacy/incompatible state requires explicit migration; do not reset")?;
    if journal.schema_version != 2
        || journal.machine != trust.machine
        || journal.scope != trust.scope
        || journal.key_fingerprint != sysroot_core::release::public_key_fingerprint(&trust.key)?
        || serde_json::to_vec(&journal)? != record.bytes
    {
        return Err("incompatible image deployment state".into());
    }
    journal.high_water.validate(&trust.scope, now()?)?;
    if let Some(legacy) = &journal.legacy {
        legacy.validate(&trust.machine, &trust.scope)?;
    }
    if let Some(document) = &journal.legacy_rollback {
        if journal.legacy.is_none() {
            return Err("legacy rollback receipt lacks preserved migration state".into());
        }
        super::verify(document, trust)?;
    }
    if let Some(op) = &journal.operation
        && (!deployment::digest_valid(&op.target_digest)
            || !deployment::digest_valid(&op.previous_booted)
            || op
                .previous_staged
                .as_ref()
                .is_some_and(|d| !deployment::digest_valid(d)))
    {
        return Err("invalid deployment operation".into());
    }
    Ok((record.revision, journal))
}
fn matches_booted(store: &Store, host: &Host, trust: &Trust) -> Result<()> {
    let receipt = recorded(store, &host.booted.digest, trust)?;
    if receipt.identity != installed(trust)?.0 {
        return Err("booted image differs from retained identity".into());
    }
    Ok(())
}
fn response(
    trust: &Trust,
    host: &Host,
    journal: Option<&Journal>,
    available: Option<&Receipt>,
    checked: bool,
) -> Result<()> {
    let time = now()?;
    let identity = installed(trust).ok().map(|(identity, _)| identity);
    let held = journal.is_some_and(|j| j.rollback_hold) || host.rollback_queued;
    let state = if journal.is_some_and(|j| j.identity_health_error.is_some()) {
        "identity_mismatch"
    } else if held {
        "held"
    } else if journal.is_none() {
        "enrollment_required"
    } else if let Some(candidate) = available {
        if let Some(staged) = &host.staged {
            if staged.digest != candidate.digest {
                "pending_other_deployment"
            } else if staged.download_only {
                "downloaded"
            } else {
                "staged"
            }
        } else if host.booted.digest == candidate.digest {
            "current"
        } else {
            "available"
        }
    } else if host.staged.is_some() {
        "staged"
    } else {
        "booted"
    };
    let resolution = available
        .map(|r| r.identity.resolved_at)
        .or_else(|| identity.as_ref().map(|i| i.resolved_at));
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":2,"kind":"signed_image_status","state":state,"enrolled":journal.is_some(),
            "scope":trust.scope,"channel":format!("{}:{}",image::REPOSITORY,image::CHANNEL),
            "host":host,"installed_identity":identity,"journal":journal,"available":available,
        "registry_checked":checked,"signature_verified":checked,
        "booted_receipt_verified":journal.is_some_and(|j|j.identity_health_error.is_none()),
            "freshness":{"registry_checked_at":if checked{Some(time)}else{journal.and_then(|j|j.last_registry_check_at)},
            "signed_resolution_at":resolution,"resolution_age_seconds":resolution.map(|r|time.saturating_sub(r)),
            "resolution_stale_warning":resolution.map(|r|time.saturating_sub(r)>36*3600),"latest_upstream_resolution_proven":false},
            "reboot_performed":false,"activation_performed":false,"home_reconciliation_checked":false
        }))?
    );
    Ok(())
}

pub(super) fn run(request: Request) -> Result<()> {
    if rustix::process::getuid().as_raw() != 0 || rustix::process::geteuid().as_raw() != 0 {
        return Err(
            "installed registry management requires explicit administrator authorization".into(),
        );
    }
    let trust = load_trust()?;
    if trust.scope.repository != image::REPOSITORY || trust.scope.target != "desktop" {
        return Err("no direct channel configured for installed scope".into());
    }
    let _lock = lock()?;
    if super::output(&["--version"], 4096)?.as_slice()
        != sysroot_core::compatibility::bootc_output()?.as_bytes()
    {
        return Err("installed bootc version is not qualified".into());
    }
    let before = observe(&trust.scope)?;
    if let Request::ChannelEnroll {
        migrate_legacy,
        expected_digest,
    } = request
    {
        if before.staged.is_some() || before.rollback_queued {
            return Err("enrollment requires no pending deployment or queued rollback".into());
        }
        if expected_digest
            .as_ref()
            .is_some_and(|d| d != &before.booted.digest)
        {
            return Err("booted digest differs from explicit enrollment expectation".into());
        }
        if migrate_legacy && expected_digest.is_none() {
            return Err(
                "legacy migration requires the explicitly reviewed booted --expected-digest".into(),
            );
        }
        let booted = authenticate(&before.booted.digest, &trust, true)?;
        if observe(&trust.scope)? != before {
            return Err("native deployment changed during enrollment".into());
        }
        let mut journal = Journal {
            schema_version: 2,
            machine: trust.machine.clone(),
            scope: trust.scope.clone(),
            key_fingerprint: sysroot_core::release::public_key_fingerprint(&trust.key)?,
            high_water: booted.clone(),
            rollback_hold: false,
            operation: None,
            last_registry_check_at: Some(now()?),
            legacy: None,
            legacy_rollback: None,
            identity_health_error: None,
        };
        if migrate_legacy {
            let mut store = Store::open(Path::new(STORE))?;
            let (revision, legacy) = super::load(&store, &trust)?;
            if legacy
                .operation
                .as_ref()
                .is_some_and(|o| matches!(o.phase, Phase::Intent | Phase::AwaitingReboot))
            {
                return Err("legacy operation must be reconciled before migration".into());
            }
            journal.rollback_hold = legacy.rollback_hold;
            if let Some(slot) = &before.rollback {
                journal.legacy_rollback = Some(legacy_rollback(&store, &slot.digest, &trust)?);
            }
            // Preserve the original canonical v1 record and every existing release receipt.
            let original = serde_json::to_vec(&legacy)?;
            if let Some(backup) = store.read("legacy-deployment-v1")? {
                if backup.bytes != original {
                    return Err("legacy backup differs; inspect interrupted migration".into());
                }
            } else {
                store.compare_exchange("legacy-deployment-v1", None, &original)?;
            }
            journal.legacy = Some(legacy);
            retain(&mut store, &booted)?;
            save(&mut store, revision, &journal)?;
        } else {
            let bytes = serde_json::to_vec(&journal)?;
            let receipt = serde_json::to_vec(&booted)?;
            Store::create(
                Path::new(STORE),
                &[(RECORD, &bytes), (&receipt_name(&booted.digest)?, &receipt)],
            )?;
        }
        return response(&trust, &before, Some(&journal), None, false);
    }
    if matches!(
        request,
        Request::ChannelStatus {} | Request::ChannelCheck {}
    ) && !Path::new(STORE).try_exists()?
    {
        if matches!(request, Request::ChannelCheck {}) {
            installed(&trust)?;
            let digest = resolve()?;
            let available = authenticate(&digest, &trust, false)?;
            if resolve()? != digest || observe(&trust.scope)? != before {
                return Err("channel or deployment changed during check".into());
            }
            return response(&trust, &before, None, Some(&available), true);
        }
        return response(&trust, &before, None, None, false);
    }
    let mut store = Store::open(Path::new(STORE))?;
    let (mut revision, mut journal) = load(&store, &trust)?;
    let old = journal.operation.clone();
    if let Some(op) = &mut journal.operation {
        op.observe(&before);
    }
    let identity_error = matches_booted(&store, &before, &trust)
        .err()
        .map(|error| error.to_string());
    let health_changed = journal.identity_health_error != identity_error;
    journal.identity_health_error = identity_error;
    if old != journal.operation || health_changed {
        revision = save(&mut store, revision, &journal)?;
    }
    // A bad current identity cannot authorize forward work. Recovery is different:
    // the retained rollback receipt and native rollback slot authorize only that slot.
    if journal.identity_health_error.is_some()
        && !matches!(request, Request::ChannelRollback { .. })
    {
        response(&trust, &before, Some(&journal), None, false)?;
        return Err("booted identity health failed; high-water preserved, inspect status and recover explicitly".into());
    }
    if matches!(request, Request::ChannelStatus {}) {
        return response(&trust, &before, Some(&journal), None, false);
    }
    let (kind, target, should_run) = match request {
        Request::ChannelCheck {} | Request::ChannelStage { .. } => {
            let digest = resolve()?;
            let available = authenticate(&digest, &trust, digest == before.booted.digest)?;
            available.follows(&journal.high_water)?;
            if resolve()? != digest || observe(&trust.scope)? != before {
                return Err("channel or deployment changed during verification".into());
            }
            if matches!(request, Request::ChannelCheck {}) {
                // A verified observation advances replay protection, never a deployment slot.
                retain(&mut store, &available)?;
                journal.high_water = available.clone();
                journal.last_registry_check_at = Some(now()?);
                save(&mut store, revision, &journal)?;
                return response(&trust, &before, Some(&journal), Some(&available), true);
            }
            let Request::ChannelStage {
                expected_digest,
                replace_staged,
                resume,
            } = request
            else {
                return Err("invalid channel request".into());
            };
            if expected_digest.as_ref().is_some_and(|d| d != &digest) {
                return Err("channel digest differs from explicit expectation".into());
            }
            let action = deployment::stage_action(
                &before,
                &digest,
                replace_staged.as_deref(),
                journal.rollback_hold,
                resume,
            )?;
            retain(&mut store, &available)?;
            journal.high_water = available;
            journal.last_registry_check_at = Some(now()?);
            if resume {
                journal.rollback_hold = false;
            }
            (OperationKind::Stage, digest, action == StageAction::Switch)
        }
        Request::ChannelRollback { replace_staged } => {
            let slot = before
                .rollback
                .as_ref()
                .ok_or("no retained rollback deployment")?;
            if slot.download_only {
                return Err("rollback slot is download-only".into());
            }
            if before
                .staged
                .as_ref()
                .is_some_and(|s| replace_staged.as_deref() != Some(&s.digest))
                || before.staged.is_none() && replace_staged.is_some()
            {
                return Err("explicit pending digest replacement is required".into());
            }
            // Receipt was authenticated before staging/enrollment, so retained rollback works offline.
            if store.read(&receipt_name(&slot.digest)?)?.is_some() {
                recorded(&store, &slot.digest, &trust)?;
            } else {
                let document = journal
                    .legacy_rollback
                    .as_ref()
                    .ok_or("retained rollback receipt missing")?;
                if super::verify(document, &trust)?.release().image_digest != slot.digest {
                    return Err("legacy receipt does not match native rollback slot".into());
                }
            }
            journal.rollback_hold = true;
            (
                OperationKind::Rollback,
                slot.digest.clone(),
                !before.rollback_queued,
            )
        }
        _ => return Err("unsupported direct registry operation".into()),
    };
    if observe(&trust.scope)? != before {
        return Err("native deployment changed before intent".into());
    }
    journal.operation = Some(Operation {
        kind: kind.clone(),
        target_digest: target.clone(),
        previous_booted: before.booted.digest.clone(),
        previous_staged: before.staged.as_ref().map(|s| s.digest.clone()),
        phase: Phase::Intent,
    });
    revision = save(&mut store, revision, &journal)?;
    let reference = format!("{}@{target}", image::REPOSITORY);
    let result = if should_run {
        let arguments = match kind {
            OperationKind::Stage => vec!["switch", "--enforce-container-sigpolicy", &reference],
            OperationKind::Rollback => vec!["rollback"],
        };
        process("30m", &arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()?
            .success()
    } else {
        true
    };
    let after = observe(&trust.scope)?;
    let operation = journal
        .operation
        .as_mut()
        .ok_or("missing deployment intent")?;
    operation.observe(&after);
    let completed = matches!(operation.phase, Phase::AwaitingReboot | Phase::Booted);
    save(&mut store, revision, &journal)?;
    response(&trust, &after, Some(&journal), None, false)?;
    if !result || !completed {
        return Err("native deployment incomplete; inspect recorded status before retrying".into());
    }
    Ok(())
}
