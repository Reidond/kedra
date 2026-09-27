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

Cargo E2E exercises the public CLI with real subprocesses, Git and generated data. check.yml runs these commands natively on x86_64 and aarch64. release-interop.py uses independent OpenSSL signatures. Linux is required for installed behavior; Windows compilation alone does not qualify a desktop.

| Workflow | Coverage |
|---|---|
| test-container.yml | Container scenarios and native tests on this commit's candidate, built natively for desktop (x86_64) and utm (aarch64). Covers image contents, the utm guest-agent block list and graphics defaults, the session's services, environment, logind session and portals, Noctalia palette and wallpaper, XDG autostart, doctor, Bitwarden, the bundled Codex, the R03/R04 home review/discard/recovery and niri line review, and GTK 3/libadwaita file choosers, with screenshots ([container](container/README.md)) |
| test-agents.yml | Real agent launcher/runtime/profile behavior with generated profiles, natively for desktop (x86_64) and utm (aarch64) |
| test-ghcr-update.yml | Direct signed registry v2 enrollment/check/stage, A/B/rollback, identity recovery and critical refusals, under UEFI Secure Boot |
| test-signed-update.yml | Signed bootc update, negative authority/replay cases and retained rollback under UEFI Secure Boot; firmware refusal of the same disk without Microsoft keys |
| test-desktop.yml | Under UEFI Secure Boot with SELinux enforcing: real tuigreet password login on VT 1, PAM login-keyring unlock, the session on the virtio GPU through niri's DRM backend, doctor's full session gate (`selinux` and `secure_boot` required), and the Xwayland and Qt 5/6 KDE file choosers driven by QEMU key events |
| test-home-transition.yml | Actual signed A/B/A home-baseline acceptance and rollback under UEFI Secure Boot |
| test-utm-image.yml | The bootc contract of the aarch64 `utm` candidate and a TCG (no KVM on hosted arm64 runners) UEFI Secure Boot boot of a disposable disk through an explicit `\EFI\fedora\shimaa64.efi` NVRAM entry with Microsoft-enrolled AAVMF variables ([vm/utm-image](vm/utm-image/README.md)) |
| test-rpm-refresh.yml | Native signed-RPM change/no-change and failure cases |

Test fixtures live in directories named after their workflows (vm/<workflow>/, agents/, rpm-refresh/); the container harness, its lab layer and guest probes live in container/; shared VM helpers and the Fedora base resolver live under common/, and the release CLI checks under cli/. OS image tests run in Actions. Local installer construction uses usr/src/kedra/installer/build-local.py; optional --smoke boots the ISO diskless through UEFI Secure Boot (KVM required) and checks the Secure Boot state, offline verification and Anaconda startup without uploading media. Manual exact-media installation must select one generated disk, preserve another sentinel disk, enable encryption, create an administrator, boot without ISO, check exact digest/policy and desktop health, then compare the untouched disk after shutdown.

Production refresh compares resolved inputs against the signed stable OCI image and requires the candidate RPM inventory to match its preflight. Native RPM fixtures exercise equality, changes and failure boundaries. No-change publishes nothing. Historical release-file interoperability remains only where needed for offline/legacy compatibility. The retired filesystem observer and GitHub ISO workflow are not production dependencies.

`test-ghcr-update.yml` uses a disposable local TLS registry mapped to the fixed GHCR hostname, generated keys and actual signed A/B VM boots. It invokes `identity-recovery.py` to prove that identity mismatch blocks forward work while retained rollback remains available. Native runs pass (for example 34820938156 at `17105c5`); the fixture image is a Fedora bootc base with test files, not the full desktop image.

Deterministic tag-race, interrupted-helper/bootc operation and native OCI-platform-mismatch cases remain explicitly not-run. Legacy-state migration is not required: the owner confirmed no r1/r2 system was installed. The wrong-architecture identity case does not substitute for an actual wrong-platform OCI image. Shared bootc compatibility changes require native qualification before signing, even when compiler and image-input checks pass.

## UEFI Secure Boot in the x86_64 VM workflows

Every boot in the four x86_64 VM workflows uses Ubuntu noble's SMM Secure Boot firmware, `/usr/share/OVMF/OVMF_CODE_4M.secboot.fd`, with a private copy of `OVMF_VARS_4M.ms.fd`: `-machine q35,smm=on`, `-global driver=cfi.pflash01,property=secure,value=on` and a writable pflash varstore. That template has Ubuntu's OVMF PK, Microsoft KEKs and a db with Microsoft Corporation UEFI CA 2011, which is the only CA signing Fedora 44's shim-x64 16.1-5. `common/secure_boot.py` (needs `python3-virt-firmware`) does three things:

- Writes `secure-boot-firmware.json` and `secure-boot-vars-{microsoft,snakeoil}.txt` into each workflow's evidence. These record package versions, file hashes, Ubuntu's enrolled-keys QEMU descriptor and the PK/KEK/db certificates.
- Creates variable stores without ever overwriting one.
- Refuses a persisted store whose PK, KEK, db, dbx, SecureBootEnable or CustomMode differ from the Microsoft template. `run_vm.py --firmware-vars` and the multi-boot runners check this before every boot.

Every guest boot runs four checks and prints `KEDRA_SECUREBOOT_PASS`, which each host runner requires:

- `mokutil --sb-state` is exactly `SecureBoot enabled`.
- The SecureBoot efivar is 1 and SetupMode is 0.
- `/sys/kernel/security/lockdown` selects integrity or confidentiality.
- `journalctl -k -b` contains `secureboot: Secure boot enabled`. The desktop's `quiet` karg hides this from the console, but not from the journal.

`test-signed-update.yml` also runs one negative case. It boots the same disk with the snakeoil-only store, no network and a 120 s timeout. It requires three results: the firmware's `Access Denied` for the boot option, QEMU still at the firmware boot menu when the timeout ends it (exit 124), and no `Linux version` line.

bootc-image-builder disks have no NVRAM boot entry, so the first boot takes the removable path. Shim's fallback then adds `\EFI\fedora\shimx64.efi` and boots it; the runners do not pass `-no-reboot` in case fallback resets. A local TCG smoke on 2026-09-25 exercised this path: Ubuntu ovmf 2024.02-2ubuntu0.9 booted Fedora 44 shim-x64 16.1-5, grub2-efi-x64 2.12-64 and kernel 7.2.7-200 (`quiet` set) into a minimal initramfs that ran the guest check function, then booted again through the new entry. The negative store was refused. The full images were not booted locally; the runners' KVM boots are the qualification.

This qualifies only the firmware → shim → GRUB → kernel chain and kernel lockdown. The initramfs, kernel command line and composefs root are not signed. No TPM is attached. Physical firmware, which may lack the 2011 CA or disable the third-party CA, is a separate qualification.

Keep generated logs, screenshots and results in Actions artifacts or disposable output directories; do not commit research output. Never use real homes, vault material, transcripts or production signing keys as fixtures. Historical evidence remains in Git history and [STATUS](../docs/STATUS.md).
