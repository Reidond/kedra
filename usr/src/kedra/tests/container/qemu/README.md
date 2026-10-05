# Native QEMU desktop lab on Apple Silicon

`kedra-lab vm` runs standalone QEMU 11.0.0 with HVF, Cocoa, VirGL and ANGLE's
Metal backend. It does not use UTM, Apple Events, UTM libraries or VM bundles.
The signed ARM image identity is `qemu-arm64` / `ghcr.io/reidond/kedra-qemu-arm64`.

The runtime and first disposable disk are prepared once. A running desktop stays
available for manual use, guest commands, screenshots and validated home sync.
Cold builds take minutes; they are outside the edit/capture loop.

Native QEMU data is deliberately outside Cargo's `target` directory, so
`cargo clean` cannot erase the private runtime or retained VM disks. By default,
rebuildable runtime/build/image data lives under
`~/Library/Caches/kedra/qemu`, while retained instances, writable disks, TPM
state, keys, logs and screenshots live under
`~/.local/share/kedra/lab/<checkout-key>`. The eight-character stable checkout key
keeps Unix socket paths below the macOS limit and prevents different checkouts
from controlling the same named instance.

Set `KEDRA_QEMU_HOME` to an absolute directory to place all native runtime,
build, image and retained-instance data below one root. Explicit `--runtime` and
`--workdir` arguments take precedence. For compatibility, an explicitly set
`KEDRA_LAB_ARTIFACTS` also keeps native QEMU data below its `qemu/` subdirectory;
other container-harness artifacts retain their existing meaning.

```sh
cargo build -p kedra-container-tests --bin kedra-lab --locked
target/debug/kedra-lab vm tools check
target/debug/kedra-lab vm tools prepare
target/debug/kedra-lab prepare-sync
target/debug/kedra-lab vm image --image stable --overlay worktree
# Use the image.json path printed by the previous command:
target/debug/kedra-lab vm up --name desktop --image /absolute/path/to/image.json
```

A prepared runtime is reused after its receipts/hashes pass. Preparation installs
no Homebrew tools: Xcode (including its Metal component), Ninja, pkg-config,
Autoconf/Automake/libtool and the glib/pixman/libslirp/json-glib/OpenSSL/libtasn1
libraries must already be present. Python build dependencies come from the pinned
`build-tools/uv.lock`, through uv with Python 3.12. Missing external prerequisites
need owner authorization before installation.

The native fixture uses a fresh SSH client/host key and a private fw_cfg seed for
each instance. SSH disables personal configuration/agents and pins the host key.
Its automatic test-account session, keyring bootstrap and single-command shutdown
permission exist only in the unsigned lab derivative. Native DRM, SELinux and the
Secure Boot chain remain enabled. Signed installer media contains none of these
fixture adaptations.

Verified native derivations can enter this same disposable VM path:

```sh
kedra-lab vm image --image composition:/private/current-context --overlay none \
  --composition-identity COMPOSITION_ID --native-plan /private/native.json \
  --derivation-identity NATIVE_ID
```

The complete tuple is required. The controller verifies native material and exact
parent layers, rebuilds the fixed fixture recipe rather than adopting a tools tag,
and checks native bytes again after fixture packages. Only generated schemas/
initramfs may be restored after package triggers; differences are recorded. Disk
receipts distinguish native image, fixture ID, imported Podman manifest and disk
hash. The fixed root observer writes `/run/kedra-lab/native-boot.json` with selected
BLS/kernel/initrd, bootc and security facts. For qualification, compare its reported
bootc reference/digest/native tuple with retained image.json on every boot; the
observer alone cannot bind the circular final fixture digest.

Use current committed graphical declarations for actual desktop qualification;
the synthetic multi-user native E2E is a different test. Unsigned fixture boot
does not establish production signature admission, install/update or encrypted
login. Retain expected trust failures and actual outcome boundaries in
[derived boot evidence](../../../../../../.specs/nix-derived-boot/verification.md).

## Retained iteration

```sh
target/debug/kedra-lab vm up --name desktop
target/debug/kedra-lab vm sync --name desktop --shot changed
target/debug/kedra-lab vm exec --name desktop -- noctalia msg settings-toggle
target/debug/kedra-lab vm key --name desktop meta_l ret
target/debug/kedra-lab vm shot --name desktop settings
target/debug/kedra-lab vm logs --name desktop
target/debug/kedra-lab vm status --name desktop
target/debug/kedra-lab vm down --name desktop
# Only for a stuck VM; may interrupt guest writes:
target/debug/kedra-lab vm down --name desktop --force
# Deletes only this stopped disposable instance, including its disk and keys:
target/debug/kedra-lab vm remove --name desktop
```

Use the existing `kedra-lab up` / `shot` container workflow for explicitly software
rendered diagnostics. Native VMs always require the qualified Cocoa/Metal path and
refuse a compositor renderer that is not VirGL/ANGLE Metal. A raw Cocoa 2D-display
experiment crashed on macOS 27.0 (26A428); that unused VM configuration is removed.

These retained-instance commands require no Docker daemon. `vm sync` uses the same
source archiver and validated guest transaction as container sync: additions,
changes and deletions of niri/Noctalia defaults, conflict refusal, retained GUI
overrides and interrupted-write recovery. Package/rootfs changes require a newly
prepared image. The ordinary-user session transport comes from the controller at
execution time, so updating lab control code does not rebuild the OS disk. Capture
receipts include controller/transport hashes. Shutdown bypasses desktop environment
lookup; a stopped user manager cannot block the poweroff request. Runtime changes require a new instance; receipts reject substitution.

