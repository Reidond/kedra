# Signed container images

The only published OS artifacts are the signed OCI images of each enabled target: `ghcr.io/reidond/kedra-desktop` (x86_64) and `ghcr.io/reidond/kedra-qemu-arm64` (aarch64, the Apple Silicon QEMU virtual machine). Each repository has its own key, signing environment, `stable` tag and rank history. `stable` only locates an image; verification and deployment use its immutable digest. A source commit, a tag name or a successful build is not signing authority.

The fixed public keys are `usr/src/kedra/image/release/authority/<target>.pub`. Confirm their SPKI DER SHA-256 independently (`sysroot release key --public-key <target>.pub` prints it):

```text
desktop  a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
qemu-arm64  80551368732ccb49be7916db9ccfd1e3caa11a767e86749378565ec0b8a1a515
```

The installed root policy requires native Sigstore signatures from the installed target's key with its exact Kedra repository as the identity. A desktop system never accepts a `qemu-arm64` image and vice versa: target, architecture, OCI platform and repository are all bound by the signed image identity. Attachment discovery and the initial-install policy must stay enabled. Never add permissive defaults or disable signature checks to make a pull work.

On an installed system, `sysroot update check` reads the identity of the target repository's `stable` image through the installed helper. `sysroot update stage` has bootc verify the signature with that policy while it stages the exact digest, after the helper has refused another target or an image older than the booted one ([UPDATES](UPDATES.md#how-an-update-is-verified)). Local writable files and caller-supplied claims cannot establish root trust.

## Testing before a change merges

