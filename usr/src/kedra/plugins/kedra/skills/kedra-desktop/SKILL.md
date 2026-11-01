---
name: kedra-desktop
description: Configure or validate Fedora niri/Noctalia, their managed home files and GUI overrides, graphical sessions, portals/audio/keyring, the container and native QEMU labs, and real desktop or future laptop hardware.
---

# Desktop integration is more than two RPMs

Read usr/src/kedra/docs/DESKTOP.md and usr/src/kedra/docs/HOME.md. Fedora 44
ships niri and Noctalia packages; the exact versions an image carries are in its
RPM lock (`/usr/share/sysroot/rpm-lock.json`). Do not mix latest upstream config
with an older distribution binary or assume a package list is a working desktop.

## Session checklist

Measured Fedora 44 candidate on 2026-09-08: niri 26.04, Noctalia 5.0.1 and
greetd 0.10.3-6.fc44. The service account is `greetd`, not upstream's `greeter`.
A desktop VM run then passed a generated-password login, IPC, service/portal
availability and an unlocked synthetic keyring; the Fedora PAM file includes
GNOME Keyring integration. This does not prove physical devices or owner auth.

Prove niri session startup, D-Bus/systemd user environment, portal backends/file
chooser/screen sharing, Xwayland application support, PipeWire/WirePlumber,
NetworkManager/Wi-Fi, Bluetooth, notifications, authentication/keyring, lock/idle,
recovery TTY and display acceleration. Use niri's supported session integration;
a bare compositor process may not establish the expected graphical services.
Preserve the packaged greetd session integration rather than copying a generic setup.
Keep SELinux enforcing and inspect denials rather than disabling it.

Keep common keybindings/appearance separate from target monitor/input/lid/power
configuration. Application-native includes are preferable to a generic deep
merge. Verify niri include/output-rule semantics before spreading the same monitor
across layers. Connector names and refresh strings require actual inventory.

## Seeing desktop changes: the container lab

Measured 2026-09-27 (niri 26.04, Noctalia 5.1.0, OrbStack 2.2.3 on an M2 Pro): the real session renders in a container. The harness README (usr/src/kedra/tests/container/README.md) has the details.

- **How it runs.** `kedra-session` starts through the greetd PAM service with `PAMName=greetd`, which gives a real logind session. niri runs nested through its winit backend: `niri --session` drops `WAYLAND_DISPLAY` unless `WSL_DISTRO_NAME` is set (src/main.rs, v26.04). The parent is a headless sway (pixman). Rendering is llvmpipe; Noctalia, GTK 3/4, Qt and Electron (Bitwarden) render.
- **Screenshots.** `kedra-lab up`, edit managed home defaults in system/home/, then `kedra-lab sync --shot changed`. `up` prepares the host source archiver; a stale one requires `prepare-sync`. Sync runs `sysroot home update --root` over the working tree's baselines, so guest edits merge and each file's declared check and reload run; it does not compile Rust. PNG receipts read actual niri output scale and identify image/home source. The default output is 2560×1600 at scale 2, set by niri IPC. Show the owner the PNGs.
- **Input.** niri's virtual keyboard reaches foot, GTK 3 (wlrctl's standard keymap only) and GTK 4 (wtype, which sends a real Return). Xwayland (xwayland-satellite) and Qt 6.11 clients received no key events. A nested niri reads raw keycodes with its own keymap, so wtype through the parent compositor types wrong characters. Keyboard-driven Qt/Xwayland checks stay in the VM.
- **Not the hardware.** Container screenshots show layout, theme and palette, not GPU, scaling-on-hardware or accessibility behavior. The kernel, uptime and hardware sensors are the host's.
- **Renderer environment** (systemd 259, 2026-09-29): service-local renderer environment does not propagate to Noctalia through the user manager, so the lab sets software rendering through `environment.d` (tests/container/lab/tools.Containerfile). Its final image layer computes ldconfig/hwdb and runs `systemd-update-done`, preserving normal update conditions. Do not mask those services or label this renderer as GPU acceleration.

## Noctalia configuration and GUI overrides

Noctalia v5 uses curated TOML plus separate GUI-written overrides; older v4
Quickshell/JSON commands are not interchangeable. The image manages
`.config/noctalia/config.toml` and the Adwaita palette as ordinary home files
(`managed` in system/desktop.kedra, kedra-home); GUI overrides in
`~/.local/state/noctalia/settings.toml` win over them and are not managed. Do not
decide a Git config update succeeded while a writable override still wins, and
never clear the whole override file to force the source theme. Read
references/noctalia.md.

