---
name: kedra-rust-workspace
description: Preserve Kedra's flat Rust workspace, pinned tooling, privilege boundaries and repository-local skills.
---

# Rust workspace

Read AGENTS.md, usr/src/kedra/docs/ARCHITECTURE.md, upstream rust-router, domain-cli and the relevant topic skill. Owner choices override upstream scaffolding and test recommendations.

Use edition 2024, resolver 3, one Cargo.lock and explicit binary/library paths. Crates live under usr/src/kedra/crates/, with no first-party src/ directory inside a crate. sysroot is the user CLI, sysroot-core shared logic and the helper protocol, sysroot-linux the Linux primitives (storage, trusted reads, file replacement, firmware) shared by the CLI and helper, and sysroot-helper the independent privileged binary. The one other member is usr/src/kedra/tests/container (`kedra-container-tests`): the owner-approved container test harness and `kedra-lab` development tool (2026-09-27). It is flat (lib.rs, main.rs with `harness = false`, lab.rs) and never enters the image. It depends on testcontainers 0.28, whose re-exported bollard it also uses, plus tokio, libtest-mimic, serde-saphyr, futures-util, bytes and tar. No xtask, other runner or speculative framework/crate expansion.

Use typed errors, explicit process arguments and separate stdout data/stderr diagnostics. Preserve subprocess failure. Avoid input panics and unsafe shortcuts; safe Rust alone does not prove privilege, race, signature or recovery correctness.

`sysroot-engine` is the independent flat-source package graph/store/profile crate
(2026-10-01). Its model/planning API is separate from Unix storage/execution;
the first backend is private-owner native aarch64 Linux through a frozen local
Unix Docker endpoint. CLI adapters live in `sysroot/engine.rs`. This is separate
from installed helper authority, OS composition, home state and signed releases.
Read `usr/src/kedra/docs/ENGINE.md` and the crate README for operations/limits.

The typed `SystemDefinition`/`SystemFile` API and `sysroot system plan/compose`
export static config/runtime contexts over an exact retained Fedora44 ARM
foundation (2026-10-01). Keep source materialization tied to one resolved plan;
never re-resolve HEAD while archiving. Match actual seven-column RPM material and
one runtime foundation. Files outside config overlays require exact foundation
bytes/mode and are explicit passthrough, never replacements. Export uses
no-replace publication and refuses unsafe foundation aliases. Read
`usr/src/kedra/docs/SYSTEM.md` and `.specs/nix-system-composition/verification.md`.
Archive inspection/package execution does not qualify native generation, boot or
installed activation. The separate closed `NativeDefinition`/`NativeStep` stage
is implemented through the sanctioned harness (2026-10-01); see NATIVE.md and
`.specs/nix-native-artifacts/verification.md`. It binds the fixed driver/recipe,
parent identity and RPM material, verifies actual output bytes and exact parent
RootFS layer prefix, and uses independent cache bindings. Never trust a mutable
FROM tag or labels alone. Tools run as root only inside that isolated offline
image build; the ordinary package executor stays nonroot. Generated outputs and
receipts must be single-link; inherited immutable foundation hardlinks are valid.
Static schema1 writes are unchanged: inherited dracut config is consumed, while
changed dracut input needs a separate versioned boundary. Do not broaden `/usr/lib`
overlays. Refuse unsupported managed skeleton deletions; never apply to live home.

Systemd Alias-only enable/disable is qualified (2026-10-02): greetd's sole
display-manager.service link is legitimate evidence only when a valid top-level
same-type alias targets the exact selected concrete unit. Nested dependency
links retain matching-name restrictions; removals require selected disable
targets, and default.target is not enable evidence. Collector and core admission
must agree. Actual enable/disable and graphical boot pass; source:
`.specs/nix-derived-boot/verification.md`, native.rs/native_derivation.rs.

Docker inspect/native execution is not complete OCI retention evidence. Observed
Docker29.4 cached Kedra candidate91e27148 exports exit0 with only a25,088-byte
manifest archive: config/all78 layers are missing. Exact-digest re-pull reports
up to date but does not repair export; the engine correctly refuses admission.
Source: composition verification/local archive diagnostic, 2026-10-01. Preserve
that refusal and obtain complete archive evidence before claiming full composition.

Replay continuation (2026-10-01) retains signed9d6eb030 under the installed strict
source policy and verifies all78 layers/config/root; omitting unsupported archive
signature sidecars does not waive source verification. `VerifiedComposition` owns
a private verified snapshot and validates payload semantics, not just outer hashes.
Use release product binaries for multi-GiB qualification; dev hashing caused a
deliberately interrupted attempt. Producer/consumer composition manifests share
a bounded64MiB limit; realistic1MiB config and GNU-name payload round trips pass.
Known Kedra assembly makes `usr/libexec/kedra-session`0755 despite Git100644;
the adapter asserts that exact foundation mode without rewriting it or source.json.
Source: `.specs/nix-context-replay/verification.md`, `image/assemble.sh`.

Docker29.4.0 image IDs in the observed retained fixtures name OCI root indexes,
not config hashes. The engine validates the complete selected ARM manifest/config/
layer descriptor graph and Docker compatibility manifest before trusting retained
archive evidence. Hashing an archive only against its own receipt is insufficient
when an unrelated claimed image is already cached. Classic-only archives refuse;
Docker owns decompression/DiffID verification. Source: first-engine E2E forged
archive refusal and `.specs/nix-engine/verification.md`.

On this macOS/ARM filesystem, renaming a sealed0555 directory across parents
returns EACCES. Engine publication seals contents, temporarily leaves only the
staging container0700 for rename under its exclusive lock, then seals the final
container and fsyncs. GC retires live objects into journaled tombstones before
recursive removal; first profiles publish complete staged directories. Real
interruption, foreign-sentinel and source-admission workflows are required evidence,
not inferred from the presence of journals. Run explicit Docker cases with
`cargo test -p sysroot --test e2e_engine --locked -- --include-ignored --test-threads 1`;
default ignored cases confer no runtime coverage.

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

Catalog configure probes (2026-10-03): the native engine's `/build` tmpfs is
observed noexec with the existing Docker flags. Real jq configure and a compiled
ELF both exit126 there on compiler image07c0e384. Recipes needing executable
probes must use named scratch inside their own output binding, map that scratch
prefix for reproducibility and remove it before successful admission; do not
weaken generic mounts or merely run configure through sh. Jq's recipe does this;
its full native rerun remains required. Source/evidence:
`.specs/nix-package-catalog/verification.md` and d3-catalog-e2e mount probe.

Canonical source identity excludes timestamps (2026-10-03 jq/Oniguruma runtime
proof). Admission/copy changed generated-file ordering and triggered unavailable
aclocal-1.16 despite unchanged source bytes; top-level maintainer mode did not
guard the vendor rule. Normalize only the recipe's disposable copied tree to its
declared SOURCE_DATE_EPOCH before configure, including link inodes without target
traversal (`find -P`, `touch -h`). Do not fake old tool names or alter readonly
inputs. Source: catalog verification and d3-catalog-e2e-fixed timestamp receipt.
