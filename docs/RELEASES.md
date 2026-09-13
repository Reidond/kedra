# Signed container images

The only published product artifact is the signed OCI image in `ghcr.io/reidond/kedra-desktop`. `stable` is discovery; verification and deployment use the immutable digest. A GitHub source commit, a tag name or a successful build alone is not image-signing authority.

The fixed public key is build/release/authority/desktop.pub. Independently confirm its SPKI DER SHA-256:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

The installed root policy requires native Sigstore signatures from that key and the exact Kedra repository. Attachment discovery and initial-install policy must stay enabled. Never add permissive defaults or disable signature checks to make a pull work.

On the installed current-source OS, `sysroot update check` inspects the fixed GHCR target through the installed helper; `sysroot update stage` independently verifies before mutation. Image-owned identity/resolved-input records are bound by the signed OCI content. Local writable files and caller-supplied claims cannot establish root trust.

Builds, isolated signing and verified stable publication run automatically in Actions after validation. The signing environment restricts deployment to main and has no human-review gate; no checkout, repository script or candidate code executes while production private keys are available. Build-time scope/rank/material checks and independent key/signature verification remain mandatory. No-change builds do not publish, sign metadata or renew checkpoints.

Create installation media locally using [INSTALL.md](INSTALL.md). Its hash records describe the local output; the installer verifies the signed embedded OS payload offline. Current code does not publish ISO parts, GitHub Releases, machine bundles or release/checksum assets.

The r1, r2 and legacy-channel GitHub Releases and all 31 uploaded assets have been removed; their source Git tags remain. The owner confirmed that nobody installed those releases, so no deployed-system migration is required. [STATUS](STATUS.md) records verified signed GHCR publication and remaining installation qualification.
