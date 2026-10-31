//! Direct signed registry updates. Caller input selects intent, never trust or references.
//!
//! bootc verifies image signatures itself: `bootc switch --enforce-container-sigpolicy`
//! evaluates the installed strict policy before it fetches the image configuration or
//! any layer. The helper adds what that policy cannot express: the signed identity's
//! target and platform, ordering against the booted image (no downgrade), explicit
//! staging without reboot and the hold that follows a rollback.
use super::{Result, Trust, hash, lock, observe, output, process};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::os::fd::AsFd;
use std::path::Path;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};
use sysroot_core::deployment::{self, Host, StageAction};
use sysroot_core::image::{self, Identity, Receipt};
use sysroot_core::protocol::Request;
use sysroot_core::{compatibility, targets};
use sysroot_linux::{storage::Store, trusted_file};

const IDENTITY: &str = "/usr/share/sysroot/image-identity.json";
const MATERIAL: &str = "/usr/share/sysroot/resolved-inputs.json";
const BOOTC: &str = "/usr/bin/bootc";
const SKOPEO: &str = "/usr/bin/skopeo";
/// Root-only persistent state. It holds only the rollback hold.
const STORE: &str = "/var/lib/sysroot/deployment";
const RECORD: &str = "deployment";
const STATE_SCHEMA: u32 = 3;

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    rollback_hold: bool,
}

/// The rollback hold. A missing store means no rollback was ever requested;
/// unknown or corrupt state is refused, never reset.
struct Hold {
    store: Option<(Store, u64)>,
    held: bool,
}
impl Hold {
    fn load() -> Result<Self> {
        if !Path::new(STORE).try_exists()? {
            return Ok(Self {
                store: None,
                held: false,
            });
        }
        let store = Store::open(Path::new(STORE))?;
        let record = store
            .read(RECORD)?
            .ok_or("deployment state lacks its record; do not reset it")?;
        let journal: Journal = serde_json::from_slice(&record.bytes)
            .map_err(|_| "incompatible deployment state; do not reset it")?;
        if journal.schema_version != STATE_SCHEMA || serde_json::to_vec(&journal)? != record.bytes {
            return Err("incompatible deployment state; do not reset it".into());
        }
        Ok(Self {
            store: Some((store, record.revision)),
            held: journal.rollback_hold,
        })
    }
    fn set(&mut self, held: bool) -> Result<()> {
        if held == self.held {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&Journal {
            schema_version: STATE_SCHEMA,
            rollback_hold: held,
        })?;
        match &mut self.store {
            Some((store, revision)) => {
                *revision = store
                    .compare_exchange(RECORD, Some(*revision), &bytes)?
                    .revision;
            }
            None => self.store = Some((Store::create(Path::new(STORE), &[(RECORD, &bytes)])?, 1)),
        }
        self.held = held;
        Ok(())
    }
}

/// Registry metadata only: anonymous, TLS-verified and bounded. It authorizes nothing.
fn registry(arguments: &[&str]) -> Result<Vec<u8>> {
    let mut all = vec!["inspect", "--no-creds", "--tls-verify=true"];
    all.extend_from_slice(arguments);
    output(SKOPEO, &all, 1_048_576)
}

