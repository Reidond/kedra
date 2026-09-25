---
name: kedra-desktop
description: Configure or validate Fedora niri/Noctalia, effective GUI-written settings, graphical sessions, portals/audio/keyring and real desktop or future laptop hardware.
---

# Desktop integration is more than two RPMs

Fedora planning sources list niri and Noctalia packages, but verify the exact
Fedora 44 versions resolved by CI. Do not mix latest upstream config with an
older distribution binary or assume a package list is a working desktop.
The actual release inventory records resolved packages; package names alone do not prove a working session.

## Session checklist

Measured Fedora 44 candidate on 2026-09-08: niri 26.04, Noctalia 5.0.1 and
greetd 0.10.3-6.fc44. The service account is `greetd`, not upstream's `greeter`.
Actions 34169415857 passed a generated-password VM login, IPC, service/portal
availability and unlocked synthetic keyring; the Fedora PAM file includes
GNOME Keyring integration. This does not prove physical devices or owner auth.
See docs/STATUS.md for later actual desktop and installer qualification.

Prove niri session startup, D-Bus/systemd user environment, portal backends/file
chooser/screen sharing, Xwayland application support, PipeWire/WirePlumber,
NetworkManager/Wi-Fi, Bluetooth, notifications, authentication/keyring, lock/idle,
recovery TTY and display acceleration. Use niri's supported session integration;
a bare compositor process may not establish the expected graphical services.
Preserve the packaged greetd session integration rather than copying a generic setup.
Keep SELinux enforcing and inspect denials rather than disabling it.

Keep common keybindings/appearance separate from host monitor/input/lid/power
configuration. Application-native includes are preferable to a generic deep
merge. Verify niri include/output-rule semantics before spreading the same monitor
across layers. Connector names and refresh strings require actual inventory.
Earlier display/GPU context is not a live hardware probe.

## Greeter adoption boundary

The 2026-09-13 evaluation of Noctalia Greeter 1.5.0 recommends a separate
qualified follow-up; PR #14 retains greetd/tuigreet. Fedora 44 default repos
lack the Greeter package. Use a pinned source/archive/hash build in Actions
with official Fedora dependencies if adopted. Do not follow the documented
Terra bootstrap with `--nogpgcheck`, add Copr, or run upstream root setup scripts
as an incidental step. Preserve Fedora's greetd user, PAM/keyring and niri
session. Start with static administrator-owned Adwaita settings; automatic sync
with Noctalia 5.1 is not qualified.

