# Build and sign GHCR images

The workflow publishes independently signed OCI images per enabled target. It does not create GitHub Releases, ISO assets, metadata signatures or checkpoints. Installation media is built separately with [usr/src/kedra/installer/build-local.py](../../installer/build-local.py), locally or by the owner-dispatched `iso.yml`, which publishes it as a GitHub Release ([RELEASES](../../docs/RELEASES.md)).

| Target | Architecture (OCI) | Signed repository | Unsigned builds | Runner | Signing environment | Authority |
|---|---|---|---|---|---|---|
| `desktop` | x86_64 (`amd64`) | `ghcr.io/reidond/kedra-desktop` | `ghcr.io/reidond/kedra-desktop-builds` | `ubuntu-24.04` | `kedra-desktop-signing` | `authority/desktop.pub`, `desktop.sha256` |
| `qemu-arm64` | aarch64 (`arm64`) | `ghcr.io/reidond/kedra-qemu-arm64` | `ghcr.io/reidond/kedra-qemu-arm64-builds` | `ubuntu-24.04-arm` | `kedra-qemu-arm64-signing` | `authority/qemu-arm64.pub`, `qemu-arm64.sha256` |

The closed table is `targets.json`: `material.py` and the other Python tooling read it, the Rust crates embed it, and the workflows read it at the exact commit (the release matrix, each job's runner, repositories, signing environment and binary cache; the signer and `iso.yml` read it as data through the API, without a checkout). Only the signer's per-target secret selection and `iso.yml`'s dispatch choice name targets literally. `xps` stays disabled and any other target/architecture pair is refused. Each repository has its own `stable` tag, rank history and high-water mark; the identity schema, `workflow` value (`.github/workflows/release.yml`), epoch and rank semantics are shared.

## Package check

Actions runs at **00:00 UTC**, on image-affecting accepted source changes, and by manual dispatch:

```sh
gh workflow run release.yml --ref main
gh run list --workflow release.yml --limit 5
```

`release.yml` holds the single `release-44` concurrency group and calls the reusable `release-target.yml` once per target, as a matrix read from `targets.json` with fail-fast off: a failed `qemu-arm64` run never blocks `desktop`. Each call runs build, exact-candidate validation, isolated signing and stable publication natively on the target's runner, with target-named artifacts (`image-build-<target>-<run>-<attempt>`, `stable-publication-<target>-<run>-<attempt>`).

Runner queues affect delivery time. For each target the workflow resolves the reviewed official Fedora 44 base index to that target's no-variant platform, then resolves the full native RPM closure once into the run's RPM lock (`sysroot catalog lock resolve`), and compares source, the lock's pins, artifacts and recipes against the signature-verified stable image of the same repository. Recipes are every reviewed script and file that decides image content (`material.py` lists them: the Containerfiles, `assemble.sh`, `catalog/install-rpms.sh`, `catalog/build-packages.sh`, `build.sh`, `compose.py`, `refresh.py`, `prepare-trust.py`, `trust.py`, `material.py`, `targets.json`, `compatibility.json`, the Cargo inputs and the target's own authority files), so changing one rebuilds that target once. The workflow files are not recipes: they only call those scripts with the target, the commit and scratch paths, and every binary they build is hashed as an artifact, so a workflow-only edit cannot change an image and rebuilds and re-signs nothing. Keep every content-affecting step in a recipe script, never inline in a workflow.

