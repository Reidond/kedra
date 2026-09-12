//! Bounded signed-image identity and monotonically ordered deployment receipts.
use crate::{deployment::digest_valid, release::Scope};
use serde::{Deserialize, Serialize};

pub const LABEL: &str = "org.kedra.image.identity";
pub const REPOSITORY: &str = "ghcr.io/reidond/kedra-desktop";
pub const CHANNEL: &str = "stable";
pub const MAX_IDENTITY: usize = 16_384;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub schema_version: u32,
    pub project: String,
    pub target: String,
    pub architecture: String,
    pub fedora_release: u32,
    pub repository: String,
    pub channel: String,
    pub source_revision: String,
    pub source_manifest_sha256: String,
    pub resolved_inputs_sha256: String,
    pub workflow: String,
    pub epoch: u64,
    pub run_number: u64,
    pub run_attempt: u64,
    pub resolved_at: u64,
    pub minimum_protocol: u32,
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl Identity {
    pub fn rank(&self) -> (u64, u64, u64) {
        (self.epoch, self.run_number, self.run_attempt)
    }

    pub fn validate(&self, scope: &Scope, now: u64) -> Result<(), &'static str> {
        if self.schema_version != 2
            || self.project != "Kedra"
            || self.target != "desktop"
            || self.target != scope.target
            || self.architecture != "x86_64"
            || self.architecture != scope.architecture
            || self.fedora_release != 44
            || self.fedora_release != scope.fedora_release
            || self.repository != REPOSITORY
            || self.repository != scope.repository
            || self.channel != CHANNEL
            || self.workflow != ".github/workflows/release.yml"
            || self.epoch != 1
            || self.run_number == 0
            || self.run_attempt == 0
            || self.minimum_protocol == 0
            || self.minimum_protocol > 2
            || !hex(&self.source_revision, 40)
            || !hex(&self.source_manifest_sha256, 64)
            || !hex(&self.resolved_inputs_sha256, 64)
            || self.resolved_at == 0
            || self.resolved_at > now.saturating_add(300)
        {
            return Err("signed image identity has incompatible scope, protocol, ordering or time");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub digest: String,
    pub identity: Identity,
}

impl Receipt {
    pub fn validate(&self, scope: &Scope, now: u64) -> Result<(), &'static str> {
        if !digest_valid(&self.digest) {
            return Err("invalid image receipt digest");
        }
        self.identity.validate(scope, now)
    }

    pub fn follows(&self, previous: &Self) -> Result<(), &'static str> {
        if self.identity.rank() < previous.identity.rank() {
            return Err("signed image rank is below the retained high-water mark");
        }
        if self.identity.rank() == previous.identity.rank() && self != previous {
            return Err("same signed image rank has different digest or identity");
        }
        if self.identity.resolved_at < previous.identity.resolved_at {
            return Err("signed package-resolution time moved backwards");
        }
        Ok(())
    }
}
