---
name: kedra-rust-workspace
description: Preserve Kedra's flat Rust workspace, pinned tooling, privilege boundaries and repository-local skills.
---

# Rust workspace

Read AGENTS.md, docs/ARCHITECTURE.md, upstream rust-router, domain-cli and the relevant topic skill. Owner choices override upstream scaffolding and test recommendations.

Use edition 2024, resolver 3, one Cargo.lock and explicit binary/library paths. No first-party src/ directory. sysroot is the user CLI, sysroot-core shared logic and sysroot-helper the independent privileged protocol. No xtask, custom runner or speculative framework/crate expansion.

Use typed errors, explicit process arguments and separate stdout data/stderr diagnostics. Preserve subprocess failure. Avoid input panics and unsafe shortcuts; safe Rust alone does not prove privilege, race, signature or recovery correctness.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
```

Only E2E/manual tests: no unit/model/mock/doctests or repository source scanners. Build Fedora-compatible binaries for image ABI compatibility. Compiler success is separate from installed/desktop qualification.

Use the pinned toolchain. RUSTUP_TOOLCHAIN can override rust-toolchain.toml; inspect rustup show active-toolchain and use the explicit installed pin rather than lowering rust-version. Do not install tools automatically. Source: https://rust-lang.github.io/rustup/overrides.html (local finding 2026-09-07).

Keep canonical ordinary skill files and Codex/Claude manifests in plugins/kedra. Preserve upstream NOTICE/provenance and review snapshot/version updates. No symlinks, submodules, generated copies, global registration or OS skill provisioning. Manual file access does not prove automatic plugin discovery.

Windows Git 2.55 rejected canonical verbatim paths in GIT_CONFIG_GLOBAL; use appropriate ordinary subprocess paths for generated fixtures, separately from filesystem path validation. Native CLI/VM E2E remains the acceptance boundary.
