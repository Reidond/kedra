# Kedra knowledge skills

These skills package Kedra's design decisions into task-sized instructions for
Codex and Claude Code. Read `kedra-context` first, then the skill the task
needs. Each skill carries the decision, practical procedure, failure modes and
primary references. usr/src/kedra/PLAN.md, usr/src/kedra/docs/ARCHITECTURE.md
and usr/src/kedra/docs/STATUS.md provide current scope.

| Task | Read |
|---|---|
| Starting or resuming any work | kedra-context |
| Rust | kedra-rust-workspace, upstream rust-router, the relevant topic skill and domain-cli |
| OS, Fedora base, filesystem layout, updates | kedra-bootc |
| Images, signing, verification, promotion | kedra-release-signing |
| Actions, installer ISO, VM drivers, provenance | kedra-github-actions |
| Dotfiles, managed home files, patches, local-only changes | kedra-home |
| Codex/Claude launchers, bundled Codex, MCP and skills | kedra-agents |
| Bitwarden, SSH, keyring, credentials | kedra-bitwarden |
| niri, Noctalia, session, hardware | kedra-desktop |
| Targets, system/ sets, provenance, scope | kedra-machines |
| Container tests, `kedra-lab`, screenshots | kedra-research and kedra-desktop |
| End-to-end qualification and evidence | kedra-research |
| Privilege, persistent state, privacy, Secure Boot/TPM, recovery | kedra-security |

First-party skills are `usr/src/kedra/plugins/kedra/skills/<name>/SKILL.md`; the
eighteen pinned upstream Rust skills are
`usr/src/kedra/plugins/rust-skills/skills/<name>/SKILL.md`, whose NOTICE.md
records upstream provenance. Read referenced material as needed, not every skill
in every prompt. Upstream advice does not override AGENTS.md, the task's
authorization, the source layout or the pinned build and lint settings.

This directory is the single canonical source for the twelve Kedra skills and
their supporting files; both plugin manifests use it directly. Edit files in
place; there is no sync command or copied agent discovery tree. Loading the
plugin does not authorize hooks, tool installation, deployment or personal
configuration changes.

## Repository development only

These skills, including the pinned upstream Rust skills, are for developing this
repository. Both plugins are registered for this repository only, by the
checked-in marketplaces and project configuration ([plugin README](../README.md)).
They are not part of the OS image, installer, managed home baseline, the `sysroot`
agent launchers or their profiles, and are not installed for other projects.
Any change to these skills bumps the plugin version in both manifests.

## Maintenance contract

When a test or experiment changes an assumption, update the skill and the
operational docs in the same change. Distinguish product requirement,
upstream-documented fact, implementation hypothesis and experimentally validated
result. A durable fact keeps its source and/or experiment, tool versions and
date, failure behavior and the feature it affects; cite a doc or source file, not
an Actions run ID or a worklog entry. Remove text about removed code. Generated
output stays under the checkout's gitignored `target/` (AGENTS.md); do not load
the entire skill collection into every agent prompt.

Both agents also maintain root `worklog.md` as AGENTS.md specifies: read its
current status and recent entries when starting or resuming, record actual work
and evidence at meaningful milestones, and update status, blockers and next steps
before handoff. Skills carry reusable knowledge; the worklog records project
progress. Neither replaces actual end-to-end evidence or grants permission to
publish or deploy.