No-change does nothing: no new image or freshness renewal. Changed inputs build one candidate (`build.sh`) offline from the lock's fixed-output RPM objects, through the same engine steps for both targets: foundation (`compose.py foundation`, which freezes the run's only source archive), catalog packages (`catalog/build-packages.sh`, which `kedra-lab`'s full local build also runs), composition and derivation of the target's declared image steps (`compose.py compose`), then the public trust layer ([one image pipeline](../../docs/SYSTEM.md#one-image-pipeline)). Every Docker-using job selects Docker's containerd image store first (`runner-containerd.sh`), as local OrbStack uses, and `runner-apparmor.sh` keeps the Ubuntu runner's AppArmor policy off the build sandbox and the candidate. Its installed RPM material must equal the lock, and its native receipt must match the steps the committed source declares. Failures never become a successful no-change result. Allowed RPM architectures are the target architecture and `noarch`, plus `i686` only for x86_64; the bootc RPM architecture must equal the target architecture.

`usr/src/kedra/image/release/compatibility.json` is shared with the compiled helper and sets the minimum bootc version (currently 1.16.13). The preflight rejects a lock with an older bootc RPM, once per run (`material.resolved_inputs`); the candidate's identity binds its resolved inputs, and so that checked inventory, through signing and publication. Newer versions pass; raising the minimum requires native qualification.

## Automatic image signing

Each `kedra-<target>-signing` environment retains main-only deployment and no administrator bypass, with no required reviewers or approval gates. `KEDRA_RELEASES_ENABLED=true` enables the pipeline; no manual signing action is needed. [Authority details](authority/README.md) describe the fixed public keys and environment names.

The public build first checks the closed target/runner/native-architecture triple and the target's environment policy, then validates source/run/attempt, image digest, identity/resolved inputs, package inventory and compatibility. The isolated signer then runs automatically, executes no checkout, image program or repository script with production private keys, and independently validates its environment-scoped public authority. `release.yml` passes no named secrets; it uses `secrets: inherit` because a called job sees its environment's secrets only then (probe run 36191986665). The repository has no repository-level secrets, and the signer job reads only its own environment's `KEDRA_<TARGET>_*` values, selected by explicit per-target expressions. It signs the exact digest once; immutable signed run tags are not overwritten.

A key-free job verifies the signed image's signature with a strict pull and its identity, checks rank order against the previous stable identity preflight verified, checks that `stable` still names the preflight's digest and that main is still this commit, then advances `stable` to the approved digest and verifies readback. The exact-main guard runs at the end of preflight, in the signer right before the private key is used, and right before `stable` advances; each property is verified once, at the stage that needs it. Keep main fixed while a candidate crosses its exact-current-main boundary. A signed image is not evidence that any workstation or virtual machine installed or booted it.

No separate promotion workflow or release/checkpoint signing remains. Consumers read signed image-owned identity, not GitHub release metadata. Keep known-good image digests and signature attachments for rollback; no automatic registry cleanup.

## First publication of a new target

Anonymous registry reads of a repository that does not exist yet return `DENIED`, not `manifest unknown` (observed during the former `utm` bootstrap on 2026-09-25). The build therefore cannot establish that `stable` is absent and fails closed; this is intended. Before the first `qemu-arm64` publication the owner must:

1. Create the `kedra-qemu-arm64-signing` environment and its secrets/variables as described in [authority](authority/README.md).
2. Make `ghcr.io/reidond/kedra-qemu-arm64` and `ghcr.io/reidond/kedra-qemu-arm64-builds` exist, linked to `Reidond/kedra` with Actions write access, and **public**. The signer inspects builds anonymously and installed helpers read `stable` with an empty auth file, like `desktop`.

Never relax the absence check or treat `DENIED` as absent to bootstrap a repository.

## Installation and compatibility

Build one local ISO from a reviewed exact signed digest, or download an owner-dispatched installer release, with [INSTALL.md](../../docs/INSTALL.md). Offline payload verification, deliberate disk selection, encryption and account creation remain mandatory. Nothing uploads from the local builder.

Fresh installations enroll directly in the signed GHCR update workflow of their own target. Nobody installed the removed r1/r2 releases, so no deployed-system migration is required. The protocol-1 release-metadata commands and helper requests were removed on 2026-10-14; `prepare-trust.py` uses only `sysroot release key`.

Changing the concurrency group from `release-desktop-44-x86_64` to `release-44` lets one already-queued run under the old group overlap the first run under the new one. Deploy that change when no release run is queued or in progress.

For failures or uncertain registry writes, inspect exact source/tag/digest/signature and current stable readback before retrying. Do not re-sign or move a tag blindly. [STATUS](../../docs/STATUS.md) distinguishes current implementation from observed native qualification.
