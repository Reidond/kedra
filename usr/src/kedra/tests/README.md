# End-to-end verification

Use end-to-end and manual testing only. No unit/model/mock tests, doctests, source assertions or repository self-scanners. The container harness in [container/](container/README.md) is the one sanctioned test runner (owner decision 2026-09-27). Standard formatting, Clippy and release builds remain required.

Test layers:

- **Host CLI checks.** Cargo E2E and the Python release checks below, with no OS image.
- **Container scenarios** in [container/](container/README.md). The OS image under test boots with systemd as PID 1, and the real Kedra session runs nested in a headless compositor.
- **Disposable VMs** for what a container cannot host: firmware and Secure Boot, SELinux enforcement, VT/greetd password login and PAM keyring unlock, bootc switch/update/rollback, the installer, and the Xwayland/Qt file choosers, which get no virtual-keyboard input in the container session.

```sh
ruff check                       # every Python script; pinned via mise.toml
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
uv run usr/src/kedra/tests/cli/release-material.py --workdir target/release-material
# Installed system in containers (needs a Docker Engine; builds or pulls the image under test):
cargo test -p kedra-container-tests --test container --locked
```

Cargo E2E exercises the public CLI with real subprocesses, Git and generated data. release-interop.py checks `sysroot release key` and public trust preparation against independently generated OpenSSL keys. Linux is required for installed behavior; Windows compilation alone does not qualify a desktop.

## Local testing and sign-off

No Actions workflow tests a change (owner decision 2026-10-08). Run the checks a change needs on its pushed commit with a clean tree, record the results in the worklog, then sign off; main's branch protection requires the `signoff` commit status on the pull request head:

```sh
uv run usr/src/kedra/tests/signoff.py --checks "fmt ruff cccc clippy e2e build interop material container"
```

`signoff.py` runs no checks. It refuses local changes and a commit GitHub does not hold, then sets `signoff` on HEAD with the GitHub CLI under your identity. Every new commit needs its own sign-off. Name only the checks that passed; [AGENTS.md](../../../../AGENTS.md) says which a change needs and how agents without the GitHub CLI or a container engine hand the sign-off to the owner.

The release workflow keeps one test: before signing, it runs the complete container harness against each exact candidate digest with no overlay, because nightly Fedora package refreshes produce images that no sign-off covered.

## Boot-level drivers

These drivers cover what a container cannot host. They run locally: on a disposable Linux host of the stated architecture (Ubuntu 24.04 packages: `podman skopeo qemu-system-x86|qemu-system-arm qemu-utils ovmf|qemu-efi-aarch64 python3-virt-firmware openssl jq e2fsprogs`, plus `xvfb xauth xdotool imagemagick` for the desktop), or, for the aarch64 Secure Boot boot, through `kedra-lab vm` on the owner's Mac.

**Disposable-host guard.** The Linux drivers run privileged Podman and bootc-image-builder, and ghcr-update also maps `ghcr.io` in `/etc/hosts` and installs a test CA. Each one therefore requires `--disposable-host`, and `common/qemu.py` refuses the host unless it is Linux with no bootc, ostree or Kedra state: `/run/ostree-booted`, `/sysroot/ostree`, `/ostree`, `/usr/share/sysroot`, `/usr/lib/sysroot`, `/usr/libexec/sysroot` or an `ostree=`/`composefs=` kernel argument in `/proc/cmdline`, which is the host kernel's even inside a container. The guard only inspects paths; nothing calls bootc on the host. Use a throwaway VM or cloud host, never a workstation. `uv run usr/src/kedra/tests/common/qemu.py guard --disposable-host` shows the decision.

**One launcher and one guest check.** `common/qemu.py` builds every VM's UEFI Secure Boot machine: Ubuntu OVMF (x86_64, SMM) or AAVMF (aarch64) with a private Microsoft-enrolled variable store from `common/secure_boot.py`, KVM when `/dev/kvm` is usable and TCG otherwise, a QMP socket and an owned process group. `kedra-lab vm` uses the same launcher with HVF and its private runtime. Every fixture installs `common/secure_boot_check.py`, the one guest check that prints `KEDRA_SECUREBOOT_PASS`; `common/disk.py` imports a candidate into rootful Podman, layers a driver's observer fixture, admits it through a generated signing key (the candidate enforces its signature policy at install; [details](vm/qemu-arm64/README.md)) and builds the generated disk.

