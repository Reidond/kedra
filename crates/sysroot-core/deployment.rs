//! Narrow bootc observations and deployment-journal transitions. No ambient I/O.
use crate::release::{Scope, TrustState};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidHost,
    InvalidState,
    UnverifiedImage,
    ScopeMismatch,
    PendingDeployment,
    RollbackQueued,
    ReadOnly,
    Overlay,
    RollbackHold,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidHost => "unsupported or malformed bootc host state",
            Self::InvalidState => "invalid deployment journal; do not reset it",
            Self::UnverifiedImage => "bootc image lacks container-policy enforcement",
            Self::ScopeMismatch => "bootc deployment is outside the installed target/repository",
            Self::PendingDeployment => {
                "another image is staged; explicitly name it for replacement"
            }
            Self::RollbackQueued => {
                "a rollback is queued; boot or cancel it through recovery first"
            }
            Self::ReadOnly => "bootc reports read-only management state",
            Self::Overlay => "temporary usr overlay is active; reboot before deployment",
            Self::RollbackHold => "updates are held after rollback; explicitly resume them",
        })
    }
}
impl std::error::Error for Error {}

pub fn digest_valid(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub digest: String,
    pub download_only: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub booted: Slot,
    pub staged: Option<Slot>,
    pub rollback: Option<Slot>,
    pub rollback_queued: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawHost {
    api_version: String,
    kind: String,
    spec: RawSpec,
    status: RawStatus,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSpec {
    boot_order: String,
    image: ImageReference,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStatus {
    booted: Option<RawSlot>,
    staged: Option<RawSlot>,
    rollback: Option<RawSlot>,
    rollback_queued: bool,
    read_only: bool,
    usr_overlay: Option<serde_json::Value>,
    #[serde(rename = "type")]
    host_type: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSlot {
    image: ImageStatus,
    download_only: bool,
    incompatible: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageStatus {
    architecture: String,
    image_digest: String,
    image: ImageReference,
}
#[derive(Deserialize)]
struct ImageReference {
    image: String,
    transport: String,
    signature: Option<String>,
}
impl ImageReference {
    fn validate(&self, scope: &Scope) -> Result<String, Error> {
        if self.signature.as_deref() != Some("containerPolicy") {
            return Err(Error::UnverifiedImage);
        }
        if self.transport != "registry" {
            return Err(Error::ScopeMismatch);
        }
        let (repository, digest) = self.image.split_once('@').ok_or(Error::ScopeMismatch)?;
        if repository != scope.repository || !digest_valid(digest) {
            return Err(Error::ScopeMismatch);
        }
        Ok(digest.to_owned())
    }
}
impl RawSlot {
    fn validate(self, scope: &Scope) -> Result<Slot, Error> {
        let digest = self.image.image.validate(scope)?;
        if self.image.architecture != "amd64"
            || scope.architecture != "x86_64"
            || self.image.image_digest != digest
            || self.incompatible
        {
            return Err(Error::ScopeMismatch);
        }
        Ok(Slot {
            digest,
            download_only: self.download_only,
        })
    }
}

/// Parse the qualified bootc v1 host API. Unknown native informational fields are
/// ignored, while duplicate/missing control fields and unsupported states fail.
pub fn observe(bytes: &[u8], scope: &Scope) -> Result<Host, Error> {
    if bytes.len() > 262_144 || scope.validate().is_err() {
        return Err(Error::InvalidHost);
    }
    let host: RawHost = serde_json::from_slice(bytes).map_err(|_| Error::InvalidHost)?;
    if host.api_version != "org.containers.bootc/v1"
        || host.kind != "BootcHost"
        || host.status.host_type != "bootcHost"
        || !["default", "rollback"].contains(&host.spec.boot_order.as_str())
    {
        return Err(Error::InvalidHost);
    }
    host.spec.image.validate(scope)?;
    if host.status.read_only {
        return Err(Error::ReadOnly);
    }
    if host.status.usr_overlay.is_some() {
        return Err(Error::Overlay);
    }
    let booted = host
        .status
        .booted
        .ok_or(Error::InvalidHost)?
        .validate(scope)?;
    let staged = host
        .status
        .staged
        .map(|slot| slot.validate(scope))
        .transpose()?;
    let rollback = host
        .status
        .rollback
        .map(|slot| slot.validate(scope))
        .transpose()?;
    if booted.download_only || host.status.rollback_queued && rollback.is_none() {
        return Err(Error::InvalidHost);
    }
    Ok(Host {
        booted,
        staged,
        rollback,
        rollback_queued: host.status.rollback_queued,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Stage,
    Rollback,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Intent,
    AwaitingReboot,
    Booted,
    NotApplied,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub kind: OperationKind,
    pub target_digest: String,
    pub release_sha256: String,
    pub previous_booted: String,
    pub previous_staged: Option<String>,
    pub phase: Phase,
}
impl Operation {
    pub fn observe(&mut self, host: &Host) {
        self.phase = if host.booted.digest == self.target_digest
            && host.staged.is_none()
            && !host.rollback_queued
        {
            Phase::Booted
        } else if host
            .staged
            .as_ref()
            .is_some_and(|slot| slot.digest == self.target_digest && !slot.download_only)
            || host.rollback_queued
                && host
                    .rollback
                    .as_ref()
                    .is_some_and(|slot| slot.digest == self.target_digest)
        {
            Phase::AwaitingReboot
        } else {
            Phase::NotApplied
        };
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema_version: u32,
    pub machine: String,
    pub scope: Scope,
    pub high_water: TrustState,
    pub rollback_hold: bool,
    pub operation: Option<Operation>,
}
impl Journal {
    pub fn validate(&self, machine: &str, scope: &Scope) -> Result<(), Error> {
        if self.schema_version != 1
            || self.machine != machine
            || self.scope != *scope
            || self.high_water.scope != *scope
            || self.high_water.validate().is_err()
            || self.machine.len() != 64
            || !self
                .machine
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            || self.operation.as_ref().is_some_and(|op| {
                !digest_valid(&op.target_digest)
                    || !digest_valid(&format!("sha256:{}", op.release_sha256))
                    || !digest_valid(&op.previous_booted)
                    || op
                        .previous_staged
                        .as_ref()
                        .is_some_and(|digest| !digest_valid(digest))
            })
        {
            return Err(Error::InvalidState);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StageAction {
    AlreadyBooted,
    AlreadyStaged,
    Switch,
}

/// This preflight is advisory until the caller verifies signed metadata, holds
/// its management lock, re-observes native state and durably records its intent.
pub fn stage_action(
    host: &Host,
    digest: &str,
    replace: Option<&str>,
    rollback_hold: bool,
    resume: bool,
) -> Result<StageAction, Error> {
    if !digest_valid(digest) || replace.is_some_and(|value| !digest_valid(value)) {
        return Err(Error::InvalidState);
    }
    if host.rollback_queued {
        return Err(Error::RollbackQueued);
    }
    if rollback_hold && !resume {
        return Err(Error::RollbackHold);
    }
    if let Some(staged) = &host.staged {
        if staged.digest == digest && !staged.download_only {
            return Ok(StageAction::AlreadyStaged);
        }
        if staged.digest != digest && replace != Some(staged.digest.as_str()) {
            return Err(Error::PendingDeployment);
        }
    } else if replace.is_some() {
        return Err(Error::PendingDeployment);
    }
    if host.booted.digest == digest && host.staged.is_none() {
        return Ok(StageAction::AlreadyBooted);
    }
    Ok(StageAction::Switch)
}

pub fn intent(
    host: &Host,
    kind: OperationKind,
    digest: &str,
    release_sha256: &str,
) -> Result<Operation, Error> {
    if !digest_valid(digest) || !digest_valid(&format!("sha256:{release_sha256}")) {
        return Err(Error::InvalidState);
    }
    Ok(Operation {
        kind,
        target_digest: digest.to_owned(),
        release_sha256: release_sha256.to_owned(),
        previous_booted: host.booted.digest.clone(),
        previous_staged: host.staged.as_ref().map(|slot| slot.digest.clone()),
        phase: Phase::Intent,
    })
}
