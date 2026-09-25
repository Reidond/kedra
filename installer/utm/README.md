# Kedra `utm` target on an Apple Silicon Mac

`kedra-utm.py` is owner tooling for the aarch64 `utm` target: an Apple Silicon Mac
running UTM 5.0.6 or newer with the QEMU backend (HVF, `virtio-gpu-gl-pci`, UEFI
Secure Boot through TPM and a Microsoft-keyed variable store). It runs with the
macOS `/usr/bin/python3` (3.9+) and uses only the standard library.

It creates files only under the directories you name and otherwise invokes UTM,
`utmctl`, AppleScript and the local Docker engine. It never touches the Mac's own
disks or bootloader, never needs sudo, never overwrites existing output and never
uploads anything. UTM 5.0.x is a pre-release series. This flow has no qualification
record yet; see [status](../../docs/STATUS.md) before relying on it.

```sh
python3 installer/utm/kedra-utm.py check-host --automation
mkdir -p ~/Kedra  # the parent of --output must exist; the output directory must not
python3 installer/utm/kedra-utm.py iso --image ghcr.io/reidond/kedra-utm@sha256:REVIEWED_DIGEST --output ~/Kedra/iso-REVIEWED
python3 installer/utm/kedra-utm.py create --iso ~/Kedra/iso-REVIEWED/kedra-utm-44-DIGEST16.iso
# install in UTM, shut the VM down, then:
python3 installer/utm/kedra-utm.py detach-installer --bundle ~/VMs/Kedra.utm
```

## Prerequisites

- An Apple Silicon Mac. UTM 5.0.6 or newer in `/Applications` (or pass `--utm-app`).
- A native linux/arm64 Docker engine. OrbStack was the qualified engine for the
  privileged nested-Podman build; others produce a warning.
- A trusted Kedra checkout whose path has no `:`, `,`, `"` or newlines. It must
  contain `build/release/authority/utm.pub` and `utm.sha256`.
- Free space: 40 GiB inside the Docker VM; on the Mac, 8 GiB for the ISO plus room
  for the VM disk to grow.

`check-host` is read-only. It reports the UTM version and its Secure Boot firmware
(`edk2-aarch64-secure-code.fd`, `edk2-arm-secure-vars.fd`) with SHA-256. It also
checks Docker's platform, free space and the checkout, and exits non-zero if a
required item is missing.

`check-host --automation` also runs `utmctl list`, which starts UTM hidden and may
show macOS's Automation prompt. It fails if macOS does not let the terminal app
control UTM (Apple Events error -1743). `detach-installer` needs that access unless
UTM is quit. To allow it, open System Settings > Privacy & Security > Automation and
enable UTM under your terminal app. macOS asks only once. After a refusal, run
`tccutil reset AppleEvents <terminal bundle id>` (for example `com.apple.Terminal`)
and retry. `utmctl` and AppleScript never work from SSH sessions or before login.

## Build the installer ISO: `iso`

Review the signed image digest first ([image verification](../../docs/RELEASES.md)).
`iso` accepts only an exact `ghcr.io/reidond/kedra-utm@sha256:` digest and a new output
directory. It prints the checkout's utm public-key fingerprint; confirm it independently.

The tool starts one disposable, privileged Fedora 44 arm64 container on the local
Docker engine. The container is pinned by digest in `inputs.json` and runs
`build-in-container.sh`, which does the following:

- delegates cgroup v2 controllers inside the container's private cgroup namespace,
  for nested crun;
- installs Podman, Skopeo, OpenSSL, sudo, Python and Netavark from Fedora's signed
  repositories;
- creates a non-root `kedra-build` user with non-interactive sudo;
- runs the unchanged `installer/build-local.py` from the checkout, which is mounted read-only.

All of `build-local.py`'s checks apply unchanged. They include the fixed-key
signature policy on pull, target/architecture/scope checks and the pinned arm64
image-builder. The output lands on a per-run Docker volume. It is then copied to
your directory and verified on macOS against `installer.json` and with
`shasum -a 256 -c SHA256SUMS`. `--smoke` is not offered: the Docker VM has no KVM
(M1/M2 Macs have no nested virtualization), so `installer.json` records no diskless smoke.

Volumes and cleanup:

- Rootful Podman storage stays in the Docker volume `kedra-utm-podman-storage`,
  which caches the payload and builder. Remove it with
  `docker volume rm kedra-utm-podman-storage`.
- A failed or interrupted run keeps its scratch volume `kedra-utm-iso-out-*` for
  inspection; the tool prints its name.
- Only one build runs at a time.

The privileged container is root inside Docker's Linux VM, not on macOS. It needs
the VM's loop devices for osbuild. Do not use a Docker engine VM that holds
unrelated privileged workloads.

