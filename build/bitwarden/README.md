# Native Bitwarden image input

`prepare.py` runs as the ordinary Actions runner after `rpm`, `rpm2cpio` and
`cpio` are available. It fetches the pinned official Desktop 2026.8.0 RPM and
matching client source, checks exact sizes/SHA-256 and RPM name/version/architecture,
and emits a deterministic root-owned tar for the explicit build context. It
does not execute RPM scripts or install Bitwarden on the build host.

The RPM digest is the official GitHub release asset digest observed on
2026-09-08. This is a reviewed content pin obtained through official HTTPS;
no separate RPM signature or reproducible-source-build verification is claimed.
Package scripts, which create alternatives and conditionally make Chromium's
sandbox setuid, are not run. The original 0755 sandbox and native runtime are
retained. The enforcing Fedora VM must launch it with its native sandbox; no
`--no-sandbox` exception is allowed.

The upstream RPM stores application files in `/opt/Bitwarden`. Kedra relocates
those unchanged files to `/usr/lib/bitwarden` so they are part of the bootc
deployment, and adjusts the desktop Exec path. The upstream launcher resolves
its own location; its process-isolation wrapper remains intact. The OS manages
updates. Electron/Chromium notices remain beside the runtime; client source,
build definitions, dependency locks and all three upstream license files are
included in `/usr/share/licenses/kedra-bitwarden`.

The login wrapper, systemd user environment and TTY login profile default to
`$HOME/.bitwarden-ssh-agent.sock`, preserving an explicit alternative. They do
not open the app, sign in, enable its agent, unlock it, export keys or suppress
signing approval. Native app startup/socket propagation is tested in R07's
generated account. Authenticated R06 states remain a separate evidence gate.

Sources: [official release](https://github.com/bitwarden/clients/releases/tag/desktop-v2026.8.0),
[license map](https://github.com/bitwarden/clients/blob/f60a4606b7581c338d18091a98640ee71392f015/LICENSE.txt),
[SSH agent](https://bitwarden.com/help/ssh-agent/), and
[systemd environment.d](https://www.freedesktop.org/software/systemd/man/latest/environment.d.html).
