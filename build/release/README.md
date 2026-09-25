# Build and sign GHCR images

The workflow publishes independently signed OCI images per enabled target. It does not create GitHub Releases, ISO assets, metadata signatures or checkpoints. Local installation media is built separately with [installer/build-local.py](../../installer/build-local.py).

| Target | Architecture (OCI) | Signed repository | Unsigned builds | Runner | Signing environment | Authority |
|---|---|---|---|---|---|---|
| `desktop` | x86_64 (`amd64`) | `ghcr.io/reidond/kedra-desktop` | `ghcr.io/reidond/kedra-desktop-builds` | `ubuntu-24.04` | `kedra-desktop-signing` | `authority/desktop.pub`, `desktop.sha256` |
| `utm` | aarch64 (`arm64`) | `ghcr.io/reidond/kedra-utm` | `ghcr.io/reidond/kedra-utm-builds` | `ubuntu-24.04-arm` | `kedra-utm-signing` | `authority/utm.pub`, `utm.sha256` |

The closed table is `TARGETS` in `material.py`. `xps` stays disabled and any other target/architecture pair is refused. Each repository has its own `stable` tag, rank history and high-water mark; the identity schema, `workflow` value (`.github/workflows/release.yml`), epoch and rank semantics are shared.

## Package check

Actions runs at **00:00 UTC**, on image-affecting accepted source changes, and by manual dispatch:

```sh
gh workflow run release.yml --ref main
gh run list --workflow release.yml --limit 5
```

`release.yml` holds the single `release-44` concurrency group and calls the reusable `release-target.yml` once per target. The two callers are independent: a failed `utm` run never blocks `desktop`. Each call runs build, isolated signing and stable publication natively on the target's runner, with target-named artifacts (`image-build-<target>-<run>-<attempt>`, `stable-publication-<target>-<run>-<attempt>`).

Runner queues affect delivery time. For each target the workflow resolves the reviewed official Fedora 44 base index to that target's no-variant platform and full native RPM closure, then compares source, package header/payload identities, artifacts and recipes against the signature-verified stable image of the same repository. Recipes include both workflow files and the target's own authority files, so changing them rebuilds that target once.

No-change does nothing: no new image or freshness renewal. Changed inputs build one candidate whose actual RPM material must match preflight. Failures never become a successful no-change result. Allowed RPM architectures are the target architecture and `noarch`, plus `i686` only for x86_64; the bootc RPM architecture must equal the target architecture.

`build/release/compatibility.json` is shared with the compiled helper and currently qualifies bootc 1.16.13. Preflight and actual-image checks reject an unsupported bootc RPM before signing; publication repeats the compatibility check. A version change requires deliberately updating the contract/helper compatibility and passing native qualification, not bypassing the gate.

## Automatic image signing

Each `kedra-<target>-signing` environment retains main-only deployment and no administrator bypass, with no required reviewers or approval gates. `KEDRA_RELEASES_ENABLED=true` enables the pipeline; no manual signing action is needed. [Authority details](authority/README.md) describe the fixed public keys and environment names.

The public build first checks the closed target/runner/native-architecture triple and the target's environment policy, then validates source/run/attempt, image digest, identity/resolved inputs, package inventory and compatibility. The isolated signer then runs automatically, executes no checkout, image program or repository script with production private keys, and independently validates its environment-scoped public authority. `release.yml` passes no secrets and does not inherit them: the signer job reads only its own environment's `KEDRA_<TARGET>_*` values, selected by explicit per-target expressions. It signs the exact digest once; immutable signed run tags are not overwritten.

A key-free job independently verifies the signed image and its identity, rechecks source/order, then advances that repository's `stable` to the approved digest and verifies readback. Keep main fixed while a candidate crosses its exact-current-main boundary. A signed image is not evidence that any workstation or virtual machine installed or booted it.

No separate promotion workflow or release/checkpoint signing remains. Consumers read signed image-owned identity, not GitHub release metadata. Keep known-good image digests and signature attachments for rollback; no automatic registry cleanup.

## First publication of a new target

Anonymous registry reads of a repository that does not exist yet return `DENIED`, not `manifest unknown` (observed for `kedra-utm` and `kedra-utm-builds` on 2026-09-25). The build therefore cannot establish that `stable` is absent and fails closed; this is intended. Before the first `utm` publication the owner must:

1. Create the `kedra-utm-signing` environment and its secrets/variables as described in [authority](authority/README.md).
2. Make `ghcr.io/reidond/kedra-utm` and `ghcr.io/reidond/kedra-utm-builds` exist, linked to `Reidond/kedra` with Actions write access, and **public**. The signer inspects builds anonymously and installed helpers read `stable` with an empty auth file, like `desktop`.

Never relax the absence check or treat `DENIED` as absent to bootstrap a repository.

## Installation and compatibility

Build one local ISO from a reviewed exact signed digest with [INSTALL.md](../../docs/INSTALL.md). Offline payload verification, deliberate disk selection, encryption and account creation remain mandatory. Nothing uploads from the local builder.

Fresh installations enroll directly in the signed GHCR update workflow of their own target. Nobody installed the removed r1/r2 releases, so no deployed-system migration is required. The legacy protocol-1 `sysroot release channel|history|unpack` paths stay desktop/x86_64 only.

Changing the concurrency group from `release-desktop-44-x86_64` to `release-44` lets one already-queued run under the old group overlap the first run under the new one. Deploy that change when no release run is queued or in progress.

For failures or uncertain registry writes, inspect exact source/tag/digest/signature and current stable readback before retrying. Do not re-sign or move a tag blindly. [STATUS](../../docs/STATUS.md) distinguishes current implementation from observed native qualification.
