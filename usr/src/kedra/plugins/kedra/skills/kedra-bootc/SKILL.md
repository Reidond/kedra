---
name: kedra-bootc
description: Maintain Fedora 44 image derivation, filesystem ownership, signed GHCR updates and local recovery media.
---

# Fedora bootc

Read docs/ARCHITECTURE.md, docs/UPDATES.md and references/update-client.md. Use plain Containerfile and explicit shared/host payload assembly. OS images build in Actions; on-demand ISO media builds locally through installer/build-local.py. No BlueBuild or live host DNF shortcut.

Prefer image-owned /usr defaults. /etc follows bootc persistence/merge; /var and home persist across deployments. Never author /usr/etc or overwrite writable home from an image. Ship safe baselines and use explicit home reconciliation. Repository skills remain checkout-local.

Resolve only the official production Fedora 44 stream, not similarly named development images. A pin prevents substitution but cannot guarantee upstream retention: Quay retired d4b9c5e... before native tests on 2026-09-12. Current tests resolve/verify one immutable platform per run. Source: [Fedora publication](https://forge.fedoraproject.org/iot/base-images/src/commit/8f30db6ad355562aeaca5c547f81ca157e8ffbf4/RELEASE.md). New base inputs require actual qualification.

The installed helper verifies signed GHCR stable discovery and stages exact digests through enforcing bootc policy. Normal bootc upgrade does not advance a digest-pinned installation. Preserve pending slots, ordering and rollback holds; no automatic reboot or home activation.

No-change CI publishes nothing. There is no checkpoint renewal in the current path. Keep local recovery independent of GitHub Releases. The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md); do not make legacy migration a delivery prerequisite. Unknown or corrupt state must still refuse; never reset it to bypass a refusal.

Anaconda media is separate and permissive under the pinned Fedora installer policy; installed SELinux remains enforcing. Local media requires offline signed-payload verification before disk installation. Preserve hash-guarded target scratch, selected non-API mounts before account creation and the physical /sysroot fstab normalization. Unknown upstream/source changes refuse rather than patch blindly.

UEFI Secure Boot is required (owner decision, 2026-09-25). Media refuse to start Anaconda without it: the helper's efivar check in `kedra-installer-verify.service` (which prints the refusal on the console), plus `AssertSecurity=uefi-secureboot` in the drop-in that makes both Anaconda entry points require that unit. Keep the assertion off the verification unit: an unmet `Assert*=` fails a unit before its program runs, which would hide the helper's message. Fedora 44 `anaconda-core` 44.30-2's `anaconda-direct.service` (`inst.notmux`) runs `/usr/sbin/anaconda` with `Requires=anaconda.service` but no `After=`, so it needs its own drop-in (unit files read in a disposable `fedora:44` container, 2026-09-25). `sysroot doctor`'s required `secure_boot` check fails without Secure Boot. Never add a boot-time hard stop on an installed system or make `sysroot update` refuse.

Facts for Fedora 44 aarch64, observed 2026-09-25 in TCG prototypes with Ubuntu `qemu-efi-aarch64` 2024.02-2ubuntu0.9:
- shim-aa64 16.1-5 is signed only through Microsoft UEFI CA 2023.
- Its `fbaa64.efi`/`mmaa64.efi` carry a Red Hat test certificate, so the removable path `\EFI\BOOT\BOOTAA64.EFI` with fallback present fails with `Security Violation`.
- The installer ISO is unaffected. The pinned builder's `org.osbuild.grub2.iso` copies only shim, mm and `gcdaa64.efi` from `EFI/fedora` (installer/README.md).
- Installed and bootc-image-builder disks carry fallback. Installed disks need bootupd's `--update-firmware` NVRAM entry for `\EFI\fedora\shimaa64.efi`; that is source-read, not observed on utm. Test disks get an explicit `virt-fw-vars --append-boot-filepath` entry (tests/vm/utm/boot.sh).
- Do not reset UEFI variables. Recover through the firmware Boot Maintenance Manager.
- bootc-image-builder appends `console=ttyS0` after the image kargs on test disks.

`hosts/utm` adds `kargs.d/30-kedra-utm-console.toml` (`console=ttyAMA0,115200 console=tty0`, aarch64 only). The aarch64 installer uses `grub2-efi-aa64-cdboot` and still needs `grub2-pc-modules`, because image-builder v82 always adds the i386-pc El Torito stage.

Native tests must distinguish image build, signed update/rollback, fresh installation, graphical health and physical hardware. Key rotation, old-reader compatibility and physical devices require independent evidence.

Primary references: [filesystems](https://bootc.dev/bootc/filesystem.html), [switch](https://bootc.dev/bootc/man/bootc-switch.8.html), [build guidance](https://bootc.dev/bootc/building/guidance.html), [physical root](https://bootc.dev/bootc/bootc-install.html#finding-and-configuring-the-physical-root-filesystem).
