---
name: kedra-context
description: Start or resume Kedra work using current source, operational contracts, and actual verification status.
---

# Start here

Read AGENTS.md, worklog.md current/latest entries, usr/src/kedra/docs/STATUS.md and usr/src/kedra/docs/ARCHITECTURE.md. Inspect Git status, branch, remote and exact CI before edits. usr/src/kedra/PLAN.md defines the product, not completion.

Use worklog.md for task learnings and AI/skill/workflow changes too, following AGENTS.md's single-ledger contract.

Use kedra-rust-workspace plus rust-router/domain-cli for Rust; kedra-bootc/release-signing/github-actions for production; kedra-home for writable configuration; kedra-agents/bitwarden for runtimes/auth; kedra-desktop/machines for sessions/targets; kedra-security for privilege/state.

Source and installed versions differ; consult STATUS before claiming newer features shipped. Current distribution is signed GHCR images, plus installer ISO GitHub Releases the owner dispatches (`iso.yml`) and local on-demand media. Preserve user changes, explicit target scope, local-only home content and private credentials. OS image builds belong in Actions; testing of changes is local and ends in a `signoff` (AGENTS.md "Local testing and sign-off"). Never enroll or format the workstation.

Skills are for developing this repository only; both plugins are registered for this repository (AGENTS.md). No generated copies or OS provisioning. Record exact checks and unresolved behavior in worklog; keep logs/artifacts in Actions or disposable local output, not tracked research directories.
