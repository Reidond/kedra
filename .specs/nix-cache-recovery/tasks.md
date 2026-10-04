# D5 tasks

Status (2026-10-05): implementation is published in draft PR37. Standard source
gates and the ordinary cache/lease workflows below have actual results; native
fault qualification remains incomplete. Exact source, receipts and preserved
failed attempts are in [verification.md](verification.md). Parent owns Git,
shared status/worklog and serialized heavy qualification.

| Task | Scope / files | Verification ownership | State |
|---|---|---|---|
| D5-1 | Closed signed receipt/policy API in engine `cache.rs`; pinned P-256/base64, private verified capability and pre-admission constraint method. | TC-C1, TC-C2 | Implemented; signed transfer, content/scope/time refusals and validity boundaries pass. |
| D5-2 | Engine `snapshot.rs`: registry/lease/locks, exact allowlisted cleanup, explicit recovery result. | TC-C4, TC-C5 | Implemented; stopped-reader recovery, hostile-lease refusals and genuine cleanup-failure/orphan recovery pass. Killed-process cleanup boundary is unclaimed. |
| D5-3 | Engine dependencies/modules and actual D3/D4 recipe/scope selection; no core/release dependency. | TC-C1, TC-C2, TC-C3; existing release interoperability/material E2E | Implemented; independent signed-cache consumer, forced rebuild and release CLI checks pass. |
| D5-4 | `Store::substitute`, private staged admission constraint, no-replace journal publication/recovery and thin CLI with independent scope. | TC-C1, TC-C2, TC-C3, TC-C7 | Implemented; admission, divergent-winner refusal and source-import journal/root ENOSPC recovery pass. Native metadata cases remain. |
| D5-5 | Guard in `context.rs`, native recipe creation and explicit recovery; mode0600 writes, native implementation identity and explicit finish through consumers. | TC-C4, TC-C5 | Implemented; genuine lease schema/member/mode/owner/inode/device/link refusals and restored recovery pass in their recorded scopes. |
| D5-6 | Public CLI E2E and standalone consumers; generated keys, executable/library output and refusal preservation. | TC-C1 through TC-C5, bounded-copy subset of TC-C7 | Listed ordinary workflows pass; no ignored case is counted as execution. |
| D5-7 | Four real native publication windows and public retries with pinned executables. | TC-C6, TC-C8 | Blocked by automatic execution review; one pre-ID window captured, zero fully qualified cases. Pending state preserved. |
| D5-8 | Bounded isolated filesystem ENOSPC copy/publication/import cases. | TC-C7, TC-C8 | Source-copy, snapshot-copy, import-journal and root-publication cases pass. Native metadata cases remain unqualified; no daemon-data exhaustion claim. |
| D5-9 | Standard compiler/linter/E2E/release gates, evidence review, operational docs and stacked draft. | All cases, exact source/PR/CI readback | Source gates and draft publication pass; full runtime acceptance remains incomplete. |

For further D5-3 changes, re-read actual D3/D4 APIs and announce API changes. Avoid changes
to an earlier owning layer through D5 merely to move a fix up the stack. No cache
or native faults share resources with another heavy test sequence.

The original preparation is now integrated. Compiler success does not establish
fault safety. Record failures/blocked prerequisites explicitly rather than
installing tools, pulling broad resources or weakening verification.
