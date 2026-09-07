# Kedra knowledge skills

These skills package the design session into task-sized instructions for Codex
and Claude Code. Read `kedra-context` first, then the relevant domain. Each skill
carries the decision, practical procedure, failure modes, research gates and
primary references. PLAN.md/RESEARCH.md and evidence reports provide depth.

| Skill | Use for |
|---|---|
| kedra-context | Project contract, rejected alternatives, reading order and next work |
| kedra-rust-workspace | Cargo layout, code boundaries, tools and upstream Rust skills |
| kedra-bootc | Fedora derivation, filesystem ownership, digest deployment and recovery |
| kedra-release-signing | Image trust, manifest identity, release approval, key lifecycle |
| kedra-github-actions | CI jobs, safe installer pipeline, VM evidence and artifact limits |
| kedra-home | Writable files, partial staging, local-only hunks, merges and activation |
| kedra-agents | Bundled/personal Codex/Claude, checkout launchers and configuration |
| kedra-bitwarden | SSH agent, first-run login, keyring, API/model/registry credentials |
| kedra-desktop | niri, Noctalia effective settings, session and hardware validation |
| kedra-machines | Shared/host scope, provenance, enrollment and independent updates |
| kedra-research | R01-R11 experiments, evidence, handoffs and knowledge maintenance |
| kedra-security | Privileged interface, path safety, journals, secret exclusion and recovery |

Canonical first-party source is here. Both `.agents/skills/` and `.claude/skills/`
contain per-skill links. Upstream Rust skills remain at their pinned submodule
source, with links to all top-level skill directories. Skill discovery does not
authorize execution of scripts, plugins, hooks, tools, or global configuration.

## Maintenance contract

When research changes an assumption, update the skill and the relevant report/ADR
in the same change. Distinguish product requirement, upstream-documented fact,
implementation hypothesis, and experimentally validated result. Record exact
versions/date and link the source or test. Keep detailed fixtures in reports and
reference notes; do not load the entire skill collection into every agent prompt.