A candidate is `docker:REF` (for example the full local build `kedra-lab image --image build --target <target> --overlay none` prints), `podman:REF` already in rootful Podman, or a registry reference such as `ghcr.io/reidond/kedra-qemu-arm64:stable`.

| Driver | Coverage | Host |
|---|---|---|
| `usr/src/kedra/tests/agents/run.py` | Real agent launcher/runtime/profile behavior with generated profiles | An installed Kedra or lab image (bundled Codex catalog package) and a separately obtained official Codex as `--personal` |
| `vm/ghcr-update/run.sh --disposable-host --work DIR` | Direct signed registry check/stage through bootc's signature policy, A/B/rollback with hold, identity recovery and signature, scope and downgrade refusals, under UEFI Secure Boot; firmware refusal of the same disk without Microsoft keys | x86_64 with KVM, after `cargo build --workspace --release --locked` |
| `vm/ghcr-update/run-hvf.py --binaries DIR --work NEW_DIR` | The same guest cases for the qemu-arm64 target | Apple Silicon Mac: local Docker engine and the `kedra-lab vm tools prepare` runtime (HVF); DIR holds Linux aarch64 `sysroot` and `sysroot-helper` |
| `vm/ghcr-update/run-arm64.sh --disposable-host --work DIR` | The composed qemu-arm64 image: public installer refusals, encrypted Anaconda installation and signed A/B/A with persistent home and data | aarch64, KVM or TCG; `--local-fixture` mode runs in the controller container ([RELEASES](../docs/RELEASES.md)) |
| `vm/desktop/run.py --disposable-host --work DIR --candidate REF` | tuigreet password login on VT 1, PAM login-keyring unlock, the session on the virtio GPU, doctor's full session gate and the Xwayland and Qt 5/6 file choosers, under Secure Boot with SELinux enforcing | x86_64 with KVM |
| `vm/qemu-arm64/run.py --disposable-host --work DIR --candidate REF` | The bootc contract of the `qemu-arm64` candidate and a UEFI Secure Boot boot through an explicit `\EFI\fedora\shimaa64.efi` NVRAM entry ([vm/qemu-arm64](vm/qemu-arm64/README.md)) | aarch64, KVM or TCG |
| `kedra-lab vm up` then `kedra-lab vm check` | The lab fixture's HVF Secure Boot boot and the shared guest check, read back for the running boot ([container/qemu](container/qemu/README.md)) | Apple Silicon Mac |
| `rpm-refresh/run.py` | Native signed-RPM change/no-change and failure cases | x86_64 with Podman |

Test fixtures live in directories named after their former workflows (vm/<workflow>/, agents/, rpm-refresh/); the container harness, its lab layer and guest probes live in container/; shared VM helpers and the Fedora base resolver live under common/, and the release CLI checks under cli/. Local installer construction uses usr/src/kedra/installer/build-local.py; optional --smoke boots the ISO diskless through UEFI Secure Boot (KVM required) and checks the Secure Boot state, offline verification and Anaconda startup without uploading media. Manual exact-media installation must select one generated disk, preserve another sentinel disk, enable encryption, create an administrator, boot without ISO, check exact digest/policy and desktop health, then compare the untouched disk after shutdown.

Production refresh compares resolved inputs, including the run's RPM lock pins, against the signed stable OCI image and requires the candidate RPM inventory to equal that lock. `e2e_rpm_lock` covers the lock: fixed-output fetch/repository refusals by default, and a signed fixture repository and real Fedora with `--ignored`. Native RPM fixtures exercise equality, changes and failure boundaries. No-change publishes nothing. The retired filesystem observer is not a production dependency.

The ghcr-update driver uses a disposable local TLS registry mapped to the fixed GHCR hostname, generated keys and actual signed A/B VM boots. It invokes `identity-recovery.py` to prove that identity mismatch blocks forward work while retained rollback remains available, and counts registry layer downloads to show that refusals fetch no layer and staging fetches each new layer once. `run.sh` and `run-hvf.py` use a Fedora bootc base with test files, not the full image. `run-hvf.py` builds and signs in a privileged controller container of the local Docker engine, installs signed A with A's own `bootc install to-disk` and boots it with the kedra-lab QEMU runtime under HVF; the guest reaches the registry and controller on the Mac's loopback through 10.0.2.2.

