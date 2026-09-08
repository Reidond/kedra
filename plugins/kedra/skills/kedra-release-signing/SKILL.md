---
name: kedra-release-signing
description: Design or test signed Kedra OCI images, release and freshness checkpoints, promotion, exact identity, key rotation, negative signature tests and anti-replay behavior.
---

# Release identity, cryptography and authority

A signed candidate is not necessarily an approved release or the right machine's
image. A signed Git commit is not an OCI signature. Container signing is also
separate from UEFI Secure Boot, filesystem integrity and automatic recovery.
This development skill remains repository-only, not part of a system-wide profile.

## Required identities

Sign the final registry image digest. Bind release metadata to source revision,
workflow run/attempt, target, architecture, release sequence, image digest,
home-baseline/provenance digest, tool/base inputs and installer checksum.
The exact schema/signature encoding remains research; avoid signing ambiguously
serialized arbitrary JSON. Verify exact bytes or a documented canonical encoding.

Use a dedicated OS-release key, unrelated to the owner's Bitwarden SSH key.
Embed public trust material only. Protect the private key and recovery copy.
Research Podman's native Sigstore signing and containers/image policy/discovery
together; do not assume an arbitrary Cosign artifact layout is consumed by the
installed bootc stack. Signature transport through installer/copy steps matters.
A separate `cosign verify` success is not acceptance evidence for bootc itself.

## Enforcement and promotion

The root-side path checks key, repository identity, target, architecture,
compatibility and approved release status independently of the unprivileged
caller. It must reject a claimed preverified flag. System signature policy and
signature attachment discovery are separate configuration inputs. A strict root
container policy may also affect developer container pulls; solve those in a
separate rootless-user policy, not by weakening OS verification.

Discover releases by a channel, but deploy by exact digest from eligible signed
metadata. Serialize promotion per target and reject slower old workflow/rerun
regressions. Explicit rollback authorization is different from accepting an old
manifest as a forward update. Offline/stale installations need a designed replay
and key-transition policy, not a naive timestamp comparison.

## Freshness without redundant OS releases

Read docs/UPDATES.md section 5 and ADR 0002. The proposed protocol separates an
immutable approved release record from a signed per-target/Fedora-major/architecture
channel checkpoint. A successful no-change Fedora reconciliation may renew a
checkpoint pointing to the same image and ISO. Do not force dummy image releases
or Git commits to prove the schedule ran. A failed resolution cannot renew a
last-successful-check value; a new but blocked candidate cannot advance the
approved image. Report resolution, build/test and promotion status separately.

Bind scope, generation, issued/expiry times and release-record hash/digest. Retain
a root-owned checkpoint high-water mark, reject same-generation changed payloads,
and do not reset it on rollback. Routine stage requires fresh trusted metadata;
local offline boot/explicit retained verified rollback remains possible. Treat
clock errors and partial/mismatched channel publication as explicit failures.
The seven-day expiry is proposed policy to validate, not an implemented guarantee.
Never use one unscoped GitHub latest-release result as authority for every host.

A writer lock does not alone order workflows: recheck current desired image inputs
and sequence under the promotion lock. Workflow recreation and rerun identities
need explicit epoch/rank handling. Exact metadata encoding and atomic publication
remain R01/R08/R10 research, not a home-grown crypto shortcut. See the supplemental
update-refresh experiment cases for no-change, failed-check, replay and race tests.

## Prove before production

Measured 2026-09-08: Fedora bootc 1.16.10 with Skopeo 1.22.2 passed the local TLS
registry/VM experiment (Actions 34167524353). The pinned builder imports a local
containers-storage image ID: it needs a strict sigstoreSigned storage rule with
the trusted key and exactRepository, in addition to the docker scope. Default
reject is retained. `/usr/lib/bootc/install/*.toml` with `[install]
`enforce-container-sigpolicy = true` preserves inherited enforcement on initial
install and rollback. Without it, initial/rolled-back A lacked the signature
setting. See R01-signatures/REPORT.md; production authority/rotation remain gated.

Read references/threat-matrix.md. Use disposable keys and VM images A/B. Exercise
unsigned, wrong key/repository/target/architecture, tampered metadata, absent
attachments, unapproved candidate, stale replay, and race cases. Check running
and staged digests and exact return codes. Do not set verification to permissive
when something fails. Establish rotation with overlap, an offline client, retained
older releases and recovery credentials. GC must preserve needed signed digests.

Gates: R01 proves compatibility; R08 proves authority/lifecycle; R02 proves the
installer trust handoff; R10 implements independent validation. Sources:
docs/SOURCES.md policy, registries, podman-sign, blob-sign, bootc-switch,
actions-security; docs/UPDATES.md. The prepared release.yml candidate workflow is
manual and enabled; its first owner-approved signature passes but the resulting
ISO fails compressed layer identity (run 34250485539). Its exact public-input and
environment requirements are in build/release/authority/README.md. No public key
file or fingerprint from a research fixture may fill the production slots.

Prepared 2026-09-08 (ADR 0023): promote.yml binds real exact-media qualification
to current-main candidate/source and the dedicated public authority. The protected
Cosign 3.1.3 job executes no checkout/artifacts; a key-free publisher repeats native
verification, publishes complete versioned assets, then one signed-pair channel
bundle. Its runtime refuses stale/changed channel state and existing version tags.
Syntax is checked; actual production publication/races/recovery are not-run.
Owner authority/environment are provisioned separately (R08); the owner confirmed
Bitwarden backup/retrieval. Exact-media promotion,
renewal/expired recovery and rotation remain open. Do not confuse preparation with
qualification. Source: build/release/README.md and the exact R08/worklog evidence.

Measured 2026-09-08: ordinary Skopeo 1.13.3 registry publication compressed the
native OCI layers. Strict anonymous pull passed, but image-builder v82's later
storage-by-ID to digest-qualified media storage copy refused the signed manifest
rewrite. Preserve native digest on the first push and qualify the complete
registry/storage/isolated-storage path before asking for production signatures.
Do not strip signatures, remove digest binding or weaken policy to repair this.
The corrected signed registry/strict-pull/isolated-storage round trip passes
34253906774 at 7e846f0 with Skopeo 1.13.3 and Podman 4.9.3. Replacement production
GHCR publication and complete ISO construction still need exact-run evidence.
Source: R08-release-protocol/owner-candidate-20260908.md; gates R01/R02/R08.

Prepared 2026-09-09 for a later accepted source: candidate schema 2 binds actual
resolved packages, explicit Kedra provenance and each ISO part hash. The producer
prepares a fixed ASCII SHA256SUMS payload; the no-checkout signer independently
derives its complete file/hash inventory before signing. Key-free publication
verifies that signature, all local assets and downloaded draft bytes. Release
and checkpoint stay protocol version 1. Earlier candidate schema 1 lacks the
new evidence and is deliberately not backfilled. Existing c660c58 media must use
its original workflow. Syntax, independent review and native disposable checksum/
signature interoperability pass; actual v2 producer/publisher qualification is
not-run. Sources: build/release/README.md and WL-20260909-02; gate R08.
