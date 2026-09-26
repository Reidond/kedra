# Signed container images

The only published product artifacts are the signed OCI images of each enabled target: `ghcr.io/reidond/kedra-desktop` (x86_64) and `ghcr.io/reidond/kedra-utm` (aarch64, the Apple Silicon UTM virtual machine). Each repository has its own key, signing environment, `stable` tag and rank history. `stable` is discovery; verification and deployment use the immutable digest. A GitHub source commit, a tag name or a successful build alone is not image-signing authority.

The fixed public keys are `build/release/authority/<target>.pub`. Independently confirm their SPKI DER SHA-256:

```text
desktop  a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
utm      76ca7a65915adb1907acbe0885af83c5c569dd2964b87decbfb67059dee366c6
```

The installed root policy requires native Sigstore signatures from the installed target's key and its exact Kedra repository. A desktop system never accepts a `utm` image and vice versa: target, architecture, OCI platform and repository are all bound by the signed image identity. Attachment discovery and initial-install policy must stay enabled. Never add permissive defaults or disable signature checks to make a pull work.

On the installed current-source OS, `sysroot update check` inspects the fixed GHCR repository of its enrolled target through the installed helper; `sysroot update stage` independently verifies before mutation. Image-owned identity/resolved-input records are bound by the signed OCI content. Local writable files and caller-supplied claims cannot establish root trust.

Builds, isolated signing and verified stable publication run automatically in Actions after validation, natively per target (`ubuntu-24.04` for desktop, `ubuntu-24.04-arm` for utm). Each signing environment restricts deployment to main and has no human-review gate; no checkout, repository script or candidate code executes while production private keys are available. Build-time scope/rank/material checks and independent key/signature verification remain mandatory. No-change builds do not publish, sign metadata or renew checkpoints. See [release operations](../build/release/README.md) for the owner steps a new target needs before its first publication.

Create installation media locally using [INSTALL.md](INSTALL.md). Its hash records describe the local output; the installer verifies the signed embedded OS payload offline. Current code does not publish ISO parts, GitHub Releases, machine bundles or release/checksum assets.

The r1, r2 and legacy-channel GitHub Releases and all 31 uploaded assets have been removed; their source Git tags remain. The owner confirmed that nobody installed those releases, so no deployed-system migration is required. The legacy protocol-1 metadata commands stay desktop/x86_64 only. [STATUS](STATUS.md) records verified signed GHCR publication per target and remaining installation qualification.
