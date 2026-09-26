# utm image and Secure Boot boot check

`.github/workflows/test-utm-image.yml` qualifies the aarch64 `utm` candidate on `ubuntu-24.04-arm`. Hosted arm64 runners expose no `/dev/kvm`, so the boot is TCG emulation. That qualifies the firmware → shim → GRUB → kernel chain and the booted deployment. It does not qualify UTM, HVF, TPM, VirGL/Venus rendering or the graphical session; those remain manual UTM checks.

| File | Role |
|---|---|
| `image-check.sh` | Read-only candidate checks, streamed into `podman run -i`: target identity, niri/Noctalia validation, greetd and masked update timer, VM packages, the guest-agent RPC block list (effective unit plus a probe of the installed `qemu-ga`), Venus ICD, VirGL DRI driver, environment.d/kargs files, Bitwarden arm64 tree, aarch64 Codex, boot-chain RPMs and the bootc contract |
| `Containerfile`, `check.sh`, `check.service` | Non-promotable observer layer over `localhost/kedra-utm:research`, tagged `localhost/kedra-utm-test:secureboot` |
| `boot.sh` | Firmware validation, fresh NVRAM with one explicit boot entry, bounded TCG boot, marker verification |

The disk comes from the arm64 bootc-image-builder pin (`build/inputs.json` `.platforms.arm64.builder`), with a generated account whose password is never stored. The account exists only so the observer can run `sysroot doctor` as a non-root user.

## Guest markers

The observer writes markers to the virtio-serial port `org.kedra.events` (host `vm/events.log`) and, best effort, to `ttyAMA0` (`vm/serial.log`). `boot.sh` requires all four markers, a `KEDRA_UTM_IMAGE_DIGEST=` line equal to podman's `.Digest` for the observer image, no `KEDRA_UTM_FAIL`, and a firmware line showing that the explicit entry was booted:

- `KEDRA_UTM_SECUREBOOT_PASS` requires:
  - `mokutil --sb-state` is exactly `SecureBoot enabled`;
  - the SecureBoot efivar is 1 and SetupMode is 0;
  - lockdown is `[integrity]` or `[confidentiality]`;
  - the kernel journal has `secureboot: Secure boot enabled`;
  - `BootCurrent` is the `shimaa64.efi` entry;
  - the doctor `secure_boot` check passes.
- `KEDRA_UTM_BOOTC_PASS`: `bootc status` shows the booted `localhost/kedra-utm-test:secureboot`, `arm64`, and no staged or rollback deployment; `source.json` is `utm`/`aarch64`.
- `KEDRA_UTM_SELINUX_PASS`: Enforcing, the targeted policy, and no `enforcing=0` or `selinux=0` on the command line. AVC listings are evidence only.
- `KEDRA_UTM_UNITS_PASS`: `systemctl is-system-running --wait` reports `running` with zero failed system units, and `qemu-guest-agent.service` is active on the host-provided channel with the `hosts/utm` `--block-rpcs` list on its command line.

Any failed unit fails the run. A unit that fails only under TCG must be investigated and documented here, not silently allowed.

