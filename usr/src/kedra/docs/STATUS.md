# Status

Main `8a48ed2`, 2026-10-19. This page lists current facts only; how each piece
works is in the linked documents, and earlier results are in Git history (the
complete record is in commit `ec16a65`, `git show ec16a65:usr/src/kedra/docs/STATUS.md`).
Terms: *local* means on the owner's Mac (Apple Silicon, Linux arm64 containers and
VMs); *release* means the Actions release workflow on hosted runners; *not-run*
means nobody has run it on the current code.

## Implemented on main

- **Two targets** (`desktop` x86_64, `qemu-arm64` aarch64), each with its own
  signing key and GHCR repository ([RELEASES](RELEASES.md)).
- **One image pipeline** for both: Fedora RPM lock → foundation → engine-built
  packages → composition → native image steps → trust layer
  ([SYSTEM](SYSTEM.md), [NATIVE](NATIVE.md)). Every Fedora RPM is pinned in the lock
  and installed offline; images carry `/usr/share/sysroot/rpm-lock.json`
  ([PACKAGES](PACKAGES.md#fedora-rpm-lock)).
- **Package engine** with an unprivileged bubblewrap sandbox over input-addressed
  store roots; package identity depends on the Fedora material, not on the
  per-commit foundation image ([ENGINE](ENGINE.md)).
- **Engine-built packages:** Codex and Bitwarden are `.kedra` packages built from
  pinned official archives; command exports are launchers in
  `/usr/share/kedra/catalog-bin`, and shadowing a foundation command needs
  `replaces` ([PACKAGES](PACKAGES.md#shipped-packages)).
- **Binary cache** in GHCR (`kedra-cache-<target>`, both public), admitted only
  against the receipts of a signature-verified image
  ([ENGINE](ENGINE.md#binary-cache)).
- **Release workflow:** a matrix read from `targets.json`; every content-affecting
  step is a hashed recipe script, so a workflow-only edit rebuilds nothing; each
  guard (exact main, bootc minimum, signature) runs once where it is needed; build
  and validate-candidate both use Docker's containerd image store, as local
  OrbStack does ([RELEASES](RELEASES.md)).
- **Updates:** bootc verifies signatures under the installed strict policy; the
  helper adds target/channel scope, downgrade refusal, explicit staging and a
  rollback hold ([UPDATES](UPDATES.md)).
- **Managed home files:** `sysroot home status|capture|update` merges image changes
  into the live files declared `managed` in `system/*.kedra` ([HOME](HOME.md)).
- **Agents:** `sysroot codex` (bundled or user runtime) and `sysroot claude` (user
  runtime only; Claude is not bundled) ([AGENT-LAUNCHERS](AGENT-LAUNCHERS.md)).
- **Installer:** local ISO builds, built once from the image-builder manifest, and
  owner-dispatched `iso.yml` releases from a signed `stable` image
  ([INSTALL](INSTALL.md)).
- **Testing:** local checks plus a `signoff` commit status, which main requires;
  the release workflow runs the container harness on each exact candidate before
  signing ([tests](../tests/README.md)). The harness labels every Docker object
  `kedra.checkout`, keeps per-checkout Cargo volumes and image tags, keeps
  `kedra-lab vm` instances in `target/kedra-lab/qemu`, and `kedra-lab clean` removes
  exactly that checkout's objects ([container harness](../tests/container/README.md)).
  Every checkout shares one cache in `<primary checkout>/.cache/kedra`: the QEMU
  runtime, VM base disks, RPM locks and the engine stores of full local builds,
  built once, verified on use and deleted by `kedra-lab gc` once no checkout uses
  them ([shared cache](../tests/container/README.md#shared-cache)).

## Tested, and where

| Area | Local | Release |
|---|---|---|
| Rust/Python checks, CLI end-to-end tests | pass on each PR head before sign-off | not run there by design |
| Container harness, qemu-arm64 | pass, also on Docker's classic image store | pass (15 of 15) |
| Container harness, desktop | not-run (no x86_64 engine here) | pass (13 of 13) |
| Both targets built, signed and published | — | pass on `e9bb7a1` |
| Codex/Bitwarden packages in a published image | — | pass on `e9bb7a1` |
| Binary cache | push/substitute pass against a local registry | first push on `e9bb7a1`; no hit yet |
| Signed update A→B, rollback hold, refusals | pass, aarch64 VM under HVF (at #52) | — |
| Secure Boot VM boot and guest check | pass, aarch64 (HVF; TCG for `qemu-arm64/run.py`) | — |
| `kedra-lab vm` with its state under `target/` | pass, aarch64 under HVF (at #64) | — |
| Shared cache: concurrent reuse, tamper refusal, `gc` | pass, two and three worktrees, aarch64 under HVF | — |

Evidence: the [release that published both targets](https://github.com/Reidond/kedra/actions/runs/39165178171)
(`e9bb7a1`), the [only installer ISO release](https://github.com/Reidond/kedra/releases/tag/desktop-2e4781ee46070725),
and the PRs [#45–#64](https://github.com/Reidond/kedra/pulls?q=is%3Apr+is%3Amerged+merged%3A2026-10-14..2026-10-19),
each of which records its exact-head local checks. Per-change detail is in the
[worklog](../../../../worklog.md).

## Not working or not-run

- **Installer media:** the only ISO release (desktop, 2026-10-14) was made before
  PRs #46–#64. Its helper writes deployment state (an enrollment journal) that
  the current helper refuses, so a system installed from it cannot keep updating;
  build media locally until a new release exists. `iso.yml` has not run since
  #63 changed it. The last full installation test (encrypted install, aarch64 VM,
  2026-09-30) predates the engine pipeline and the current update helper.
- **Not-run:** x86_64 VM drivers (need a disposable x86_64 KVM host); a VM or
  hardware boot of an engine-derived desktop image; TPM enrollment and boot unlock;
  installation on real hardware; the first binary-cache hit; logged-in Bitwarden
  and authenticated agent use; killed-process recovery of `sysroot home update`.
- **Broken driver:** `tests/vm/ghcr-update/run-arm64.sh` (the full ARM release
  fixture) still calls the removed agent and Bitwarden packing scripts, so it
  cannot run on main.
- **Known limits:** the x86_64 Codex binary (258.7 MB) is close to the engine's
  256 MiB per-file limit. Full local images are still built per checkout (each
  foundation carries its checkout's label), so a VM disk is shared only between
  checkouts whose fixture layers are identical. The old
  shared Docker volumes `kedra-lab-target-aarch64`, `kedra-lab-cargo-registry` and
  `kedra-qemu-podman-cache` belong to no checkout any more; the owner removes them.

## Next

1. Check that a release with unchanged packages reports substituted closures.
2. Owner: dispatch `iso.yml` for fresh media from the current `stable`, then test
   an installation in a disposable VM.
3. Run the x86_64 VM drivers on a disposable KVM host, and boot an engine-derived
   desktop image in a VM.
4. Fix `tests/vm/ghcr-update/run-arm64.sh`.