Click inside the native window until its title shows `Ctrl+Option+G` as the release
shortcut; this confirms input capture. Control+Option+G releases it. Recapture
after reopening or switching away before using Command shortcuts. Physical
Command+Left/Right was verified to move niri focus while remaining on VT1 and
workspace 1. The broader Cocoa full-grab event tap stays disabled; normal capture
does not require global keyboard monitoring.
`vm key` sends QEMU virtual-hardware qcodes, including modifier combinations, for
repeatable agent-driven input. The launcher restores normal scheduling/I/O policy
for its own QEMU and TPM processes so a background agent shell does not throttle
the retained desktop.

Default display is 2560×1600 at scale 2, six vCPUs and 4 GiB RAM; initial `up`
accepts `--display`, `--cpus` and `--memory-mib`. `shot` captures through guest
`grim`, with a PNG/source/image/runtime/display receipt under
`~/.local/share/kedra/lab/<checkout-key>/shots`. A screenshot alone never
asserts GPU qualification.
Private QMP/TPM/guest-agent sockets, serial logs and state live below
`~/.local/share/kedra/lab/<checkout-key>/instances/<name>`. The only network
listener is loopback SSH with per-instance credentials. No host disk, production
home or existing VM is enrolled, migrated or reformatted.

## Signed installer workflow

```sh
target/debug/kedra-lab vm iso --image ghcr.io/reidond/kedra-qemu-arm64@sha256:REVIEWED_DIGEST --output /new/iso/directory
target/debug/kedra-lab vm installer --name install --sentinel --iso /path/to/kedra-qemu-arm64-44-DIGEST16.iso
# Finish installation in the native window, then shut it down:
target/debug/kedra-lab vm detach-installer --name install
target/debug/kedra-lab vm up --name install
```

[macOS media preparation](../../../installer/macos/README.md) retains the existing
fixed-key signed-image verification and installer receipts. The VM receives an
APFS clone of the verified ISO and a new sparse disk; the original ISO is retained.
Installer VMs have no lab SSH access, automatic login or fixture shutdown grant.
`--sentinel` adds only a generated, instance-owned marker disk and verifies its
hash when detaching the installer; it never accepts a host disk path. Detaching
media requires a stopped VM. The former UTM identity is retired; create a new
QEMU instance.

## Inputs and qualification

`inputs.json` pins every top-level source, downloaded compiler archive, firmware
and local compatibility patch. `angle-dependencies.json` records selected Git/CIPD
inputs; unrelated formatter/Linux-sysroot GCS downloads and upstream hooks are
excluded. QEMU's pinned Meson wraps select its remaining source dependencies.
The runtime bundles the actual dylib closure, uses relative load commands and
ad-hoc Hypervisor signing, and checks Microsoft UEFI CA 2023 enrollment. Build
receipts permit retrying packaging without recompiling unchanged binaries.

The integration candidate comes from
[akihikodaki/v](https://github.com/akihikodaki/v/tree/cd8293b463ed9963f511a23b2b80025c65a81303).
Its recorded epoxy/VirGL revisions were unavailable on 2026-09-28; exact reachable
`v` revisions are pinned instead. The QEMU EGL patch explicitly converts ANGLE's
integer native-display type for the platform-display pointer argument, matching
the type correction in pinned VirGL commit `619f0a0`. Chromium's compiler is used
with Apple's linker because the pinned LLD cannot parse macOS 27's new SDK TAPI
architecture; Xcode provides the build-time libLTO bridge. Upstream sudo/vmnet,
Pipenv and update scripts are not executed.

Current local evidence (2026-09-30): runtime build/relocation/signatures and
Secure Boot firmware checks pass. Native niri and Wayland EGL report
`virgl (ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, ...))`; the Noctalia 5.2
fixture is healthy with Secure Boot enabled and SELinux Enforcing. Physical
Command+Enter, Command+arrows after capture, scrolling, window movement and audible
output were confirmed by the owner. Native toolkit/chooser workflows, unlocked
scaling and restored 2560×1600 scale 2 geometry pass.

Five final visible edit/capture samples pass median 4.453 s/max 5.136 s; warm boots
median 18.690 s/max 23.580 s and captures median 0.937 s/max 1.055 s. Five 30 s presentation
observations pass at 96.57–101.01 Hz with p95 interval 16.667 ms on guest Virtual-1.
These are measured warm results on this M2 Pro and the recorded fixture; cold
runtime/disk preparation and other hosts are separate.

Generated instance keys/disks are isolated; prepared sync/capture works without
Docker. Failed startup and literal shutdown/explicit-force workflows preserve
owned storage and unrelated state. First signed-media fresh encrypted installation,
original ISO/sentinel preservation and stopped detach pass. Its ISO-free graphical
unlock exposed missing early virtual GPU/input drivers; the corrected signed image
and verified ISO were freshly installed. Visible LUKS unlock, greetd login,
installed doctor/security/Metal/TPM dry-run and exact signed bootc status pass.
Final graceful stop required explicit force; ISO/sentinel hashes still match. Readiness JSON
keeps `gpu_qualified: false`: process startup or a PNG does not establish every
manual qualification case.
See [STATUS](../../../docs/STATUS.md) and the
[verification packet](../../../../../../.specs/qemu-desktop-iteration/test-plan.md).

The pinned QEMU revision also needs the reviewed stable-11.0
[HVF WFI correction](https://github.com/qemu/qemu/commit/3b98370b55de7fff540092c1a6760726a6816625).
Without it, an idle guest consumed several host CPU cores and control/boot timing
was unreliable. `patches/hvf-wfi.patch` preserves the upstream attribution and
is checksummed in the input lock. A separate Cocoa user-activity patch keeps the
requested VM work eligible to run while its window is hidden, using Apple's
[activity API](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/PrioritizeWorkAtTheAppLevel.html);
normal idle system sleep remains allowed.
