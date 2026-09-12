---
name: kedra-release-signing
description: Maintain signed GHCR image identity, protected manual publication, direct updates and retained recovery.
---

# Signed OCI authority

Read docs/RELEASES.md, docs/UPDATES.md and docs/ARCHITECTURE.md. The published artifact is a signed OCI image in ghcr.io/reidond/kedra-desktop. Stable is discovery; deployment uses the exact verified digest. No GitHub Releases or ISO/metadata assets.

Use the dedicated OS-image authority, separate from SSH keys. The isolated protected signer runs no checkout, candidate or repository code while private keys are available. Manual review and exact current-main checks remain. Preserve signatures and native OCI digest through copies.

Image-owned identity/resolved-input records are bound by the signed image. The helper independently verifies fixed public trust, exact repository, target/architecture and retained ordering. Writable source or --verified claims are never root authority. Wrong key/repository/digest or missing attachment fails closed.

Changed inputs require a signed image before stable advances. No-change publishes nothing and does not renew checkpoints. Legacy release/checkpoint verification remains solely for migration/offline recovery. Before bridging, legacy update status must reconcile earlier operations with no intent/awaiting reboot/staged replacement/queued rollback; preserve hold, journals and high-water. The v2 import refuses unresolved legacy state.

Retain known-good digests/signatures. Rollback holds forward updates and preserves persistent data. Key rotation and older-reader migration need actual qualification. Container signatures do not establish Secure Boot.

Local ISO construction consumes a reviewed signed image and verifies the embedded payload offline. It does not upload or create a release signature for local output. Keep this skill checkout-local.
