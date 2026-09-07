# R02: minimal image and VM feasibility

Status: **fail** for the initial guest-check harness; image/QCOW2 builds passed.
Updated 2026-09-08 (Europe/Kiev). The corrective rerun is pending.

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

## Remaining acceptance cases

- Minimal QCOW2 build and UEFI boot: not-run.
- Interactive multi-disk installer, encryption and account creation: not-run.
- Registry origin and enforced signed A-to-B updates: not-run (R01).
- Realistic desktop image size and session checks: not-run (R07).
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
