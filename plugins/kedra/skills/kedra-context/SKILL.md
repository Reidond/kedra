---
name: kedra-context
description: Start or resume any Kedra/sysroot task; recover the session decisions, architecture, research order, and non-negotiable boundaries before changing the repository.
---

# Kedra context and routing

Kedra is a personal Fedora 44 bootc OS, not a general-purpose distro framework.
`sysroot` is its management command. The owner prompts an official coding agent;
the agent edits the source and approved dotfile changes, pushes when authorized,
follows GitHub Actions, and stages the approved signed image digest. Reboot and
post-boot health are separate states. Deterministic tooling must work without AI.

## Load and inspect

Read AGENTS.md, docs/HANDOFF.md, docs/SESSION.md, PLAN.md, the relevant R01-R11
packet in RESEARCH.md, and actual code/CI/research status. Check the branch,
remote and dirty state. Never assume plans are implemented. At bootstrap only
help/version/status work; all operational commands deliberately refuse execution.

## Keep these decisions

Rust edition 2024, Cargo workspace, no first-party src/ folders, explicit entry
paths, pinned toolchain, rustfmt and Clippy. The TS/Effect/Vite Plus idea and
BlueBuild were rejected. Shared Linux-shaped files plus small host overrides
produce independently updated target images and installers. No host branches.

Real home files stay writable. Support selected line publication, visible
uncommitted edits, explicit local-only hunks/settings, and safe discard. This is
not whole-home synchronization or a read-only symlink system. Private review
history never becomes pushed source history.

Bundled agents are private OS tools reached through sysroot. Personal executable
versions, MCP, skills and settings remain independent and optionally tracked.
Bitwarden SSH is integrated, but account setup and authorization remain user
operations. No private-key export or general unlocked vault session for agents.

## Route the task

Use kedra-rust-workspace plus rust-router/domain-cli for Rust. Use kedra-bootc,
kedra-release-signing and kedra-github-actions for OS releases; kedra-home for
configuration; kedra-agents and kedra-bitwarden for coding/auth; kedra-desktop and
kedra-machines for host/session changes. kedra-security applies to any privileged
or persistent-state change. kedra-research defines how to turn assumptions into
evidence rather than repeatedly drafting speculative architecture.

## Deliver a useful continuation

State the selected packet/scope and affected targets. Implement the smallest
reproducible experiment or gated production change. Never build an OS locally,
enroll the current home, format disks, or weaken verification during research.
Preserve user work. Update the skill when a durable finding changes the known
procedure. Report exact checks, not-run cases, and a concrete next step.

Sources: the owner's session contract in docs/SESSION.md and PLAN.md. These are
requirements, not third-party product claims or records of completed deployment.
