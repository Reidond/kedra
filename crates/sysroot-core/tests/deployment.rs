use serde_json::Value;
use sysroot_core::deployment::*;
use sysroot_core::release::{Scope, TrustState};

const NATIVE: &[u8] = include_bytes!("fixtures/bootc-1.16.10-staged.json");
fn scope() -> Scope {
    Scope {
        target: "desktop".into(),
        architecture: "x86_64".into(),
        fedora_release: 44,
        repository: "registry.kedra.test:5000/kedra/r01".into(),
    }
}
fn digest(letter: char) -> String {
    format!("sha256:{}", letter.to_string().repeat(64))
}
fn fixture() -> Host {
    observe(NATIVE, &scope()).unwrap()
}
fn changed(change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(NATIVE).unwrap();
    change(&mut value);
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn reads_exact_native_bootc_evidence_without_conflating_slots() {
    let host = fixture();
    assert_eq!(
        host.booted.digest,
        "sha256:fd6a5b8dfb30d18ec3708898731f21dc63aeeb71f5be695c885a815afa5b74c4"
    );
    assert_eq!(
        host.staged.as_ref().unwrap().digest,
        "sha256:03923abb3f49590293df0107bf723b37fb9efbf657ead792d745239de052e9d3"
    );
    assert!(host.rollback.is_none());
    assert!(!host.rollback_queued);
}

#[test]
fn every_native_image_requires_enforcement_and_exact_scope() {
    for pointer in [
        "/spec/image/signature",
        "/status/booted/image/image/signature",
        "/status/staged/image/image/signature",
    ] {
        assert_eq!(
            observe(
                &changed(|v| *v.pointer_mut(pointer).unwrap() = Value::Null),
                &scope()
            ),
            Err(Error::UnverifiedImage)
        );
    }
    for pointer in [
        "/spec/image/image",
        "/status/booted/image/image/image",
        "/status/staged/image/image/image",
    ] {
        assert_eq!(
            observe(
                &changed(|v| *v.pointer_mut(pointer).unwrap() =
                    Value::String(format!("other.invalid/os@{}", digest('a')))),
                &scope()
            ),
            Err(Error::ScopeMismatch)
        );
    }
    let architecture = changed(|v| v["status"]["staged"]["image"]["architecture"] = "arm64".into());
    assert_eq!(observe(&architecture, &scope()), Err(Error::ScopeMismatch));
    let mismatch = changed(|v| v["status"]["booted"]["image"]["imageDigest"] = digest('a').into());
    assert_eq!(observe(&mismatch, &scope()), Err(Error::ScopeMismatch));
}

#[test]
fn rejects_unknown_api_missing_duplicate_and_incompatible_native_state() {
    assert_eq!(
        observe(&changed(|v| v["apiVersion"] = "future/v2".into()), &scope()),
        Err(Error::InvalidHost)
    );
    assert_eq!(
        observe(
            &changed(|v| {
                v["status"].as_object_mut().unwrap().remove("readOnly");
            }),
            &scope()
        ),
        Err(Error::InvalidHost)
    );
    let duplicate = String::from_utf8(changed(|_| {}))
        .unwrap()
        .replace("\"readOnly\":false", "\"readOnly\":true,\"readOnly\":false");
    assert_eq!(
        observe(duplicate.as_bytes(), &scope()),
        Err(Error::InvalidHost)
    );
    assert_eq!(
        observe(
            &changed(|v| v["status"]["staged"]["incompatible"] = true.into()),
            &scope()
        ),
        Err(Error::ScopeMismatch)
    );
    assert_eq!(
        observe(
            &changed(|v| v["status"]["readOnly"] = true.into()),
            &scope()
        ),
        Err(Error::ReadOnly)
    );
    assert_eq!(
        observe(
            &changed(|v| v["status"]["usrOverlay"] = serde_json::json!({"active":true})),
            &scope()
        ),
        Err(Error::Overlay)
    );
}

#[test]
fn manual_staging_requires_the_exact_replacement_receipt() {
    let host = fixture();
    let target = digest('a');
    assert_eq!(
        stage_action(&host, &target, None, false, false),
        Err(Error::PendingDeployment)
    );
    assert_eq!(
        stage_action(&host, &target, Some(&digest('b')), false, false),
        Err(Error::PendingDeployment)
    );
    assert_eq!(
        stage_action(
            &host,
            &target,
            Some(&host.staged.as_ref().unwrap().digest),
            false,
            false
        ),
        Ok(StageAction::Switch)
    );
    assert_eq!(
        stage_action(
            &host,
            &host.staged.as_ref().unwrap().digest,
            None,
            false,
            false
        ),
        Ok(StageAction::AlreadyStaged)
    );
}

#[test]
fn no_change_download_only_and_absent_pending_are_distinct() {
    let mut host = fixture();
    host.staged.as_mut().unwrap().download_only = true;
    assert_eq!(
        stage_action(
            &host,
            &host.staged.as_ref().unwrap().digest,
            None,
            false,
            false
        ),
        Ok(StageAction::Switch)
    );
    host.staged = None;
    assert_eq!(
        stage_action(&host, &host.booted.digest, None, false, false),
        Ok(StageAction::AlreadyBooted)
    );
    assert_eq!(
        stage_action(&host, &digest('a'), Some(&digest('b')), false, false),
        Err(Error::PendingDeployment)
    );
}

#[test]
fn rollback_queue_and_persistent_hold_stop_forward_staging() {
    let mut host = fixture();
    host.staged = None;
    assert_eq!(
        stage_action(&host, &digest('a'), None, true, false),
        Err(Error::RollbackHold)
    );
    assert_eq!(
        stage_action(&host, &digest('a'), None, true, true),
        Ok(StageAction::Switch)
    );
    host.rollback_queued = true;
    assert_eq!(
        stage_action(&host, &digest('a'), None, false, true),
        Err(Error::RollbackQueued)
    );
}

#[test]
fn interrupted_intent_reconciles_only_to_observed_native_effects() {
    let mut host = fixture();
    let target = host.staged.take().unwrap();
    let mut operation =
        intent(&host, OperationKind::Stage, &target.digest, &"a".repeat(64)).unwrap();
    assert_eq!(operation.phase, Phase::Intent);
    operation.observe(&host);
    assert_eq!(operation.phase, Phase::NotApplied);
    host.staged = Some(Slot {
        download_only: true,
        ..target.clone()
    });
    operation.observe(&host);
    assert_eq!(operation.phase, Phase::NotApplied);
    host.staged = Some(target.clone());
    operation.observe(&host);
    assert_eq!(operation.phase, Phase::AwaitingReboot);
    host.booted = target;
    host.staged = None;
    operation.observe(&host);
    assert_eq!(operation.phase, Phase::Booted);
    host.staged = Some(Slot {
        digest: digest('c'),
        download_only: false,
    });
    operation.observe(&host);
    assert_eq!(operation.phase, Phase::NotApplied);
}

#[test]
fn journal_rejects_cross_machine_scope_and_unknown_version() {
    let machine = "a".repeat(64);
    let mut journal = Journal {
        schema_version: 1,
        machine: machine.clone(),
        scope: scope(),
        high_water: TrustState {
            schema_version: 1,
            scope: scope(),
            generation: 3,
            checkpoint_sha256: "b".repeat(64),
            highest_release_sequence: 8,
            highest_release_sha256: "c".repeat(64),
            last_successful_resolution: 100,
        },
        rollback_hold: true,
        operation: None,
    };
    journal.validate(&machine, &scope()).unwrap();
    assert_eq!(
        journal.validate(&"d".repeat(64), &scope()),
        Err(Error::InvalidState)
    );
    let mut other = scope();
    other.target = "other".into();
    assert_eq!(journal.validate(&machine, &other), Err(Error::InvalidState));
    journal.schema_version = 2;
    assert_eq!(
        journal.validate(&machine, &scope()),
        Err(Error::InvalidState)
    );
    journal.schema_version = 1;
    journal.high_water.generation = 0;
    assert_eq!(
        journal.validate(&machine, &scope()),
        Err(Error::InvalidState)
    );
}
