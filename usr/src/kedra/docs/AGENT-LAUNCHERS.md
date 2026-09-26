# Agent launchers

On Linux, `sysroot codex` and `sysroot claude` select a runtime and open the
verified Kedra checkout. The launchers do not install agent binaries or
authorize publishing, deployment or reboot.

```sh
sysroot codex --repo ~/src/kedra --runtime user --print-plan
sysroot codex --repo ~/src/kedra --runtime user -- --help
sysroot claude --repo ~/src/kedra --runtime user --config-scope personal -- --version
sysroot codex --repo ~/src/kedra --host xps --runtime user -- 'Review the target inputs'
```

Wrapper options precede `--`; everything after it goes directly to the official
CLI. `--runtime user` uses the personal executable on absolute PATH entries, or an
explicit absolute `--executable`. Missing personal executables fail without a
bundled fallback. The default bundled path is
`/usr/libexec/sysroot/agents/NAME/bin/NAME`, outside ordinary PATH. The desktop
image packages the pinned official
Codex runtime, whose native VM startup and profile-retention checks pass. Claude
is not packaged pending the owner's preinstallation terms choice. Authenticated
model workflows and the complete runtime/profile matrix remain unqualified.
`--print-plan` runs `--version` in
a disposable private configuration directory; it does not create a persistent
profile, take the checkout lock, or start an agent session. Native CLI startup can
write runtime files even when only help/version was requested.

`sysroot status --json` schema 2 reports implemented capabilities, their platform
and runtime requirements, and separate qualification limits. It replaces the
bootstrap availability flags and does not inspect installed runtimes, enrollment,
home state or credentials. A checkout build therefore does not claim that the
selected agent exists or is authenticated on the current machine.

The checkout comes from `--repo`, `SYSROOT_REPO`, or `~/src/kedra`. Its local origin
must identify Reidond/kedra using the documented GitHub HTTPS or SSH spelling.
This catches mistakes; a remote string is not a security boundary. No fetch,
clone, stash, reset or index change happens. Editing a disabled target is allowed;
the source builder still refuses to build it. A cooperative checkout lock prevents
two wrappers from editing the same checkout. Use separate worktrees for deliberate
concurrency. Other tools can ignore this advisory lock. Agent children can retain
it after the main agent exits, so a remaining child can keep the checkout busy.

Runtime and configuration scope are separate. Management state lives at
`$XDG_STATE_HOME/sysroot/agents/NAME/RUNTIME/VERSION` (default state root:
`~/.local/state`). Version-specific profiles prevent a newer executable from
migrating state that an older bundled release later reads. A new version can
require a new official login. Existing profiles are retained; no OAuth/session
copying or automatic migration is performed. New Codex profiles choose `keyring`
credential storage without plaintext fallback. Existing preferences are preserved.
The user must provide a working unlocked Secret Service and complete official login.

`--config-scope personal` explicitly uses personal configuration, including custom
inherited config-directory controls. Nested wrapper invocations restore controls
saved by the outer wrapper when selecting personal scope. The wrapper never changes
HOME. Codex/Claude config-directory variables are inherited by their own child
processes; they do not form a sandbox. Ordinary commands outside this invocation
remain personal. Test actual external skill, hook, MCP and keyring behavior against
each pinned runtime before treating profile separation as qualified.

Only bundled Claude receives invocation-local `DISABLE_UPDATES=1`. User runtime
launches restore the caller's previous setting if nested inside that invocation.
All launches remove `BW_SESSION`; SSH_AUTH_SOCK and official provider authentication
remain available. The wrapper never collects or copies login credentials and
executes as the ordinary user with normal upstream interaction, signals and exit
status. No repository skill collection is installed globally or in these profiles.

See [verified status](STATUS.md) for remaining qualification and
distribution prerequisites. Official configuration sources, retrieved 2026-09-08:
[Codex configuration](https://learn.chatgpt.com/docs/config-file/config-advanced),
[Codex credentials](https://learn.chatgpt.com/docs/auth),
[Claude environment](https://code.claude.com/docs/en/env-vars) and
[Claude updates](https://code.claude.com/docs/en/setup).
