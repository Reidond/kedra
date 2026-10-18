---
name: kedra-context
description: Start or resume Kedra work using current source, operational contracts, and actual verification status.
---

# Start here

Read AGENTS.md, worklog.md current/latest entries, usr/src/kedra/docs/STATUS.md and usr/src/kedra/docs/ARCHITECTURE.md. Inspect Git status, branch, remote, the `signoff` status and exact release runs before edits. usr/src/kedra/PLAN.md defines the product, not completion.

Use worklog.md for task learnings and AI/skill/workflow changes too, following AGENTS.md's single-ledger contract.

Pick the domain skill from skills/README.md: kedra-rust-workspace plus rust-router/domain-cli for Rust; kedra-bootc/release-signing/github-actions for images, updates and releases; kedra-home for managed home files; kedra-agents/bitwarden for runtimes and auth; kedra-desktop/machines for sessions and targets; kedra-research for tests and evidence; kedra-security for privilege and state.

Source and installed versions differ; consult STATUS before claiming a feature shipped. Distribution is signed GHCR images per target, plus installer ISO GitHub Releases the owner dispatches (`iso.yml`) and local on-demand media. Both targets build through one engine pipeline (RPM lock, foundation, `.kedra` packages, compose, derive, trust layer; docs/SYSTEM.md). OS images are built and signed only in Actions; changes are tested locally and merge on a `signoff` status (AGENTS.md "Local testing and sign-off"). Never enroll, format or bootc-switch the workstation.

Preserve user changes, explicit target scope, local-only home content and private credentials. Everything a session creates stays under the checkout's gitignored `target/`, with Docker objects labeled by checkout (AGENTS.md). Skills are for developing this repository only (plugin README). Record exact checks and unresolved behavior in the worklog; do not commit logs or research reports.
