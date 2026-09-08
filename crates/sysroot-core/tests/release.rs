use base64::{Engine, engine::general_purpose::STANDARD};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::{EncodePublicKey, LineEnding};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sysroot_core::release::*;

const NOW: u64 = 1_788_800_000;
fn key(byte: u8) -> SigningKey {
    // Publicly reproducible test keys only. Never a production signing default.
    SigningKey::from_slice(&[byte; 32]).unwrap()
}
fn pem(key: &SigningKey) -> String {
    key.verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap()
}
fn signature(bytes: &[u8], key: &SigningKey) -> Vec<u8> {
    let signature: Signature = key.sign(bytes);
    STANDARD.encode(signature.to_der().as_bytes()).into_bytes()
}
fn signed<T: Serialize>(value: &T, key: &SigningKey) -> (Vec<u8>, Vec<u8>) {
    let bytes = serde_json::to_vec(value).unwrap();
    let sig = signature(&bytes, key);
    (bytes, sig)
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn fixture() -> Release {
    Release {
        schema_version: 1,
        project: "Kedra".into(),
        scope: Scope {
            target: "desktop".into(),
            architecture: "x86_64".into(),
            fedora_release: 44,
            repository: "ghcr.io/reidond/kedra-desktop".into(),
        },
        sequence: 42,
        source_revision: "a".repeat(40),
        build: Build {
            repository: "Reidond/kedra".into(),
            workflow: ".github/workflows/release.yml".into(),
            run_id: 123,
            run_attempt: 1,
        },
        image_digest: format!("sha256:{}", "b".repeat(64)),
        home_manifest_sha256: "c".repeat(64),
        installer: Artifact {
            filename: "kedra-desktop-44-42.iso".into(),
            size_bytes: 3,
            sha256: hash(b"abc"),
        },
        approval: Approval::Promoted,
        minimum_protocol: 1,
    }
}
fn verified(release: &Release, key: &SigningKey) -> VerifiedRelease {
    let (bytes, sig) = signed(release, key);
    verify_release(&bytes, &sig, &pem(key), Some(&release.scope)).unwrap()
}
fn checkpoint(release: &Release, generation: u64) -> Checkpoint {
    Checkpoint {
        schema_version: 1,
        project: "Kedra".into(),
        scope: release.scope.clone(),
        generation,
        release_sha256: hash(&serde_json::to_vec(release).unwrap()),
        issued_at: NOW,
        expires_at: NOW + 3600,
        last_successful_resolution: NOW,
    }
}
fn update(
    release: &Release,
    cp: &Checkpoint,
    state: Option<&TrustState>,
    now: u64,
) -> Result<VerifiedUpdate, Error> {
    let key = key(7);
    let (bytes, sig) = signed(cp, &key);
    verify_update(
        verified(release, &key),
        &bytes,
        &sig,
        &pem(&key),
        &release.scope,
        state,
        now,
    )
}

#[test]
fn signatures_bind_exact_bytes_and_expected_key() {
    let release = fixture();
    let (bytes, sig) = signed(&release, &key(7));
    let result = verify_release(&bytes, &sig, &pem(&key(7)), Some(&release.scope)).unwrap();
    assert_eq!(
        result.release().image_reference(),
        format!("ghcr.io/reidond/kedra-desktop@sha256:{}", "b".repeat(64))
    );
    let mut tampered = bytes.clone();
    tampered.push(b' ');
    assert!(matches!(
        verify_release(&tampered, &sig, &pem(&key(7)), None),
        Err(Error::InvalidSignature)
    ));
    assert!(matches!(
        verify_release(&bytes, &sig, &pem(&key(8)), None),
        Err(Error::InvalidSignature)
    ));
    assert!(matches!(
        verify_release(&bytes, b"not base64!", &pem(&key(7)), None),
        Err(Error::InvalidSignature)
    ));
    assert!(matches!(
        verify_release(&bytes, &sig, "not a public key", None),
        Err(Error::InvalidKey)
    ));
}

#[test]
fn even_signed_duplicate_unknown_fields_and_new_schemas_are_rejected() {
    let key = key(7);
    let bytes = serde_json::to_string(&fixture()).unwrap();
    for text in [
        bytes.replacen('{', "{\"schema_version\":1,", 1),
        bytes.replacen('{', "{\"caller_verified\":true,", 1),
    ] {
        assert!(matches!(
            verify_release(
                text.as_bytes(),
                &signature(text.as_bytes(), &key),
                &pem(&key),
                None
            ),
            Err(Error::InvalidDocument(_))
        ));
    }
    let mut release = fixture();
    release.schema_version = 2;
    let (bytes, sig) = signed(&release, &key);
    assert!(matches!(
        verify_release(&bytes, &sig, &pem(&key), None),
        Err(Error::UnsupportedSchema(2))
    ));
}

#[test]
fn valid_signature_cannot_promote_a_candidate_or_change_target() {
    let mut release = fixture();
    release.approval = Approval::Candidate;
    let (bytes, sig) = signed(&release, &key(7));
    assert!(matches!(
        verify_release(&bytes, &sig, &pem(&key(7)), None),
        Err(Error::Unpromoted)
    ));
    release.approval = Approval::Promoted;
    release.minimum_protocol = 2;
    let (bytes, sig) = signed(&release, &key(7));
    assert!(matches!(
        verify_release(&bytes, &sig, &pem(&key(7)), None),
        Err(Error::Incompatible)
    ));
    let release = fixture();
    let (bytes, sig) = signed(&release, &key(7));
    let mut other = release.scope.clone();
    other.target = "xps".into();
    assert!(matches!(
        verify_release(&bytes, &sig, &pem(&key(7)), Some(&other)),
        Err(Error::ScopeMismatch)
    ));
}

#[test]
fn malformed_references_and_artifact_paths_never_become_commands() {
    for repository in [
        "--help",
        "ghcr.io/reidond/kedra-desktop;reboot",
        "https://ghcr.io/a",
        "ghcr.io/a@sha256:abc",
        "ghcr.io/../a",
        "HOST/a",
        "ghcr.io:0/a",
    ] {
        let mut release = fixture();
        release.scope.repository = repository.into();
        assert!(release.validate().is_err(), "{repository}");
    }
    for filename in [
        "../../disk.iso",
        "kedra-xps-44-42.iso",
        "kedra-desktop-44-42.iso;reboot",
        "kedra-desktop-44-42.iso/else",
    ] {
        let mut release = fixture();
        release.installer.filename = filename.into();
        assert!(release.validate().is_err(), "{filename}");
    }
    let mut release = fixture();
    release.image_digest = "sha256:abc".into();
    assert!(release.validate().is_err());
}

#[test]
fn initial_repeat_and_no_change_freshness_preserve_release_identity() {
    let release = fixture();
    let cp = checkpoint(&release, 10);
    let state = update(&release, &cp, None, NOW).unwrap().next_trust_state;
    assert_eq!(
        update(&release, &cp, Some(&state), NOW)
            .unwrap()
            .next_trust_state,
        state
    );
    let mut next = cp;
    next.generation += 1;
    next.issued_at += 1800;
    next.expires_at += 1800;
    next.last_successful_resolution += 1800;
    let fresh = update(&release, &next, Some(&state), NOW + 1800).unwrap();
    assert_eq!(
        fresh.next_trust_state.highest_release_sha256,
        state.highest_release_sha256
    );
    assert_eq!(fresh.next_trust_state.highest_release_sequence, 42);
    assert_eq!(fresh.next_trust_state.generation, 11);
}

#[test]
fn old_and_equivocating_checkpoints_are_rejected() {
    let release = fixture();
    let cp = checkpoint(&release, 10);
    let state = update(&release, &cp, None, NOW).unwrap().next_trust_state;
    let mut old = cp.clone();
    old.generation = 9;
    assert!(matches!(
        update(&release, &old, Some(&state), NOW),
        Err(Error::Replay(_))
    ));
    let mut changed = cp;
    changed.expires_at += 1;
    assert!(matches!(
        update(&release, &changed, Some(&state), NOW),
        Err(Error::Replay(_))
    ));
}

#[test]
fn rollback_does_not_reset_forward_release_high_water_marks() {
    let release = fixture();
    let state = update(&release, &checkpoint(&release, 10), None, NOW)
        .unwrap()
        .next_trust_state;
    let mut old = release.clone();
    old.sequence = 41;
    // Retained old media can still be verified offline, but not replayed as an update.
    verified(&old, &key(7));
    assert!(matches!(
        update(&old, &checkpoint(&old, 11), Some(&state), NOW),
        Err(Error::Replay(_))
    ));
    let mut equivocation = release;
    equivocation.image_digest = format!("sha256:{}", "d".repeat(64));
    assert!(matches!(
        update(
            &equivocation,
            &checkpoint(&equivocation, 11),
            Some(&state),
            NOW
        ),
        Err(Error::Replay(_))
    ));
}

#[test]
fn expiry_clock_binding_and_corrupt_state_fail_closed() {
    let release = fixture();
    let cp = checkpoint(&release, 10);
    assert!(matches!(
        update(&release, &cp, None, cp.expires_at),
        Err(Error::Time(_))
    ));
    assert!(matches!(
        update(&release, &cp, None, NOW - 301),
        Err(Error::Time(_))
    ));
    let mut too_long = cp.clone();
    too_long.expires_at = NOW + 604801;
    assert!(matches!(
        update(&release, &too_long, None, NOW),
        Err(Error::Time(_))
    ));
    let mut wrong_release = cp.clone();
    wrong_release.release_sha256 = "0".repeat(64);
    assert!(matches!(
        update(&release, &wrong_release, None, NOW),
        Err(Error::InvalidDocument(_))
    ));
    let mut state = update(&release, &cp, None, NOW).unwrap().next_trust_state;
    state.schema_version = 2;
    assert!(matches!(
        update(&release, &cp, Some(&state), NOW),
        Err(Error::UnsupportedSchema(2))
    ));
    state.schema_version = 1;
    state.generation = 0;
    assert!(matches!(
        update(&release, &cp, Some(&state), NOW),
        Err(Error::InvalidDocument(_))
    ));
}

#[test]
fn credential_rotation_requires_the_explicit_verification_key() {
    let release = fixture();
    let cp = checkpoint(&release, 10);
    let (bytes, sig) = signed(&cp, &key(8));
    assert!(matches!(
        verify_update(
            verified(&release, &key(7)),
            &bytes,
            &sig,
            &pem(&key(7)),
            &release.scope,
            None,
            NOW
        ),
        Err(Error::InvalidSignature)
    ));
    // Caller explicitly trusts both keys during an overlap. No auto-key discovery.
    assert!(
        verify_update(
            verified(&release, &key(7)),
            &bytes,
            &sig,
            &pem(&key(8)),
            &release.scope,
            None,
            NOW
        )
        .is_ok()
    );
}

#[test]
fn installer_verification_checks_size_and_content() {
    let path = std::env::temp_dir().join(format!(
        "kedra-installer-fixture-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let release = verified(&fixture(), &key(7));
    std::fs::write(&path, b"abc").unwrap();
    verify_artifact(&release, std::fs::File::open(&path).unwrap()).unwrap();
    for bad in [b"abd".as_slice(), b"ab", b"abcd"] {
        std::fs::write(&path, bad).unwrap();
        assert!(matches!(
            verify_artifact(&release, std::fs::File::open(&path).unwrap()),
            Err(Error::ArtifactMismatch)
        ));
    }
    std::fs::remove_file(path).unwrap();
}
