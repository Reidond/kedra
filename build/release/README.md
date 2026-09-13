# Build and sign GHCR images

The workflow publishes signed OCI images in `ghcr.io/reidond/kedra-desktop`. It does not create GitHub Releases, ISO assets, metadata signatures or checkpoints. Local installation media is built separately with [installer/build-local.py](../../installer/build-local.py).

## Package check

Actions runs at **00:00 UTC**, on image-affecting accepted source changes, and by manual dispatch:

```sh
gh workflow run release.yml --ref main
gh run list --workflow release.yml --limit 5
```

Runner queues affect delivery time. The workflow resolves the reviewed official Fedora44 base and full native RPM closure, then compares source, package header/payload identities, artifacts and recipes against the signature-verified stable image.

No-change does nothing: no new image or freshness renewal. Changed inputs build one candidate whose actual RPM material must match preflight. Failures never become a successful no-change result.

`build/release/compatibility.json` is shared with the compiled helper and currently qualifies bootc 1.16.10. Preflight and actual-image checks reject an unsupported bootc RPM before signing; publication repeats the compatibility check. A version change requires deliberately updating the contract/helper compatibility and passing native qualification, not bypassing the gate.

## Automatic image signing

The `kedra-desktop-signing` environment retains main-only deployment and no administrator bypass, with no required reviewers or approval gates. `KEDRA_RELEASES_ENABLED=true` enables the pipeline; no manual signing action is needed. [Authority details](authority/README.md) describe the fixed public key and environment names.

The public build validates source/run/attempt, image digest, identity/resolved inputs, package inventory and compatibility. The isolated signer then runs automatically, executes no checkout, image program or repository script with production private keys, and independently validates its environment-scoped public authority. It signs the exact digest once; immutable signed run tags are not overwritten.

A key-free job independently verifies the signed image and its identity, rechecks source/order, then advances `stable` to the approved digest and verifies readback. Keep main fixed while a candidate crosses its exact-current-main boundary. A signed image is not evidence that any workstation installed or booted it.

No separate promotion workflow or release/checkpoint signing remains. Consumers read signed image-owned identity, not GitHub release metadata. Keep known-good image digests and signature attachments for rollback; no automatic registry cleanup.

## Installation and compatibility

Build one local ISO from a reviewed exact signed digest with [INSTALL.md](../../docs/INSTALL.md). Offline payload verification, deliberate disk selection, encryption and account creation remain mandatory. Nothing uploads from the local builder.

Fresh installations enroll directly in the signed GHCR update workflow. Nobody installed the removed r1/r2 releases, so no deployed-system migration is required.

For failures or uncertain registry writes, inspect exact source/tag/digest/signature and current stable readback before retrying. Do not re-sign or move a tag blindly. [STATUS](../../docs/STATUS.md) distinguishes current implementation from observed native qualification.
