---
name: kedra-agents
description: Build sysroot Codex/Claude launchers while preserving personal installations, frequent personal updates, MCP/skill ownership, checkout state and official account authentication.
---

# Two runtimes, explicit configuration scope

Bundled official CLIs are OS-management tools under
/usr/libexec/sysroot/agents/, invoked by absolute path through sysroot. They must
not install /usr/bin/codex or /usr/bin/claude, add aliases or shadow personal PATH
commands. Ordinary codex/claude belong to the user's independently updated setup.
The Linux launcher prototype now selects these paths (with bin/NAME below each
agent directory); image packaging and complete R05 qualification remain pending.
See docs/AGENT-LAUNCHERS.md and docs/research/R05-agents/REPORT.md for actual scope.

## Intended interfaces (not upstream flags)

`sysroot codex` / `sysroot claude`: bundled runtime, management profile, OS checkout.
`--runtime user`: explicit user executable, still the checkout and management
profile by default. `--config-scope personal`: explicitly choose personal settings.
`--host xps`: edit another target, not execute on or deploy to it. Separate wrapper
options from upstream arguments with a documented delimiter/forwarding rule.
Read references/launch-matrix.md for the R05 matrix.

Resolve a configurable user-owned checkout (proposed default ~/src/kedra), verify
its expected remote, preserve staged/unstaged/untracked work, and coordinate
concurrent sessions without silently stashing/resetting. Offline use of an existing
checkout should work. Report selected executable/version/profile and executing
host versus editing target. Never silently fall back to the bundled binary when
a requested personal runtime is missing. Launch as the ordinary user.

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
images. Avoid custom OAuth/token synchronization. Sources: docs/SOURCES.md
codex-config, codex-auth, codex-skills, claude-env, claude-auth, claude-skills,
claude-distribution. Use kedra-bitwarden for credential boundaries.

## Repository skill discovery — Codex 0.153.4

Audited 2026-09-09 against pinned source
`3d2ee51ca2d5db578f328aa75e20aa22c0197c9a`. Native Windows version and
top-level/plugin/marketplace/app-server help pass with a disposable CODEX_HOME;
no installation, registration or model turn was performed. R11's earlier empty
listing is explained by the runtime's loading boundary: CLI plugin listing uses
configured marketplace roots, while standalone repository discovery scans
`.agents/skills`, not `plugins/kedra/skills`. An `AVAILABLE` marketplace entry
does not load the plugin; its loader requires an active cache installation.

Continue by reading canonical files through the checkout's AGENTS.md routing.
Do not copy or link the skill tree into discovery folders, register it in a
personal/management profile, or globally install it as an incidental remedy.
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

Evidence and exact source links: [R11 discovery audit](../../../../docs/research/R11-rust-workspace/REPORT.md#repository-only-codex-discovery-audit--2026-09-09),
[pinned CLI root selection](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/cli/src/plugin_cmd.rs#L261),
[pinned skill roots](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/ext/skills/src/host_roots.rs#L28),
[official App Server reference](https://learn.chatgpt.com/docs/app-server).