Review predictable `/tmp` runtime-directory handling in the tagged session
wrapper and replace `-march=native` assumptions with portable build settings
before adoption. Require wrong/correct login, session selection, TTY recovery,
1×/1.5× scaling and enforcing-SELinux VM evidence. These checks are not-run.
Sources: [Greeter docs](https://docs.noctalia.dev/greeter/),
[1.5.0 packaging](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/PACKAGING.md),
[session wrapper](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/scripts/noctalia-greeter-session).
See docs/DESKTOP.md and WL-20260913-09 for the completed evaluation scope.

## Noctalia effective settings

Current v5 docs describe curated TOML and separate GUI-generated overrides. Older
v4 Quickshell/JSON commands are not interchangeable. Check the installed major
version first. Read references/noctalia.md for the proposed narrow projection.
Noctalia 5.0.1 native full-export/IPC projection passes R07 run 34179189185 at
d74c4c0 (2026-09-08). The safe three-field model rejects unqualified versions and
missing/malformed selected fields before persistence; see docs/HOME-REVIEW.md.
The full export remains transient. Native activation separately coordinates writers.
Do not decide a Git config update succeeded while a writable override still wins.
Review/export only selected safe settings, not the entire state tree.

Validate with the actual built application's validator where supported. A validator
success is not proof that a screen-sharing portal, physical audio device or suspend
cycle works. Preparing config for new software must not overwrite live config
under the old software; coordinate with the explicit home activation workflow.

## Adwaita appearance baseline

The 2026-09-13 desktop configuration work uses GNOME HIG styling and typography
as the design reference; see [desktop defaults](../../../../docs/DESKTOP.md).
Keep native GTK/libadwaita styling and user preferences intact. Noctalia's custom
semantic palette is an approximation in a different toolkit, not libadwaita CSS
or proof of high-contrast/accessibility behavior. GNOME Shell bar conventions and
niri navigation are desktop design choices, not HIG app requirements.

For the measured Noctalia 5.0.1 / niri 26.04 baseline, inspect native validation
and effective exports before claiming a setting works. A valid-looking TOML key
can still be unsupported or lose to a GUI override; never clear the entire live
override file to force the source theme. Appearance defaults do not expand the
three-field Noctalia home projection. Validate custom themes through the native
application and test rendered behavior separately when changing its version.

Measured 2026-09-13 on Noctalia 5.0.1: both custom palette variants require
terminal-color objects even when terminal templates are disabled. Missing objects
passed configuration validation and displayed Custom/Adwaita in settings, but the
runtime fell back to built-in yellow/navy. Adding complete objects restored
blue/charcoal in a disposable nested-niri session. Inspect the tagged
[theme service](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/theme/theme_service.cpp),
actual rendered output and managed application logs; config/UI labels alone are
not effective-palette evidence. Exact-source desktop run 34769161693 at
71cb8c9158871f82cb38f1fa59df2e7260dd51cf passes startup/restart fallback-warning
checks and all six native toolkit/file-chooser workflows. Independent artifact
10321492608 screenshot review confirms blue/charcoal. This is VM evidence;
physical/accessibility qualification is separate. See WL-20260913-08.
The same source passes signed home-transition 34769161731 with STAGE_B,
ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME. These exact VM/source results
do not establish production publication, physical installation or a full
accessibility audit.

Sources: [GNOME HIG styling](https://developer.gnome.org/hig/guidelines/ui-styling.html),
[typography](https://developer.gnome.org/hig/guidelines/typography.html), and
[palette](https://developer.gnome.org/hig/reference/palette.html), consulted 2026-09-13.

## Native GTK and KDE styles

The 2026-09-13 adw-gtk3 follow-up selects official Fedora 44
`adw-gtk3-theme` (6.4-3.fc44 at evaluation), GNOME `gtk-theme=adw-gtk3` and the
same GTK 3 Xwayland settings.ini fallback. Keep RPM-owned GTK 4 CSS intact for
package/material integrity: plain GTK 4 may consume it through the shared theme
setting, whereas libadwaita uses its native Adwaita-empty stylesheet. Dark GTK 3
requires explicit `adw-gtk3-dark` selection in GSettings and the personal
settings.ini fallback; color-scheme alone is insufficient. Consult
[Fedora](https://packages.fedoraproject.org/pkgs/adw-gtk3-theme/adw-gtk3-theme/fedora-44.html),
[upstream](https://github.com/lassekongo83/adw-gtk3) and WL-20260913-10 for actual
validation scope; the earlier Adwaita GUI pass does not qualify this new theme.
The disposable native probe ADW_GTK3_RUNTIME_PASS verifies GTK 3.24.52 X11 and
nested-niri Wayland widgets, user dark/reset in new Wayland clients and strict
schema compilation. Libadwaita 1.9.3/PyGObject 3.56.3 Adw.init selects
Adwaita-empty and per-display StyleManager light/default values. Use
StyleManager.get_for_display for the display-level dark/high-contrast checks;
retain version-correct calls. Exact-source desktop 34784038994 at
4d0305194341b702f3e39fb6abba6a1f6b3f29a0 passes the GTK 3 Wayland/X11, native
libadwaita and Qt/KDE chooser workflows; independent artifact 10325794069 review
confirms adw-gtk3 controls. Signed home 34784039080 also passes under Noctalia
5.1.0. Runtime allowlisting is exactly 5.0.1/5.1.0; persisted APP_VERSION remains
5.0.1 at source level. A historical 5.0.1-created record to 5.1 CLI migration
E2E is not-run; do not infer it from same-source signed A/B/A. Unknown versions
still refuse. Physical/high-contrast qualification remains separate.

The 2026-09-13 integration uses KDE platform-theme plugins and Breeze for both
Qt generations, with KDE Qt Quick Controls desktop styles and Breeze icons.
Native disposable Xvfb QApplications with Qt 5.15.18 / 6.11.2 and Plasma
integration/Breeze 6.7.5 resolved Breeze, Adwaita Sans 11 and Breeze icons. See
`docs/DESKTOP.md` and WL-20260913-08 for scope and later graphical evidence.
`/etc/xdg/kdeglobals` supplies defaults; personal `~/.config/kdeglobals` wins.
Keep the platform-theme default in systemd `environment.d`, preserving an
explicit value. Fedora systemd 259.8's generator was checked for unset and
explicit `qt6ct` cases; a duplicate session-wrapper default can overwrite the
manager's user preference during environment import.

GTK 3 Xwayland reads `/etc/gtk-3.0/settings.ini` without an XSettings provider,
with personal settings taking precedence. Restart those applications after
changes. A disposable GNOME XSettings 50.1 experiment outside GNOME published
startup values but failed live font propagation, so it is not a supported
session service. Do not add it merely to claim dynamic synchronization.
GTK settings behavior is documented in the upstream
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
Document measured workarounds under the host with a reason/removal condition.

Gates: R07 desktop, R03/R04 effective settings, R06 keyring/auth, R09 host isolation.
Primary references: [niri](https://niri-wm.github.io/niri/),
[Fedora niri package](https://packages.fedoraproject.org/pkgs/niri/niri/),
[Fedora Noctalia package](https://packages.fedoraproject.org/pkgs/noctalia/noctalia/),
[Noctalia configuration](https://docs.noctalia.dev/noctalia/configuration/) and
[systemd environment scope](https://man7.org/linux/man-pages/man5/environment.d.5.html).
