# Agent launchers

On Linux, `sysroot codex` and `sysroot claude` select an official agent runtime and
run it as the ordinary user in a Kedra checkout. They refuse to run through sudo.
The launchers install no agent binaries and authorize no publishing, deployment or
reboot.

```sh
sysroot codex --print-plan
sysroot codex -- --help
sysroot claude --runtime user --config-scope personal -- --version
sysroot codex --repo ~/src/kedra --host qemu-arm64 -- 'Review the target inputs'
```

Wrapper options come before `--`; everything after it goes unchanged to the official
CLI, which keeps its normal interaction, signals and exit status.

## Runtimes

- **Bundled** (`--runtime bundled`, the default) runs
  `/usr/share/kedra/catalog-libexec/NAME`, the launcher of a catalog package's private
  `libexec` export, which is never on PATH ([exports and PATH](PACKAGES.md#exports-and-path)).
  Both images build the pinned official Codex package from
  `usr/src/kedra/image/packages/codex.kedra` in the engine; the launcher `exec`s its
  exact store output, so Codex finds its bundled helpers next to its real path.
- **Claude is not bundled.** Anthropic's published policy for preinstalling Claude
  requires the owner's agreement to its Commercial Terms, unmodified binaries, its own
  authentication choices and direct end-user billing; without that agreement there is
  no package, and `sysroot claude` needs `--runtime user`.
- **Personal** (`--runtime user`) uses the first executable of that name on an
  absolute PATH entry, or an explicit absolute `--executable PATH`. A missing personal
  runtime fails; there is no bundled fallback.

Bitwarden Desktop is also an engine-built catalog package
(`usr/src/kedra/image/packages/bitwarden.kedra`), but an ordinary `bitwarden` command
on PATH rather than a launcher runtime. Login shells and the user session point
`SSH_AUTH_SOCK` at its SSH agent socket, `~/.bitwarden-ssh-agent.sock`, unless you set
your own.

## Checkout and target

The checkout is the Git work tree containing `--repo`, `SYSROOT_REPO` or
`~/src/kedra`; the agent starts at its top level. Any clone or worktree works,
whatever its remote; the launcher never fetches, clones, stashes, resets or locks it.
Use separate worktrees for concurrent sessions. `--host` (default `desktop`) must name
a target declared in that work tree (`usr/src/kedra/image/targets/<target>/target.toml`).
It only tells the agent which target to edit, through `SYSROOT_EDITING_TARGET`; it
never selects a machine to run on or deploy to.

## Profiles

With the default `--config-scope management`, the agent's configuration directory
(`CODEX_HOME` or `CLAUDE_CONFIG_DIR`) is
`$XDG_STATE_HOME/sysroot/agents/NAME/RUNTIME/VERSION` (default state root:
`~/.local/state`), created private. Version-specific profiles keep a newer runtime from
migrating state that an older bundled release reads, for example after an image
rollback, so a new version can require a new official login. Existing profiles are
kept; no login or session is copied or migrated. New Codex profiles store credentials
in the Secret Service (`cli_auth_credentials_store = "keyring"`) with no plaintext
fallback, and an existing preference is kept. The user provides a working unlocked
Secret Service and completes the official login.

`--config-scope personal` leaves the inherited configuration variables as they are,
so the agent uses personal configuration (inside a session the launcher started, those
variables still point at that session's management profile). The variables are
inherited by the agent's child processes; they are not a sandbox. The wrapper never
changes `HOME`.

`--print-plan` prints the selection as JSON. It runs `--version` with a disposable
private configuration directory, because native Codex writes helper files there even
for version or help, and it creates no profile and starts no session.

## Environment

Every launch removes `BW_SESSION`; `SSH_AUTH_SOCK` and official provider
authentication remain available. A bundled Claude, if one is ever packaged, would get
invocation-local `DISABLE_UPDATES=1` because the image updates it; personal runtimes
keep their own update settings. The wrapper never collects or copies login
credentials. No repository skill collection is installed globally or in these
profiles.

A checkout build does not show that the selected agent exists or is authenticated on
the current machine. `usr/src/kedra/tests/agents/run.py` checks real launcher, runtime
and profile behavior on an installed Kedra or lab image; [STATUS](STATUS.md) records
its results and which published image first carries the catalog Codex and Bitwarden
packages.

Official configuration sources, retrieved 2026-09-08:
[Codex configuration](https://learn.chatgpt.com/docs/config-file/config-advanced),
[Codex credentials](https://learn.chatgpt.com/docs/auth),
[Claude environment](https://code.claude.com/docs/en/env-vars) and
[Claude updates](https://code.claude.com/docs/en/setup).