Documented TCG case (2026-09-25): in run [36186175108](https://github.com/Reidond/kedra/actions/runs/36186175108) `tuned.service` (tuned 2.28.0, from the Fedora bootc base) used 41.3 s of CPU in 53 s and hit Fedora's default start timeout 44 s after starting; its process then exited 0. Secure Boot, bootc and SELinux markers had passed. The test layer therefore sets `DefaultTimeoutStartSec=10min` in `/usr/lib/systemd/system.conf.d/90-kedra-research-tcg.conf`. Units must still reach active and the zero-failed-units gate is unchanged; the product image and UTM/HVF runs keep Fedora's defaults.

## Why the NVRAM boot entry exists

Fedora 44 `shim-aa64-16.1-5` has two kinds of signature:

- `shimaa64.efi` (the same file as `EFI/BOOT/BOOTAA64.EFI`) is signed only by Microsoft UEFI CA 2023.
- `fbaa64.efi` and `mmaa64.efi` are signed by a Red Hat *test* certificate.

bootc-image-builder disks carry no NVRAM entry. The firmware therefore takes the removable path: `BOOTAA64.EFI` finds `fbaa64.efi`, and shim stops with `Verification failed: (0x1A) Security Violation`.

An installed system does not use that path. Anaconda and bootupd `--update-firmware` create an entry for `\EFI\fedora\shimaa64.efi`. `boot.sh` reproduces that entry by adding it to a fresh copy of `AAVMF_VARS.ms.fd` with `virt-fw-vars --append-boot-filepath`. This writes a short-form `FilePath` entry, and edk2 expands it to the disk's EFI System Partition. The entry is placed first in `BootOrder`.

Fedora 45 `shim-16.1-7` fixes the fallback signatures. Do not remove the entry until Fedora 44 ships that fix.

## Local prototype (2026-09-25)

Run on an Apple M2 Pro, Docker 29.4.0 on OrbStack, `linux/arm64`. There was no KVM, so this is TCG like the runner. Only disposable `--rm` containers and `/tmp` scratch were used.

1. Download and extract the Fedora RPMs:
   - Command: `dnf download --repo=fedora --repo=updates shim-aa64 grub2-efi-aa64 kernel-core` in `registry.fedoraproject.org/fedora:44`.
   - `rpm -Kv` was OK with key `36f612dcf27f7d1a48a835e4dbfcf71c6d9f90a6`. The files were extracted with `rpm2cpio | cpio -idm`.
   - RPM sha256:

     | RPM | sha256 |
     |---|---|
     | `shim-aa64-16.1-5.aarch64` | `778ca345d675d05de36745a1d582828d85d77f4f78b580b7caa340cd9f0bde19` |
     | `grub2-efi-aa64-2.12-64.fc44.aarch64` | `cdd156a80ec489c7097875d6f6b97f4ba544ab88a0a13ba402a1eb82e69de7ac` |
     | `kernel-core-7.2.7-200.fc44.aarch64` | `a591f2309d2837073710838e729912d8c7b60e5dd7e6b6433361abb3f28e827d` |

2. Install the tools in `ubuntu:24.04`: `apt-get install --no-install-recommends qemu-system-arm qemu-efi-aarch64 python3-virt-firmware dosfstools mtools fdisk`. Installed versions:
   - `qemu-system-arm 1:8.2.2+ds-0ubuntu1.18`
   - `qemu-efi-aarch64 2024.02-2ubuntu0.9`
   - `python3-virt-firmware 24.1.1-2`

   Firmware files (checked on the same package versions):
   - `/usr/share/AAVMF/AAVMF_CODE.ms.fd` is a symlink to `AAVMF_CODE.secboot.fd`, sha256 `e37b6dcdce78c96f629ab7f48803e809a76df5c1c2582c6d997fad3041aadf60`.
   - `AAVMF_VARS.ms.fd` has sha256 `5266c6c60f28de37c0f478bf59b156540be05e2cd691a638dc4178e9627af4c0`.
   - Both are 64 MiB raw.
   - `/usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json` pairs these two files with the features `enrolled-keys` and `secure-boot`.
   - `AAVMF_VARS.ms.fd` contains:
     - PK: Ubuntu OVMF Secure Boot (PK/KEK key).
     - KEK: MS KEK CA 2011 and MS KEK 2K CA 2023.
     - db: MS Windows Production PCA 2011, MS Corporation UEFI CA 2011, MS UEFI CA 2023, MS Option ROM UEFI CA 2023 and Windows UEFI CA 2023.
     - SecureBootEnable: ON.
     - No `BootOrder`.
3. Build the test disk: a GPT disk with a 100 MiB `EFI-SYSTEM` partition laid out like bootupd's ESP:
   - `EFI/BOOT/`: `BOOTAA64.EFI` and `fbaa64.efi`.
   - `EFI/fedora/`: `shimaa64.efi`, `grubaa64.efi`, `mmaa64.efi`, `BOOTAA64.CSV` and a one-entry `grub.cfg`.
   - `/vmlinuz`, with no initramfs. A root-mount panic is therefore the expected end state.
4. Run both cases with `qemu-system-aarch64 -machine virt -cpu max,pauth-impdef=on -accel tcg -m 2048 -smp 2 -drive if=pflash,…,readonly=on,file=AAVMF_CODE.secboot.fd -drive if=pflash,…,file=<vars copy> -drive file=disk.img,if=virtio,format=raw,snapshot=on -nic none -no-reboot -display none -serial file:…`:

   | Case | VARS | Result |
   |---|---|---|
   | fallback | plain copy of `AAVMF_VARS.ms.fd` | `BdsDxe: starting Boot0001 "UEFI Misc Device"`, then `Verification failed: (0x1A) Security Violation`. No kernel ran; QEMU exited after 305 s. |
   | entry | copy + `virt-fw-vars --inplace vars.fd --append-boot-filepath '\EFI\fedora\shimaa64.efi'` | `BdsDxe: starting Boot0003 "file shimaa64.efi" from \EFI\fedora\shimaa64.efi`, then `Booting 'sbtest'`, `Linux version 7.2.7-200.fc44.aarch64`, `secureboot: Secure boot enabled`, `Kernel is locked down from EFI Secure Boot mode` and `integrity: secureboot mode enabled`, ending in the expected root-mount panic after 12 s. After boot, `BootOrder` was `0003, 0000, 0001, 0002`, so the entry was kept first. |

5. Full disk boot through the committed scripts:
   - Build: in a privileged `fedora:44` container with podman 5.8.7, a derivative of the local arm64 `quay.io/fedora/fedora-bootc:44` pull was built. The pull had RepoDigest `sha256:6718b0634e138d1909e752dc0f7d8a203ede6ece33bc637f72282bde3d44887b`, version `44.20260925.0` and bootc 1.16.13.
     - The derivative added `qemu-guest-agent`; `rpm -V --configfiles` was clean.
     - It added kargs.d files for `quiet` and for the utm console, and the committed observer.
     - The observer's Kedra-only lines, doctor and `source.json`, were removed because the plain base has no `sysroot`. Those checks were therefore **not exercised**.
   - The arm64 bootc-image-builder pin, `linux/arm64`, `sha256:a4779fc2307a7c2e82fda09e5c7712871fdb2dfae8a587f61d1dab32e7c4edc8`, revision `a686afed6dde14fa5444a3d3be0f269acc783470`, built the qcow2 in 118 s.
   - `tests/vm/utm/boot.sh` then ran unchanged in `ubuntu:24.04` with `GITHUB_ACTIONS=true RUNNER_OS=Linux GITHUB_REPOSITORY=Reidond/kedra`. That container is disposable, like the runner.
   - **pass**, QEMU exiting 0 after 126 s:
     - The firmware booted `Boot0003 "file shimaa64.efi"`.
     - `systemd-analyze` reported 3.2 s kernel + 25.3 s initrd + 52.5 s userspace. `graphical.target` was reached, the state was `running` with zero failed units, and `qemu-guest-agent` was active.
     - All four markers appeared, and the reported `imageDigest` equalled podman's `.Digest`.
     - `BootCurrent` was `0003`, and `BootOrder` remained `0003,0000,0001,…`.
   - Earlier attempts found two harness bugs, both since fixed:
     - The NIC needs `romfile=` because `ipxe-qemu` is not installed.
     - `bootc status --json` has no trailing newline, which hid a marker until `evidence()` added one.
   - Observed on the bib disk:
     - bootc-image-builder appends `console=ttyS0` after the kargs.d arguments. `/sys/class/tty/console/active` was `ttyAMA0 tty0`.
     - Evidence-only AVC denials:
       - `bootupctl` reading an `unlabeled_t` `/boot/bootupd-state.json` (`bootupd_t` is permissive);
       - `chcon` `mac_admin` from an `unconfined_service_t` unit.

6. Workflow rehearsal against the real utm candidate:
   - Source: a scratch copy of the working tree committed only in `/tmp`, while other owners were still editing. It is not a reviewed revision.
     - `sysroot` was built in `rust:1.98.1`. `source plan --host utm` produced 62 packages and 17 files.
   - The `run:` blocks of `test-utm-image.yml` were extracted and executed in order in a privileged `ubuntu:24.04` arm64 container with podman 4.9.3 and skopeo 1.13.3.
     - The Rust step was replaced by the prebuilt binaries.
     - Agent, Bitwarden and base preparation passed. The base resolved to arm64 `sha256:9f81b011…6fd6`.
     - The candidate build passed in about 8 min, with bootc lint `Checks passed: 11`.
     - `image-check.sh` **pass**.
   - Nested podman in that container could not create bib's cgroup. bootc-image-builder therefore ran without `sudo` from a Fedora 44 podman 5.8.7 container on the same storage, in 339 s for a 2.9 GB qcow2. This is a rehearsal-environment limit, not a runner result.
   - Step 7 then ran as written: **pass** in 237 s.
     - `systemd-analyze` reported 4.2 s kernel + 28.9 s initrd + 2 min 31.8 s userspace, reaching `graphical.target` with zero failed units.
     - The doctor `secure_boot` check passed as the unprivileged account (`UEFI Secure Boot is enabled; kernel lockdown: integrity`).
     - The digest matched and every marker appeared.
     - The same two AVC sources as above appeared: 3 `bootupctl`, 8 `chcon`.

The runner is slower than an M2 core. The `boot.sh` bound (90 min) and step bound (100 min) leave wide margins.

To reproduce, run `boot.sh` as above in a disposable `ubuntu:24.04` arm64 container after installing `qemu-system-arm qemu-efi-aarch64 qemu-utils python3-virt-firmware jq`. Give it the disk, a new work directory, an evidence directory and the image digest. Never run it on a workstation host or against a real disk.

## Status

As of 2026-09-25 the workflow has not run in Actions (`not-run`). The local prototype and rehearsal are emulation evidence only; they are not a runner, UTM or hardware result.

The guest-agent RPC block list and its checks were added after that rehearsal. The `image-check.sh` block passed locally against `quay.io/fedora/fedora-bootc:44` (arm64, `sha256:6718b063…887b`) with `qemu-guest-agent-10.2.2-1.fc44` and the drop-in installed, not against a utm candidate. The `check.sh` command-line assertion has not run in a booted guest (`not-run`).
