//! Installed release scope and P-256 release-key identity.
//!
//! No subprocess, network, privilege or persistent-state writes. A caller-supplied
//! key/policy is advisory; privileged callers must load independently trusted state.
use std::fmt;

use p256::ecdsa::VerifyingKey;
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::targets;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    SizeLimit,
    InvalidKey,
    InvalidDocument(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "release I/O: {e}"),
            Self::SizeLimit => write!(f, "release input exceeds its size limit"),
            Self::InvalidKey => write!(f, "expected a trusted P-256 public key in SPKI PEM format"),
            Self::InvalidDocument(e) => write!(f, "invalid release document: {e}"),
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
    /// Accept only enabled table pairs. The repository is checked for shape here;
    /// signed-image identity and registry management also require the production one.
    pub fn validate(&self) -> Result<(), Error> {
        if targets::enabled(&self.target, &self.architecture).is_none()
            || self.fedora_release != 44
            || !repository(&self.repository)
        {
            return Err(invalid("unsupported or malformed target scope"));
        }
        Ok(())
    }
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
