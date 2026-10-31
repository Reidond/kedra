---
name: kedra-release-signing
description: Maintain automatically signed GHCR image identity, isolated publication, direct updates and retained recovery.
---

# Signed OCI authority

Read usr/src/kedra/docs/RELEASES.md, usr/src/kedra/docs/UPDATES.md and usr/src/kedra/docs/ARCHITECTURE.md. The published artifacts are independently signed OCI images per enabled target. Stable is discovery; deployment uses the exact verified digest. The only GitHub Releases are owner-dispatched installer ISO releases (`iso.yml`); no release metadata or signature assets.

| Target | Repository | Builds repository | Environment | Secret/variable prefix | Authority |
|---|---|---|---|---|---|
| desktop (x86_64) | ghcr.io/reidond/kedra-desktop | ghcr.io/reidond/kedra-desktop-builds | kedra-desktop-signing | KEDRA_DESKTOP | usr/src/kedra/image/release/authority/desktop.pub, .sha256 |
| qemu-arm64 (aarch64) | ghcr.io/reidond/kedra-qemu-arm64 | ghcr.io/reidond/kedra-qemu-arm64-builds | kedra-qemu-arm64-signing | KEDRA_QEMU_ARM64 | usr/src/kedra/image/release/authority/qemu-arm64.pub, .sha256 |

The closed table is usr/src/kedra/image/release/targets.json. The owner retired `utm` on 2026-09-28; the replacement `qemu-arm64` authority has fingerprint `80551368…1a515` and its own main-only environment. Never relabel an old signed image: target, repository and source are part of its signed identity. Ranks are per repository. `sysroot release key` only fingerprints a public key; it establishes no trust.

Use the dedicated OS-image authority, separate from SSH keys. The isolated automatic signer runs no checkout, candidate or repository code while private keys are available. There is no human-review/signing gate; main-only environment restrictions, exact current-main/rank checks and strict signature verification remain. Preserve signatures and native OCI digest through copies. The binary cache (`kedra-cache-<target>`) adds no signing authority: a closure is admitted only if every object equals a receipt in a signature-verified image's composition.json (docs/ENGINE.md "Binary cache").

Image-owned identity/resolved-input records are bound by the signed image. bootc verifies the signature when the helper stages an exact digest with `bootc switch --enforce-container-sigpolicy`; the helper checks that the installed policy is exactly the one the booted image's trust implies, the signed identity's target/architecture/channel, and that the candidate is not older than the booted image. Writable source or --verified claims are never root authority. Wrong key/repository/digest or missing attachment fails closed.

Signature enforcement facts (2026-10-15; Fedora 44 `bootc-1.16.13`, `skopeo-1.22.3`, `ostree-2026.4`, registry 3.1.2; bootc's own importer through `ostree container image pull`, the code path `bootc switch` uses):
- The proxy evaluates `containers-policy.json` when it opens the image: unsigned ("A signature was required, but no signature exists"), wrong key ("cryptographic signature verification failed") and wrong signed repository ("Signature for identity … is not accepted") are refused after fetching only the manifest and signature attachment, never the configuration or a layer. The policy applies with the unverified transport too.
- containers/image prefers `$HOME/.config/containers/policy.json` to `/etc/containers/policy.json`, and `--enforce-container-sigpolicy` checks only the default entry of whichever file it found: a per-user policy with `default: reject` and `docker: {"": insecureAcceptAnything}` admitted an unsigned image. The helper therefore runs bootc and Skopeo with a cleared environment, `HOME=/var/lib/sysroot` and refuses if `/var/lib/sysroot/.config` exists.
- bootc orders nothing: it deploys any validly signed digest. Downgrade refusal is the helper's rank/resolution check against `/usr/share/sysroot/image-identity.json` of the booted image, on the candidate's label: the helper hashes stable's manifest into the digest it stages and the raw configuration (`skopeo inspect --raw --config`) against that manifest's configuration digest. `check` does not verify signatures. bootc downloads each new layer once.
Code: `crates/sysroot-helper/management/ghcr.rs`. VM coverage: `usr/src/kedra/tests/vm/ghcr-update` (`run.sh` on x86_64 KVM, `run-hvf.py` on Apple Silicon); references/threat-matrix.md lists what it covers.

Changed inputs require a signed image before stable advances. No-change publishes nothing and does not renew checkpoints. Older or corrupt helper state is refused, never migrated, and the rollback hold is preserved.

Retain known-good digests/signatures. Rollback holds forward updates and preserves persistent data. Key rotation needs actual qualification. Container signatures do not establish Secure Boot.

ISO construction consumes an exact signed image and verifies the embedded payload offline. Locally it uploads nothing. The owner-dispatched `iso.yml` builds the same media from a target's current `stable` digest and attaches its parts, PARTS.SHA256SUMS, SHA256SUMS and installer.json to a GitHub Release; those checksums are not release signatures, and the workflow holds no signing key.