Validator limits: Noctalia 5.1 `config validate` accepted an unknown `theme.mode`
string (2026-09-28) and 5.2.1 still only warns, exit 0, for an unknown enum value
(2026-10-14); malformed TOML is rejected. Inspect the effective rendering instead
of treating validator success as semantic proof. A validator success is not proof
that a screen-sharing portal, physical audio device or suspend cycle works.

## Greeter adoption boundary

The 2026-09-13 evaluation of Noctalia Greeter 1.5.0 recommends a separate
qualified follow-up; Kedra keeps greetd/tuigreet. Fedora 44 default repos lack
the Greeter package. If adopted, use a pinned source/archive/hash build with
official Fedora dependencies. Do not follow the documented Terra bootstrap with
`--nogpgcheck`, add Copr, or run upstream root setup scripts as an incidental
step. Preserve Fedora's greetd user, PAM/keyring and niri session. Review the
tagged session wrapper's predictable `/tmp` runtime-directory handling and replace
`-march=native` assumptions before adoption, then require wrong/correct login,
session selection, TTY recovery, 1×/1.5× scaling and enforcing-SELinux VM
evidence (not-run). Sources: [Greeter docs](https://docs.noctalia.dev/greeter/),
[1.5.0 packaging](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/PACKAGING.md),
[session wrapper](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/scripts/noctalia-greeter-session)
and [desktop defaults](../../../../docs/DESKTOP.md).

## Adwaita appearance baseline

The desktop configuration uses GNOME HIG styling and typography as the design
reference (2026-09-13). Keep native GTK/libadwaita styling and user preferences
intact. Noctalia's custom semantic palette is an approximation in a different
toolkit, not libadwaita CSS or proof of high-contrast/accessibility behavior.
GNOME Shell bar conventions and niri navigation are desktop design choices, not
HIG app requirements. A valid-looking TOML key can still be unsupported or lose
to a GUI override; validate custom themes through the native application and test
rendered behavior separately when changing its version.

Measured 2026-09-13 on Noctalia 5.0.1: both custom palette variants require
terminal-color objects even when terminal templates are disabled. Missing objects
passed configuration validation and displayed Custom/Adwaita in settings, but the
runtime fell back to built-in yellow/navy; complete objects restored blue/charcoal
in a disposable nested-niri session. Inspect the tagged
[theme service](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/theme/theme_service.cpp),
actual rendered output and application logs; config/UI labels alone are not
effective-palette evidence. Physical and accessibility qualification is separate.

Sources: [GNOME HIG styling](https://developer.gnome.org/hig/guidelines/ui-styling.html),
[typography](https://developer.gnome.org/hig/guidelines/typography.html), and
[palette](https://developer.gnome.org/hig/reference/palette.html), consulted 2026-09-13.

## Native GTK and KDE styles

GTK 3 uses official Fedora 44 `adw-gtk3-theme` (6.4-3.fc44 at evaluation, 2026-09-13),
GNOME `gtk-theme=adw-gtk3` and the same GTK 3 Xwayland settings.ini fallback. Keep
RPM-owned GTK 4 CSS intact: plain GTK 4 may consume it through the shared theme
setting, whereas libadwaita uses its native Adwaita-empty stylesheet. Dark GTK 3
requires explicit `adw-gtk3-dark` selection in GSettings and the personal
settings.ini fallback; color-scheme alone is insufficient. A disposable native
probe verified GTK 3.24.52 X11 and nested-niri Wayland widgets, user dark/reset in
new Wayland clients and strict schema compilation. Libadwaita 1.9.3/PyGObject
3.56.3 Adw.init selects Adwaita-empty; use StyleManager.get_for_display for
display-level dark/high-contrast checks. Sources:
[Fedora](https://packages.fedoraproject.org/pkgs/adw-gtk3-theme/adw-gtk3-theme/fedora-44.html),
[upstream](https://github.com/lassekongo83/adw-gtk3).

Qt uses KDE platform-theme plugins and Breeze for both Qt generations, with KDE
Qt Quick Controls desktop styles and Breeze icons. Native disposable Xvfb
QApplications with Qt 5.15.18 / 6.11.2 and Plasma integration/Breeze 6.7.5
resolved Breeze, Adwaita Sans 11 and Breeze icons (2026-09-13).
`/etc/xdg/kdeglobals` supplies defaults; personal `~/.config/kdeglobals` wins.
Keep the platform-theme default in systemd `environment.d`, preserving an
explicit value. Fedora systemd 259.8's generator was checked for unset and
explicit `qt6ct` cases; a duplicate session-wrapper default can overwrite the
manager's user preference during environment import.

GTK 3 Xwayland reads `/etc/gtk-3.0/settings.ini` without an XSettings provider,
with personal settings taking precedence. Restart those applications after
changes. A disposable GNOME XSettings 50.1 experiment outside GNOME published
startup values but failed live font propagation, so it is not a supported
session service. GTK settings behavior is documented in the upstream
[GtkSettings reference](https://docs.gtk.org/gtk3/class.Settings.html), consulted
2026-09-13. Avoid forcing `GTK_THEME` or `QT_STYLE_OVERRIDE`; retain native
application preferences and niri's portal/session identity. Independently bundled
and sandboxed Qt runtimes need their own compatible integration plugins.

## Hardware scope

VM tests prove boot/session plumbing. The current desktop needs user-approved,
redacted hardware inventory and physical GPU/scaling/mode/audio/USB/network/suspend
checks. The potential XPS remains disabled until its exact CPU/GPU/Wi-Fi/camera/
audio/dock are known. Another distribution's vendor certification is not Fedora
qualification. Laptop battery/lid/external-display behavior has separate tests.
Document measured workarounds in a set only that target uses, with a reason/removal condition.
Primary references: [niri](https://niri-wm.github.io/niri/),
[Fedora niri package](https://packages.fedoraproject.org/pkgs/niri/niri/),
[Fedora Noctalia package](https://packages.fedoraproject.org/pkgs/noctalia/noctalia/),
[Noctalia configuration](https://docs.noctalia.dev/noctalia/configuration/) and
[systemd environment scope](https://man7.org/linux/man-pages/man5/environment.d.5.html).

## Native QEMU lab on the Mac

`kedra-lab vm` builds pinned QEMU 11/HVF/Cocoa, VirGL and ANGLE Metal
(tests/container/qemu/README.md); `vm tools check` only
inspects prerequisites and never infers GPU qualification from readiness. A real
ARM guest reported Secure Boot enabled, Enforcing SELinux and the Apple M2 Pro
Metal renderer; guest grim and the native Cocoa window showed the same desktop
(2026-09-28). This is separate from the software container renderer. CUA
synthetic modifier keys did not follow the same path as letters and clicks; QMP
`vm key` is available for virtual hardware input. Noctalia can intercept the ACPI
power key, so the disposable fixture grants one exact sudo command
(`systemctl poweroff --no-block`, tests/container/qemu/tools.Containerfile) for
bounded shutdown. Never add fixture grants to production images.

Observed 2026-09-29 (M2 Pro, QEMU 11): Weston EGL and presentation clients at
2560×1600, scale 2, measured guest Virtual-1 presentation feedback of
81.634–91.126 Hz with p95 intervals 16.667 ms; this is not a physical-monitor
timing claim. The owner confirmed physical Command+Enter, letters, scrolling,
window movement and audible audio. A quiet test signal was initially inaudible:
the 40% guest volume maps to about −23.88 dB. Use a short modest signal, verify
real stream routing and ask for audibility; restore temporary guest volume and
leave host settings unchanged. `pw-play` success alone does not prove audible output.

Idle CPU (2026-09-28): QEMU 11 candidate `29d25d77` contains the HVF idle WFI
regression of [upstream issue 3433](https://gitlab.com/qemu-project/qemu/-/work_items/3433);
an idle guest (load about 0.03) consumed 437.3% host CPU. `qemu/patches/hvf-wfi.patch`
is the reviewed stable-11.0 backport
[3b98370](https://github.com/qemu/qemu/commit/3b98370b55de7fff540092c1a6760726a6816625),
which halts idle vCPUs and arms their wake timers. Retain its checksum and
attribution when refreshing the graphics fork. Measure CPU-time deltas over an
idle interval; lifetime `ps %cpu` hides bursts. The Cocoa activity guard addresses
App Nap separately and allows ordinary Mac idle sleep. Recheck startup,
background SSH, frame capture and idle load after runtime changes.

Keyboard capture (2026-09-30): physical Command+Left/Right works after clicking
inside QEMU until the title shows Ctrl+Option+G as the release shortcut.
Reopening or losing capture can make host shortcuts interfere; recapture before
testing and do not change niri bindings to hide a capture problem. Cocoa
full-grab uses a global CGEventTap/Accessibility permission, so it stays off by default.
