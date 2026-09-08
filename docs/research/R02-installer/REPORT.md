# R02: minimal image and VM feasibility

Status: **pass** for the minimal image/QCOW2/UEFI guest experiment; full R02
installation gate remains **blocked**. Updated 2026-09-08 (Europe/Kiev).

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
- Interactive multi-disk installer, encryption and account creation: not-run.
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
