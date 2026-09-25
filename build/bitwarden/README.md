# Native Bitwarden image input

`prepare.py --target TARGET --context DIR --evidence JSON` runs as the ordinary
Actions runner. The target's architecture comes from the closed release table in
`build/release/material.py`, and `inputs.json` pins exactly one official Desktop
2026.8.0 package per architecture plus the matching client source:

| Target | Architecture | Official package | Needs |
|---|---|---|---|
| desktop | x86_64 | `Bitwarden-2026.8.0-x86_64.rpm` | `rpm`, `rpm2cpio`, `cpio` |
| utm | aarch64 | `bitwarden_2026.8.0_arm64.tar.gz` (no aarch64 RPM exists) | Python only |

It checks exact sizes/SHA-256, and emits a deterministic root-owned tar for the
explicit build context plus the same receipt as evidence. It does not execute
package scripts or install Bitwarden on the build host.

The RPM digest is the official GitHub release asset digest observed on
2026-09-08; the arm64 tarball digest was observed on 2026-09-25 and rechecked by
download. These are reviewed content pins obtained through official HTTPS; no
separate package signature or reproducible-source-build verification is claimed.
The RPM path also checks the RPM name/version/architecture and file list.
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

The arm64 tarball is a flat Electron tree (97 entries in 2026.8.0, owned by the
upstream build user). `prepare.py` reads it in memory without extracting to disk
and accepts only the reviewed layout: the exact top-level and `resources/` file
names, `locales/<name>.pak`, `resources/icons/<N>x<N>.png` for the seven reviewed
sizes and the reviewed directories. Links, hard links, devices, absolute or `..`
paths, duplicates, setuid/setgid/sticky bits and any other name are refused.
The unchanged tree is placed at `/usr/lib/bitwarden` with 0755/0644 modes
(`chrome-sandbox` stays 0755, never setuid). Its
`resources/com.bitwarden.desktop.desktop` is installed as
`/usr/share/applications/com.bitwarden.desktop.desktop` with only
`Exec=bitwarden %u` changed to `Exec=/usr/bin/bitwarden %u`; `resources/icons`
are copied to `/usr/share/icons/hicolor/<N>x<N>/apps/com.bitwarden.desktop.png`
to match its `Icon=`. Both paths add the same `/usr/bin/bitwarden` wrapper,
license files, corresponding source and `/usr/share/sysroot/bitwarden.json`
receipt. The tarball ships no package scripts. Its entry differs from the RPM's
(`StartupWMClass=com.bitwarden.desktop` rather than `Bitwarden`, categories
`System;Security;`); window/launcher matching under niri is unqualified until
the utm image is checked in UTM. Linux arm64 support, NAPI loading and the SSH
agent on aarch64 are likewise not yet qualified.

The login wrapper, systemd user environment and TTY login profile default to
`$HOME/.bitwarden-ssh-agent.sock`, preserving an explicit alternative. They do
not open the app, sign in, enable its agent, unlock it, export keys or suppress
signing approval. Native app startup/socket propagation is tested in R07's
generated account. Authenticated R06 states remain a separate evidence gate.

Sources: [official release](https://github.com/bitwarden/clients/releases/tag/desktop-v2026.8.0),
[license map](https://github.com/bitwarden/clients/blob/f60a4606b7581c338d18091a98640ee71392f015/LICENSE.txt),
[SSH agent](https://bitwarden.com/help/ssh-agent/), and
[systemd environment.d](https://www.freedesktop.org/software/systemd/man/latest/environment.d.html).
