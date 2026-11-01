---
name: kedra-agents
description: Maintain the sysroot Codex/Claude launchers and the bundled Codex package while preserving personal installations and updates, MCP/skill ownership, checkout state and official account authentication.
---

# Two runtimes, explicit configuration scope

Read usr/src/kedra/docs/AGENT-LAUNCHERS.md and usr/src/kedra/docs/STATUS.md.
Bundled official CLIs are OS-management tools: catalog packages with a private
`libexec` export, launched by sysroot through /usr/share/kedra/catalog-libexec/NAME
(crates/sysroot/agents/linux.rs). They never install /usr/bin/codex or
/usr/bin/claude, add aliases or shadow personal PATH commands; a libexec export
never reaches PATH, so `--runtime user` cannot find the bundled one. Ordinary
codex/claude belong to the user's independently updated setup. Codex 0.153.4 is
the `codex` package (system/packages/codex.kedra) on both targets.
Claude is not bundled until the owner accepts its preinstallation terms.
Authenticated model use, MCP and skills remain unqualified.

## Bundled Codex package

The Codex archive is one SHA-256 pin per architecture in system/packages.lock.json
(`by_architecture` in codex.kedra). Provenance is checked when a pin is added or
bumped, not in builds: `uv run usr/src/kedra/image/packages/verify-codex.py`
verifies the Sigstore bundles of codex, codex-code-mode-host and bwrap (identity
`https://github.com/openai/codex/.github/workflows/rust-release.yml@refs/tags/rust-v<version>`,
issuer token.actions.githubusercontent.com, Cosign 3.1.3), then `pin.py` records
the bytes (docs/PACKAGES.md "Updating a pin"). The package build runs
`codex --version` against the declared version with a private `CODEX_HOME`.
Size limit (2026-10-16): the x86_64 binary is 258.7 MB, close to the engine store's
256 MiB per-file bound (`MAX_FILE`, crates/sysroot-engine/tree.rs); a larger
release is refused at admission, so a bump past it needs an engine change first.

## Launcher interface

`sysroot codex` / `sysroot claude`: bundled runtime, management profile, Kedra
checkout. `--runtime user`: the personal executable on absolute PATH entries or
an explicit absolute `--executable`, still the checkout and management profile by
default. `--config-scope personal`: inherit personal configuration variables as
they are. `--host xps`: the target to edit (`SYSROOT_EDITING_TARGET`), never a
machine to run on or deploy to; it must be declared in the work tree. Wrapper
options precede `--`; everything after goes unchanged to the official CLI. Read
references/launch-matrix.md before changing runtime or profile selection.

The checkout is the Git work tree containing `--repo`, `SYSROOT_REPO` or
~/src/kedra. Any clone or worktree is accepted; the launcher never fetches,
stashes, resets or locks it, and concurrent sessions use separate worktrees. A
missing personal runtime fails; there is no bundled fallback. Launch as the
ordinary user. The launcher keeps version-specific management profiles
(`$XDG_STATE_HOME/sysroot/agents/NAME/RUNTIME/VERSION`), the keyring-only Codex
config for new profiles, `BW_SESSION` removal on every launch, invocation-local
`DISABLE_UPDATES=1` for bundled Claude only, and `--print-plan`, whose `version`
and `deployment_authorized` fields the container probe
(tests/container/lab/probes/agents.py) and tests/agents/run.py read. The launcher
deliberately has no origin-remote check or checkout lock: neither is a security
boundary (a local clone's origin is a path; an advisory lock is inherited by
agent children).

## Config and credentials

Use writable dedicated profiles outside tracked home inputs. CODEX_HOME and
CLAUDE_CONFIG_DIR are documented building blocks, not complete isolation; agent
children inherit them. Do not change real HOME to fake separation. Test external
skill discovery, keyring account names, MCP/hooks, credentials, child environments
and shared paths against pinned versions. Official login flows remain intact.

