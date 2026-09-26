# Third-party source and distribution boundaries

The pinned `actionbook/rust-skills` material is kept as ordinary repository
files under `plugins/kedra/skills/`, with upstream provenance and notices in
`plugins/kedra/third-party/rust-skills/` (`NOTICE.md`, `README.md`,
`metadata.json`). It is repository-local development knowledge. It is not
installed into the OS image, installer, home baseline or any global agent
profile, and no upstream hook, plugin, MCP or setup script is executed.

The desktop (x86_64) and utm (aarch64) images each bundle two unmodified
non-Fedora runtimes for their own architecture:

- Codex, from the pinned official release archive with Sigstore verification:
  `codex-package-x86_64-unknown-linux-musl` for desktop and
  `codex-package-aarch64-unknown-linux-musl` for utm, both from the same
  0.153.4 release. Its corresponding source and notices ship beside it
  (`build/agents/`).
- Bitwarden Desktop, from the pinned official release package and matching
  client source, relocated to `/usr/lib/bitwarden` without running package
  scripts (`build/bitwarden/`): the x86_64 RPM for desktop and, because no
  aarch64 RPM is published, the official `bitwarden_2026.8.0_arm64.tar.gz`
  for utm.

Each pin records an exact size and SHA-256 and was reviewed for distribution
when it was added. A public download URL, private registry or subscription is
not proof of redistribution permission. Claude Code is not bundled: its
preinstallation policy requires the owner's Commercial Terms agreement, and
`public_preinstallation_approved` remains false.

The owner has not selected a first-party distribution license. Do not invent
one on the owner's behalf as part of an unrelated implementation.