A build that fails immediately with `failed to open 2048 locks in /libpod_lock` was
observed after an amd64 builder had run in the same Docker VM. image-builder shares
the VM-wide devtmpfs, and the lock layout differs by architecture. Restart the
Docker VM, then retry. Do not alternate amd64 and arm64 media builds in one VM.

## Create the VM: `create`

`create --iso PATH [--name Kedra] [--dir ~/VMs] [--disk-gib 96] [--memory-mib 8192] [--cpus 6] [--serial-port PORT] [--no-register]`

The ISO's `installer.json` and `SHA256SUMS` must sit next to it and name a
`kedra-utm` image. The tool refuses an existing `<dir>/<name>.utm`. On any failure it
removes only the bundle it just created. The bundle contains:

- `config.plist` (UTM `ConfigurationVersion` 4), written with `plistlib` and checked
  with `plutil -lint`:
  - QEMU backend, `aarch64` `virt`, `Hypervisor`, `UEFIBoot`, `TPMDevice`, RNG, balloon;
  - `virtio-gpu-gl-pci` with dynamic resolution;
  - shared network with a random locally administered MAC;
  - a serial console in UTM's built-in terminal (a localhost TCP server only with
    `--serial-port`; see [serial console](#graphics-serial-console-and-guest-agents));
  - `intel-hda` sound and clipboard sharing;
  - no additional QEMU arguments.
- `Data/efi_vars.fd`: a byte-identical copy of UTM's `edk2-arm-secure-vars.fd`.
- `Data/kedra-system.qcow2`: created empty by Fedora's `qemu-img` in the pinned
  container, then streamed out, so no host directory is mounted.
- `Data/kedra-utm-44-*.iso`: an APFS clone (`cp -c`), re-hashed against
  `installer.json`. Attached read-only as a USB CD and listed first, so it boots first.

The system disk is VirtIO with serial `KEDRASYSTEM`. In Anaconda it is the only disk,
and the installed system sees it as `/dev/disk/by-id/virtio-KEDRASYSTEM`.

`create` then runs `open -a UTM` on the bundle, which registers it in place as a
shortcut. Start the VM from UTM or with `utmctl start <UUID>`. Install as in
[INSTALL.md](../../docs/INSTALL.md): select `KEDRASYSTEM`, enable encryption and create
the owner account.

When Anaconda finishes, shut the VM down instead of rebooting. With the installer
still attached, the VM boots the ISO again. Then run `detach-installer`.

## Detach the installer: `detach-installer`

The VM must be stopped. The tool checks this in two ways:

- `utmctl status <UUID>` must report `stopped`. The first run may ask for macOS
  Automation permission. If `utmctl` cannot control UTM, the tool shows `utmctl`'s
  own error and the Automation fix from `check-host --automation`.
- `lsof` must show that no process holds the disk, variable store, TPM state or ISO.

The tool then does the following:

1. Removes the `KEDRA-INSTALLER` drive from `config.plist`, leaving every other key
   unchanged. It writes a new file, checks it with `plutil -lint` and renames it
   into place.
2. Deletes the cloned ISO.
3. Asks UTM to `reload configuration`.

The original ISO in your output directory is untouched.

Without Automation access, for example over SSH or after declining the prompt, quit
UTM (UTM > Quit UTM, which also stops its VMs) and use `--utm-quit`. The tool refuses
while any process named `UTM` runs (`pgrep -x UTM`), keeps the open-file check and
edits `config.plist` without contacting UTM. UTM reads the bundle again at its next
start, so no reload is needed. Do not start UTM while the command runs.

`--unregistered` is only for bundles created with `--no-register` that UTM has never
opened. It skips `utmctl` and the reload, but keeps the open-file check. Do not use it
on a registered VM while UTM runs: UTM would keep the installer drive in memory.

## Secure Boot and TPM semantics

UTM has no separate Secure Boot key. With `UEFIBoot` and `TPMDevice` set, it boots
`edk2-aarch64-secure-code.fd` and attaches `Data/tpmdata` (swtpm, TPM 2.0 CRB). The
variable store decides whether Secure Boot is enforced:

- UTM preloads keys only from its GUI TPM toggle, or when a TPM VM starts without a
  store. A configuration save before the first start would create a keyless store,
  so `create` copies the keyed template itself.
- The copied `edk2-arm-secure-vars.fd` holds UTM's own PK, the Microsoft KEK 2011/2023
  and a db with Microsoft UEFI CA 2011 and 2023. It has `SecureBootEnable` set.
- Fedora 44's `shimaa64.efi` is signed only by Microsoft UEFI CA 2023. GRUB and the
  kernel are then verified against Fedora's key in shim, and the kernel enables lockdown.

Check it in the guest with `mokutil --sb-state`, which should print `SecureBoot enabled`.
`sysroot doctor`'s required `secure_boot` check also covers it. Limits:

- QEMU `virt` on Arm has no SMM, so a malicious guest kernel could alter the
  variable store.
- The host can always read `Data/tpmdata`.
- Secure Boot covers firmware, shim, GRUB and the kernel only.

Fedora 44 aarch64 fallback caveat: `shim-aa64` 16.1-5 ships test-signed
`fbaa64.efi`/`mmaa64.efi`. The installed system therefore boots only through the NVRAM
boot entry the installer creates for `\EFI\fedora\shimaa64.efi`. The removable-media
fallback path fails with a Security Violation. The installer ISO itself is not affected,
because its `EFI/BOOT` has no `fbaa64.efi` (see [installer/README.md](../README.md)).

- Keep `Data/efi_vars.fd`.
- Do not use Reset UEFI Variables or toggle the TPM in UTM's settings. Turning the TPM
  off selects the non-secure firmware; turning it on or resetting replaces the store
  and loses the Fedora boot entry.
- To recover, press Esc at boot. In Boot Maintenance Manager, add a boot option for
  `\EFI\fedora\shimaa64.efi` on the EFI partition of the `KEDRASYSTEM` disk and move
  it first.

## Graphics, serial console and guest agents

- **VirGL (OpenGL)** is Mesa's default driver on `virtio-gpu-gl-pci`.
- **Venus (Vulkan 1.3)** is added by UTM when two app-wide settings allow it:
  UTM > Settings > Display renderer is Default or ANGLE (Metal), and the Vulkan
  driver is not Disabled. `check-host` reports these settings when it can read them.
- **GTK:** UTM 5.0.6 notes that Linux desktop rendering through Vulkan does not
  work, so the `utm` image sets `GSK_RENDERER=gl`.
- **In the guest,** check with `vulkaninfo --summary` and `eglinfo -B`.
- **No suspend or snapshots** while a GL display is attached.
- **Serial console:** the PL011 port carries the UEFI firmware console, the GRUB
  menu and, on the installed system (`console=ttyAMA0,115200 console=tty0`), kernel
  messages and a login prompt. By default `create` connects it to UTM's built-in
  terminal, which opens as a separate window when the VM starts. Like the display,
  it is reached only through UTM.
- **TCP serial (`--serial-port PORT`):** adds an unauthenticated TCP server on
  `127.0.0.1:PORT`. Connect with `nc 127.0.0.1 PORT`; the port is stored in the VM's
  notes. Any local account, any app allowed to open network connections, and
  containers that can reach the Mac's localhost can connect without macOS Automation
  consent. They then have the access of someone at the VM's physical console:
  - firmware setup (Esc at boot), where Secure Boot can be disabled;
  - the GRUB menu (shown for one second, no GRUB password), where the unsigned kernel
    command line can be edited, for example `systemd.debug_shell=ttyAMA0` gives a
    passwordless root shell once the disk is unlocked;
  - a login prompt.

  Use it only for short evidence captures on a single-user Mac, then remove the
  serial device in UTM's settings.
- **Guest agents:** `qemu-guest-agent` and `spice-vdagent` are installed. UTM always
  connects the guest agent, which runs as root. Fedora 44 blocks none of its commands
  (`QEMU_GA_ARGS` is commented out in `/etc/sysconfig/qemu-ga`), so any Mac process
  allowed to script UTM could run commands as root or reset a password in the
  unlocked guest. The `utm` image therefore blocks `guest-exec`, `guest-exec-status`,
  `guest-file-open/close/read/write/seek/flush`, `guest-set-user-password` and
  `guest-ssh-get/add/remove-authorized-keys` in
  `/usr/lib/systemd/system/qemu-guest-agent.service.d/50-kedra-block-rpcs.conf`.
  `utmctl exec` and `utmctl file push|pull` are refused. UTM's guest clock resync
  after the Mac wakes and its IP address display (`utmctl ip-address`) still work.
  The block also holds if `QEMU_GA_ARGS` adds `--allow-rpcs`. Do not override or
  remove it. The Mac still controls the VM in other ways, for example through
  `Data/tpmdata`.

Sources: UTM v5.0.6 source (commit `968fef31`: `UTMQemuConfiguration*.swift` including the
`serialArguments` builder, `UTMConfigurationTerminal.swift`, `UTMQemuVirtualMachine.swift`,
`UTM.sdef`, `utmctl/UTMCtl.swift` for the stderr Apple Events diagnosis,
`Platform/UTMData.swift` for loading bundles at launch, `Scripting/UTMScripting*Impl.swift`
for the guest-agent calls and `reload configuration`), `qemu-guest-agent-10.2.2-1.fc44` as
installed in `fedora:44` on 2026-09-25, the GRUB configuration in `quay.io/fedora/fedora-bootc:44`
(`/usr/lib/bootupd/grub2-static/grub-static-pre.cfg` `timeout=1`, `configs.d/01_users.cfg`) read
2026-09-25, UTM 5.0.6 release notes and the
[UTM scripting cheat sheet](https://docs.getutm.app/scripting/cheat-sheet/).
