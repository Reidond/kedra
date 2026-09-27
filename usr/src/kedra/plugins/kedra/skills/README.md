# Kedra knowledge skills

These skills package the design session into task-sized instructions for Codex
and Claude Code. Read `kedra-context` first, then the relevant domain. Each skill
carries the decision, practical procedure, failure modes and primary references.
usr/src/kedra/PLAN.md, usr/src/kedra/docs/ARCHITECTURE.md and
usr/src/kedra/docs/STATUS.md provide current scope.

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

This directory is the single canonical source for the twelve Kedra skills and
their supporting files; both plugin manifests use it directly. The eighteen
selected upstream Rust skills live in the separate repository-local `rust-skills`
plugin (`usr/src/kedra/plugins/rust-skills/skills/`), whose NOTICE.md records
upstream provenance. Edit files in place; there is no sync command or copied
agent discovery tree.
Loading the plugin does not authorize hooks, tool installation, deployment or
personal configuration changes.

## Repository development only

These skills, including the pinned upstream Rust skills, are for developing this
repository. Both plugins are registered for this repository only, by the
checked-in marketplaces and project configuration described in `AGENTS.md`. They
are not part of the OS image, installer, managed home baseline, the `sysroot`
agent launchers or their profiles, and are not installed for other projects.
Any change to these skills bumps the plugin version in both manifests.

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
