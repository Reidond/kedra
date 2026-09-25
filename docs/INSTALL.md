# Build and install local media

Kedra publishes signed container images per target: `ghcr.io/reidond/kedra-desktop` (x86_64) and `ghcr.io/reidond/kedra-utm` (aarch64, an Apple Silicon Mac running UTM). Installation media is constructed locally from an explicitly reviewed digest. No GitHub Release or ISO download is required.

## Secure Boot prerequisites

UEFI Secure Boot is required. The installer refuses to start Anaconda unless the media booted through UEFI with Secure Boot enabled and a platform key enrolled (not setup mode). The helper's embedded-payload verification checks the SecureBoot and SetupMode variables and prints the reason for a refusal on the installer console. Both Anaconda entry points (`anaconda.service` and the `inst.notmux` `anaconda-direct.service`) require that verification and independently assert `AssertSecurity=uefi-secureboot`. Legacy BIOS boot of the hybrid ISO is refused as well. If Anaconda does not start, read the console message, or switch to the tty2 shell and run `journalctl -b -u kedra-installer-verify`. On an installed system, `sysroot doctor` fails its required `secure_boot` check without Secure Boot. The system still boots, so local recovery remains available, and `sysroot update` does not refuse.

Configure the firmware before installing:

- **Enable UEFI Secure Boot** in user/deployed mode, not setup mode.
- **Trust the Microsoft third-party UEFI CA.** On Secured-core PCs, turn on "Allow Microsoft 3rd party UEFI CA". Fedora 44's shims carry one signature each, so the matching CA must be in the firmware's `db`:
  - x86_64 shim-x64 16.1-5 is signed only through Microsoft Corporation UEFI CA 2011. Firmware does not enforce that CA's 2026-06-27 expiry for enrolled certificates.
  - aarch64 shim-aa64 16.1-5 is signed only through Microsoft UEFI CA 2023.
- **Check `db` if unsure:** run `mokutil --db` from Linux live media, or `Get-SecureBootUEFI db` on Windows. Fedora 44 x86_64 live media uses the same shim, so a Secure Boot boot of it is a quick pre-check. The Fedora 44 GA aarch64 netinst ISO is not a valid pre-check: its kernel is unsigned.

Secure Boot here covers firmware, shim, GRUB and the kernel, plus kernel lockdown. It does not sign the initramfs, the kernel command line or the composefs root, so it is not verified boot.

## Build hosts

The pinned image-builder only builds media for its own architecture. Use a build host that matches the target:

| Target | Image repository | Build host |
|---|---|---|
| desktop | `ghcr.io/reidond/kedra-desktop` | x86_64 Linux |
| utm | `ghcr.io/reidond/kedra-utm` | aarch64 Linux, or an Apple Silicon Mac through [installer/utm](../installer/utm/README.md) |

A Linux build host needs Python 3.11+, sudo, rootful Podman using its default `/var/lib/containers/storage`, Podman's Netavark network helper, Skopeo and OpenSSL already installed. On Ubuntu, include the `netavark` package explicitly when installing Podman with `--no-install-recommends`; omitting it can prevent cleanup of inspection containers. Allow sufficient temporary disk space for the payload, Anaconda image and ISO. The script does not install prerequisites or change host trust policy.

## Build one ISO

Use a trusted Kedra checkout and independently confirm the target's public-key fingerprint. The script checks `build/release/authority/<target>.pub` against `<target>.sha256` and prints the value it uses:

```text
desktop  a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
utm      76ca7a65915adb1907acbe0885af83c5c569dd2964b87decbfb67059dee366c6
```

Review the signed image's exact digest through the completed Actions signing result or [image verification](RELEASES.md). Substitute that full digest:

```sh
python3 installer/build-local.py --image ghcr.io/reidond/kedra-desktop@sha256:REVIEWED_DIGEST --output-dir /absolute/path/to/new-installer
python3 installer/build-local.py --image ghcr.io/reidond/kedra-utm@sha256:REVIEWED_DIGEST --output-dir /absolute/path/to/new-installer
```

The repository selects the target. Other repositories, tags and cross-architecture builds are refused. The output directory must not already exist.

The script checks the fixed public fingerprint and strict native container signature policy before extracting the image's helper, source and trust. It then requires the payload, builder and Anaconda environment to have the target's OCI architecture and the signed scope to name that target. It uses the image's recorded Fedora base and the per-architecture digest-pinned builder in installer/inputs.json. Legacy images without recorded resolved inputs additionally require an explicitly reviewed `--base-image quay.io/fedora/fedora-bootc@sha256:BASE_DIGEST`.