Codex 0.153.4 writes helper files to its configuration directory before it parses
`--help` or `--version`, so version discovery uses a disposable private directory
while preserving HOME; it is not a read-only operation on the caller's CODEX_HOME
(observed 2026-09-08: a version probe with the caller's CODEX_HOME wrote helper
files into it; source: the `--version` probe in agents/linux.rs and the
codex.kedra build).

Bundle updates arrive through images; personal versions update through their own
tools. Never globally set DISABLE_UPDATES or modify personal settings. Updating
or rolling back Kedra does not downgrade or repair personal agents.

User installation declarations, safe settings, MCP and skills can each be managed
or unmanaged independently. Track declarative installation intent, not binary
caches. No blanket .codex/.claude adoption: credentials, transcripts and sessions
stay private. Kedra AGENTS.md is shared via CLAUDE.md import; repository
instructions are not a sandbox.

Review distribution terms and notices before baking non-RPM binaries into public
images. Avoid custom OAuth/token synchronization. Primary references:
[Codex configuration](https://developers.openai.com/codex/config-advanced/),
[Codex authentication](https://developers.openai.com/codex/auth/),
[Codex skills](https://developers.openai.com/codex/skills/),
[Claude environment](https://code.claude.com/docs/en/env-vars),
[Claude authentication](https://code.claude.com/docs/en/authentication) and
[Claude setup](https://code.claude.com/docs/en/setup).
Use kedra-bitwarden for credential boundaries and usr/src/kedra/THIRD_PARTY.md for distribution.

## Repository skill discovery (Codex)

Audited 2026-09-09 against pinned source
`3d2ee51ca2d5db578f328aa75e20aa22c0197c9a` (0.153.4) with a disposable
CODEX_HOME; no installation or model turn was performed. CLI plugin listing uses
configured marketplace roots, while standalone repository discovery scans
`.agents/skills`, not `usr/src/kedra/plugins/kedra/skills`, so an empty listing
there is expected. An `AVAILABLE` marketplace entry does not load the plugin; its
loader requires an active cache installation.

Owner decision (2026-09-27): register both repository plugins for this repository.
Source review of `openai/codex` at `06f97622f852` (identical at `rust-v0.157.1`,
2026-09-26) and local 0.156.1 experiments with a disposable `CODEX_HOME` found:

- A trusted project's `.codex/config.toml` may set `[plugins."<name>@<marketplace>"]`
  (`config/src/loader/mod.rs`, `core-plugins/src/marketplace_policy.rs`). An
  untrusted project's layer is ignored.
- A `[marketplaces.<name>]` local `source` is resolved against the process
  working directory, not the config file, so the checked-in config omits it.
- The TUI and app server discover `<git root>/.agents/plugins/marketplace.json`.
  CLI `codex plugin` commands do not (`cli/src/plugin_cmd.rs`).
- Plugins load only from `$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<version>/`.
  The background refresh reinstalls only when the manifest version changes
  (`core-plugins/src/loader.rs`), so skill changes bump the plugin version.

Hence: checked-in `.codex/config.toml` plus once-per-machine `codex plugin
marketplace add .` and `codex plugin add` from the repository root. Do not link
the skill tree into discovery folders or provision it into launcher profiles.
The inspected native CLI has no `--plugin-dir`/`--skills-dir` option;
`skills.config` only enables/disables discovered skills and is not a root mapping.

The documented App Server `skills/extraRoots/set` endpoint replaces additional
roots without persistence and exists in the 0.153.4 schema. It is a host
integration mechanism, not a native CLI route, and supplies user-scope roots to
the whole process, so a shared process could expose Kedra skills to unrelated
checkouts. Do not claim loaded-plugin/model qualification from help, schema
availability or direct file access; extra-root execution and model use of the
repository skills are not-run. Sources:
[pinned CLI root selection](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/cli/src/plugin_cmd.rs#L261),
[pinned skill roots](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/ext/skills/src/host_roots.rs#L28),
[official App Server reference](https://learn.chatgpt.com/docs/app-server).