#[derive(Deserialize)]
struct Manifest {
    config: Descriptor,
}
#[derive(Deserialize)]
struct Descriptor {
    digest: String,
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

/// stable's digest, computed here from its manifest bytes, and that manifest's
/// configuration digest. An index is refused: deployment needs one platform image.
fn resolve(trust: &Trust) -> Result<(String, String)> {
    let reference = format!("docker://{}:{}", trust.scope.repository, image::CHANNEL);
    let bytes = registry(&["--raw", &reference])?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value.get("manifests").is_some() {
        return Err("stable must identify one signed platform manifest, not an index".into());
    }
    let manifest: Manifest = serde_json::from_value(value)?;
    Ok((format!("sha256:{}", hash(&bytes)), manifest.config.digest))
}

/// The identity label of `digest`, from a configuration whose hash is checked here
/// against the manifest that the digest names: the label belongs to the exact image
/// bootc later verifies and stages. No signature is checked here.
fn candidate(trust: &Trust, digest: &str, config_digest: &str) -> Result<Receipt> {
    if !deployment::digest_valid(digest) || !deployment::digest_valid(config_digest) {
        return Err("invalid image digest".into());
    }
    let reference = format!("docker://{}@{digest}", trust.scope.repository);
    let raw = registry(&["--raw", "--config", &reference])?;
    if format!("sha256:{}", hash(&raw)) != config_digest {
        return Err("image configuration differs from its manifest".into());
    }
    let config: Config = serde_json::from_slice(&raw)?;
    if config.os != "linux"
        || targets::oci_architecture(&trust.scope.architecture)
            != Some(config.architecture.as_str())
    {
        return Err("image platform differs from the installed target".into());
    }
    let label = config
        .config
        .labels
        .get(image::LABEL)
        .ok_or("image identity label missing")?;
    if label.len() > image::MAX_IDENTITY {
        return Err("image identity label exceeds its bound".into());
    }
    let identity: Identity = serde_json::from_str(label)?;
    identity.validate(&trust.scope, now()?)?;
    Ok(Receipt {
        digest: digest.to_owned(),
        identity,
    })
}

/// The booted image's identity file, bound to its installed source and resolved inputs.
fn booted_identity(trust: &Trust) -> Result<Identity> {
    let identity: Identity = serde_json::from_slice(&trusted_file::read(
        Path::new(IDENTITY),
        0,
        image::MAX_IDENTITY,
    )?)?;
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
    Ok(identity)
}

/// stable's image, refused unless its identity is in scope and not older than the
/// booted image. bootc would deploy any validly signed digest, including older ones.
fn available(trust: &Trust, host: &Host) -> Result<Receipt> {
    let booted = Receipt {
        digest: host.booted.digest.clone(),
        identity: booted_identity(trust)?,
    };
    let (digest, config_digest) = resolve(trust)?;
    let candidate = candidate(trust, &digest, &config_digest)?;
    candidate.follows(&booted)?;
    Ok(candidate)
}

fn state(
    identity_valid: bool,
    held: bool,
    host: &Host,
    available: Option<&Receipt>,
) -> &'static str {
    if !identity_valid {
        return "identity_mismatch";
    }
    if held || host.rollback_queued {
        return "held";
    }
    let Some(candidate) = available else {
        return if host.staged.is_some() {
            "staged"
        } else {
            "booted"
        };
    };
    match &host.staged {
        Some(staged) if staged.digest != candidate.digest => "pending_other_deployment",
        Some(staged) if staged.download_only => "downloaded",
        Some(_) => "staged",
        None if host.booted.digest == candidate.digest => "current",
        None => "available",
    }
}

fn response(
    trust: &Trust,
    host: &Host,
    held: bool,
    available: Option<&Receipt>,
    checked: bool,
) -> Result<()> {
    let time = now()?;
    let identity = booted_identity(trust);
    let state = state(identity.is_ok(), held, host, available);
    // bootc verified a digest's signature when it deployed it as the booted or staged image.
    let deployed = available.is_some_and(|candidate| {
        host.booted.digest == candidate.digest
            || host
                .staged
                .as_ref()
                .is_some_and(|s| s.digest == candidate.digest && !s.download_only)
    });
    let (identity, identity_error) = match identity {
        Ok(identity) => (Some(identity), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let resolution = available
        .map(|r| r.identity.resolved_at)
        .or_else(|| identity.as_ref().map(|i| i.resolved_at));
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":sysroot_core::protocol::PROTOCOL,"kind":"signed_image_status","state":state,
            "scope":trust.scope,"channel":format!("{}:{}",trust.scope.repository,image::CHANNEL),
            "host":host,"installed_identity":identity,"installed_identity_error":identity_error,
            "rollback_hold":held,"available":available,"registry_checked":checked,"signature_verified":deployed,
            "freshness":{"registry_checked_at":checked.then_some(time),
            "signed_resolution_at":resolution,"resolution_age_seconds":resolution.map(|r|time.saturating_sub(r)),
            "resolution_stale_warning":resolution.map(|r|time.saturating_sub(r)>36*3600),"latest_upstream_resolution_proven":false},
            "reboot_performed":false,"activation_performed":false,"home_reconciliation_checked":false
        }))?
    );
    Ok(())
}

/// bootc with its progress on the caller's terminal; stdout stays the response channel.
fn bootc(arguments: &[&str]) -> Result<bool> {
    Ok(process(BOOTC, "30m", arguments)
        .stdout(Stdio::from(std::io::stderr().as_fd().try_clone_to_owned()?))
        .stderr(Stdio::inherit())
        .status()?
        .success())
}

