---
name: kedra-context
description: Start or resume Kedra work using current source, operational contracts, and actual verification status.
---

# Start here

Read AGENTS.md, worklog.md current/latest entries, docs/STATUS.md and docs/ARCHITECTURE.md. Inspect Git status, branch, remote and exact CI before edits. PLAN.md defines the product, not completion.

Use kedra-rust-workspace plus rust-router/domain-cli for Rust; kedra-bootc/release-signing/github-actions for production; kedra-home for writable configuration; kedra-agents/bitwarden for runtimes/auth; kedra-desktop/machines for sessions/targets; kedra-security for privilege/state.

Source and installed release versions differ. Published r1 is c660c58; consult STATUS before claiming newer features shipped. Preserve user changes, explicit target scope, local-only home content and private credentials. OS builds belong in Actions. Never enroll or format the workstation.

Skills remain checkout-local. No global installation, generated copies or OS provisioning. Record exact checks and unresolved behavior in worklog; keep logs/artifacts in Actions, not tracked research directories.
