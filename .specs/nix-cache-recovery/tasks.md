# D5 tasks

Status: preparation only; no item below is a runtime pass. Parent owns integration,
Git, shared status/worklog and serialized heavy qualification.

| Task | Scope / files | Verification ownership | State |
|---|---|---|---|
| D5-1 | Closed signed receipt/policy API in new engine `cache.rs`; direct pinned P-256/base64, private verified capability and pre-admission constraint method. | TC-C1, TC-C2 | Prepared, unwired; compile not-run. |
| D5-2 | New engine `snapshot.rs`: registry/lease/locks, exact allowlisted cleanup, explicit recovery result. | TC-C4, TC-C5 | Prepared, unwired; compile not-run. |
| D5-3 | After D4 freeze, parent adopts engine direct dependencies/modules and actual D3/D4 recipe/scope selection; no core/release dependency. | TC-C1, TC-C2, TC-C3; existing release interoperability/material E2E | Shadow dependency/module wiring prepared; adoption not started. |
| D5-4 | Parent integrates `Store::substitute`, private staged admission constraint, complete no-replace journal publication/recovery and thin CLI with independent scope. | TC-C1, TC-C2, TC-C3, TC-C7 | Shadow source prepared; adoption not started. |
| D5-5 | Parent integrates guard into `context.rs`, native recipe creation and explicit recovery; mode 0600 writes, advanced native implementation identity and explicit finish through all consumers. | TC-C4, TC-C5 | Shadow source prepared; adoption not started. |
| D5-6 | Extend existing public CLI E2E and standalone consumer fixtures; generated keys, actual executable/library output and refusal preservation. | TC-C1 through TC-C5, bounded-copy subset of TC-C7 | Shadow signed-transfer, stopped-reader/crash recovery and real bounded tmpfs import E2E source prepared; not-run. |
| D5-7 | Parent freezes source, pins private final executables, captures four real native fault windows and public retries. | TC-C6, TC-C8 | Not-run. |
| D5-8 | Parent provisions bounded isolated filesystem and executes real ENOSPC copy/publication/import cases; separate daemon-data case if claimed. | TC-C7, TC-C8 | Not-run. |
| D5-9 | Parent runs required standard compiler/linter/E2E/release gates; reviews evidence/cleanup, updates docs/skills/manifests/status/worklog, submits own stacked draft. | All cases, exact source/PR/CI readback | Not started. |

Before D5-3, re-read actual D3/D4 APIs and announce any header change. Avoid changes
to an earlier owning layer through D5 merely to move a fix up the stack. No cache
or native faults share resources with another heavy test sequence.

Unwired files are intentionally outside Cargo/module discovery until D5. Neither
the parent nor worker may claim their presence validates parsing, linking, runtime
behavior or fault safety. Record failures/blocked prerequisites explicitly rather
than installing tools, pulling broad resources or weakening verification.
