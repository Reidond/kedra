# ADR 0001: Rust workspace and repository-local skills

Date: 2026-09-07. Decision accepted by the owner. Execution evidence is tracked
separately under R11; accepting a design does not certify runtime integration.

## Decision

Use Rust edition 2024 and a Cargo workspace with one Cargo.lock. All first-party
packages declare explicit `main.rs` or `lib.rs` paths directly in their package
directories. No root or nested first-party `src/` folders. Modules and crate-local
tests remain ordinary Rust modules/tests. Do not rewrite third-party source.

Packages: `crates/sysroot` (ordinary-user CLI), `crates/sysroot-core` (small pure
shared types/logic), `crates/sysroot-helper` (future installed privileged boundary),
`xtask` (unprivileged development/CI tasks). The helper may eventually depend on
pure shared types, never on the user CLI, agent executor or checkout task runner.
Do not split every future subsystem into a crate before a boundary justifies it.

The bootstrap pins Rust 1.98.1, selected from the official release information
available on 2026-09-07. CI determines whether the pin and workspace actually
work. Pin changes are reviewed. Use rustfmt and Clippy, safe Rust, explicit process
arguments, typed errors for implemented behavior, and Git itself for staging and
merge operations. No custom Git engine. Start without third-party Rust crates;
choose small justified dependencies in the relevant research rather than building
a general framework. The minimal bootstrap validator checks our manifest spelling;
it is not a general TOML parser or a complete privilege analysis.

This supersedes TypeScript/Effect v4/Vite Plus/Oxlint/Oxfmt. A third-party program
may use another runtime; the decision applies to our implementation.

## Skills

Keep first-party domain skills under `skills/`. Pin actionbook/rust-skills as a
Git submodule at `vendor/rust-skills` to
`5c40d3ad785193231b7d0dbfb8e1eb447e5edd94`. `skills.lock.toml` records the source and
skills-only integration. Per-skill symlinks under `.agents/skills` and
`.claude/skills` point to the canonical first-party/upstream directories, retaining
upstream support references. Git records real symlinks and a mode-160000 gitlink.

Kedra owns AGENTS.md; CLAUDE.md imports it. Never replace these with upstream
project defaults. No global skills, hooks, plugin registration, background agent
setup, example permissions, automatic MCP/browser installation, or moving-branch
updates. Mentioned optional tools are not assumed available. Read the router and
relevant topics, not the whole third-party tree. Kedra's layout/lints/authorization
rules override incompatible generic suggestions, including global skill creation.

Update the gitlink, skills.lock.toml, validator pin, and owned links together.
Initialize explicitly on clone; launchers never fetch missing skills. Static
validation checks pin/link consistency and frontmatter markers. Real CLI
recognition, reference loading, profile conflicts, and editor behavior are
separate R11 cases. Preserve upstream notices; distribution/license review stays
open before copying third-party content into OS releases.

## Checks

`cargo xtask check` runs layout/link validation, Cargo metadata, rustfmt check,
Clippy with warnings denied, unit/integration/doc tests and release compilation.
CI uses least-privilege checkout pinned to a full commit SHA. It never builds an
OS or grants deployment permission. Production helper and home safety are still
R01-R10 gates; Rust memory safety does not establish their correctness.

## Sources

- https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- https://doc.rust-lang.org/cargo/reference/workspaces.html
- https://blog.rust-lang.org/releases/latest/
- https://github.com/actionbook/rust-skills/tree/5c40d3ad785193231b7d0dbfb8e1eb447e5edd94
- https://developers.openai.com/codex/skills/
- https://code.claude.com/docs/en/skills
