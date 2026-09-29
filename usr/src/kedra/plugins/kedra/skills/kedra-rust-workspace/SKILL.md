---
name: kedra-rust-workspace
description: Preserve Kedra's flat Rust workspace, pinned tooling, privilege boundaries and repository-local skills.
---

# Rust workspace

Read AGENTS.md, usr/src/kedra/docs/ARCHITECTURE.md, upstream rust-router, domain-cli and the relevant topic skill. Owner choices override upstream scaffolding and test recommendations.

Use edition 2024, resolver 3, one Cargo.lock and explicit binary/library paths. Crates live under usr/src/kedra/crates/, with no first-party src/ directory inside a crate. sysroot is the user CLI, sysroot-core shared logic and the helper protocol, sysroot-linux the Linux primitives (storage, trusted reads, file replacement, firmware) shared by the CLI and helper, and sysroot-helper the independent privileged binary. The one other member is usr/src/kedra/tests/container (`kedra-container-tests`): the owner-approved container test harness and `kedra-lab` development tool (2026-09-27). It is flat (lib.rs, main.rs with `harness = false`, lab.rs) and never enters the image. It depends on testcontainers 0.28, whose re-exported bollard it also uses, plus tokio, libtest-mimic, serde-saphyr, futures-util, bytes and tar. No xtask, other runner or speculative framework/crate expansion.

Use typed errors, explicit process arguments and separate stdout data/stderr diagnostics. Preserve subprocess failure. Avoid input panics and unsafe shortcuts; safe Rust alone does not prove privilege, race, signature or recovery correctness.

The container crate also uses the workspace base64 dependency for bounded desktop
payloads and signal-hook 0.4.4 for cooperative interruption. `prepare-sync` caches
the host-native source archiver; config-only sync must not call the Linux binary
builder. Rust/Cargo/toolchain input changes invalidate that cache explicitly.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
```

Only E2E/manual tests: no unit/model/mock/doctests or repository source scanners. Build Fedora-compatible binaries for image ABI compatibility. Compiler success is separate from installed/desktop qualification.

Use the pinned toolchain. RUSTUP_TOOLCHAIN can override rust-toolchain.toml; inspect rustup show active-toolchain and use the explicit installed pin rather than lowering rust-version. Do not install tools automatically. Source: https://rust-lang.github.io/rustup/overrides.html (local finding 2026-09-07).

The workspace disables debuginfo in the Cargo `dev` profile; Cargo's `test`
profile inherits it. This keeps local builds and their artifacts smaller while
preserving debug assertions, overflow checks and incremental compilation. Set
`CARGO_PROFILE_DEV_DEBUG=2` for a build that needs full debugger information.
On the owner's M2 Pro with Rust 1.98.1 (2026-09-29), a clean `kedra-lab` build
improved from 42.98 s to 39.85 s, a touched-source rebuild from 1.39 s to 1.15 s,
and the resulting debug tree from 1,411,224 KiB to 801,284 KiB. Source:
https://kobzol.github.io/rust/rustc/2025/05/20/disable-debuginfo-to-improve-rust-compile-times.html

Keep canonical ordinary skill files and Codex/Claude manifests in usr/src/kedra/plugins/kedra, with the pinned upstream Rust skills in usr/src/kedra/plugins/rust-skills. Preserve upstream NOTICE/provenance and review snapshot/version updates. No symlinks, submodules, generated copies or OS skill provisioning; the plugins are registered for this repository only (AGENTS.md), and skill changes bump the plugin version. Manual file access does not prove automatic plugin discovery.

Windows Git 2.55 rejected canonical verbatim paths in GIT_CONFIG_GLOBAL; use appropriate ordinary subprocess paths for generated fixtures, separately from filesystem path validation. Native CLI/VM E2E remains the acceptance boundary.
