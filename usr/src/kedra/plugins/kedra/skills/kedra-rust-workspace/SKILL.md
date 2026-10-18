---
name: kedra-rust-workspace
description: Preserve Kedra's flat Rust workspace, pinned tooling, engine and privilege boundaries and repository-local skills.
---

# Rust workspace

Read AGENTS.md, usr/src/kedra/docs/ARCHITECTURE.md, upstream rust-router, domain-cli and the relevant topic skill. Owner choices override upstream scaffolding and test recommendations.

Use edition 2024, resolver 3, one Cargo.lock and explicit binary/library paths. Crates live under usr/src/kedra/crates/, with no first-party src/ directory inside a crate:
- `sysroot`: the user CLI, with engine adapters in `sysroot/engine.rs`.
- `sysroot-core`: shared logic, the helper protocol (schema 3) and the embedded `image/release/targets.json` and `compatibility.json`.
- `sysroot-linux`: Linux primitives (storage, trusted reads, file replacement, firmware) shared by the CLI and helper.
- `sysroot-helper`: the independent privileged binary.
- `sysroot-engine`: the package graph/store/sandbox/profile/composition library.
- `sysroot-catalog`: the `.kedra` language frontend and the RPM lock document (`rpm_lock`).
The one other member is usr/src/kedra/tests/container (`kedra-container-tests`): the owner-approved container test harness and `kedra-lab` development tool (2026-09-27). It is flat (lib.rs, main.rs with `harness = false`, lab.rs) and never enters the image. It depends on sysroot-engine, testcontainers 0.28 (whose re-exported bollard it also uses), tokio, libtest-mimic, serde-saphyr, futures-util, bytes, tar, base64 (bounded desktop payloads) and signal-hook 0.4.4 (cooperative interruption). No xtask, other runner or speculative framework/crate expansion.

Use typed errors, explicit process arguments and separate stdout data/stderr diagnostics. Preserve subprocess failure. Avoid input panics and unsafe shortcuts; safe Rust alone does not prove privilege, race, signature or recovery correctness.

## Engine

Read usr/src/kedra/docs/ENGINE.md and the crate README. The model/planning API is separate from Unix storage/execution. Builds, `run`, `profile run`, `develop` and foundation observation execute without a daemon in an unprivileged bubblewrap sandbox on native Linux (aarch64 or x86_64, the architecture `sysroot` was compiled for), inside input-addressed store roots (`rootfs/root-<sha256>`). Graph nodes name a `builder_root` and a `runtime_root`; in a release the runtime root is the RPM lock's Fedora base plus its foundation role and each builder root that plus one builder role, so `out-` identities follow the base digest, the locked RPMs and the package's inputs, not the per-commit foundation image (a `--no-cache` build whose ID changes every run). A composition accepts an output only if its runtime root's RPM inventory equals the foundation's exactly. The local Docker engine remains only for daemon image import (`store add-image --image`), RPM resolution and native derivation (BuildKit); `store add-image --archive` needs no daemon. This is separate from installed helper authority, home state and signed releases.

Unprivileged roots (2026-10-15, Fedora 44 rpm 6.0.2, bubblewrap 0.12, OrbStack kernel 7.0; crates/sysroot-engine/rootfs.rs): rpm running as uid 0 in a single-uid user namespace fails `chown` on packaged files owned by other users (`/var/spool/mail` root:mail), while rpm as a non-root uid skips chown, so a role installs as the store owner with `rpm -U` inside the sandbox, scriptlets, sysusers and alternatives included. `_pkgverify_level signature` reports "no signature" for Fedora's header-only V4 signatures; `rpmkeys --checksig -v` verifies them. The RPM lock holds only the RPMs installed over the base image, so roots start from an image archive. bubblewrap tmpfs mounts are nosuid/nodev but executable. Layers can hard-link symlinks (linux-firmware). Ubuntu 24.04 ships bubblewrap 0.9 (no `--size`) and confines unprivileged user namespaces through AppArmor (`kernel.apparmor_restrict_unprivileged_userns`). In Docker, bwrap needs `seccomp=unconfined` and `systempaths=unconfined` (for `--proc`), and an init that reaps orphans (tests/container/localbuild.rs).

