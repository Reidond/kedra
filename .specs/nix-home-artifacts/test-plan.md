# Verification plan — integrated niri baseline artifact

Status: **local cases qualified in the scopes below; signed gate pending**. Conditions are AC-01–AC-08 in
[spec.md](spec.md#acceptance-criteria); implementing owners are [H1–H7](tasks.md).
Only real public CLI/Git, sanctioned installed-system container and disposable
signed-VM end-to-end/manual workflows are allowed. Unit/model/mock/doctests,
source-string/layout scanners and new runners are excluded. Compiler, formatting
and lint gates supplement actual behavior; they cannot qualify installed use.

## Entry and derivation

Enter only after explicit current D1–D5 acceptance, implementation authorization,
source/CLI/image pins, complete source history and admitted owned runtime
resources. Use generated public baselines and synthetic private canaries, never
actual owner home or secrets. Record authentic old State/Journal producers and
old-image absence rather than hand-writing a purported compatible state.

| Condition | Requirements | Technique and cases |
|---|---|---|
| Source artifact reuse and existing image transport | AC-01, AC-02 | State transitions: plan → genuine source import → verified readback → context → image; TC-01/02 |
| Present/absent record authority | AC-03 | Decision table below; TC-03/06 |
| Selections, local choices, later edits and old state | AC-04, AC-05 | State transitions across old/new CLI, source publication and installed acceptance; TC-04/06 |
| Native validation/reload/recovery | AC-06 | Valid/stale approval and success/failure transitions at actually observed journal phases; TC-05/07 |
| Privacy, GC and publication failures | AC-07 | Real producer/consumer removal plus valid/fault filesystem partitions; TC-07/08/09 |
| Same existing user workflow | AC-08 | Old/new image × old/new CLI pairs, ordinary default command regressions; TC-04/06/08 |

Installed-loader decision table, after the existing source-manifest checks:

| Record | Baseline/source checks | Required result |
|---|---|---|
| Genuinely absent | Pass | Existing verified legacy baseline behavior; absence does not establish image age |
| Present and valid | Pass and receipt matches bytes/shape | Same Baseline plus successful engine receipt verification |
| Present but malformed/unsafe/mismatched | Pass | Specific record refusal; no fallback |
| Any | Fail | Existing source/installed authority refusal, never rescued by a record |

Keep the two authority-negative classes separate: an unsigned/wrong-authority
image must fail existing deployment admission; a trusted fixture image with
inconsistent baseline metadata must fail the home loader. Neither refusal stands
in for the other. No new signature authority is supplied by the artifact record.

## Cases

Each case owns exactly one level; assertions inspect runtime results and actual
produced files/objects, not repository source presence. Native/signed behavior is
assigned upward because public artifact checks cannot prove it. All cases are
automated E2E unless marked manual orchestration. Establish a working control
before each negative mutation and require its intended refusal cause.

| Case | Owning level / procedure | Expected result and non-vacuous guard | Task owner | Status |
|---|---|---|---|---|
| TC-01 | Public composition CLI E2E. Build two retained commits with identical niri bytes and a third with a real config change; use the actual adapter/build store, run plan then compose and verify stored source objects. Include shared/target source provenance, dirty checkout and a separate source plan with no niri baseline. | Plan does not claim admission; compose really imports/reads the one-file object, unchanged bytes reuse its identity and changed bytes change it. Stored data and emitted ordinary baseline match the selected committed blob; no live/dirty bytes enter. Require observed nonempty genuine object/receipt and zero references/images/derivation for positive cases; the no-baseline case emits no record and is not counted as artifact production (AC-01). | H1, H2, H3, H5 | Pass locally |
| TC-02 | Public composition/context CLI E2E. Export the produced context, make the original build store unavailable, verify/replay through the existing path and inspect the actual image baseline/record. Tamper each context member in independent negative fixtures. Compare catalog contribution outputs with the unchanged input receipt. | Baseline and record are hashed ordinary payload files; exact source/record bytes reach the image. Context tampering refuses, no source object is passed off as a runtime output and no extra installed store tree appears. Require successful context verification/replay plus actual final-image bytes, not only a predicted plan (AC-02). | H1, H3, H5 | Pass locally |
| TC-03 | Installed-system container E2E. Exercise each loader table row through normal home commands. Present-record partitions: unknown/duplicate/schema fields; wrong object/tree or receipt kind; extra reference/runtime/derivation; bad ownership/mode/link/hardlink; mismatched baseline/source. Test4096-byte record bound with legal JSON whitespace padding and4097-byte refusal; retain existing niri text-limit controls. | Each intended record/source refusal preserves logical review state/live data and reports its actual cause; absent means ENOENT, not broken link/unreadable file. Valid bounded controls and matching ordinary bytes load successfully (AC-03). No parser-only substitute; fixture records used for negative inputs are never called successful producer receipts. | H2, H4, H5 | Pass locally |
| TC-04 | Installed-system container E2E. Retained old CLI creates real adopted records/history/S/I/P; new CLI reads them and produces compatible records for old CLI readback. Select one line, keep another exact-local, edit the selected live value later, preview a disjoint source change, export only selection and record a real source commit. Include missing history/ambiguous overlap/source-layer rebind and orphan-state controls. | Old/new records, domains and plan semantics remain compatible without migration. Later edit stays unstaged, local-change semantics and source layer remain correct, index/worktree and accepted baseline remain independent of publication; conflicts refuse without markers (AC-04, AC-05). Require distinct nonempty selected/local/later values and genuine old executable/state provenance; retain legacy source-layout anchors. | H1, H4, H5, H6 | Partial; TC06 pending |
| TC-05 | Installed-system container E2E. Use the actual new-image record and ordinary `activate-plan/apply`; exercise native relative includes, stale live/plan and invalid config/reload controls. Capture the resulting visible setting with existing `kedra-lab` workflow. | Ordinary writable main file and unchanged includes; validation uses its real parent context, and B advances only after a new successful ConfigLoaded event. Stale/invalid cases refuse or leave the actual recovery state. Require a fresh event and visible application behavior, not just acknowledgement or screenshot existence (AC-06). | H4, H6 | Pass locally |
| TC-06 | Disposable signed-VM E2E / manual orchestration. Signed legacy A without record → new B with record → A rollback, preserving S/I/P and user data. Run retained old CLI on B and new CLI on A where supported; include unsigned/wrong-authority deployment and source-only future-baseline negatives. | Existing trust admission stays authoritative; normal home commands reconcile only actual installed source with explicit approval. All four old/new image/CLI combinations retain their specified baseline behavior, ordinary paths and state; rollback does not silently accept N or discard user data (AC-03, AC-05, AC-08). Require actual booted image/signature chain and exact CLIs. | H1, H4, H6 | Hosted gate pending |
| TC-07 | Installed-system container E2E / manual actual boundary capture. Remove the disposable producer/build-store availability before installed planning; capture a real niri apply interruption after durable J, then exercise resume/abort/keep-current, later edits and native failure controls using retained checkpoints. | Loading/activation/recovery never consults the removed build store. Existing checkpoint, installed equality and fresh-event rules govern completion; failures/later edits preserve B/J appropriately (AC-05, AC-06, AC-07). A missed interruption window is not a pass; no blocked native C6 state or debugger is involved. | H4, H6 | Pass locally |
| TC-08 | Installed-system container E2E. Use the exact public-source composition from TC-01/02 with unrelated synthetic private/live/include canaries; run ordinary niri/Noctalia/caller-home commands and selected export in the real installed fixture. Inspect produced build objects/image record and actual new Git objects; compare normal-user filesystem changes. | No canary/full-live/checkpoint data enters store/Git/public evidence; only explicit selected publication crosses into source. No new desktop cache, prepare/store options or deployment side effect appears; default Noctalia and source-preview behavior stays intact (AC-07, AC-08). Require each private canary actually exists and inspect produced outputs; keep raw private fixture output out of reports. | H3, H5, H6 | Pass locally |
| TC-09 | Public CLI E2E / manual bounded filesystem faults. During actual source-artifact production/context export, reach real ENOSPC or an observed interruption in an owned fixture. Recover through existing engine/context workflow, release only filler and retry. Separately use existing pin/unpin/GC on produced artifacts after final context export. | No successful partial baseline/record context is reported; exact failure/recovery and unchanged unrelated roots/sentinel are recorded. Pinned object survives normal GC, explicit unpin permits collection, and already exported context remains verifiable (AC-02, AC-07). Actual errno/boundary and own cleanup required; this does not claim all crash orders or power-loss durability. | H2, H3, H5 | Pass locally |

## Scenarios, evidence and completion

Group TC-01→02 as the common build/transport fixture; reuse its exact final image
for TC-03/04/05/07/08 where compatible. TC-06 owns signed installation/rollback;
TC-09 has a separately admitted filesystem window. Shared setup reduces duplicate
builds, not the need for each observed result. Broken prerequisites make downstream
cases blocked/not-run. Cases assigned to installed containers require the real
niri/profile environment even when the assertions themselves are public CLI/Git.

Retain source/old and new CLI/image hashes, actual object receipts and output
bytes, command exits, fresh native events, safe before/after state identities,
expected refusal causes, resource limits/peaks and actual owned cleanup. Require
both producer and installed consumer evidence; a sidecar containing an expected
hash alone is not shared-pipeline qualification. Exact new source must also pass
ordinary formatting/Clippy/build and applicable existing public E2E gates.

Exit requires all cases in scope actually pass, no lost S/I/P/J, no authority
weakening, old-image/old-state compatibility and native/signed evidence. Existing
D1–D5 results are prerequisites, not new-path passes. No production rollout or
workstation home apply follows automatically.

## Risk bands and non-coverage

Risk bands follow spec.md: Medium/High authority/privacy/identity/lifetime risks
receive the owning level's adversarial cases above. Medium/Medium extra-record
cost is checked through actual producer/consumer reuse and observed resource use;
no invented latency or performance target is asserted.

| Deferred scope | Band / rationale |
|---|---|
| Power loss, every crash ordering or unobserved interruption window | Medium/High lifetime risk; only actually captured TC-07/09 boundaries can pass |
| Include transactions, package-linked config, all-dotfiles/Noctalia artifacts | Medium/High privacy/scope risk; zero-reference niri main-file experiment only |
| Production signing/deployment and other architectures/hardware | Medium/High authority risk; existing dedicated authorization and qualification remain required |
| Malicious unrestricted same-UID isolation | Medium/High lifetime/privacy risk; not an existing store security boundary; ordinary corruption/races/refusals remain covered |
| Automatic store retention, desktop cache or custom artifact transport | Medium/Medium cost; no such mechanism is introduced, so no parallel user workflow to qualify |

Traceability: AC-01→TC-01; AC-02→TC-02/09; AC-03→TC-03/06;
AC-04→TC-04; AC-05→TC-04/06/07; AC-06→TC-05/07;
AC-07→TC-07/08/09; AC-08→TC-06/08. Every case has an H-task owner and every
task cites its cases. Mapping consistency is not executed coverage.

## Actual execution ledger (2026-10-06)

| Case | Current result and evidence scope |
|---|---|
| TC01 | Pass — real public composer/store/Git E2E, shared/target provenance, same/changed bytes, dirty exclusion and no-baseline/unsupported-content controls. |
| TC02 | Pass — actual full-source context/static image and installed-record consumer; source object is not a runtime output. Public tampering and self-contained verification after real collection pass. |
| TC03 | Pass — sanctioned installed-container public home commands, all implemented record/metadata/size/link partitions; logical existing state/live remain unchanged. |
| TC04 | Partial — actual selected/local/later edits and public source export/index/privacy pass; retained old/new empty state readback passes. Nonempty old S/I/P across new B awaits TC06. |
| TC05 | Pass — composed record consumed by actual native niri home cycle, relative includes, stale plans, fresh reload and installed acceptance. Lab screenshot inspected. |
| TC06 | Not-run — fixture and native artifact producer are implemented; hosted signed old/new A/B/A remains pending. |
| TC07 | Pass — actual native SIGKILL publication, resume/abort/keep-current/later edits and store-independent consumer; actual host object GC precedes the privacy/recovery rerun. |
| TC08 | Pass locally — random private/live/include canaries, actual selected patch and committed public blob, preserved unrelated index/dirty data, review-history exclusion, public baseline/object equality and unchanged normal workflow. |
| TC09 | Pass — actual owned APFS export ENOSPC, no published partial context/stage, filler-only release, same-identity retry/verification and explicit engine unpin/GC. No power-loss or every-crash-order claim. |

The broader static-image suite passes12/13. Its sole failure is disabled greetd
in the static fixture; native derivation owns unit enablement. Native home review
and toolkit workflows pass. This is not full installed OS or boot qualification.
Exact local identities/reports and all source gates are recorded in worklog;
TC06 cannot borrow an older signed transition pass.
