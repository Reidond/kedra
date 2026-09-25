---
name: kedra-release-signing
description: Maintain automatically signed GHCR image identity, isolated publication, direct updates and retained recovery.
---

# Signed OCI authority

Read docs/RELEASES.md, docs/UPDATES.md and docs/ARCHITECTURE.md. The published artifacts are independently signed OCI images per enabled target. Stable is discovery; deployment uses the exact verified digest. No GitHub Releases or ISO/metadata assets.

| Target | Repository | Builds repository | Environment | Secret/variable prefix | Authority |
|---|---|---|---|---|---|
| desktop (x86_64) | ghcr.io/reidond/kedra-desktop | ghcr.io/reidond/kedra-desktop-builds | kedra-desktop-signing | KEDRA_DESKTOP | build/release/authority/desktop.pub, .sha256 |
| utm (aarch64) | ghcr.io/reidond/kedra-utm | ghcr.io/reidond/kedra-utm-builds | kedra-utm-signing | KEDRA_UTM | build/release/authority/utm.pub, .sha256 |

Recorded 2026-09-25. The utm key was generated offline for the owner (encrypted P-256, fingerprint `76ca7a65…66c6`); its main-only environment and public, repository-linked packages exist. See docs/STATUS.md. Ranks and high-water marks are per repository. A key never signs another target's repository. Legacy protocol-1 release files stay desktop/x86_64 only.

Use the dedicated OS-image authority, separate from SSH keys. The isolated automatic signer runs no checkout, candidate or repository code while private keys are available. There is no human-review/signing gate; main-only environment restrictions, exact current-main/rank checks and strict signature verification remain. Preserve signatures and native OCI digest through copies.

Image-owned identity/resolved-input records are bound by the signed image. The helper independently verifies fixed public trust, exact repository, target/architecture and retained ordering. Writable source or --verified claims are never root authority. Wrong key/repository/digest or missing attachment fails closed.

Changed inputs require a signed image before stable advances. No-change publishes nothing and does not renew checkpoints. The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md); legacy migration is not a publication or installation prerequisite. Retained compatibility verification must still refuse unresolved state and preserve hold, journals and high-water.

Retain known-good digests/signatures. Rollback holds forward updates and preserves persistent data. Key rotation needs actual qualification. Container signatures do not establish Secure Boot.

Local ISO construction consumes a reviewed signed image and verifies the embedded payload offline. It does not upload or create a release signature for local output. Keep this skill checkout-local.
