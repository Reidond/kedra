# Status

Main `ec16a65`, 2026-10-18. This page lists current facts only; how each piece
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
- **Binary cache** in GHCR (`kedra-cache-<target>`), admitted only against the
  receipts of a signature-verified image ([ENGINE](ENGINE.md#binary-cache)).
- **Updates:** bootc verifies signatures under the installed strict policy; the
  helper adds target/channel scope, downgrade refusal, explicit staging and a
  rollback hold ([UPDATES](UPDATES.md)).
- **Managed home files:** `sysroot home status|capture|update` merges image changes
  into the live files listed in `usr/share/kedra/home-files.toml` ([HOME](HOME.md)).
- **Agents:** `sysroot codex` (bundled or user runtime) and `sysroot claude` (user
  runtime only; Claude is not bundled) ([AGENT-LAUNCHERS](AGENT-LAUNCHERS.md)).
- **Installer:** local ISO builds, and owner-dispatched `iso.yml` releases from a
  signed `stable` image ([INSTALL](INSTALL.md)).
- **Testing:** local checks plus a `signoff` commit status, which main requires;
  the release workflow runs the container harness on each exact candidate before
  signing ([tests](../tests/README.md)).

## Tested, and where

| Area | Local | Release |
|---|---|---|
| Rust/Python checks, CLI end-to-end tests | pass on each PR head before sign-off | not run there by design |
| Container harness, qemu-arm64 | pass on full local builds | pass (14 of 14) |
| Container harness, desktop | not-run (no x86_64 engine here) | pass (12 of 12) |
| Both targets built, signed and published | — | pass on `9871976` |
| Codex/Bitwarden packages in an image | pass, qemu-arm64 full local build | built; candidate failed validation |
| Binary cache push/substitute | pass against a local registry | not-run |
| Signed update A→B, rollback hold, refusals | pass, aarch64 VM under HVF (at #52) | — |
| Secure Boot VM boot and guest check | pass, aarch64 (HVF; TCG for `qemu-arm64/run.py`) | — |

Evidence: the [release that published both targets](https://github.com/Reidond/kedra/actions/runs/39129810946)
(`9871976`), the [release that failed validation](https://github.com/Reidond/kedra/actions/runs/39133795401)
(`969f72e`), the [only installer ISO release](https://github.com/Reidond/kedra/releases/tag/desktop-2e4781ee46070725),
and the PRs [#45–#60](https://github.com/Reidond/kedra/pulls?q=is%3Apr+is%3Amerged+merged%3A2026-10-14..2026-10-18),
each of which records its exact-head local checks. Per-change detail is in the
[worklog](../../../../worklog.md).

## Not working or not-run

- **Release blocked:** since `969f72e` the harness case `native::catalog_shadowing`
  fails on the hosted runners: it looks up an image by a registry index digest that
  Docker's classic image store does not hold. Nothing after `9871976` is
  published, so Codex/Bitwarden packages and the binary cache are not in a
  published image yet.
- **Installer media:** the only ISO release (desktop, 2026-10-14) was made before
  PRs #46–#60. Its helper writes deployment state (an enrollment journal) that
  the current helper refuses, so a system installed from it cannot keep updating;
  build media locally until a new release exists. The last full installation test
  (encrypted install, aarch64 VM, 2026-09-30) predates the engine pipeline and the
  current update helper.
- **Not-run:** x86_64 VM drivers (need a disposable x86_64 KVM host); a VM or
  hardware boot of an engine-derived desktop image; TPM enrollment and boot unlock;
  installation on real hardware; the first binary-cache push and hit; logged-in
  Bitwarden and authenticated agent use; killed-process recovery of
  `sysroot home update`.
- **Broken driver:** `tests/vm/ghcr-update/run-arm64.sh` (the full ARM release
  fixture) still calls the removed agent and Bitwarden packing scripts, so it
  cannot run on main.
- **Known limits:** the x86_64 Codex binary (258.7 MB) is close to the engine's
  256 MiB per-file limit. The harness does not follow the artifact rule yet: local
  builds share one Cargo volume across checkouts (pass exact binaries with
  `KEDRA_LAB_BINARIES`), its Docker labels are not `kedra.checkout`, and
  `kedra-lab vm` keeps disks outside `target/` unless `KEDRA_QEMU_HOME` and
  `KEDRA_LAB_ARTIFACTS` are set ([container harness](../tests/container/README.md)).

## Next

1. Fix `native::catalog_shadowing` for the classic image store; a release then
   publishes Codex/Bitwarden packages and pushes the first cache closures.
2. Owner: set the `kedra-cache-desktop` and `kedra-cache-qemu-arm64` packages
   public; the following release should substitute unchanged packages.
3. Owner: dispatch `iso.yml` for fresh media from the current `stable`, then test
   an installation in a disposable VM.
4. Run the x86_64 VM drivers on a disposable KVM host, and boot an engine-derived
   desktop image in a VM.
