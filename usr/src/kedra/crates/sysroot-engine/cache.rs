//! Producer authorization for ordinary-user closure substitution.
//!
//! The signed bytes do not authorize installed OS changes. Transport and signing
//! stay with the caller; admission must still verify the complete bundle.

use crate::{Error, MAX_JSON, ObjectReceipt, ResolvedBuildSpec, Result, plan};
use base64::{Engine, engine::general_purpose::STANDARD};
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const CACHE_RECEIPT_PURPOSE: &str = "sysroot-engine-binary-cache-v1";
const MAX_BUNDLE: u64 = 16 * 1024 * 1024 * 1024;

/// Independently selected consumer policy. Keys are SHA-256 SPKI fingerprints.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CachePolicy {
    pub schema: u32,
    pub scope: String,
    pub revision: u64,
    pub valid_from: u64,
    pub valid_until: u64,
    pub trusted_keys: Vec<String>,
}

/// Serialize these bytes and sign externally with a dedicated cache key.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheReceipt {
    pub schema: u32,
    pub purpose: String,
    pub scope: String,
    pub policy_revision: u64,
    pub valid_from: u64,
    pub valid_until: u64,
    pub bundle_sha256: String,
    pub bundle_bytes: u64,
    pub root: ObjectReceipt,
}

/// Authenticated metadata, never constructible from deserialized caller data.
#[derive(Debug)]
pub struct VerifiedCacheReceipt {
    receipt: CacheReceipt,
    fingerprint: String,
    policy_valid_from: u64,
    policy_valid_until: u64,
}

impl VerifiedCacheReceipt {
    pub fn receipt(&self) -> &CacheReceipt {
        &self.receipt
    }

    pub fn signer_fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Call after complete private staging/validation and BEFORE creating an
    /// import journal, admitting any object/image, pinning, or selecting a root.
    pub(crate) fn validate_bundle(
        &self,
        roots: &[String],
        root: &ObjectReceipt,
        sha256: &str,
        bytes: u64,
        now: u64,
    ) -> Result<()> {
        valid_at(self.policy_valid_from, self.policy_valid_until, now)?;
        valid_at(self.receipt.valid_from, self.receipt.valid_until, now)?;
        if roots.len() != 1
            || roots[0] != self.receipt.root.object
            || sha256 != self.receipt.bundle_sha256
            || bytes != self.receipt.bundle_bytes
            || serde_json::to_vec(root)? != serde_json::to_vec(&self.receipt.root)?
        {
            return Err(invalid("staged closure differs from authorized receipt"));
        }
        Ok(())
    }
}

/// Authenticate exact signed bytes against current independently supplied policy
/// and the consumer's resolved recipe. Neither policy nor key comes from a bundle.
pub fn verify_cache_receipt(
    payload: &[u8],
    signature: &[u8],
    public_key: &str,
    policy: &CachePolicy,
    expected: &ResolvedBuildSpec,
    now: u64,
) -> Result<VerifiedCacheReceipt> {
    if payload.is_empty() || payload.len() as u64 > MAX_JSON {
        return Err(invalid("receipt exceeds the bounded document size"));
    }
    validate_policy(policy, now)?;
    let fingerprint = verify_signature(payload, signature, public_key)?;
    if !policy.trusted_keys.contains(&fingerprint) {
        return Err(invalid("producer key is not authorized by consumer policy"));
    }
    let receipt: CacheReceipt = serde_json::from_slice(payload)?;
    valid_at(receipt.valid_from, receipt.valid_until, now)?;
    if receipt.schema != 1
        || receipt.purpose != CACHE_RECEIPT_PURPOSE
        || receipt.scope != policy.scope
        || receipt.policy_revision != policy.revision
        || receipt.valid_from < policy.valid_from
        || receipt.valid_until > policy.valid_until
        || !plan::hex(&receipt.bundle_sha256)
        || receipt.bundle_bytes == 0
        || receipt.bundle_bytes > MAX_BUNDLE
    {
        return Err(invalid(
            "receipt scope, policy, validity or archive limit differs",
        ));
    }
    let root = &receipt.root;
    let actual = root
        .derivation
        .as_ref()
        .ok_or_else(|| invalid("cache root has no resolved recipe"))?;
    let expected_bytes = serde_json::to_vec(expected)?;
    if root.schema != 1
        || !plan::hex(&root.tree_sha256)
        || root.references.len() > 256
        || expected_bytes.len() as u64 > MAX_JSON
        || root.object != plan::derivation_id(expected)?
        || root.runtime_image.as_deref() != Some(expected.runtime_image.as_str())
        || serde_json::to_vec(actual)? != expected_bytes
    {
        return Err(invalid(
            "receipt root differs from the independently resolved recipe",
        ));
    }
    let mut references = BTreeSet::new();
    for reference in &root.references {
        plan::object_id(reference)?;
        if reference == &root.object || !references.insert(reference) {
            return Err(invalid("receipt has duplicate or self references"));
        }
    }
    Ok(VerifiedCacheReceipt {
        receipt,
        fingerprint,
        policy_valid_from: policy.valid_from,
        policy_valid_until: policy.valid_until,
    })
}

fn validate_policy(policy: &CachePolicy, now: u64) -> Result<()> {
    if policy.schema != 1
        || policy.revision == 0
        || policy.scope.is_empty()
        || policy.scope.len() > 256
        || !policy
            .scope
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
        || policy.trusted_keys.is_empty()
        || policy.trusted_keys.len() > 64
        || policy.trusted_keys.iter().any(|key| !plan::hex(key))
        || policy.trusted_keys.iter().collect::<BTreeSet<_>>().len() != policy.trusted_keys.len()
    {
        return Err(invalid("unsupported consumer cache policy"));
    }
    valid_at(policy.valid_from, policy.valid_until, now)
}

// Match the existing interoperable P-256/SPKI and base64-DER encodings while
// keeping this reusable engine independent of OS release models and authority.
fn verify_signature(payload: &[u8], signature: &[u8], public_key: &str) -> Result<String> {
    if signature.len() > 1024 || public_key.len() > 4096 {
        return Err(invalid("signature or public key exceeds its size limit"));
    }
    let key = VerifyingKey::from_public_key_pem(public_key)
        .map_err(|_| invalid("expected a P-256 SPKI public key"))?;
    let encoded =
        std::str::from_utf8(signature).map_err(|_| invalid("signature is not base64 text"))?;
    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|_| invalid("signature is not base64 text"))?;
    let signature =
        Signature::from_der(&bytes).map_err(|_| invalid("signature is not P-256 DER"))?;
    key.verify(payload, &signature)
        .map_err(|_| invalid("signature verification failed"))?;
    let der = key
        .to_public_key_der()
        .map_err(|_| invalid("public key SPKI encoding failed"))?;
    Ok(plan::hash(der.as_bytes()))
}

fn valid_at(from: u64, until: u64, now: u64) -> Result<()> {
    if from >= until || now < from || now >= until {
        return Err(invalid("cache authorization is not currently valid"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(format!("binary cache: {}", message.into()))
}
