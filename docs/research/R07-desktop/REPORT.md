# R07: Fedora desktop candidate

Status: **fail** for initial image assembly, 2026-09-08 (Europe/Kiev).
It is not a qualified desktop release.

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

All tests remain not-run until recorded: RPM resolution; config validators;
graphical VM login; niri/Noctalia IPC; portals/file chooser/screen sharing; audio;
lock/idle; Secret Service/keyring authentication; physical devices/suspend.
The package build is the first step and cannot establish the later runtime cases.

Sources reviewed 2026-09-08:
[Fedora Noctalia](https://packages.fedoraproject.org/pkgs/noctalia/noctalia/)
lists 5.0.1-1.fc44; the actual DNF result remains authoritative.
[Niri session integration](https://niri-wm.github.io/niri/Getting-Started.html),
[Noctalia compositor startup](https://docs.noctalia.dev/noctalia/getting-started/running-the-shell/),
[Noctalia IPC](https://docs.noctalia.dev/noctalia/compositor-settings/niri/),
[Noctalia polkit setting](https://docs.noctalia.dev/noctalia/configuration/shell/),
[greetd configuration](https://github.com/kennylevinsen/greetd/blob/master/config.toml)
and [DNF5 repository options](https://dnf5.readthedocs.io/en/latest/dnf5.8.html).
