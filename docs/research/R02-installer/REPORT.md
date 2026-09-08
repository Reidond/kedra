# R02: minimal image and VM feasibility

Status: **pass** for the minimal image/QCOW2/UEFI guest experiment; full R02
installation gate remains **blocked**. Updated 2026-09-08 (Europe/Kiev).

Signed-payload follow-up: [34222699188](https://github.com/Reidond/kedra/actions/runs/34222699188)
at `d8a76a9` passes signature-preserving assembly, strict positive/wrong-key
checks and offline signature verification before Anaconda startup. The downloaded
2,856,105,984-byte ISO matches SHA-256
`94c58c5c4ea0a9b39832cc60e530426390968d97974885e46ff96a95f621a00e`.
Its research-scoped payload is
`sha256:09cb69b122daf8fb7fb6d29ac33168ba4834c4cbd0befd9bb4e29beab570bda5`.
A fresh local encrypted two-disk installation **passes** Anaconda completion,
clean shutdown and identical sentinel retention. ISO-free UEFI boot unlocks LUKS,
authenticates the administrative owner and reaches niri/Noctalia without repairs
or overrides. Enforcing SELinux, running system/user service managers, native niri
validation, a responding desktop portal, unlocked login keyring, read-only
`/sysroot` and writable `/var/home` pass. Native bootc reports the exact payload
digest above with registry origin and `containerPolicy` enforcement. The installed
`sysroot update status` independently accepts the inherited public trust, strict
policy and source scope, and reports `enrolled: false` as expected. Local evidence
is in `output/installer-34222699188/signed-*.png`; QEMU 8.2.2/OVMF uses the same
isolated 8 GiB/two generated 64 GiB disk arrangement described below. No host
devices, network or owner credentials were attached. This qualifies the signed
payload's installer handoff; the outer ISO remains unsigned/unpromoted and all
authority is disposable research scope. Production release/enrollment, full
recovery, Secure Boot and physical hardware remain open.

Earlier fresh-install result: [34207121856](https://github.com/Reidond/kedra/actions/runs/34207121856)
at `85ed4ab` passes Anaconda completion and first-boot desktop health on 2026-09-08.
The fully downloaded ISO is 2,865,981,440 bytes with SHA-256
`d74e2a1eb79e8c93f52da82a8626bad43ad65498382941cf8982f07f41174ed3`.
Local QEMU 8.2.2/OVMF boots UEFI/KVM with 8 GiB RAM and two generated 64 GiB disks,
no network or host disks. Only vda was deliberately selected after verifying
its KEDRA-INSTALL-ONLY serial, enabled encryption, and created the generated owner
account with wheel membership and a generated password. No repair or kernel
override was used on this installation.

Anaconda completed; the owner directory is on the separate mounted home subvolume
and the generated fstab correctly addresses /sysroot. Clean shutdown returns
QEMU exit 0; qemu-img compares the unselected KEDRA-KEEP-DATA disk identically to
its original sentinel copy. Boot without the ISO unlocks LUKS, authenticates the
owner and reaches niri/Noctalia. Native niri validation, enforcing SELinux, zero
failed system/user units, read-only /sysroot, writable home and unlocked login
keyring all pass. The installed origin remains the deliberately unsigned
localhost research payload, so this is **not promoted owner installation media**.
Signed-origin enrollment/update, recovery and hardware qualification remain open.

Earlier interactive result: cleanup-fixed media
[34195114452](https://github.com/Reidond/kedra/actions/runs/34195114452) at `923a282`
completed encrypted installation and owner creation in a fresh two-disk local VM.
Its verified ISO SHA-256 is
`24161c57cac073137bc4730684a106815fc8d654b65483c52e46c88b89fa0e91`
(2,552,686,592 bytes). Sentinel comparison passes. Boot without the ISO unlocks
LUKS and authenticates the owner with enforcing SELinux, but **fails health**:
the owner home is hidden behind its separate subvolume, and systemd-remount-fs
tries to remount the logical overlay root from a physical-root fstab entry.
The native mount omission and proposed corrections are recorded in
[ADR 0015](../../adr/0015-installer-persistent-mounts.md). Rebuilt-media acceptance
is pending; diagnostic repair does not turn this run into a healthy-install pass.

The companion Codex-containing ISO build
[34197344823](https://github.com/Reidond/kedra/actions/runs/34197344823) at `ba011ec`
passes assembly/startup but has the same uncorrected installation issues and has
not been installed. Codex itself passes the separate Fedora runtime VM.

[Corrected run 34165475139](https://github.com/Reidond/kedra/actions/runs/34165475139)
passed at `cfbfc05405f13d16dbbe5a604bc116b0763a421a`. The guest emitted
`KEDRA_R02_BOOT_PASS` after reading bootc state, checking root and persistent /var
mounts, and requiring enforcing SELinux, then powered off successfully. The QCOW2
is 1,323,933,184 bytes; SHA-256
`ad20491cd85267e831e5238d1b3d53025cf7c5111c56eebd066b911cab1b89ea`.
The pinned builder reports revision `a686afe`, build time 2026-06-18T11:23:37Z,
and `build_tainted: true`; its exact container digest is the reproducible input,
not an inferred current upstream release. Host Podman 4.9.3/Skopeo 1.13.3 and QEMU
8.2.2 were recorded. Initial disk origin is deliberately localhost research input,
so this result does not establish an installed update source.

[Initial run 34164873575](https://github.com/Reidond/kedra/actions/runs/34164873575)
at source `9016832dfad58c3cdb99cc5ac1b40ed12a6cea62` built and linted the pinned
Fedora derivative, built QCOW2 and reached Fedora 44 with kernel
`7.1.13-200.fc44.x86_64` and bootc `1.16.10` in UEFI/KVM. The guest emitted bootc
status before the harness used an invalid multi-path findmnt invocation. That
command returned nonzero; the service exited and QEMU timed out (exit 124).
Corrective change: query each mount separately, emit failure location/status and
power off on failure, and allow systemd's console prefix on the success marker.
This remains a failed overall check, not a passed installation test.

The first experiment builds a minimal Fedora 44 derivative and QCOW2 in Actions,
boots it using QEMU/KVM and UEFI, and requires a guest success marker after bootc
status, persistent mounts and enforcing SELinux checks. It has no user account,
password, SSH key, installer, desktop or production signing material. Its only
guest service reports evidence and powers off. This disk is a disposable fixture,
not installation media for the owner. No registry is published by this workflow.

Inputs are pinned by the x86_64 manifest digest in
`build/research/inputs.json`, resolved from Quay's registry API on 2026-09-08.
The builder records its actual version at runtime. Runner package versions,
resource inventory, RPM inventory, image inspection, build logs and serial output
are uploaded even after failure. A boot-success disk is retained for three days;
the evidence is retained for fourteen days. These are research artifacts.

## Reproduction

Push reviewed changes on `codex/usable-system` affecting the research workflow or
`build/research/`, or dispatch `Research image and VM` after that workflow exists
on the default branch. The workflow uses an ephemeral GitHub Ubuntu 24.04 runner.
It never builds OS images on the owner's workstation.

Local VM runtime installation was authorized by the owner's implementation task.
Existing Ubuntu WSL2 has `/dev/kvm`. Installed QEMU 8.2.2
(`1:8.2.2+ds-0ubuntu1.18`) and OVMF `2024.02-2ubuntu0.9`; no OS disk has been
downloaded or booted yet. Approximately 64 GiB host RAM and 817 GiB free disk were
observed. This does not establish VM boot capability until a guest has run.

Local follow-up 2026-09-08: the exact passing-run QCOW2 was downloaded and its
SHA-256 matched the recorded value. It booted in the existing Ubuntu WSL2/KVM
runtime with OVMF, reached `KEDRA_R02_BOOT_PASS` and powered off (QEMU exit 0).
The disk used QEMU snapshot mode and no host disks/network interface. A first
shell wrapper returned 1 despite a guest pass; a saved-script rerun recorded
both QEMU exit 0 and guest pass explicitly. Guest serial/audit lines can interleave,
so later graphical tests use a separate marker port. No workstation OS was installed.

## Remaining acceptance cases

Installer attempt [34171335800](https://github.com/Reidond/kedra/actions/runs/34171335800)
at `0a5991a` built the desktop and separate Anaconda environment, then failed on
the documented `--bootc-installer-payload-ref` spelling. Inspection of current
osbuild source shows that spelling belongs to the image-builder CLI; its
bootc-image-builder compatibility interface exposes `--installer-payload-ref`.
The installer experiment now pins the current official GHCR v82.0.0 builder in
installer/inputs.json and checks its build help before expensive assembly. The
earlier Quay builder remains pinned for the already-tested QCOW2/signature cases.
No installer execution or disk-choice success is claimed from the failed attempt.

The corrected legacy build [34173909609](https://github.com/Reidond/kedra/actions/runs/34173909609)
at `eca177c` produced a 3,490,482,176-byte ISO (SHA-256
`67f3628bb66ea63b729b3b52dd1c49f67852e23faf1260b71e10c4656820c7f9`). It was not
booted. Inspection found forced `inst.text`, generated `clearpart --all`, and a
legacy ostreecontainer import followed by bootc origin mutation. These defaults
are not the selected Kedra installation contract. The new experiment uses the
current canonical image-builder v82.0.0 container and `bootc-generic-iso`, with
an explicit native bootc interactive-defaults file and graphical/rescue entries.
The upstream sample's SELinux-disable flag is not copied. Installer signing,
disk choice, encryption/account setup and the documented remount-service concern
still require VM evidence. The ISO size also requires a deliberate release-asset
splitting/reassembly design before public promotion.

Generic ISO [run 34176407860](https://github.com/Reidond/kedra/actions/runs/34176407860)
at `76dc82a` passes build and boot-configuration inspection. The 2,540,959,744-byte
ISO has SHA-256 `8734723fb87db17a129d4293858a0638164ee4db898990453fadd03eed788205`.
Extracted defaults contain only native bootc source/target references and locked
root. Graphical/rescue entries contain no preset disk selection or partitioning.
This is unsigned localhost-origin research media; interactive installation has
not yet been tested. The older legacy ISO remains rejected and was not booted.

- Minimal QCOW2 build and UEFI boot: pass (run 34165475139).
- Generic ISO local UEFI boot: fail before Anaconda (2026-09-08). Exact downloaded
  SHA-256 matched, but systemd froze on SELinux permission errors. Neither disk
  was selected; qemu-img comparison confirms the sentinel disk remains identical.
  v82.0.0's generic OSFromContainer pipeline lacks SELinux labeling, also noted in
  the upstream demonstration README. Added the standard osbuild SELinux stage
  before packaging, with pipeline-drift refusal and an enforcing-userspace smoke
  test. Corrected build/runtime results are pending.
  Corrected [run 34180796587](https://github.com/Reidond/kedra/actions/runs/34180796587)
  at `da140ff` passes the standard labeling stage, packaged systemd `init_exec_t`
  check and diskless enforcing-userspace boot. ISO: 2,541,139,968 bytes, SHA-256
  `85753341b9070938590d9e9c9a70bd7f67d44e4ba685e0be5627eb390566b74e`.
  This smoke uses the ISO's extracted kernel/initramfs and unchanged stage2;
  corrected-media UEFI/UI/disk/encryption/account installation remains pending.
  The local UEFI follow-up reaches userspace but still fails before the UI:
  Anaconda's direct rescue shell runs as getty_t and cannot access systemctl or
  getenforce. No disks were selected. ADR 0010 records upstream Lorax's separate
  permissive installer policy and media-only install-user account. New media
  follows that contract while the desktop payload stays enforcing; a stronger
  Anaconda service/log smoke and actual installed enforcement are required.
- Updated [run 34185915639](https://github.com/Reidond/kedra/actions/runs/34185915639)
  at `cfc956d` passes Anaconda service/log startup under the upstream installer
  SELinux mode, the separate enforcing desktop configuration check, labels and
  both guarded native-property adaptations (ADR 0011). ISO: 2,550,966,272 bytes,
  SHA-256 `f6240416ff5adae95b98e34d3f093329228c72588abb527aecfced1871209db5`.
  The old-media diagnostic VM was stopped without starting installation; the
  unselected disk compares identical. New-media transfer is in progress; actual
  acceptance must use a fresh VM without the diagnostic kernel override.
- Fresh 34185915639 media UEFI/UI: pass without a kernel override. Both disks start
  unselected; explicit serial inspection identifies vda as KEDRA-INSTALL-ONLY and
  vdb as KEDRA-KEEP-DATA. Selected only vda and encryption. Offline networking is
  not blocking. Begin Installation remains disabled without an owner and with a
  non-admin owner, then enables when wheel membership is restored; root stays locked.
- Actual installation: fail during bootc GetBlob import with ENOSPC in /var/tmp.
  The encrypted target is mounted and has 59 GiB free; the separate installer
  writable filesystem has 1.6 GiB capacity. Owner creation/first boot did not finish.
  After stopping the VM, the unselected sentinel compares identical. ADR 0013
  implements selected-disk scratch with exact-source and inert lifecycle checks;
  corrected-media installation remains pending.
- Registry origin and enforced signed A-to-B updates: not-run (R01).
- Desktop package build and graphical session subset: pass (R07); physical qualification remains open.
- Secure Boot, physical devices, recovery and installer checksum verification:
  not-run.

## Primary sources

- [Current builder source](https://github.com/osbuild/image-builder/tree/main/bootc-image-builder)
  and [migration documentation](https://osbuild.org/docs/bootc/), retrieved 2026-09-08:
  QCOW2 supports an explicit root filesystem; the bootc-installer type takes a
  separate Anaconda environment and payload. The unattended anaconda-iso default
  is unsuitable for Kedra's deliberate disk-choice requirement.
- [bootc kernel arguments](https://bootc.dev/bootc/building/kernel-arguments.html),
  retrieved 2026-09-08: architecture-scoped kargs.d provides the serial console.

Documentation establishes experiment inputs, not passed test results.

Labeling sources reviewed 2026-09-08:
[v82 OSFromContainer](https://github.com/osbuild/image-builder/blob/v82.0.0/pkg/manifest/os_from_container.go),
[standard SELinux stage](https://github.com/osbuild/osbuild/blob/main/stages/org.osbuild.selinux),
[upstream ISO limitation](https://github.com/ondrejbudai/bootc-isos#quirks).