fn check(trust: &Trust, before: &Host, held: bool) -> Result<()> {
    let available = available(trust, before)?;
    if observe(&trust.scope)? != *before {
        return Err("native deployment changed during check".into());
    }
    response(trust, before, held, Some(&available), true)
}

fn stage(
    trust: &Trust,
    before: &Host,
    hold: &mut Hold,
    expected_digest: Option<&str>,
    replace_staged: Option<&str>,
    resume: bool,
) -> Result<()> {
    let available = available(trust, before)?;
    if expected_digest.is_some_and(|digest| digest != available.digest) {
        return Err("stable's digest differs from the explicit expectation".into());
    }
    let action =
        deployment::stage_action(before, &available.digest, replace_staged, hold.held, resume)?;
    if action == StageAction::Switch {
        if observe(&trust.scope)? != *before {
            return Err("native deployment changed before staging".into());
        }
        let reference = format!("{}@{}", trust.scope.repository, available.digest);
        eprintln!(
            "sysroot-helper: staging {reference}; bootc verifies its signature with the \
             installed policy and fetches only layers it does not already have"
        );
        if !bootc(&["switch", "--enforce-container-sigpolicy", &reference])? {
            return Err(
                "bootc refused or could not stage the image; inspect `sysroot update status`"
                    .into(),
            );
        }
    }
    let after = observe(&trust.scope)?;
    let deployed = if action == StageAction::AlreadyBooted {
        after.booted.digest == available.digest && after.staged.is_none()
    } else {
        after
            .staged
            .as_ref()
            .is_some_and(|s| s.digest == available.digest && !s.download_only)
    };
    if !deployed {
        return Err(
            "bootc did not stage the verified digest; inspect `sysroot update status`".into(),
        );
    }
    if resume {
        hold.set(false)?;
    }
    response(trust, &after, hold.held, Some(&available), true)
}

fn rollback(
    trust: &Trust,
    before: &Host,
    hold: &mut Hold,
    replace_staged: Option<&str>,
) -> Result<()> {
    let slot = before
        .rollback
        .as_ref()
        .ok_or("no retained rollback deployment")?;
    if slot.download_only {
        return Err("rollback slot is download-only".into());
    }
    if before.staged.as_ref().map(|s| s.digest.as_str()) != replace_staged {
        return Err("explicit pending digest replacement is required".into());
    }
    // bootc verified the retained deployment when it deployed it. Hold forward
    // updates first, so an interrupted rollback still leaves them held.
    hold.set(true)?;
    if !before.rollback_queued {
        if observe(&trust.scope)? != *before {
            return Err("native deployment changed before rollback; updates are held".into());
        }
        if !bootc(&["rollback"])? {
            return Err("bootc could not queue the rollback; updates are held".into());
        }
    }
    let after = observe(&trust.scope)?;
    if !after.rollback_queued || after.rollback.as_ref() != Some(slot) {
        return Err("bootc did not queue the retained rollback; updates are held".into());
    }
    response(trust, &after, hold.held, None, false)
}

pub(super) fn run(request: Request, trust: &Trust) -> Result<()> {
    // The scope must be an enabled production target built for this helper's own architecture.
    if targets::enabled(&trust.scope.target, &trust.scope.architecture)
        .is_none_or(|spec| spec.repository != trust.scope.repository)
        || trust.scope.architecture != std::env::consts::ARCH
    {
        return Err("no direct channel configured for installed scope".into());
    }
    let _lock = lock()?;
    compatibility::require_bootc(&output(BOOTC, &["--version"], 4096)?)?;
    let before = observe(&trust.scope)?;
    let mut hold = Hold::load()?;
    match request {
        Request::ChannelStatus {} => response(trust, &before, hold.held, None, false),
        Request::ChannelCheck {} => check(trust, &before, hold.held),
        Request::ChannelStage {
            expected_digest,
            replace_staged,
            resume,
        } => stage(
            trust,
            &before,
            &mut hold,
            expected_digest.as_deref(),
            replace_staged.as_deref(),
            resume,
        ),
        Request::ChannelRollback { replace_staged } => {
            rollback(trust, &before, &mut hold, replace_staged.as_deref())
        }
        Request::VerifyInstaller {} => Err("unsupported direct registry operation".into()),
    }
}
