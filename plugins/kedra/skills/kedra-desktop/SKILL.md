---
name: kedra-desktop
description: Configure or validate Fedora niri/Noctalia, effective GUI-written settings, graphical sessions, portals/audio/keyring and real desktop or future laptop hardware.
---

# Desktop integration is more than two RPMs

Fedora planning sources list niri and Noctalia packages, but verify the exact
Fedora 44 versions resolved by CI. Do not mix latest upstream config with an
older distribution binary or assume a package list is a working desktop.
The root package list is a research candidate, not a complete bill of materials.

## Session checklist

Measured Fedora 44 candidate on 2026-09-08: niri 26.04, Noctalia 5.0.1 and
greetd 0.10.3-6.fc44. The service account is `greetd`, not upstream's `greeter`.
Actions 34169415857 passed a generated-password VM login, IPC, service/portal
availability and unlocked synthetic keyring; the Fedora PAM file includes
GNOME Keyring integration. This does not prove physical devices or owner auth.
See R07-desktop/REPORT.md for images, harness failures and visual refinements.

Prove niri session startup, D-Bus/systemd user environment, portal backends/file
chooser/screen sharing, Xwayland application support, PipeWire/WirePlumber,
NetworkManager/Wi-Fi, Bluetooth, notifications, authentication/keyring, lock/idle,
recovery TTY and display acceleration. Use niri's supported session integration;
a bare compositor process may not establish the expected graphical services.
Choose the display/login manager in R07, not by blindly copying a generic setup.
Keep SELinux enforcing and inspect denials rather than disabling it.

Keep common keybindings/appearance separate from host monitor/input/lid/power
configuration. Application-native includes are preferable to a generic deep
merge. Verify niri include/output-rule semantics before spreading the same monitor
across layers. Connector names and refresh strings require actual inventory.
Earlier display/GPU context is not a live hardware probe.

## Noctalia effective settings

Current v5 docs describe curated TOML and separate GUI-generated overrides. Older
v4 Quickshell/JSON commands are not interchangeable. Check the installed major
version first. Read references/noctalia.md for the proposed narrow projection.
Do not decide a Git config update succeeded while a writable override still wins.
Review/export only selected safe settings, not the entire state tree.

Validate with the actual built application's validator where supported. A validator
success is not proof that a screen-sharing portal, physical audio device or suspend
cycle works. Preparing config for new software must not overwrite live config
under the old software; coordinate with the home activation research.

## Hardware scope

VM tests prove boot/session plumbing. The current desktop needs user-approved,
redacted hardware inventory and physical GPU/scaling/mode/audio/USB/network/suspend
checks. The potential XPS remains disabled until its exact CPU/GPU/Wi-Fi/camera/
audio/dock are known. Another distribution's vendor certification is not Fedora
qualification. Laptop battery/lid/external-display behavior has separate tests.
Document measured workarounds under the host with a reason/removal condition.

Gates: R07 desktop, R03/R04 effective settings, R06 keyring/auth, R09 host isolation.
Sources: docs/SOURCES.md niri, fedora-niri, fedora-noctalia, noctalia, environment.
