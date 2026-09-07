# Measured bootstrap result, 2026-09-07

At source 98efe4d944a2bcfd27865edf6cbee0bd3ce4b94e, GitHub Actions run
34138882781 passed cargo xtask check on Rust/Cargo 1.98.1 and Ubuntu 24.04.4.
It verified four flat-source packages, the exact Rust-skills gitlink, 50 canonical
skills exposed by 100 per-agent symlinks, formatting, Clippy, 8 unit/integration
tests plus 1 doctest, release compilation and unchanged tracked source/lockfile.

Canonical evidence: docs/research/R11-rust-workspace/REPORT.md, environment.json
and results.json. The initial formatting failure is preserved in that report.
Actual Codex/Claude discovery, editor behavior, full adversarial validator checks,
upstream distribution clearance and OS/Fedora ABI integration are not proven.
Do not convert this narrow CI success into a blanket R11 or OS-readiness claim.

When changing the workspace or skills, rerun checks and preserve the distinction
between static link validity and runtime skill recognition. Never relax tests to
match an unverified upstream default. Subsequent evidence supersedes this snapshot
only with a new source/run identity and explicit outcomes.
