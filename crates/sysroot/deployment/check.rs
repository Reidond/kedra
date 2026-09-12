//! Anonymous channel discovery with installed trust and advisory deployment state.
use crate::release_channel::{Bundle, MAX_BUNDLE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use sysroot_core::deployment::{Host, Operation, digest_valid};
use sysroot_core::release::{self, Scope, TrustState};
use sysroot_helper::protocol::{Envelope, Request};
use sysroot_helper::trusted_file;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const POLICY: &str = "/usr/lib/sysroot/trust/release-policy.json";
const KEY: &str = "/usr/lib/sysroot/trust/release.pub";
const SOURCE: &str = "/usr/share/sysroot/source.json";
const CHANNEL: &str =
    "https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json";

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

#[derive(PartialEq, Eq)]
struct Trust {
    scope: Scope,
    key: String,
    fingerprint: String,
    source_revision: String,
    source_manifest_sha256: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Status {
    schema_version: u32,
    enrolled: bool,
    scope: Scope,
    host: Host,
    journal: Option<Journal>,
    reboot_performed: bool,
    activation_performed: bool,
    home_reconciliation_checked: bool,
    home_reconciliation_required: Option<bool>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Journal {
    high_water: TrustState,
    rollback_hold: bool,
    operation: Option<Operation>,
}

impl Status {
    fn validate(&self) -> Result<()> {
        self.scope.validate()?;
        if self.schema_version != 1
            || self.enrolled != self.journal.is_some()
            || self.reboot_performed
            || self.activation_performed
            || self.home_reconciliation_checked
            || self.home_reconciliation_required.is_some()
            || self.host.booted.download_only
            || self.host.rollback_queued && self.host.rollback.is_none()
            || !digest_valid(&self.host.booted.digest)
            || self
                .host
                .staged
                .as_ref()
                .is_some_and(|slot| !digest_valid(&slot.digest))
            || self
                .host
                .rollback
                .as_ref()
                .is_some_and(|slot| !digest_valid(&slot.digest))
        {
            return Err("installed helper status is incompatible".into());
        }
        if let Some(journal) = &self.journal {
            journal.high_water.validate()?;
            if journal.high_water.scope != self.scope {
                return Err("installed helper high-water scope is incompatible".into());
            }
        }
        Ok(())
    }

    fn high_water(&self) -> Option<&TrustState> {
        self.journal.as_ref().map(|journal| &journal.high_water)
    }

    fn rollback_hold(&self) -> bool {
        self.journal
            .as_ref()
            .is_some_and(|journal| journal.rollback_hold)
    }
}

fn load_trust(status: &Status) -> Result<Trust> {
    let policy: Policy =
        serde_json::from_slice(&trusted_file::read(Path::new(POLICY), 0, 16_384)?)?;
    policy.scope.validate()?;
    if policy.schema_version != 1 || policy.scope != status.scope {
        return Err("installed release policy disagrees with helper status".into());
    }
    let key = String::from_utf8(trusted_file::read(Path::new(KEY), 0, 4096)?)?;
    let fingerprint = release::public_key_fingerprint(&key)?;
    if fingerprint != policy.key_fingerprint {
        return Err("installed release key differs from its policy fingerprint".into());
    }
    let source_bytes = trusted_file::read(Path::new(SOURCE), 0, 1_048_576)?;
    let source: Source = serde_json::from_slice(&source_bytes)?;
    if source.schema_version != 1
        || !source.target.candidate_target
        || source.target.id != policy.scope.target
        || source.target.architecture != policy.scope.architecture
        || source.target.image != policy.scope.repository
        || source.target.fedora_release != policy.scope.fedora_release
    {
        return Err("installed source and release policy disagree about the target".into());
    }
    Ok(Trust {
        scope: policy.scope,
        key,
        fingerprint,
        source_revision: source.source_revision,
        source_manifest_sha256: Sha256::digest(&source_bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}

fn channel(scope: &Scope) -> Result<&'static str> {
    if scope.target == "desktop"
        && scope.architecture == "x86_64"
        && scope.fedora_release == 44
        && scope.repository == "ghcr.io/reidond/kedra-desktop"
    {
        Ok(CHANNEL)
    } else {
        Err("no published channel is configured for this installed target".into())
    }
}

fn capture(
    mut command: Command,
    input: Option<&[u8]>,
    limit: usize,
    name: &str,
) -> Result<Vec<u8>> {
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()?;
    let result = (|| -> Result<Vec<u8>> {
        if let Some(input) = input {
            child
                .stdin
                .take()
                .ok_or("child input is unavailable")?
                .write_all(input)?;
        }
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("child output is unavailable")?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(format!("{name} exceeded its output bound").into());
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(format!("{name} failed ({status}); no update result is available").into());
        }
        Ok(bytes)
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn status() -> Result<Status> {
    let request = serde_json::to_vec(&Envelope {
        schema_version: 1,
        request: Request::Status {},
    })?;
    let mut command = Command::new("/usr/bin/sudo");
    command
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args(["--", "/usr/libexec/sysroot/helper"]);
    let bytes = capture(
        command,
        Some(&request),
        1_048_576,
        "installed helper status",
    )?;
    let status: Status = serde_json::from_slice(&bytes)
        .map_err(|_| "installed helper status is malformed or incompatible")?;
    status.validate()?;
    Ok(status)
}

fn fetch(url: &'static str) -> Result<Vec<u8>> {
    let mut command = Command::new("/usr/bin/curl");
    command
        .env_clear()
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--disallow-username-in-url",
            "--max-redirs",
            "5",
            "--connect-timeout",
            "10",
            "--max-time",
            "45",
            "--max-filesize",
            "300000",
            "--url",
            url,
        ])
        .stderr(Stdio::null());
    // curl bounds the transfer time; this reader also bounds bytes regardless of headers.
    capture(command, None, MAX_BUNDLE, "anonymous channel download")
}

struct Discovery {
    status: Status,
    trust: Trust,
    bundle: Bundle,
    update: release::VerifiedUpdate,
    now: u64,
    url: &'static str,
}

fn discover() -> Result<Discovery> {
    let uid = rustix::process::getuid().as_raw();
    if uid == 0 || rustix::process::geteuid().as_raw() != uid {
        return Err("channel operations run as the ordinary invoking user, never root".into());
    }
    eprintln!(
        "sysroot: reading installed deployment state through the helper; an existing operation may be reconciled"
    );
    let before = status()?;
    let trust = load_trust(&before)?;
    let url = channel(&trust.scope)?;
    let bytes = fetch(url)?;
    let after = status()?;
    if before != after || trust != load_trust(&after)? {
        return Err(
            "installed deployment or trust changed during channel verification; retry the operation".into(),
        );
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let bundle = Bundle::parse(&bytes)?;
    let update = bundle.verify(&trust.key, &trust.scope, after.high_water(), now)?;
    let release = update.release.release();
    let matches_booted = release.image_digest == after.host.booted.digest;
    if matches_booted
        && (release.source_revision != trust.source_revision
            || release.home_manifest_sha256 != trust.source_manifest_sha256)
    {
        return Err("running image provenance differs from the signed channel release".into());
    }
    Ok(Discovery {
        status: after,
        trust,
        bundle,
        update,
        now,
        url,
    })
}

/// Return the exact authenticated payloads; the helper verifies them again at mutation time.
pub(super) fn documents() -> Result<(
    sysroot_helper::protocol::SignedDocument,
    sysroot_helper::protocol::SignedDocument,
)> {
    let discovery = discover()?;
    eprintln!(
        "sysroot: verified channel release {} ({}) for independent helper verification",
        discovery.update.release.release().sequence,
        discovery.update.release.release().image_reference()
    );
    Ok(discovery.bundle.into_documents())
}

pub(super) fn run(json: bool) -> Result<()> {
    let Discovery {
        status: after,
        trust,
        update,
        now,
        url,
        ..
    } = discover()?;
    let release = update.release.release();
    let matches_booted = release.image_digest == after.host.booted.digest;
    let matches_staged = after
        .host
        .staged
        .as_ref()
        .is_some_and(|slot| slot.digest == release.image_digest);
    let state = if after.rollback_hold() || after.host.rollback_queued {
        "held"
    } else if !after.enrolled {
        "enrollment_required"
    } else if let Some(staged) = &after.host.staged {
        if !matches_staged {
            "pending_other_deployment"
        } else if staged.download_only {
            "downloaded"
        } else {
            "staged"
        }
    } else if matches_booted {
        "current"
    } else {
        "available"
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "kind": "update_check",
                "state": state,
                "verified_at": now,
                "channel_url": url,
                "signature_valid": true,
                "channel_freshness_verified": true,
                "replay_checked": after.enrolled,
                "key_fingerprint_sha256": trust.fingerprint,
                "channel": {
                    "release_sequence": release.sequence,
                    "release_sha256": update.release.sha256(),
                    "checkpoint_generation": update.next_trust_state.generation,
                    "last_successful_resolution": update.next_trust_state.last_successful_resolution,
                    "image_reference": release.image_reference(),
                    "image_matches_booted": matches_booted,
                    "image_matches_staged": matches_staged,
                    "advances_release_sequence": after.high_water().map(|floor| release.sequence > floor.highest_release_sequence),
                    "advances_checkpoint_generation": after.high_water().map(|floor| update.next_trust_state.generation > floor.generation)
                },
                "installed_status": after,
                "deployment_authorized": false,
                "staging_performed": false,
                "reboot_performed": false,
                "activation_performed": false,
                "high_water_advanced": false,
                "helper_status_reconciliation_possible": true
            }))?
        );
    } else {
        println!(
            "Fresh signed channel: release {}, checkpoint {}",
            release.sequence, update.next_trust_state.generation
        );
        println!("Update state: {state}");
        println!(
            "Running image: {}@{}",
            trust.scope.repository, after.host.booted.digest
        );
        println!("Channel image: {}", release.image_reference());
        if let Some(staged) = &after.host.staged {
            println!(
                "Pending image: {} (download only: {})",
                staged.digest, staged.download_only
            );
        }
        if !after.enrolled {
            println!("Enrollment is required; earlier accepted channel history was not checked.");
        }
        if after.rollback_hold() || after.host.rollback_queued {
            println!(
                "Rollback hold: {}; rollback queued: {}. Review recovery before explicitly resuming updates.",
                after.rollback_hold(),
                after.host.rollback_queued
            );
        }
        println!(
            "No image was staged, reboot requested, home activated or high-water state advanced by this check."
        );
    }
    Ok(())
}
