# Agent launchers

On Linux, `sysroot codex` and `sysroot claude` select an official runtime and run it
as the ordinary user in a Kedra checkout. The launchers do not install agent binaries
or authorize publishing, deployment or reboot.

```sh
sysroot codex --repo ~/src/kedra --runtime user --print-plan
sysroot codex --repo ~/src/kedra --runtime user -- --help
sysroot claude --repo ~/src/kedra --runtime user --config-scope personal -- --version
sysroot codex --repo ~/src/kedra --host xps --runtime user -- 'Review the target inputs'
```

Wrapper options precede `--`; everything after it goes unchanged to the official CLI,
which runs with normal upstream interaction, signals and exit status.

Runtime. The default bundled runtime is `/usr/libexec/sysroot/agents/NAME/bin/NAME`,
outside ordinary PATH. The desktop image packages the pinned official Codex runtime,
whose native VM startup and profile-retention checks pass. Claude is not packaged
pending the owner's preinstallation terms choice. `--runtime user` uses the personal
executable on absolute PATH entries, or an explicit absolute `--executable`. A missing
personal executable fails; there is no bundled fallback. Authenticated model
workflows and the complete runtime/profile matrix remain unqualified.

Checkout and target. The checkout is the Git work tree containing `--repo`,
`SYSROOT_REPO` or `~/src/kedra`; the agent starts at its top level. Any clone or
worktree works, whatever its remote; the launcher never fetches, clones, stashes,
resets or locks it. Use separate worktrees for concurrent sessions. `--host` (default
`desktop`) must name a target declared in that work tree. It only tells the agent
which target to edit, through `SYSROOT_EDITING_TARGET`; it never selects a machine to
run on or deploy to.

Profiles. With the default `--config-scope management`, the agent's configuration
directory (`CODEX_HOME` or `CLAUDE_CONFIG_DIR`) is
`$XDG_STATE_HOME/sysroot/agents/NAME/RUNTIME/VERSION` (default state root:
`~/.local/state`), created private. Version-specific profiles keep a newer runtime
from migrating state that an older bundled release, for example after an image
rollback, later reads. A new version can therefore require a new official login.
Existing profiles are kept; no OAuth/session copying or automatic migration is
performed. New Codex profiles choose `keyring` credential storage without plaintext
fallback, and existing preferences are preserved. The user must provide a working
unlocked Secret Service and complete the official login. `--config-scope personal`
leaves the inherited configuration variables as they are, so it uses personal
configuration; inside a session the launcher started, those still point at that
session's management profile. The configuration variables are inherited by the
agent's own child processes; they are not a sandbox. The wrapper never changes HOME.

`--print-plan` prints the selection as JSON. It runs `--version` with a disposable
private configuration directory, because native Codex writes helper files there even
for version or help, and does not create a profile or start a session.

Environment. Every launch removes `BW_SESSION`; `SSH_AUTH_SOCK` and official provider
authentication remain available. Only bundled Claude receives invocation-local
`DISABLE_UPDATES=1`, because the image updates it; personal runtimes keep their own
update settings. The wrapper never collects or copies login credentials. No
repository skill collection is installed globally or in these profiles.

`sysroot status --json` schema 2 reports implemented capabilities, their platform
and runtime requirements, and separate qualification limits. It does not inspect
installed runtimes, enrollment, home state or credentials, so a checkout build does
not claim that the selected agent exists or is authenticated on the current machine.

See [verified status](STATUS.md) for remaining qualification and distribution
prerequisites. Official configuration sources, retrieved 2026-09-08:
[Codex configuration](https://learn.chatgpt.com/docs/config-file/config-advanced),
[Codex credentials](https://learn.chatgpt.com/docs/auth),
[Claude environment](https://code.claude.com/docs/en/env-vars) and
[Claude updates](https://code.claude.com/docs/en/setup).
