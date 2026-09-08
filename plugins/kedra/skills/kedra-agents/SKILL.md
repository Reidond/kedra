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