Docker image identity: an image ID is the configuration digest on the classic overlay2 store but the registry index digest on the containerd store; `docker image save` writes that index as the archive root on containerd and a manifest Docker generates on classic (Docker 29.9.1 classic and OrbStack 29.4.0 containerd, 2026-10-18; tests/container/native.rs `archive_identity`). `store add-image --archive` admits both; the daemon import binds Docker's ID to the archive root and so needs the containerd store, which every release job selects (image/release/runner-containerd.sh). Never treat a registry digest as an image ID. The engine validates the complete selected manifest/config/layer descriptor graph before trusting an archive, and unpacking checks every layer's uncompressed DiffID itself (host gzip/zstd); hashing an archive only against its own receipt is insufficient when an unrelated claimed image is cached. Docker 29.4 once exported a cached image with exit 0 as a 25,088-byte archive missing its config and all 78 layers, and an exact-digest re-pull did not repair it (2026-10-01); the engine refuses such archives, and that refusal must stay.

On the owner's macOS/ARM filesystem, renaming a sealed 0555 directory across parents returns EACCES. Engine publication seals contents, temporarily leaves only the staging container 0700 for rename under its exclusive lock, then seals the final container and fsyncs. GC retires live objects into journaled tombstones before recursive removal; profiles publish complete staged directories. Engine store objects stay sealed read-only: `chmod -R u+w` before deleting a store under target/. Real interruption, foreign-sentinel and source-admission workflows are required evidence, not inferred from the presence of journals. Run the sandbox cases on Linux with bubblewrap user namespaces:
`cargo test -p sysroot --test e2e_engine --locked -- --include-ignored --test-threads 1`;
default ignored cases confer no runtime coverage.

