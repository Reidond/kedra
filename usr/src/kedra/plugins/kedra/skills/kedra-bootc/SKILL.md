---
name: kedra-bootc
description: Maintain Fedora 44 image derivation, filesystem ownership, signed GHCR updates and local recovery media.
---

# Fedora bootc

Read usr/src/kedra/docs/ARCHITECTURE.md, usr/src/kedra/docs/SYSTEM.md ("One image pipeline"), usr/src/kedra/docs/UPDATES.md and references/update-client.md.

Both targets build through one engine pipeline, natively on their own runner: RPM lock → foundation (the plain usr/src/kedra/image/Containerfile and assemble.sh, installing the locked RPMs offline over the Fedora bootc base) → `.kedra` packages → `sysroot system compose` → `sysroot derive build` → trust layer. Each target's image steps (GLib schemas, systemd enable/mask/default target, skeleton, initramfs, home checks) are declared in system/catalog.kedra and the sets it uses; assemble.sh applies none and only reads the declared foundation edits (os-release names, hidden autostart entries) from the source manifest. system/*.kedra declares every image file and the home baseline; the crates/ and usr/src/kedra/ development trees never enter the image. OS images build in Actions; on-demand ISO media builds locally through usr/src/kedra/installer/build-local.py. No BlueBuild or live host DNF shortcut.

Prefer image-owned /usr defaults. /etc follows bootc persistence/merge; /var and home persist across deployments. Never author /usr/etc or overwrite writable home from an image. Ship safe baselines; home changes merge only through explicit `sysroot home update` (kedra-home).

Resolve only the official production Fedora 44 stream, not similarly named development images. A digest pin prevents substitution but cannot guarantee upstream retention: Quay retired a pinned base digest (d4b9c5e…) before native tests on 2026-09-12. Each run resolves and verifies one immutable platform digest per target. Source: [Fedora publication](https://forge.fedoraproject.org/iot/base-images/src/commit/8f30db6ad355562aeaca5c547f81ca157e8ffbf4/RELEASE.md). New base inputs require actual qualification.

The installed helper reads GHCR stable's digest-bound identity label, refuses another target or an image older than the booted one, and stages the exact digest with `bootc switch --enforce-container-sigpolicy`, so bootc verifies the signature (kedra-release-signing). The helper requires bootc at least `image/release/compatibility.json`'s minimum (1.16.13) and parses its v1 status strictly. Normal `bootc upgrade` does not advance a digest-pinned installation. Preserve pending slots and the rollback hold; no automatic reboot or home activation.

An unchanged release run publishes nothing; there is no checkpoint renewal. Keep local recovery independent of GitHub Releases. Older or corrupt deployment state is refused, never migrated or reset to bypass a refusal.

Anaconda media is separate and permissive under the pinned Fedora installer policy; installed SELinux remains enforcing. Local media requires offline signed-payload verification before disk installation. Preserve hash-guarded target scratch, selected non-API mounts before account creation and the physical /sysroot fstab normalization (usr/src/kedra/installer/README.md). Unknown upstream/source changes refuse rather than patch blindly.

UEFI Secure Boot is required (owner decision, 2026-09-25). Media refuse to start Anaconda without it: the helper's efivar check in `kedra-installer-verify.service` (installer/media/verify-payload.service, which prints the refusal on the console), plus `AssertSecurity=uefi-secureboot` in the drop-in installer/media/require-verification.conf that makes both Anaconda entry points require that unit. Keep the assertion off the verification unit: an unmet `Assert*=` fails a unit before its program runs, which would hide the helper's message. Fedora 44 `anaconda-core` 44.30-2's `anaconda-direct.service` (`inst.notmux`) runs `/usr/sbin/anaconda` with `Requires=anaconda.service` but no `After=`, so it needs its own drop-in (unit files read in a disposable `fedora:44` container, 2026-09-25). `sysroot doctor`'s required `secure_boot` check fails without Secure Boot. Never add a boot-time hard stop on an installed system or make `sysroot update` refuse.

Facts for Fedora 44 aarch64, observed 2026-09-25 in TCG prototypes with Ubuntu `qemu-efi-aarch64` 2024.02-2ubuntu0.9:
- shim-aa64 16.1-5 is signed only through Microsoft UEFI CA 2023.
- Its `fbaa64.efi`/`mmaa64.efi` carry a Red Hat test certificate, so the removable path `\EFI\BOOT\BOOTAA64.EFI` with fallback present fails with `Security Violation`.
- The installer ISO is unaffected: the pinned builder's `org.osbuild.grub2.iso` copies only shim, mm and `gcdaa64.efi` from `EFI/fedora` (usr/src/kedra/installer/README.md).
- Installed and bootc-image-builder disks carry fallback. Installed disks need bootupd's `--update-firmware` NVRAM entry for `\EFI\fedora\shimaa64.efi`; test disks get the same entry from the shared launcher (`create_variables` in usr/src/kedra/tests/common/qemu.py, `virt-fw-vars --append-boot-filepath`; tests/vm/qemu-arm64/README.md).
- Do not reset UEFI variables. Recover through the firmware Boot Maintenance Manager.
- bootc-image-builder appends `console=ttyS0` after the image kargs on test disks.

`usr/src/kedra/image/targets/qemu-arm64` adds `kargs.d/30-kedra-qemu-arm64-console.toml` (`console=ttyAMA0,115200 console=tty0`, aarch64 only). The aarch64 installer uses `grub2-efi-aa64-cdboot` and still needs `grub2-pc-modules`, because image-builder v82 always adds the i386-pc El Torito stage.

Encrypted-root prompt on qemu-arm64 (observed 2026-09-30, Fedora 44 aarch64 kernel 7.2.7 / native QEMU 11): the generic bootc initramfs omits virtio_gpu, virtio_input and their virtio_dma_buf dependency, so Cocoa showed no output before LUKS unlock while the serial log had the prompt; blind password entry is not a qualification. The qemu-arm64 target therefore declares those three drivers and the `qemu` dracut module in its initramfs step (catalog.kedra), and its dracut.conf.d forces the GPU/input drivers. The derive step regenerates every kernel's generic (`--no-hostonly`) initramfs and refuses a result missing a declared module or driver (docs/NATIVE.md), because an unchanged kernel does not regenerate merely when a target config is added. Keep this target-specific. With those drivers a fresh LUKS2 installation showed a visible unlock prompt and greetd login (owner-confirmed, 2026-09-30).

Native tests must distinguish image build, signed update/rollback, fresh installation, graphical health and physical hardware. Key rotation, old-reader compatibility and physical devices require independent evidence.

Primary references: [filesystems](https://bootc.dev/bootc/filesystem.html), [switch](https://bootc.dev/bootc/man/bootc-switch.8.html), [build guidance](https://bootc.dev/bootc/building/guidance.html), [physical root](https://bootc.dev/bootc/bootc-install.html#finding-and-configuring-the-physical-root-filesystem).
