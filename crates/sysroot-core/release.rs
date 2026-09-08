//! Exact-byte ECDSA/P-256/SHA-256 verification and release eligibility.
//!
//! No subprocess, network, privilege or persistent-state writes. A caller-supplied
//! key/policy is advisory; privileged callers must load independently trusted state.
use std::fmt;
use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub const PROTOCOL: u32 = 1;
pub const MAX_DOCUMENT: usize = 65_536;
const MAX_LIFETIME: u64 = 7 * 24 * 60 * 60;
const CLOCK_SKEW: u64 = 300;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    SizeLimit,
    InvalidKey,
    InvalidSignature,
    InvalidDocument(String),
    UnsupportedSchema(u32),
    ScopeMismatch,
    Unpromoted,
    Incompatible,
    Replay(&'static str),
    Time(&'static str),
    ArtifactMismatch,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "release I/O: {e}"),
            Self::SizeLimit => write!(f, "release input exceeds its size limit"),
            Self::InvalidKey => write!(f, "expected a trusted P-256 public key in SPKI PEM format"),
            Self::InvalidSignature => write!(f, "signature verification failed"),
            Self::InvalidDocument(e) => write!(f, "invalid release document: {e}"),
            Self::UnsupportedSchema(v) => {
                write!(f, "unsupported release schema {v}; upgrade the reader")
            }
            Self::ScopeMismatch => write!(
                f,
                "target, architecture, Fedora release or repository mismatch"
            ),
            Self::Unpromoted => write!(f, "candidate has not been promoted"),
            Self::Incompatible => write!(f, "release requires a newer management protocol"),
            Self::Replay(e) => write!(f, "release replay rejected: {e}"),
            Self::Time(e) => write!(f, "release freshness check failed: {e}"),
            Self::ArtifactMismatch => write!(
                f,
                "artifact size or SHA-256 does not match the signed release"
            ),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
fn invalid(s: impl Into<String>) -> Error {
    Error::InvalidDocument(s.into())
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn target_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn repository(s: &str) -> bool {
    if s.len() > 255 {
        return false;
    }
    let Some((host, path)) = s.split_once('/') else {
        return false;
    };
    let hostname = if let Some((hostname, port)) = host.split_once(':') {
        if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if port.parse::<u16>().ok().is_none_or(|port| port == 0) {
            return false;
        }
        hostname
    } else {
        host
    };
    (hostname.contains('.') || hostname == "localhost")
        && hostname.split('.').all(|p| {
            !p.is_empty()
                && p.as_bytes()[0].is_ascii_alphanumeric()
                && p.as_bytes()[p.len() - 1].is_ascii_alphanumeric()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && path.split('/').all(|p| {
            !p.is_empty()
                && p.as_bytes()[0].is_ascii_alphanumeric()
                && p.as_bytes()[p.len() - 1].is_ascii_alphanumeric()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub target: String,
    pub architecture: String,
    pub fedora_release: u32,
    pub repository: String,
}
impl Scope {
    pub fn validate(&self) -> Result<(), Error> {
        if !target_id(&self.target)
            || self.architecture != "x86_64"
            || self.fedora_release != 44
            || !repository(&self.repository)
        {
            return Err(invalid("unsupported or malformed target scope"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    pub repository: String,
    pub workflow: String,
    pub run_id: u64,
    pub run_attempt: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Approval {
    Candidate,
    Promoted,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub schema_version: u32,
    pub project: String,
    pub scope: Scope,
    pub sequence: u64,
    pub source_revision: String,
    pub build: Build,
    pub image_digest: String,
    pub home_manifest_sha256: String,
    pub installer: Artifact,
    pub approval: Approval,
    pub minimum_protocol: u32,
}
impl Release {
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != PROTOCOL {
            return Err(Error::UnsupportedSchema(self.schema_version));
        }
        self.scope.validate()?;
        if self.project != "Kedra"
            || self.sequence == 0
            || self.minimum_protocol == 0
            || !(hex(&self.source_revision, 40) || hex(&self.source_revision, 64))
            || !self
                .image_digest
                .strip_prefix("sha256:")
                .is_some_and(|s| hex(s, 64))
            || !hex(&self.home_manifest_sha256, 64)
            || !hex(&self.installer.sha256, 64)
            || self.installer.size_bytes == 0
            || self.installer.size_bytes > 64 * 1024 * 1024 * 1024
            || self.installer.filename.len() > 128
            || !self.installer.filename.ends_with(".iso")
            || !self
                .installer
                .filename
                .starts_with(&format!("kedra-{}-44-", self.scope.target))
            || !self
                .installer
                .filename
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
            || self.build.repository != "Reidond/kedra"
            || self.build.workflow != ".github/workflows/release.yml"
            || self.build.run_id == 0
            || self.build.run_attempt == 0
        {
            return Err(invalid(
                "identity, digest, build provenance or installer field",
            ));
        }
        if self.minimum_protocol > PROTOCOL {
            return Err(Error::Incompatible);
        }
        if self.approval != Approval::Promoted {
            return Err(Error::Unpromoted);
        }
        Ok(())
    }
    pub fn image_reference(&self) -> String {
        format!("{}@{}", self.scope.repository, self.image_digest)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub schema_version: u32,
    pub project: String,
    pub scope: Scope,
    pub generation: u64,
    pub release_sha256: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub last_successful_resolution: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrustState {
    pub schema_version: u32,
    pub scope: Scope,
    pub generation: u64,
    pub checkpoint_sha256: String,
    pub highest_release_sequence: u64,
    pub highest_release_sha256: String,
    pub last_successful_resolution: u64,
}
impl TrustState {
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema_version != PROTOCOL {
            return Err(Error::UnsupportedSchema(self.schema_version));
        }
        self.scope.validate()?;
        if self.generation == 0
            || self.highest_release_sequence == 0
            || self.last_successful_resolution == 0
            || !hex(&self.checkpoint_sha256, 64)
            || !hex(&self.highest_release_sha256, 64)
        {
            return Err(invalid("corrupt trust high-water state"));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct VerifiedRelease {
    release: Release,
    sha256: String,
    key_fingerprint: String,
}
impl VerifiedRelease {
    pub fn release(&self) -> &Release {
        &self.release
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn key_fingerprint(&self) -> &str {
        &self.key_fingerprint
    }
}
#[derive(Debug)]
pub struct VerifiedUpdate {
    pub release: VerifiedRelease,
    pub next_trust_state: TrustState,
}

/// Identify a bounded P-256 SPKI public key without accepting any signed content.
pub fn public_key_fingerprint(public_key: &str) -> Result<String, Error> {
    if public_key.len() > 4096 {
        return Err(Error::InvalidKey);
    }
    let key = VerifyingKey::from_public_key_pem(public_key).map_err(|_| Error::InvalidKey)?;
    Ok(hash(
        key.to_public_key_der()
            .map_err(|_| Error::InvalidKey)?
            .as_bytes(),
    ))
}

fn signed<T: DeserializeOwned>(
    payload: &[u8],
    signature: &[u8],
    public_key: &str,
) -> Result<(T, String), Error> {
    if payload.is_empty()
        || payload.len() > MAX_DOCUMENT
        || signature.len() > 1024
        || public_key.len() > 4096
    {
        return Err(Error::SizeLimit);
    }
    let key = VerifyingKey::from_public_key_pem(public_key).map_err(|_| Error::InvalidKey)?;
    let signature = std::str::from_utf8(signature).map_err(|_| Error::InvalidSignature)?;
    let bytes = STANDARD
        .decode(signature.trim())
        .map_err(|_| Error::InvalidSignature)?;
    let signature = Signature::from_der(&bytes).map_err(|_| Error::InvalidSignature)?;
    key.verify(payload, &signature)
        .map_err(|_| Error::InvalidSignature)?;
    let fingerprint = hash(
        key.to_public_key_der()
            .map_err(|_| Error::InvalidKey)?
            .as_bytes(),
    );
    let document = serde_json::from_slice(payload).map_err(|e| invalid(e.to_string()))?;
    Ok((document, fingerprint))
}

/// Verify immutable promoted release bytes. This does not establish channel freshness.
pub fn verify_release(
    payload: &[u8],
    signature: &[u8],
    public_key: &str,
    scope: Option<&Scope>,
) -> Result<VerifiedRelease, Error> {
    let (release, key_fingerprint): (Release, _) = signed(payload, signature, public_key)?;
    release.validate()?;
    if let Some(scope) = scope {
        scope.validate()?;
        if release.scope != *scope {
            return Err(Error::ScopeMismatch);
        }
    }
    Ok(VerifiedRelease {
        release,
        sha256: hash(payload),
        key_fingerprint,
    })
}

/// Validate a fresh signed checkpoint against independently loaded enrollment and
/// high-water state. Returning next state never writes it or stages/reboots an OS.
pub fn verify_update(
    release: VerifiedRelease,
    payload: &[u8],
    signature: &[u8],
    public_key: &str,
    scope: &Scope,
    previous: Option<&TrustState>,
    now: u64,
) -> Result<VerifiedUpdate, Error> {
    let (checkpoint, _): (Checkpoint, _) = signed(payload, signature, public_key)?;
    if checkpoint.schema_version != PROTOCOL {
        return Err(Error::UnsupportedSchema(checkpoint.schema_version));
    }
    scope.validate()?;
    if checkpoint.scope != *scope || release.release.scope != *scope {
        return Err(Error::ScopeMismatch);
    }
    if checkpoint.project != "Kedra"
        || checkpoint.generation == 0
        || checkpoint.release_sha256 != release.sha256
    {
        return Err(invalid("checkpoint identity or release binding"));
    }
    let Some(lifetime) = checkpoint.expires_at.checked_sub(checkpoint.issued_at) else {
        return Err(Error::Time("expiry precedes issue"));
    };
    if lifetime == 0 || lifetime > MAX_LIFETIME {
        return Err(Error::Time(
            "checkpoint lifetime must be at most seven days",
        ));
    }
    if checkpoint.last_successful_resolution == 0
        || checkpoint.last_successful_resolution > checkpoint.issued_at
    {
        return Err(Error::Time("resolution timestamp is invalid"));
    }
    if checkpoint.issued_at
        > now
            .checked_add(CLOCK_SKEW)
            .ok_or(Error::Time("clock overflow"))?
    {
        return Err(Error::Time(
            "checkpoint is from the future; check the clock",
        ));
    }
    if now >= checkpoint.expires_at {
        return Err(Error::Time("checkpoint has expired"));
    }
    let checkpoint_sha256 = hash(payload);
    if let Some(previous) = previous {
        previous.validate()?;
        if previous.scope != *scope {
            return Err(Error::ScopeMismatch);
        }
        if checkpoint.generation < previous.generation {
            return Err(Error::Replay("older checkpoint generation"));
        }
        if checkpoint.generation == previous.generation
            && checkpoint_sha256 != previous.checkpoint_sha256
        {
            return Err(Error::Replay("same generation with changed contents"));
        }
        if release.release.sequence < previous.highest_release_sequence {
            return Err(Error::Replay("older release sequence"));
        }
        if release.release.sequence == previous.highest_release_sequence
            && release.sha256 != previous.highest_release_sha256
        {
            return Err(Error::Replay("same release sequence with changed contents"));
        }
        if checkpoint.last_successful_resolution < previous.last_successful_resolution {
            return Err(Error::Replay("resolution history moved backwards"));
        }
    }
    let next_trust_state = TrustState {
        schema_version: PROTOCOL,
        scope: scope.clone(),
        generation: checkpoint.generation,
        checkpoint_sha256,
        highest_release_sequence: release.release.sequence,
        highest_release_sha256: release.sha256.clone(),
        last_successful_resolution: checkpoint.last_successful_resolution,
    };
    Ok(VerifiedUpdate {
        release,
        next_trust_state,
    })
}

/// Hash a local installer as an ordinary user; compare both size and exact bytes.
pub fn verify_artifact(release: &VerifiedRelease, mut file: impl Read) -> Result<(), Error> {
    let expected = &release.release.installer;
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        total += length as u64;
        if total > expected.size_bytes {
            return Err(Error::ArtifactMismatch);
        }
        digest.update(&buffer[..length]);
    }
    let actual: String = digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if total != expected.size_bytes || actual != expected.sha256 {
        return Err(Error::ArtifactMismatch);
    }
    Ok(())
}