Binary cache (2026-10-18; docs/ENGINE.md "Binary cache"): `sysroot-engine/cache.rs` admits a registry closure only if every staged object equals a receipt of a signature-verified image's installed composition.json; the registry, its tags and manifests are never trusted, and every failure falls back to building. Tags are `closure-` plus the hash of the closure's receipts. Transport is skopeo with a cleared environment, private home and its own policy (a Kedra machine's strict policy would otherwise reject the cache). Composition/native/catalog temporary inputs use lock-backed `ManagedSnapshot` leases and explicit `finish`; keep the guard alive through every child reader, and `sysroot system recover` collects only recognized abandoned entries. Compiler success does not qualify a real cache hit, stopped-reader/SIGKILL recovery or Linux ENOSPC.

## Composition and native steps

The typed `SystemDefinition`/`SystemFile` API and `sysroot system plan/compose` export static config/runtime contexts over an exact Fedora 44 foundation (usr/src/kedra/docs/SYSTEM.md). Keep source materialization tied to one resolved plan; never re-resolve HEAD while archiving. Match actual seven-column RPM material. Files outside config overlays require exact foundation bytes/mode and are explicit passthrough, never replacements. Export uses no-replace publication and refuses unsafe foundation aliases. Producer/consumer composition manifests share a bounded 64 MiB limit. assemble.sh makes `usr/libexec/kedra-session` 0755 although Git records 100644; `sysroot system` asserts that exact foundation mode without rewriting it or source.json (crates/sysroot/system.rs). Use release binaries for multi-GiB composition work; debug-build hashing is far slower.

The closed `NativeDefinition`/`NativeStep` stage is product code (`sysroot derive plan|build`, docs/NATIVE.md, `native_apply.rs`): it binds the executor binary (`driver_sha256`), fixed recipe, parent identity and RPM material, verifies actual output bytes and the exact parent RootFS layer prefix, and uses independent bindings. Never trust a mutable FROM tag or labels alone. Each enabled unit needs a recorded link to it other than default.target (an Alias-only install such as greetd's `display-manager.service` link counts), each mask a `/dev/null` link and default.target the declared target (`validate_systemd_links` in native.rs). Do not broaden `/usr/lib` overlays; new dracut configuration needs a reviewed payload file. Refuse unsupported managed skeleton deletions; never apply to a live home.

## Package frontend

Owner decision (2026-10-06): sysroot-catalog parses versioned .kedra data into common typed Intent; direct Rust consumers use the same lowering (docs/PACKAGES.md "Language v1"). Module loading/check/format/plan executes no author shell or Cargo. Inline scripts are ordinary resource file inputs to /bin/sh -eu; patches and overlays create separate owned preparation outputs and never mutate admitted source. Every command export gets a typed launcher under /usr/share/kedra/catalog-bin (libexec exports under catalog-libexec, off PATH; output directories never go on PATH), preserving global RPM paths; an export the foundation already provides needs `replaces` (docs/PACKAGES.md "Exports and PATH"). Atomic plan packets use the closed Catalog managed-snapshot purpose and `system recover`.

Public CLI planning and build input loading run in an owned child with a 60-second deadline, a 512 MiB Linux address-space ceiling and macOS resident-memory observation every 100 ms (crates/sysroot/catalog_process.rs). Refusal discards stdout and kills/reaps the child; owner death stops it. Direct typed Rust consumers own supervision. Formatter and plan emission stage only closed members in leased snapshots.

Build reproducibility facts from the jq fixture (crates/sysroot/tests/fixtures/catalog/jq.kedra, 2026-10-03): canonical source identity excludes timestamps, and admission/copy changed generated-file ordering so an unchanged tree triggered an unavailable aclocal-1.16. Normalize only the recipe's disposable copied tree to its declared SOURCE_DATE_EPOCH before configure, including link inodes without target traversal (`find -P`, `touch -h`). Do not fake old tool names or alter read-only inputs.

## Checks and tools

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
cccc --table --min 41 .
```

`cccc` 1.7.0 (pinned in mise.toml; a local check before sign-off) gates every Rust and Python function, method and closure at cognitive and cyclomatic complexity 40 through the root `cccc.toml`. Closures are measured as their own units, so a parent's score excludes them; split long functions into cohesive named helpers instead of wrapping logic in closures or macros. The initial adoption (2026-10-07) found 31 functions over the limit, led by the `.kedra` checker's `typed` (cognitive 118) and `validate` (cyclomatic 68); all were refactored with unchanged behavior. `--min 41` still lists every file, offenders under theirs. Before refactoring a file, compare HEAD and new binaries on the same inputs. Run E2E as an ordinary user: owner-private stores and home state refuse root by design.

Only E2E/manual tests: no unit/model/mock/doctests or repository source scanners. Build Fedora-compatible binaries for image ABI compatibility. Compiler success is separate from installed/desktop qualification. Cargo output and every other check artifact stay under the checkout's target/ (AGENTS.md).

Use the pinned toolchain. RUSTUP_TOOLCHAIN can override rust-toolchain.toml; inspect rustup show active-toolchain and use the explicit installed pin rather than lowering rust-version. Do not install tools automatically. Source: https://rust-lang.github.io/rustup/overrides.html (local finding 2026-09-07).

The workspace disables debuginfo in the Cargo `dev` profile; Cargo's `test` profile inherits it. This keeps local builds and their artifacts smaller while preserving debug assertions, overflow checks and incremental compilation. Set `CARGO_PROFILE_DEV_DEBUG=2` for a build that needs full debugger information. On the owner's M2 Pro with Rust 1.98.1 (2026-09-29), a clean `kedra-lab` build improved from 42.98 s to 39.85 s, a touched-source rebuild from 1.39 s to 1.15 s, and the resulting debug tree from 1,411,224 KiB to 801,284 KiB. Source: https://kobzol.github.io/rust/rustc/2025/05/20/disable-debuginfo-to-improve-rust-compile-times.html

`kedra-lab prepare-sync` caches the host-native source archiver; config-only sync must not call the Linux binary builder. Rust/Cargo/toolchain input changes invalidate that cache explicitly.

Keep canonical ordinary skill files and Codex/Claude manifests in usr/src/kedra/plugins/kedra, with the pinned upstream Rust skills in usr/src/kedra/plugins/rust-skills. Preserve upstream NOTICE/provenance and review snapshot/version updates. No symlinks, submodules, generated copies or OS skill provisioning; the plugins are registered for this repository only (plugin README), and skill changes bump the plugin version. Manual file access does not prove automatic plugin discovery.

Windows Git 2.55 rejected canonical verbatim paths in GIT_CONFIG_GLOBAL; use appropriate ordinary subprocess paths for generated fixtures, separately from filesystem path validation. Native CLI/VM E2E remains the acceptance boundary.