The result is one complete `kedra-<target>-44-<first 16 digest hex>.iso`, `installer.json` and `SHA256SUMS` in the selected directory. No existing output is overwritten and nothing is uploaded. Local checksum files describe the built ISO; they are not release signatures. The embedded signed OS payload is independently verified offline before installation.

Rootful Podman retains its normal image/build cache. Temporary build data is removed after success and retained with its reported location after failure. Do not share the container store with another image-build operation while creating media.

For an additional diskless startup check on a host with usable KVM, add `--smoke`:

- **x86_64:** install QEMU and Debian/Ubuntu `ovmf`. The check uses `OVMF_CODE_4M.secboot.fd` with a copy of the Microsoft-enrolled `OVMF_VARS_4M.ms.fd`.
- **aarch64:** install QEMU and `qemu-efi-aarch64`. The check uses `AAVMF_CODE.secboot.fd` with a copy of `AAVMF_VARS.ms.fd`.

The ISO boots as a CD through UEFI Secure Boot without disks. The check passes only if the guest reports Secure Boot enabled with kernel lockdown, successful embedded signature verification and Anaconda startup. It does not perform an installation. Apple M1/M2 Macs have no KVM, so the macOS wrapper builds without `--smoke`.

## UTM on an Apple Silicon Mac

The `utm` target runs in UTM's QEMU backend with UEFI boot and a TPM. With both enabled, UTM uses its Secure Boot firmware with a variable store that enrolls the Microsoft UEFI CAs. `installer/utm/kedra-utm.py` runs with the macOS `/usr/bin/python3` and has four steps:

```sh
python3 installer/utm/kedra-utm.py check-host --automation
mkdir -p ~/Kedra
python3 installer/utm/kedra-utm.py iso --image ghcr.io/reidond/kedra-utm@sha256:REVIEWED_DIGEST --output ~/Kedra/iso-REVIEWED
python3 installer/utm/kedra-utm.py create --iso ~/Kedra/iso-REVIEWED/kedra-utm-44-DIGEST16.iso
# install in UTM as below, shut the VM down, then:
python3 installer/utm/kedra-utm.py detach-installer --bundle ~/VMs/Kedra.utm
```

- `check-host` is read-only. `--automation` additionally runs `utmctl list` (this starts UTM hidden) and fails if macOS blocks Automation access to UTM.
- `iso` runs the unchanged `build-local.py` (without `--smoke`) in a disposable arm64 Linux container on the local Docker engine and checks `SHA256SUMS` on macOS. The parent of `--output` must exist; the output directory itself must not.
- `create` makes a UTM bundle with UEFI, TPM, the keyed Secure Boot variable store, `virtio-gpu-gl-pci`, the ISO as a read-only CD, an empty VirtIO system disk and a serial console in UTM's built-in terminal. A localhost TCP serial port is added only with `--serial-port`; anyone who can connect to it has the VM's physical-console access, including firmware setup and GRUB editing.
- `detach-installer` removes the installer drive from the stopped VM and asks UTM to reload it through `utmctl` and AppleScript. If macOS denies Automation access (Apple Events error -1743) or you work over SSH, allow the terminal app under System Settings > Privacy & Security > Automation, or quit UTM and rerun with `--utm-quit`.

See [installer/utm/README.md](../installer/utm/README.md) for the exact flow, graphics notes, guest-agent limits and serial console.

Fedora 44's shim-aa64 16.1-5 ships test-signed fallback and MokManager binaries. The installer ISO does not include the fallback binary, so it boots normally. The installed disk does include it, so it boots only through the NVRAM entry the installer creates for `\EFI\fedora\shimaa64.efi`; the removable-media fallback path fails with a Security Violation. Do not reset the VM's UEFI variables or queue MOK requests. To recover, add a boot option for `\EFI\fedora\shimaa64.efi` in the firmware's Boot Maintenance Manager.

## Install deliberately

1. Confirm the firmware settings above. Attach the local ISO to a disposable UEFI Secure Boot VM, or write it to a deliberately selected USB using a trusted image-writing tool.
2. Select only the intended system disk in Anaconda. Verify its identity and capacity; leave data disks unselected.
3. Enable encryption, retain its recovery passphrase and create an administrative owner account. No disk or password is preset.
4. Review pending changes, then install the offline-verified payload.
5. Shut down, remove the ISO/USB, boot the installed disk and sign in.

The installed system uses enforcing SELinux; installer-media policy is separate. Check `sysroot doctor` (including `secure_boot`) and `sysroot update status` after login. Home remains writable and system root read-only.

After a fresh installation, use `sysroot update enroll`, then the [update workflow](UPDATES.md).

Physical hardware and each target's Secure Boot installation require separate qualification. See [status](STATUS.md) for actual tested media and remaining limits.
