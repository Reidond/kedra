# Kedra knowledge skills

These skills package the design session into task-sized instructions for Codex
and Claude Code. Read `kedra-context` first, then the relevant domain. Each skill
carries the decision, practical procedure, failure modes and primary references.
PLAN.md, docs/ARCHITECTURE.md and docs/STATUS.md provide current scope.

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
| kedra-research | End-to-end qualification, exact evidence and knowledge maintenance |
| kedra-security | Privileged interface, path safety, journals, secret exclusion and recovery |

This directory is the single canonical source for twelve Kedra skills and
eighteen selected upstream Rust skills, with their supporting files. Both plugin
manifests use it directly. Edit files here; there is no sync command or copied
agent discovery tree. The plugin's third-party/rust-skills/NOTICE.md records upstream provenance.
The plugin is development knowledge for a Kedra checkout. Loading it does not
authorize hooks, tool installation, deployment or personal configuration changes.

## Repository scope only

This entire collection, including the pinned upstream Rust skills, belongs only
to the Kedra checkout/worktrees. It must not be installed or registered in global
user skill directories, system directories, global plugins, or shared agent
profiles. Do not package it as a system-wide library in the OS image, installer,
or managed home baseline. A normal Kedra clone contains these repository files;
that is not a global skill installation.

The bundled `sysroot` agent launchers should find the collection by opening this
repository, not by copying it into personal or global configuration. This does
not change the user's independent ownership of personal skills or their optional
tracking choices. See `AGENTS.md` for the full scope and authorization contract.

## Maintenance contract

When qualification changes an assumption, update the skill and operational docs
in the same change. Distinguish product requirement, upstream-documented fact,
implementation hypothesis, and experimentally validated result. Record exact
versions/date and link the source or Actions run. Keep generated output in Actions
artifacts; do not load the entire skill collection into every agent prompt.

Both agents must also maintain root `worklog.md` as specified in `AGENTS.md`:
read its current status and recent entries when starting/resuming, record actual
work and evidence at meaningful milestones, and update status/blockers/next steps
before handoff. Skills carry reusable knowledge; the worklog records project
progress. Neither replaces actual end-to-end evidence or grants permission to
publish or deploy.
