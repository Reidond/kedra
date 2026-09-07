---
name: kedra-release-signing
description: Design or test signed Kedra OCI images, manifest verification, exact release identity, promotion, key rotation, negative signature tests and anti-replay behavior.
---

# Release identity, cryptography and authority

A signed candidate is not necessarily an approved release or the right machine's
image. A signed Git commit is not an OCI signature. Container signing is also
separate from UEFI Secure Boot, filesystem integrity and automatic recovery.

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

## Prove before production

Read references/threat-matrix.md. Use disposable keys and VM images A/B. Exercise
unsigned, wrong key/repository/target/architecture, tampered metadata, absent
attachments, unapproved candidate, stale replay, and race cases. Check running
and staged digests and exact return codes. Do not set verification to permissive
when something fails. Establish rotation with overlap, an offline client, retained
older releases and recovery credentials. GC must preserve needed signed digests.

Gates: R01 proves compatibility; R08 proves authority/lifecycle; R02 proves the
installer trust handoff; R10 implements independent validation. Sources:
docs/SOURCES.md policy, registries, podman-sign, blob-sign, bootc-switch,
actions-security. No production signing workflow exists in bootstrap.
