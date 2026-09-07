# Session decisions and rejected alternatives

This is a design recap supplied to future agents, not an execution log or a
private reasoning transcript. The current user decisions take precedence over
early exploratory examples. Prepared 2026-09-07.

## Evolution of the design

1. The owner wanted a Fedora 44 bootc OS defined in a Git repo with Linux-shaped
   paths, niri/Noctalia, machine configuration and user dotfiles.
2. BlueBuild was explicitly rejected. Use a plain Containerfile and small
   purpose-built tooling, not a universal distribution framework.
3. `sysroot` was chosen as the management interface. OS builds must happen in
   GitHub Actions and produce signed OCI images plus installation ISOs. A prompt
   to an agent should eventually result in a reviewed, published, staged update.
4. Home files constantly change through apps and users. Read-only symlinks into
   the OS image were rejected as the general live-home model. Use ordinary files
   and a Git-backed baseline/review/merge layer. Selecting, leaving uncommitted,
   keeping local/ignored, and discarding must work down to lines within a file.
5. Add `sysroot codex` and `sysroot claude` opening the verified OS checkout.
   Bitwarden SSH must be integrated out of the box. Setup still requires the
   owner's accounts, vault unlock/authorization, and supported sign-in flows.
6. Bundled CLIs must NOT take over personal `codex`/`claude`. Users can install
   newer personal versions and retain local MCP, skills, and settings, optionally
   tracking any safe subset. Runtime and configuration scope are independent.
7. Kedra became the user-facing OS/project name; `sysroot` remains the command.
8. Multiple machines use one repo, shared defaults, per-host overrides and
   separate signed images/installers. Desktop and future XPS update independently.
   No permanent machine branches, cloned repositories, or fleet server are needed.
9. Research gates precede risky implementation. Prove signature enforcement,
   installer, home safety, agent coexistence, credentials and privileged state.
10. The user briefly selected TypeScript/Effect v4/Vite Plus/Oxlint/Oxfmt, then
    explicitly replaced that with Rust/Cargo monorepo and no `src/` folders.
    Do not reopen the superseded JavaScript toolchain decision.
11. Integrate actionbook/rust-skills at a reviewed commit, locally and skills-only.
    The owner then supplied Reidond/kedra and authorized bootstrapping it so Codex
    can continue with this session's knowledge packaged as skills.

## Important distinctions to preserve

- Image signing is not commit signing, release promotion, Secure Boot, filesystem
  integrity, or automatic rollback. Each has different evidence and authority.
- A signed image can still be the wrong target or an unapproved candidate.
- A mutable channel tag is discovery, not deployment identity. Deploy by digest;
  digest-pinned bootc requires a new switch rather than a no-op upgrade.
- Same source commit can resolve a different Fedora base or RPM set. Record all
  inputs and retain exact image digests; do not claim bit-for-bit rebuildability.
- OS rollback does not rewind persistent /var, /etc modifications, home, database
  migrations, credential stores, or newer personal agent installations.
- Root filesystem overlays and Git ignore flags do not provide line-level home
  reconciliation. A filtered diff alone does not prevent publishing hidden data.
- Capturing a change is not staging it; staging is not publishing it; publishing
  is not deployment; staged is not booted; booted is not healthy.
- Persistent local-only hunk policy differs from app-owned structured settings.
  A newly changed ignored value should reappear unless an explicit ownership
  rule makes that setting application-local.
- Noctalia GUI override state can win over curated TOML. Review effective values,
  but never indiscriminately adopt its entire state directory.
- CODEX_HOME/CLAUDE_CONFIG_DIR are configuration mechanisms, not security sandboxes.
  User skills, credential backends, hooks, and subprocess environments may cross
  presumed boundaries and require testing.
- Bitwarden SSH signing does not provide gh API auth, model OAuth, Secret Service,
  or GHCR pull credentials. Do not export vault keys or automate broad vault access.
- `/sysroot` is also a filesystem path in bootc; our `sysroot` command is separate.

## User-specific scope without private data

`desktop` means the owner's current intended machine. Earlier context mentioned
an AMD Radeon RX 9070 XT and a 4K/160 Hz display with 200% scaling, but this is not
a live inventory or validated connector/mode specification. Probe only with
permission, redact serials, and add host workarounds only for measured problems.
`xps` reserves a potential Dell XPS target; model and components are unknown.
`andrii` in old examples was a proposed account name, not a required UID/name.
The checkout default is proposed as `~/src/kedra`, configurable; do not hard-code
it into privileges or assume it exists. The repo was empty and public at bootstrap.

## Explicit non-goals

No custom agent UI, custom OAuth implementation, global agent policy takeover,
unrestricted root agent, background AI dependency, SSH-key export, automatic
publication of all dotfiles, mandatory personal config adoption, arbitrary app
config universal parser, or automatic enrollment of the current workstation.
No promise that every XPS subsystem works because Fedora booted in a VM.
