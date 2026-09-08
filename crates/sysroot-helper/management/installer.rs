//! Read-only-to-disks media precheck. Native storage may add a deduplicated alias.
use super::{Result, Trust, hash, trusted_file};
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Stdio};

const INPUT: &str = "/usr/lib/sysroot/trust/installer-payload.json";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    schema_version: u32,
    image: String,
    source_manifest_sha256: String,
}
fn validate(payload: &Payload, trust: &Trust) -> Result<()> {
    let prefix = format!("{}@sha256:", trust.scope.repository);
    let digest = payload
        .image
        .strip_prefix(&prefix)
        .ok_or("installer payload repository mismatch")?;
    if payload.schema_version != 1
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || payload.source_manifest_sha256 != trust.source_manifest_hash
    {
        return Err("installer payload identity or installed source binding is invalid".into());
    }
    Ok(())
}
pub(super) fn verify(trust: &Trust) -> Result<()> {
    let descriptor = trusted_file::read(Path::new(INPUT), 0, 16_384)?;
    let payload: Payload = serde_json::from_slice(&descriptor)
        .map_err(|_| "installer payload descriptor is malformed")?;
    validate(&payload, trust)?;
    let source = format!("containers-storage:{}", payload.image);
    // containers/image verifies the source signature even when the layer data
    // already exists. Reuse avoids copying the complete image into RAM scratch.
    let status = Command::new("/usr/bin/timeout")
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin")
        .env("HOME", "/root")
        .env("LANG", "C.UTF-8")
        .current_dir("/")
        .args([
            "--signal=TERM",
            "--kill-after=10s",
            "300s",
            "/usr/bin/skopeo",
            "--policy",
            "/etc/containers/policy.json",
            "copy",
            "--preserve-digests",
            &source,
            "containers-storage:localhost/kedra-installer:verified",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(
            "embedded payload signature verification failed; installation must not start".into(),
        );
    }
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"installer_payload_verified":true,
        "image":payload.image,"source_manifest_sha256":trust.source_manifest_hash,
        "key_fingerprint_sha256":sysroot_core::release::public_key_fingerprint(&trust.key)?,
        "descriptor_sha256":hash(&descriptor),"installation_disk_writes_performed":false})
    );
    Ok(())
}
