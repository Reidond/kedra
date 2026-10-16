---
name: kedra-agents
description: Build sysroot Codex/Claude launchers while preserving personal installations, frequent personal updates, MCP/skill ownership, checkout state and official account authentication.
---

# Two runtimes, explicit configuration scope

Bundled official CLIs are OS-management tools: catalog packages with a private
`libexec` export, launched through /usr/share/kedra/catalog-libexec/NAME by
sysroot. They must not install /usr/bin/codex or /usr/bin/claude, add aliases or
shadow personal PATH commands; a libexec export never reaches PATH, so `--runtime
user` cannot find the bundled one. Ordinary codex/claude belong to the user's
independently updated setup. Codex 0.153.4 is the `codex` package
(usr/src/kedra/image/packages/codex.kedra) on both targets; Claude is not bundled
until the owner accepts its preinstallation terms. Authenticated model use, MCP
and skills remain unqualified.

Settled 2026-10-16 (engine packages): the Codex archive is one SHA-256 pin per
architecture in packages.lock.json. Sigstore verification of the codex,
codex-code-mode-host and bwrap blobs (identity
`openai/codex/.github/workflows/rust-release.yml@refs/tags/rust-v<version>`,
issuer token.actions.githubusercontent.com, Cosign 3.1.3) runs when the pin is
added or bumped (`uv run usr/src/kedra/image/packages/verify-codex.py`, then
`pin.py`); builds trust the pin. The former exact six-file list and manifest
equality are gone: the pin fixes those bytes, and the package build runs
`codex --version` against the declared version. The x86_64 binary is 258.7 MB,
close to the engine's 256 MiB per-file store limit; a bump past it needs an
engine change first.
See usr/src/kedra/docs/AGENT-LAUNCHERS.md and usr/src/kedra/docs/STATUS.md for actual scope.

## Intended interfaces (not upstream flags)

`sysroot codex` / `sysroot claude`: bundled runtime, management profile, OS checkout.
`--runtime user`: explicit user executable, still the checkout and management
profile by default. `--config-scope personal`: explicitly choose personal settings.
`--host xps`: edit another target, not execute on or deploy to it. Separate wrapper
options from upstream arguments with a documented delimiter/forwarding rule.
Read references/launch-matrix.md for the R05 matrix.

Resolve a configurable user-owned checkout (default ~/src/kedra) and preserve
staged/unstaged/untracked work without fetching, stashing or resetting. Offline use
of an existing checkout should work. Report selected executable/version/profile and
executing host versus editing target. Never silently fall back to the bundled binary
when a requested personal runtime is missing. Launch as the ordinary user.

Settled 2026-10-14 (launcher simplification): the launcher no longer checks the
origin remote or takes an advisory checkout lock. Neither was a security boundary:
the remote string refused legitimate clones (a local clone's origin is a path) and
the lock was advisory, inherited by agent children and ignored by other tools. Any
work tree is accepted; concurrent sessions use separate worktrees. Nested launches
no longer restore saved outer configuration variables: personal scope inherits the
environment as is. Kept: the ordinary-user refusal, version-specific management
profiles, the keyring-only Codex config, the disposable `--version` probe, the
declared-target check for `--host`, `BW_SESSION` removal and bundled-only Claude
`DISABLE_UPDATES=1`. `--print-plan` keeps `version` and `deployment_authorized`,
which the container and native agent probes read.

## Config and credentials

Use writable dedicated profiles outside tracked home inputs. CODEX_HOME and
CLAUDE_CONFIG_DIR are documented building blocks, not complete isolation. Do not
change real HOME to fake separation. Test external skill discovery, keyring account
names, MCP/hooks, credentials, child environments and shared paths against pinned
versions. Separate personal-runtime management state from bundled-runtime state
when schema versions can differ. Official login flows remain intact.

Codex 0.153.4 performs helper-path setup before parsing help/version. Version
discovery must use a disposable config directory while preserving HOME; it is
not necessarily a read-only operation with the caller's normal CODEX_HOME.
R05 run 34182176074 exposed the side effect; corrected run 34182776503 at 43873cb
(2026-09-08) passes native 0.153.4/0.153.3 runtime/scope probes and three Sigstore
checks. Model turns, keyring login and external discovery remain separate gates.

Bundle updates through images; personal versions update through their own tools.
Disable bundled Claude updates only in that invocation and prevent leakage into
user-runtime launches. Never globally set DISABLE_UPDATES or modify personal
settings. Updating/rolling back Kedra does not downgrade or repair personal agents.

User installation declarations, safe settings, MCP and skills can each be managed
or unmanaged independently. Track declarative installation intent, not binary
caches. No blanket .codex/.claude adoption: credentials, transcripts and sessions
stay private. A mixed-secret configuration needs field projection or exclusion.
Kedra AGENTS.md is shared via CLAUDE.md import; repo instructions are not a sandbox.

## Gates and sources

R05 tests runtime/profile/update/distribution; R06 auth; R11 repository discovery.
Review distribution terms and notices before baking non-RPM binaries into public
images. Avoid custom OAuth/token synchronization. Primary references:
[Codex configuration](https://developers.openai.com/codex/config-advanced/),
[Codex authentication](https://developers.openai.com/codex/auth/),
[Codex skills](https://developers.openai.com/codex/skills/),
[Claude environment](https://code.claude.com/docs/en/env-vars),
[Claude authentication](https://code.claude.com/docs/en/authentication) and
[Claude setup](https://code.claude.com/docs/en/setup).
Use kedra-bitwarden for credential boundaries and usr/src/kedra/THIRD_PARTY.md for distribution.

## Repository skill discovery — Codex 0.153.4

Audited 2026-09-09 against pinned source
`3d2ee51ca2d5db578f328aa75e20aa22c0197c9a`. Native Windows version and
top-level/plugin/marketplace/app-server help pass with a disposable CODEX_HOME;
no installation, registration or model turn was performed. R11's earlier empty
listing is explained by the runtime's loading boundary: CLI plugin listing uses
configured marketplace roots, while standalone repository discovery scans
`.agents/skills`, not `usr/src/kedra/plugins/kedra/skills`. An `AVAILABLE` marketplace entry
does not load the plugin; its loader requires an active cache installation.

Owner decision (2026-09-27): register both repository plugins for this
repository instead of relying on manual reads. Source review of `openai/codex`
at `06f97622f852` (identical at `rust-v0.157.1`, 2026-09-26) and local 0.156.1
experiments with a disposable `CODEX_HOME` found:

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

The documented App Server `skills/extraRoots/set` endpoint can replace additional
roots without persistence and exists in the retained 0.153.4 schema. That is a
host integration mechanism, not a demonstrated native CLI route. It supplies
standalone user-scope roots to the whole process; using it in a shared process
could expose Kedra skills to unrelated checkouts. A custom client/daemon is
outside the current launcher task. Do not claim loaded-plugin/model qualification
from help, schema availability or direct file access. Extra-root execution and
actual root/crate model use remain not-run under R11.

Evidence and exact source links: [R11 discovery audit](https://github.com/Reidond/kedra/blob/main/usr/src/kedra/docs/STATUS.md),
[pinned CLI root selection](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/cli/src/plugin_cmd.rs#L261),
[pinned skill roots](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/ext/skills/src/host_roots.rs#L28),
[official App Server reference](https://learn.chatgpt.com/docs/app-server).
