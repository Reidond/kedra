---
name: kedra-home
description: Maintain writable home review, selected publication, local-only policy, native activation and recovery.
---

# Writable home

Read docs/HOME-REVIEW.md, docs/TEXT-REVIEW.md, docs/ARCHITECTURE.md and references/state-model.md. Native Noctalia safe fields and explicitly adopted niri text paths are supported; wider arbitrary groups are not. Check docs/STATUS.md for actual qualification rather than assuming all failure cases pass.

Keep accepted baseline B, live L, next baseline N, selected S, local policy I, published receipts P and recovery J independent. Later app writes never mutate selected content. Publication does not accept a new installed baseline. Preserve private source ancestry and host/shared path provenance.

Capture only adopted safe paths/projections. Full live text must not become Git objects: Git 2.55 supports diff --no-index against stdin with exits 0/1. Only approved selected content crosses into source. Do not reset the user's source index or expose private review ancestry, credentials, transcripts, caches or vault data.

Compute candidates away from live files. Never write conflict markers. Validate and coordinate native writers, recheck reviewed content/metadata, journal changes, then reload/start and verify before clearing pending recovery. Failed or partial apply never advances baseline. Unknown/corrupt records refuse, not initialize over existing state.

Niri 26.04 relative includes use the main file's parent; preserve that validation context. IPC LoadConfigFile may change runtime path, so argv/environment are not active-file proof. Subscribe before reload, consume the initial prior ConfigLoaded state, request reload and require a new event. Acknowledgement alone only queues asynchronous work. Failure retains recovery. Native Actions 34237287511 at 5e238c7 covered relative includes and real killed-CLI recovery; signed A/B/A coverage is recorded in STATUS.

Noctalia 5.0.1 writes GUI overrides separately from curated TOML. Project only the supported safe fields, retain unknown/private data only in the live native file, and coordinate service stop/readback/restart. Keep-current must validate the writer before clearing recovery. Actual process-kill coverage does not establish power-loss/full-disk guarantees.

Use real CLI/Git and desktop/home-transition E2E. Include selected/local/later edits, deletes, ambiguous upstream context, symlink/hardlink refusal, metadata, source drift and rollback. No immutable-home symlinks, blanket rsync, source scanners or synthetic model tests.
