# Signed container images

The only published product artifacts are the signed OCI images of each enabled target: `ghcr.io/reidond/kedra-desktop` (x86_64) and `ghcr.io/reidond/kedra-qemu-arm64` (aarch64, the Apple Silicon QEMU virtual machine). Each repository has its own key, signing environment, `stable` tag and rank history. `stable` is discovery; verification and deployment use the immutable digest. A GitHub source commit, a tag name or a successful build alone is not image-signing authority.

The fixed public keys are `usr/src/kedra/image/release/authority/<target>.pub`. Independently confirm their SPKI DER SHA-256:

```text
desktop  a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
qemu-arm64  80551368732ccb49be7916db9ccfd1e3caa11a767e86749378565ec0b8a1a515
```

The installed root policy requires native Sigstore signatures from the installed target's key and its exact Kedra repository. A desktop system never accepts a `qemu-arm64` image and vice versa: target, architecture, OCI platform and repository are all bound by the signed image identity. Attachment discovery and initial-install policy must stay enabled. Never add permissive defaults or disable signature checks to make a pull work.

On the installed current-source OS, `sysroot update check` inspects the fixed GHCR repository of its enrolled target through the installed helper; `sysroot update stage` independently verifies before mutation. Image-owned identity/resolved-input records are bound by the signed OCI content. Local writable files and caller-supplied claims cannot establish root trust.

Builds, isolated signing and verified stable publication run automatically in Actions after validation, natively per target (`ubuntu-24.04` for desktop, `ubuntu-24.04-arm` for qemu-arm64). Each signing environment restricts deployment to main and has no human-review gate; no checkout, repository script or candidate code executes while production private keys are available. Build-time scope/rank/material checks and independent key/signature verification remain mandatory. No-change builds do not publish, sign metadata or renew checkpoints. See [release operations](../image/release/README.md) for the owner steps a new target needs before its first publication.

The ARM release source now routes its unsigned candidate through
`image/release/compose.py`: a fresh Fedora foundation defers native transforms,
the public composition APIs freeze committed configuration, and the existing
harness derives schemas, systemd links, initial defaults and QEMU initramfs.
Source and full RPM material must match before and after generation. Pinned
recipes, binaries and source inputs determine no-change equality; realized
native identities and artifact hashes are recorded separately. The existing
public-trust derivative and isolated signer consume the verified result.
Desktop retains complete assembly. This integration's current execution status
is in [D2 verification](../../../../.specs/nix-release-composition/verification.md);
source implementation does not establish protected-main publication success.

The disposable ARM installer/update workflow has an explicit `--local-fixture`
mode restricted to its dedicated ordinary-user controller container. It uses
generated fixture keys and actual committed source, then the normal encrypted
Anaconda installer and public updater. Its A/B fixture ranks share one native
derivation, so that workflow does not qualify a changed-kernel upgrade.
Production preparation, candidate inspection and publication retain their
current-main guards; read-only base resolution grants no signing authority.

For fixture-only corrections, local runs may select `--retained-candidate` with
its independent `--retained-candidate-sha256` and an explicit `--fixture-revision`.
The candidate's original `--source-revision` remains separate. Admission requires
an ancestor source, a closed set of fixture/document changes, unchanged production
recipes and executables, and fresh image/material readback before generating keys.
Product changes require a fresh candidate. Actions always build their selected
source; retained admission is local-only and creates no production authority.

Create installation media locally using [INSTALL.md](INSTALL.md). Its hash records describe the local output; the installer verifies the signed embedded OS payload offline. Current code does not publish ISO parts, GitHub Releases, machine bundles or release/checksum assets.

The r1, r2 and legacy-channel GitHub Releases and all 31 uploaded assets have been removed; their source Git tags remain. The owner confirmed that nobody installed those releases, so no deployed-system migration is required. The legacy protocol-1 metadata commands stay desktop/x86_64 only. [STATUS](STATUS.md) records verified signed GHCR publication per target and remaining installation qualification.
