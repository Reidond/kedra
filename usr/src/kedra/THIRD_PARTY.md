# Third-party source and distribution boundaries

The pinned `actionbook/rust-skills` material is kept as ordinary repository
files in the separate repository-local plugin `usr/src/kedra/plugins/rust-skills/`:
skills under `skills/`, upstream provenance and notices in `NOTICE.md`,
`upstream/README.md` and `upstream/metadata.json`. It is repository-local
development knowledge. It is not installed into the OS image, installer, home
baseline or any global agent profile, and no upstream hook, plugin, MCP or setup
script is executed.

The desktop (x86_64) and qemu-arm64 (aarch64) images each bundle two unmodified
non-Fedora runtimes for their own architecture:

- Codex, from the pinned official release archive, whose Sigstore signatures
  are verified when the pin is added or bumped:
  `codex-package-x86_64-unknown-linux-musl` for desktop and
  `codex-package-aarch64-unknown-linux-musl` for qemu-arm64, both from the same
  0.153.4 release (`usr/src/kedra/image/packages/codex.kedra`). Its license,
  notices, component licenses and corresponding source ship as the
  `codex_licenses` and `codex_components` store outputs that
  `/usr/share/kedra/catalog.json` names.
- Bitwarden Desktop 2026.8.0, the unchanged official Linux tarball
  (`bitwarden_2026.8.0_x64.tar.gz` for desktop, `bitwarden_2026.8.0_arm64.tar.gz`
  for qemu-arm64) in the engine store, with its license files and matching client
  source in the `bitwarden_licenses` output
  (`usr/src/kedra/image/packages/bitwarden.kedra`). The tarball has no package
  scripts.

Each pin records an exact size and SHA-256 and was reviewed for distribution
when it was added. A public download URL, private registry or subscription is
not proof of redistribution permission. Claude Code is not bundled: its
preinstallation policy requires the owner's Commercial Terms agreement, and
`public_preinstallation_approved` remains false.

The owner has not selected a first-party distribution license. Do not invent
one on the owner's behalf as part of an unrelated implementation.