Deterministic tag-race, interrupted-helper/bootc operation and native OCI-platform-mismatch cases remain explicitly not-run. Legacy-state migration is not required: the owner confirmed no r1/r2 system was installed. The wrong-architecture identity case does not substitute for an actual wrong-platform OCI image. Raising the shared minimum bootc version requires native qualification, even when compiler and image-input checks pass.

## UEFI Secure Boot in the VM drivers

Every x86_64 boot uses Ubuntu noble's SMM Secure Boot firmware, `/usr/share/OVMF/OVMF_CODE_4M.secboot.fd`, with a private copy of `OVMF_VARS_4M.ms.fd`: `-machine q35,smm=on`, `-global driver=cfi.pflash01,property=secure,value=on` and a writable pflash varstore. That template has Ubuntu's OVMF PK, Microsoft KEKs and a db with Microsoft Corporation UEFI CA 2011, which is the only CA signing Fedora 44's shim-x64 16.1-5. `common/secure_boot.py` (needs `python3-virt-firmware`) does three things:

- Writes `secure-boot-firmware.json` and `secure-boot-vars-{microsoft,snakeoil}.txt` into each driver's evidence. These record package versions, file hashes, Ubuntu's enrolled-keys QEMU descriptor and the PK/KEK/db certificates.
- Creates variable stores without ever overwriting one.
- Refuses a persisted store whose PK, KEK, db, dbx, SecureBootEnable or CustomMode differ from the Microsoft template. `desktop/session.py --firmware-vars` and `qemu.py boot` check this before every boot, and `qemu.py boot` again after it.

aarch64 boots use Ubuntu's `AAVMF_CODE.secboot.fd` with `AAVMF_VARS.ms.fd` (`secure_boot.py --architecture aarch64`), whose db must hold Microsoft UEFI CA 2023, the only CA signing Fedora 44's shimaa64; the launcher adds the `\EFI\fedora\shimaa64.efi` entry ([why](vm/qemu-arm64/README.md#why-the-nvram-boot-entry-exists)).

Every guest boot runs `common/secure_boot_check.py`, which prints `KEDRA_SECUREBOOT_PASS` after four checks, and each host runner requires that marker:

- `mokutil --sb-state` is exactly `SecureBoot enabled`.
- The SecureBoot efivar is 1 and SetupMode is 0.
- `/sys/kernel/security/lockdown` selects integrity or confidentiality.
- `journalctl -k -b` contains `secureboot: Secure boot enabled`. The desktop's `quiet` karg hides this from the console, but not from the journal.

With `--boot-entry shimaa64.efi` (qemu-arm64, the native lab and its boot observer) it also requires that firmware's `BootCurrent` entry names that loader.

One negative case runs before the positive boots of `vm/ghcr-update/run.sh`, through `qemu.py refuse` (it moved there from the removed signed-update driver). It boots the same disk read-only with the snakeoil-only store, no network and a 120 s bound. It requires three results: the firmware's `Access Denied` for the boot option, QEMU still at the firmware boot menu when the bound ends, and no `Linux version` or `KEDRA_` line.

bootc-image-builder disks have no NVRAM boot entry, so the first boot takes the removable path. Shim's fallback then adds `\EFI\fedora\shimx64.efi` and boots it; the runners do not pass `-no-reboot` in case fallback resets. A local TCG smoke on 2026-09-25 exercised this path: Ubuntu ovmf 2024.02-2ubuntu0.9 booted Fedora 44 shim-x64 16.1-5, grub2-efi-x64 2.12-64 and kernel 7.2.7-200 (`quiet` set) into a minimal initramfs that ran the guest check function, then booted again through the new entry. The negative snakeoil store was refused; the protocol-1 signed-update driver that repeated that refusal on every run was removed with its protocol on 2026-10-14, and `qemu.py refuse` now repeats it in ghcr-update (above). The full images were not booted locally; the runners' KVM boots are the qualification.

This qualifies only the firmware → shim → GRUB → kernel chain and kernel lockdown. The initramfs, kernel command line and composefs root are not signed. No TPM is attached. Physical firmware, which may lack the 2011 CA or disable the third-party CA, is a separate qualification.

Keep generated logs, screenshots and results in disposable output directories; do not commit research output. Never use real homes, vault material, transcripts or production signing keys as fixtures. Historical evidence remains in Git history and [STATUS](../docs/STATUS.md).
