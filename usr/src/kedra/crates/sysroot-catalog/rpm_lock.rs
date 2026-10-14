//! The RPM lock: every Fedora RPM one target's foundation and builder roles
//! install, resolved once per run and then installed offline.
//!
//! The committed `packages.lock.json` keeps the author's source pins. This lock is
//! produced per release run, embedded in the image at
//! `/usr/share/sysroot/rpm-lock.json` and bound to its signed identity through
//! the hash of its pins: the lock without mirror URLs, which differ between runs
//! that resolved the same packages.
use crate::language::Intent;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use sysroot_engine::{
    Error, MAX_RPMS, Result, RpmFile, RpmRequest, RpmResolution, RpmRole, package_name,
    pinned_image, rpm_architectures, validate_rows,
};

pub const FORMAT: &str = "kedra-rpm-lock";
/// Installed image path of the lock that built it.
pub const IMAGE_PATH: &str = "/usr/share/sysroot/rpm-lock.json";
pub const MAX_LOCK_BYTES: u64 = sysroot_engine::MAX_JSON;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpmLock {
    pub schema_version: u32,
    pub format: String,
    pub target: String,
    pub architecture: String,
    pub fedora_release: u32,
    pub base: String,
    pub repositories: Vec<String>,
    /// SHA-256 of the selected `.kedra` intent the requests came from.
    pub intent_sha256: String,
    #[serde(deserialize_with = "sysroot_engine::deserialize_unique_map")]
    pub rpms: BTreeMap<String, RpmFile>,
    pub foundation: RpmRole,
    #[serde(deserialize_with = "sysroot_engine::deserialize_unique_map")]
    pub builders: BTreeMap<String, RpmRole>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// SHA-256 of canonical JSON: sorted keys, no whitespace.
pub fn canonical(value: &serde_json::Value) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl RpmLock {
    /// Wrap one resolution for a target, then check it like any read lock.
    pub fn new(
        target: &str,
        intent_sha256: &str,
        request: &RpmRequest,
        resolution: RpmResolution,
    ) -> Result<Self> {
        let lock = Self {
            schema_version: 1,
            format: FORMAT.into(),
            target: target.into(),
            architecture: request.architecture.clone(),
            fedora_release: 44,
            base: request.base.clone(),
            repositories: request.repositories.clone(),
            intent_sha256: intent_sha256.into(),
            rpms: resolution.rpms,
            foundation: resolution.foundation,
            builders: resolution.builders,
        };
        lock.validate()?;
        Ok(lock)
    }

    /// Parse a lock file. Only its exact canonical encoding is accepted, so the
    /// pins hash of a file is stable wherever it is computed.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        if bytes.len() as u64 > MAX_LOCK_BYTES {
            return Err(invalid("RPM lock exceeds 8 MiB"));
        }
        let lock: Self = serde_json::from_slice(bytes)?;
        if lock.to_bytes()? != bytes {
            return Err(invalid("RPM lock is not canonical JSON"));
        }
        lock.validate()?;
        Ok(lock)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        canonical(&serde_json::to_value(self)?)
    }

    /// The lock without per-run mirror locations.
    pub fn pins(&self) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(self)?;
        if let Some(rpms) = value.get_mut("rpms").and_then(|v| v.as_object_mut()) {
            for entry in rpms.values_mut() {
                entry.as_object_mut().map(|e| e.remove("mirror_url"));
            }
        }
        Ok(value)
    }

    /// Identity of what the lock installs; equal for runs that resolved the same RPMs.
    pub fn pins_sha256(&self) -> Result<String> {
        Ok(sha256(&canonical(&self.pins()?)?))
    }

    /// The foundation role, or one builder role by its catalog name.
    pub fn role(&self, builder: Option<&str>) -> Result<&RpmRole> {
        match builder {
            None => Ok(&self.foundation),
            Some(name) => self
                .builders
                .get(name)
                .ok_or_else(|| invalid(format!("RPM lock has no builder role {name:?}"))),
        }
    }

    /// The locked files one role installs.
    pub fn files(&self, role: &RpmRole) -> Vec<&RpmFile> {
        role.install.iter().map(|file| &self.rpms[file]).collect()
    }

    pub fn validate(&self) -> Result<()> {
        let architectures = rpm_architectures(&self.architecture)?;
        let header = self.schema_version == 1
            && self.format == FORMAT
            && self.fedora_release == 44
            && package_name(&self.target)
            && pinned_image(&self.base)
            && !self.repositories.is_empty()
            && self.repositories.len() <= 16
            && self.repositories.iter().all(|r| package_name(r))
            && hex(&self.intent_sha256)
            && self.rpms.len() <= MAX_RPMS
            && self.builders.len() <= 16
            && !architectures.is_empty();
        if !header {
            return Err(invalid("unsupported RPM lock header"));
        }
        for (file, rpm) in &self.rpms {
            rpm.validate(file, &self.architecture)?;
        }
        self.validate_role("foundation", &self.foundation, None)?;
        for (name, role) in &self.builders {
            if name.is_empty() || name.len() > 256 || name.bytes().any(|b| b < 33 || b == 127) {
                return Err(invalid("invalid builder role name"));
            }
            self.validate_role(name, role, Some(&self.foundation))?;
        }
        let used: BTreeSet<&String> = std::iter::once(&self.foundation)
            .chain(self.builders.values())
            .flat_map(|role| &role.install)
            .collect();
        if used.len() != self.rpms.len() {
            return Err(invalid("RPM lock lists files no role installs"));
        }
        Ok(())
    }

    fn validate_role(&self, name: &str, role: &RpmRole, parent: Option<&RpmRole>) -> Result<()> {
        validate_rows(&role.packages, &self.architecture)?;
        let names: BTreeSet<&str> = role.packages.iter().map(|row| row[0].as_str()).collect();
        let rows: BTreeSet<&Vec<String>> = role.packages.iter().collect();
        let parent_rows: BTreeSet<&Vec<String>> =
            parent.map_or_else(BTreeSet::new, |p| p.packages.iter().collect());
        let requests = !role.requests.is_empty()
            && role.requests.len() <= 512
            && role
                .requests
                .iter()
                .all(|n| package_name(n) && names.contains(n.as_str()));
        let remove = role.remove.len() <= 512
            && (parent.is_none() || role.remove.is_empty())
            && role
                .remove
                .iter()
                .all(|n| package_name(n) && !names.contains(n.as_str()));
        if !requests || !remove {
            return Err(invalid(format!(
                "{name}: requests must be installed and removals absent"
            )));
        }
        for file in &role.install {
            let rpm = self
                .rpms
                .get(file)
                .ok_or_else(|| invalid(format!("{name}: installs unlocked {file}")))?;
            let row = rpm.row();
            if !rows.contains(&row) || parent_rows.contains(&row) {
                return Err(invalid(format!(
                    "{name}: {file} is not a new package of this role"
                )));
            }
        }
        Ok(())
    }

    /// Bind the lock to the selected `.kedra` intent and its target.
    pub fn check_intent(&self, intent: &Intent, intent_sha256: &str) -> Result<()> {
        let builders: BTreeMap<&String, &BTreeSet<String>> = self
            .builders
            .iter()
            .map(|(name, role)| (name, &role.requests))
            .collect();
        let selected: BTreeMap<&String, &BTreeSet<String>> = intent.builders.iter().collect();
        if self.target != intent.target
            || self.intent_sha256 != intent_sha256
            || self.foundation.requests != intent.packages
            || self.foundation.remove != intent.remove
            || builders != selected
        {
            return Err(invalid(
                "RPM lock differs from the selected .kedra foundation and builder sets",
            ));
        }
        Ok(())
    }
}
