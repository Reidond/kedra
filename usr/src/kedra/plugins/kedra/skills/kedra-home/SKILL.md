---
name: kedra-home
description: Maintain writable home review, selected publication, local-only policy, native activation and recovery.
---

# Writable home

Read usr/src/kedra/docs/HOME-REVIEW.md, usr/src/kedra/docs/TEXT-REVIEW.md, usr/src/kedra/docs/ARCHITECTURE.md and references/state-model.md. Native Noctalia safe fields and explicitly adopted niri text paths are supported; wider arbitrary groups are not. Check usr/src/kedra/docs/STATUS.md for actual qualification rather than assuming all failure cases pass.

Keep accepted baseline B, live L, next baseline N, selected S, local policy I, published receipts P and recovery J independent. Later app writes never mutate selected content. Publication does not accept a new installed baseline. Preserve private source ancestry and host/shared path provenance.

Capture only adopted safe paths/projections. Full live text must not become Git objects: Git 2.55 supports diff --no-index against stdin with exits 0/1. Only approved selected content crosses into source. Do not reset the user's source index or expose private review ancestry, credentials, transcripts, caches or vault data.

Compute candidates away from live files. Never write conflict markers. Validate and coordinate native writers, recheck reviewed content/metadata, journal changes, then reload/start and verify before clearing pending recovery. Failed or partial apply never advances baseline. Unknown/corrupt records refuse, not initialize over existing state.

Niri 26.04 relative includes use the main file's parent; preserve that validation context. IPC LoadConfigFile may change runtime path, so argv/environment are not active-file proof. Subscribe before reload, consume the initial prior ConfigLoaded state, request reload and require a new event. Acknowledgement alone only queues asynchronous work. Failure retains recovery. Native Actions 34237287511 at 5e238c7 covered relative includes and real killed-CLI recovery; signed A/B/A coverage is recorded in STATUS.

Noctalia 5.0.1 writes GUI overrides separately from curated TOML. Project only the supported safe fields, retain unknown/private data only in the live native file, and coordinate service stop/readback/restart. Keep-current must validate the writer before clearing recovery. Actual process-kill coverage does not establish power-loss/full-disk guarantees.

Use real CLI/Git and desktop/home-transition E2E. Include selected/local/later edits, deletes, ambiguous upstream context, symlink/hardlink refusal, metadata, source drift and rollback. No immutable-home symlinks, blanket rsync, source scanners or synthetic model tests.

Noctalia 5.2.1 qualification (2026-10-06): exact ARM candidate f3f61f5a carries
noctalia-5.2.1-1.fc44.aarch64 and prints exactly `noctalia v5.2.1`. Native config
validation and full export retain the three approved field types. The new exact
allowlist entry passes the full13/13 container suite1791294686-58725 with actual
Noctalia selection/discard/recovery, niri privacy/recovery and no cleanup failures.
Persisted APP_VERSION remains5.0.1; unknown outputs still refuse. This qualifies
the existing adapter, not Noctalia artifact production or a production release.
Source: home/linux.rs, core/noctalia.rs and worklog PR39 entry.

Niri artifact integration (2026-10-06): qemu-arm64 ordinary composition imports a
public single-file source object using the engine's canonical tree encoder and
verified admission/readback. Its root-owned0644 home-artifacts.json is at most
4096 bytes and is checked against the already trusted ordinary baseline without
opening a desktop build store. ENOENT alone selects legacy loading; malformed,
linked, unsupported or mismatched present records refuse. Keep schema1 home
records/journals and native activation domains unchanged. Actual composition/GC,
installed-loader refusals and retained old CLI readback pass locally; native home
reload/killed-CLI recovery passes on the composed fixture. Signed A/B/A remains a
separate exact-run gate. Source: home_artifact.rs, engine store/tree.rs,
tests/e2e_system.rs, tests/container/home_artifact_tests.rs and worklog PR39 entry.

Qualified 2026-09-30, Fedora 44 `noctalia-5.2.0-1.fc44`: native output is exactly
`noctalia v5.2.0`. Shipped TOML passes native validation; the full export retains
`theme.mode`, `shell.button_borders` and `shell.input_borders` with the same types.
The real home-review cycle passes (`1790750757-17043`), as does the full ARM
container suite (`1790751520-28479`, 13/13). A live export with 29 sections,
including weather/plugins, exposes only the three approved keys through public
home status. Add only this exact runtime to both native-output matching and the
projection allowlist. Keep persisted `APP_VERSION = 5.0.1`, so previously captured
records and image rollback remain readable. Unknown runtime outputs still refuse
before capture. See worklog WL-20260930-01 and docs/HOME-REVIEW.md; raw exports are
private fixture evidence and do not belong in Git.
