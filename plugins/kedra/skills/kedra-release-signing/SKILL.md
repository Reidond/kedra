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

Authentication transfer cache (2026-09-26; Fedora 44 `skopeo-1.22.3-1.fc44` vendors containers/image v5.39.3). Every enroll, check and stage still runs `skopeo --policy /etc/containers/policy.json copy --preserve-digests --remove-signatures docker://REPO@DIGEST oci:/var/lib/sysroot/verified-oci:sha256-<hex>`. The policy check runs on the registry source before any layer is copied (`copy/single.go`, `IsRunningImageAllowed`), so the signature is verified on every run. `--remove-signatures` is required because the OCI layout refuses signatures (`oci/layout/oci_dest.go`: "Pushing signatures for OCI images is not supported").

`TryReusingBlobWithOptions` in `oci_dest.go` skips a layer whose blob path exists, without rehashing it. The copy rewrites the manifest and config from the registry. The helper hashes the manifest it reads back. `inspect --config` refuses a config that differs from the manifest. The helper never reads layers, and bootc pulls and verifies the digest itself, so a corrupt cached layer cannot make an image acceptable.

In a local Fedora 44 test against registry 3.1.1, a repeat copy downloaded only the config and signature blobs. Unsigned and wrong-key images were still rejected with `--remove-signatures`. A repeat on 2026-09-26 against registry 3.1.2 deleted the signature attachment of a digest the cache already held. The next copy failed with "Source image rejected: A signature was required, but no signature exists", fetched no blob, and left the cache unchanged. A cached digest is never trusted without its current registry signature. Code: `crates/sysroot-helper/management/{ghcr,oci_cache}.rs`. VM coverage: `tests/vm/ghcr`.

Changed inputs require a signed image before stable advances. No-change publishes nothing and does not renew checkpoints. The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md); legacy migration is not a publication or installation prerequisite. Retained compatibility verification must still refuse unresolved state and preserve hold, journals and high-water.

Retain known-good digests/signatures. Rollback holds forward updates and preserves persistent data. Key rotation needs actual qualification. Container signatures do not establish Secure Boot.

Local ISO construction consumes a reviewed signed image and verifies the embedded payload offline. It does not upload or create a release signature for local output. Keep this skill checkout-local.
