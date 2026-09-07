---
name: kedra-rust-workspace
description: Implement, test or refactor Rust in Kedra; preserve the flat Cargo workspace, pinned tooling, privilege boundaries and repository-local actionbook Rust skills.
---

# Rust/Cargo implementation contract

Read ADR 0001 and the upstream rust-router, domain-cli and applicable topic skill
from vendor/rust-skills/skills. Kedra's explicit choices override generic upstream
scaffolding, lint defaults, global skill creation and optional-tool suggestions.

## Layout and dependencies

Use a virtual workspace, edition 2024, resolver 3, one Cargo.lock. Every binary
has an explicit `[[bin]]` path to main.rs; the library uses `[lib] path = "lib.rs"`.
No first-party src/ directory at the repository root or within a crate/module.
Crate-local tests use normal tests/ paths. Root tests/ is for system fixtures,
not automatically discovered tests for the virtual workspace. Do not rewrite
third-party trees to match our layout.

sysroot is the ordinary-user CLI. sysroot-core contains small pure shared logic.
sysroot-helper is the future installed privileged executable, independent of CLI
and xtask orchestration. xtask is developer/CI tooling, never a root service.
Avoid speculative crate proliferation, frameworks, async runtimes and custom
parsers. Choose a real parser/serialization crate when production needs it; the
bootstrap's fixed manifest checks are not a general TOML implementation.

Use explicit process arguments, preserve nonzero exits, separate stdout data
from stderr diagnostics, and return typed recoverable errors when behavior is
implemented. Avoid panics on user input. Start with safe Rust; do not weaken the
workspace's unsafe prohibition as a shortcut. Rust does not prove authorization,
TOCTOU safety, merge correctness, crash recovery or signature enforcement.

## Tooling and checks

rust-toolchain.toml pins the bootstrap toolchain; Cargo.lock pins crate resolution.
The initial code uses only std. Changing pins requires review and CI, not a
moving `stable` update during agent startup. The current repository check is:

```sh
cargo xtask check
```

It validates first-party layout and skill wiring, then runs metadata, rustfmt
check, Clippy with warnings denied, tests/doctests and release compilation. Its
success is R11 bootstrap evidence, not OS or real-agent qualification. Verify
rust-analyzer separately. An installed OS binary must be built with a compatible
Fedora/target ABI, not copied blindly from a newer Ubuntu build environment.

The bootstrap passed this check on Rust 1.98.1 in Actions. Read
[the evidence summary](references/bootstrap-evidence.md) and the linked R11 report
for the exact source/run and remaining untested cases. Do not present planned
research behavior as implemented merely because the scaffold compiles.

## Skill lifecycle

Pin upstream via gitlink plus skills.lock.toml. Initialize explicitly with
`git submodule update --init --recursive`; never use --remote automatically.
Per-skill links expose the same canonical source to Codex/Claude. Preserve support
files. Updating the pin also updates the validator's bootstrap pin and reruns
link/discovery tests. Missing source produces a setup message, not a global
installer invocation. No upstream hook/plugin/MCP execution is implied.

Evidence gate: R11 for layout/tools/discovery; R10 for privileged protocol.
Sources: docs/SOURCES.md entries cargo-targets, cargo-workspaces, rust-release,
rust-skills, codex-skills, claude-skills; ADR 0001.
