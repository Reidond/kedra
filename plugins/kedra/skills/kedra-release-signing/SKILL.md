---
name: kedra-release-signing
description: Maintain signed OCI images, exact release records, freshness, promotion, replay and recovery.
---

# Release signing

Read docs/ARCHITECTURE.md, docs/RELEASES.md, docs/UPDATES.md and build/release/README.md. A signed candidate is not a promoted release; a staged digest is not booted or healthy.

Sign final registry manifest bytes/digest. Release protocol 1 uses P-256/SHA-256, SPKI PEM keys and detached base64 DER signatures over exact JSON bytes. Bind source/run/attempt, target/architecture/repository, image/home/ISO identities and monotonic release sequence/checkpoint generation. Never reserialize signed bytes.

The installed helper independently verifies fixed image trust, scope, freshness and replay floors; user-provided verification is not authority. bootc switch must preserve strict container policy and exact repository. Native registry/storage copy can rewrite compressed manifests; preserve digest and verify complete copy paths before signing.

The protected signer executes no checkout or candidate code with production keys. Production authority is dedicated and separate from SSH; never use fixture keys in production. Candidate/publication jobs recheck current source and previous channel under serialization. Preserve immutable versions and inspected recovery after uncertain writes.

Fresh checkpoints have a maximum seven-day lifetime. Failed checks or blocked candidates cannot renew successful freshness. Historical-only release history may authenticate an expired predecessor's ordering; it never authorizes incoming enrollment/staging or claims current freshness. Rollback retains high-water and holds forward updates. Retain signed recovery assets; key rotation is separately qualified.

Use references/threat-matrix.md for adversarial E2E coverage. Current measured status belongs in docs/STATUS.md and exact Actions artifacts; keep this skill durable and checkout-local.