Actions runs no check or test workflow for changes. Each change is tested locally on the exact commit that merges, and `main` requires a successful `signoff` commit status on the pull request head, set by `uv run usr/src/kedra/tests/signoff.py --checks "..."` after those checks pass ([tests](../tests/README.md#local-testing-and-sign-off)). The release workflow still runs the complete container harness on every exact candidate image before signing it, because nightly Fedora package refreshes produce images that no sign-off covered.

## Release workflow

`release.yml` runs at **00:00 UTC**, on pushes to `main` that change image inputs, and by manual dispatch. It calls `release-target.yml` once per target; the two are independent, so one target failing never blocks the other. Each target runs natively on its own runner (`ubuntu-24.04` for desktop, `ubuntu-24.04-arm` for qemu-arm64):

1. **Build.** Resolve the official Fedora 44 base and the run's RPM lock, then compare source, lock pins, external inputs and recipes with the strictly verified current `stable` image. Unchanged inputs stop here: no image, no metadata, no renewal. Otherwise build the candidate offline through the engine pipeline (foundation, catalog packages, composition, the target's declared image steps, public trust layer; [SYSTEM](SYSTEM.md)), check that its installed RPMs equal the lock, and push it unsigned to the target's `-builds` repository by digest.
2. **Validate.** Run the complete container harness against that exact unsigned digest with no working-tree overlay.
3. **Sign.** The `kedra-<target>-signing` environment deploys only from `main`, has no administrator bypass and no reviewer gate. The signing job has no checkout and runs no repository script or candidate program while it holds the private key. It checks the image's identity label and signs the exact digest once under an immutable run tag.
4. **Publish.** A key-free job verifies the signed image strictly, rechecks source and order, moves `stable` to that digest and reads it back.

A failed repository, solver, harness or signature check is an error, never a successful no-change result. See [release operations](../image/release/README.md) for the owner steps a new target needs before its first publication. The first release run built entirely through this pipeline, desktop included, published `stable` for both targets on 2026-10-18; [STATUS](STATUS.md) records the current results.

## Binary cache

Package builds use one binary cache per target, `ghcr.io/reidond/kedra-cache-desktop` and `ghcr.io/reidond/kedra-cache-qemu-arm64` ([ENGINE](ENGINE.md#binary-cache)). The build job copies `/usr/share/sysroot/composition.json` out of the strictly verified previous `stable` image; `sysroot catalog build` then pulls, anonymously, each package closure that image records and admits it only if every object equals its record. Anything else is built: a cache miss, refusal or unreachable registry never fails the build. The one exception is a substitution that leaves the local store needing recovery (an interrupted import journal or a recovery-required store error); that stops the build, because building over an unrecovered store is unsafe. After the candidate passes inspection, the build job pushes every realized package closure with its existing `packages: write` token; that step may fail without failing the release. The cache adds no signing authority and no secret.

The cache is bounded, unlike signed images, which are never garbage-collected. After `publish-stable` succeeds, the `prune-cache` job deletes every cache version whose tag this run did not push or find present, which leaves exactly the closures the newly published `stable` image records. It prunes only when every package output has a recorded tag, so a failed or partial push prunes nothing, and it may fail without failing the release. A machine on an older `stable` finds its closures pruned and builds them.

**Owner action**, once for each target after the first run that pushes to its cache: in the `kedra-cache-<target>` package settings, set visibility to public (anonymous pulls by later runs and by machines need it; until then every pull misses and the run builds), and confirm under "Manage Actions access" that Reidond/kedra has the Admin role, which the GitHub API requires for `prune-cache` to delete versions. If the API still refuses, `prune-cache` reports it and old versions can be deleted on the package's versions page. No cache hit has been observed yet.

## Installer media

Installer media is published only when the owner dispatches `.github/workflows/iso.yml` for one target: a full new installer release. On the target's native runner it builds an ISO from the current signed `stable` digest with `usr/src/kedra/installer/build-local.py`, which checks the fixed public key, the strict signature and repository policy and the image's target scope. It splits the ISO into parts below 2 GiB and attaches them, with `PARTS.SHA256SUMS`, the builder's unchanged `SHA256SUMS` and `installer.json` (marked `"uploaded": true`), to a new GitHub Release tagged `<target>-<first 16 digest hex>` at the image's source revision. An existing release for that image is never replaced. The checksums detect damaged downloads and are not signatures; the installer verifies its signed embedded OS image offline. The workflow holds no signing secret and never signs or moves an image, and it does not boot or install the media. [INSTALL](INSTALL.md) covers downloading and building media locally.

The earlier r1, r2 and legacy-channel GitHub Releases and their assets were removed (their Git tags remain); nobody installed them. Release-metadata files, metadata signatures and update enrollment no longer exist: `sysroot release key` is the only release subcommand.

## Disposable ARM release test

`usr/src/kedra/tests/vm/ghcr-update/run-arm64.sh` exercises the composed qemu-arm64 image end to end with generated keys: public installer refusals, the normal encrypted Anaconda installation, then signed A/B/A updates through the public updater with persistent home and data. It runs on a disposable aarch64 Linux host (`--disposable-host --work DIR`, admitted by the shared VM launcher's guard) or, with `--local-fixture`, only inside its dedicated ordinary-user controller container. Its A/B fixture images share one native derivation, so it does not cover a changed-kernel upgrade.

- **Retained candidates.** For fixture-only corrections, a local run may reuse a candidate with `--retained-candidate`, its independent `--retained-candidate-sha256` and an explicit `--fixture-revision`; the candidate's `--source-revision` stays separate. It is admitted only for an ancestor source whose later changes are limited to fixtures and documents, with unchanged production recipes and executables, and after a fresh image and material readback. Product changes need a fresh candidate. Actions always builds its selected source; this admission is local-only and creates no production authority.
- **Supervised admission.** Local runs against a retained registry also require `--supervised-admission`: before the registry starts and before each installer phase, the owning supervisor checks the live wrapper process, the fixture context, the expected phase, monitor health and free host disk space, then answers with a one-time nonce on the wrapper's private stdin within the run's existing deadline. A missing, late or mismatched answer fails through the normal cleanup. There is no command hook or stored approval, and a nonce never waives source or storage checks. The wrapper rechecks disk capacity and the fixture TLS certificate's remaining lifetime before each call.
- **Private diagnostics.** Logs of a failed installer phase are kept, size-limited, in the runner's private (0700) `kedra-ghcr-diagnostics` directory, outside public evidence, which records only their size, hash and exit codes. They can contain generated credentials: keep them local and never upload them. A capture failure never replaces the original failure exit.
