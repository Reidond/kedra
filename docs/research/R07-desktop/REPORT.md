# R07: Fedora desktop candidate

Status: **pass** for desktop package resolution/configuration/build, 2026-09-08
(Europe/Kiev). Graphical login/session subset passes; not a qualified desktop release.

Latest [run 34184975810](https://github.com/Reidond/kedra/actions/runs/34184975810)
at `afb8a67` passes the guest 1280x768 mode assertion and actual QEMU window
geometry. Inspected screenshots fill the frame with readable Noctalia controls;
the Xvfb/GTK capture-size issue is resolved. Login, services, keyring and native
projection still pass. This remains a graphical VM subset, not physical qualification.

Earlier [run 34179189185](https://github.com/Reidond/kedra/actions/runs/34179189185)
at `d74c4c0` passes graphical login, services, keyring and native Noctalia 5.0.1
export/IPC projection. The startup help overlay is gone. Inspected screenshots
still show a cramped virtual scanout within the requested display; explicit
guest output sizing remains open. These tests do not qualify physical displays.

The captured niri output report explains the cramped scanout: QEMU advertises
640x505 as preferred despite the requested host display size. Its advertised modes
also include 1280x768. The test-only derivative now selects that existing mode
and checks the logical dimensions before screenshots. No physical monitor setting
or custom modeline is added to the desktop payload. The new runtime check is pending.

[Corrected run 34167524359](https://github.com/Reidond/kedra/actions/runs/34167524359)
at `14be822` passed the image build, niri validator, Noctalia validator and bootc
lint. Actual versions: niri 26.04 and Noctalia 5.0.1. The Fedora greetd PAM file
includes GNOME Keyring integration. Lint reports three warnings including
generated /var cache files; these are not claimed resolved. The next workflow
adds graphical UEFI/KVM testing with host software GL, virtual HDA audio and no
network interface. QMP submits a generated disposable account password without
logging it, and captures login/desktop/settings images. A separate guest serial
port carries test markers so kernel/audit output cannot split them.

First graphical attempt [34168467701](https://github.com/Reidond/kedra/actions/runs/34168467701)
at `891d6cc` built the desktop and test disk and reached the login-ready marker.
The host harness failed before submitting credentials: QMP reported `no surface`,
and Mesa logged failure to attach X11 shared memory. Xvfb and QEMU had different
users; the next run uses the same user for both and captures the Xvfb display when
a GL scanout has no QMP software surface. Graphical login remains unpassed.

Corrected [run 34169415857](https://github.com/Reidond/kedra/actions/runs/34169415857)
at `b3e569a` passed real password login, niri/Noctalia IPC, PipeWire/WirePlumber,
portal D-Bus availability and an unlocked synthetic login keyring. Login, launcher
and settings screenshots were downloaded and visually inspected. They confirm
rendered Noctalia, but show the startup hotkey overlay covering the UI, a small
virtual preferred mode and audit text over the login prompt. Follow-up changes
provide an explicit help shortcut without startup overlay, a quiet boot console,
and a larger fullscreen VM display. Physical audio, screen sharing, lock/idle,
hardware and owner-account authentication are separate, unpassed cases.

[Initial run 34167255886](https://github.com/Reidond/kedra/actions/runs/34167255886)
at `9e2b752` resolved and installed the entire requested desktop package set. It
then failed at the service-account check: Fedora greetd 0.10.3-6.fc44 creates the
account `greetd`, whereas the upstream example uses `greeter`. The source config
and check now use the actual Fedora account. Config validators and graphical
runtime remain not-run until the corrected build proceeds.

The candidate uses Fedora 44 packages: niri, Noctalia, greetd/tuigreet login,
PipeWire/WirePlumber, NetworkManager, Bluetooth, portals, Xwayland satellite,
GNOME Keyring/PAM, Firefox, Nautilus, Foot and Fuzzel. The latter provides an
independent application launcher if the shell is unavailable. No monitor, disk,
personal username, UID or future XPS hardware identity is specified.

The ordinary source archive feeds a plain root Containerfile. The build enables
the graphical login service, masks bootc's automatic fetch/apply timer and service,
validates niri/Noctalia settings and records the RPM inventory. Packages resolve
only from Fedora/updates with refreshed metadata, signature checks and required
repository errors. This manual research build does not prove scheduled no-change
or stale-cache cases; the update-refresh experiment remains required.

Initial user configuration is seeded into /etc/skel during the image build,
using the inert `/usr/share/sysroot/home/default` profile. This only affects new
accounts created later by an installer. It does not update an existing home or
establish home adoption. The files remain ordinary writable files after account
creation. No repository skills, personal profiles or credentials enter the payload.

Remaining runtime cases: portal file chooser/screen sharing, actual audio,
lock/idle, physical devices/suspend and owner credentials. Service availability
and synthetic keyring success do not establish these cases.

Sources reviewed 2026-09-08:
[Fedora Noctalia](https://packages.fedoraproject.org/pkgs/noctalia/noctalia/)
lists 5.0.1-1.fc44; the actual DNF result remains authoritative.
[Niri session integration](https://niri-wm.github.io/niri/Getting-Started.html),
[Noctalia compositor startup](https://docs.noctalia.dev/noctalia/getting-started/running-the-shell/),
[Noctalia IPC](https://docs.noctalia.dev/noctalia/compositor-settings/niri/),
[Noctalia polkit setting](https://docs.noctalia.dev/noctalia/configuration/shell/),
[greetd configuration](https://github.com/kennylevinsen/greetd/blob/master/config.toml)
and [DNF5 repository options](https://dnf5.readthedocs.io/en/latest/dnf5.8.html).
