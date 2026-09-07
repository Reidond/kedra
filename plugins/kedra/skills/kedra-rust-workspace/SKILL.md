---
name: kedra-rust-workspace
description: Implement, test or refactor Rust in Kedra; preserve the flat Cargo workspace, pinned tooling, privilege boundaries and repository-local actionbook Rust skills.
---

# Rust/Cargo implementation contract

Read ADRs 0001 and 0003 and the upstream rust-router, domain-cli and applicable topic skill
from plugins/kedra/skills/. Kedra's explicit choices override generic upstream
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
and development orchestration. The custom xtask package has been removed.
Avoid speculative crate proliferation, frameworks, async runtimes and custom
parsers. Choose a real parser/serialization crate when production needs it.

Use explicit process arguments, preserve nonzero exits, separate stdout data
from stderr diagnostics, and return typed recoverable errors when behavior is
implemented. Avoid panics on user input. Start with safe Rust; do not weaken the
workspace's unsafe prohibition as a shortcut. Rust does not prove authorization,
TOCTOU safety, merge correctness, crash recovery or signature enforcement.

## Tooling and checks

rust-toolchain.toml pins the bootstrap toolchain; Cargo.lock pins crate resolution.
The initial code uses only std. Changing pins requires review and CI, not a
moving `stable` update during agent startup. Use standard Cargo commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

CI runs these directly. There is no xtask, custom layout checker or skill-copy
validator. Explicit source paths and safe Rust remain project requirements.
These checks are Rust evidence, not OS or real-agent qualification.
Verify rust-analyzer separately. Fedora binaries need a compatible target ABI.

If Cargo reports Rust 1.97.0 despite the repository pin, inspect
`rustup show active-toolchain`: RUSTUP_TOOLCHAIN can override rust-toolchain.toml.
Use `cargo +1.98.1 <command>` with the installed pin. In PowerShell, removing the
process override lets the repository pin apply unless a directory override wins.
Do not lower rust-version or automatically install missing tools.
Source: https://rust-lang.github.io/rustup/overrides.html; local diagnosis 2026-09-07.

## Skill lifecycle

ADR 0003 packages twelve Kedra skills and eighteen selected upstream Rust skills
as one ordinary-file tree in plugins/kedra/skills, with Codex and Claude manifests.
Edit that tree directly. No submodules, symlinks, duplicate copies or sync step.
The plugin's third-party/rust-skills/NOTICE.md records selection/revision,
upstream MIT declarations and the missing LICENSE-text gap.
Review provenance and bump the plugin version when updating the snapshot.
Optional references to omitted skills/tools do not authorize their installation.
Actual plugin discovery and profile isolation remain separate R11 experiments.
Historical bootstrap results are preserved in references/bootstrap-evidence.md;
they describe the former four-package layout, not the present three-package one.

Evidence gate: R11 for layout/tools/discovery; R10 for privileged protocol.
Sources: docs/SOURCES.md entries cargo-targets, cargo-workspaces, rust-release,
rust-skills, codex-skills, claude-skills; ADR 0001.

## Local findings, 2026-09-07

The R03 prototype is an explicit `r03_home` example target in sysroot-core with
`test = true`. Standard workspace tests execute its 12 synthetic Git tests; no
new crate, dependency, custom check runner or production library API is needed.
Git 2.55.0.windows.1 rejected a Rust canonical Windows verbatim path passed as
GIT_CONFIG_GLOBAL (exit 128); the fixture converts generated Git config/index
arguments to ordinary slash paths. Keep filesystem and subprocess path handling
distinct; this is not proof of path-race safety. See
[R03 evidence](../../../../docs/research/R03-home-review/REPORT.md).

Codex 0.153.4 root/crate marketplace queries returned no Kedra entries; fresh-profile
app-server skills/list also returned zero repository plugin skills. Canonical
manual reads work. Claude 2.1.263 authoring validation passes; loaded-plugin model
sessions and editor behavior are not-run. Do not treat valid manifests, a restart
suggestion or manual reads as automatic discovery success; do not install globally
to unblock unrelated Rust research. See
[the discovery report](../../../../docs/research/R11-rust-workspace/discovery-20260907.md).
