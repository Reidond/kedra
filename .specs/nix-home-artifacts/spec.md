# Niri baseline artifacts in the existing image pipeline

Status: **implementing**; owner authorized PR39 completion on 2026-10-06. Medium combined
specification with [tasks](tasks.md) and [test plan](test-plan.md). Implementation
uses the owner-merged D1–D5 source as its foundation. Their incomplete runtime
qualification remains recorded and is not waived or relabeled here.
Source inspection is pinned to `e908bb281c3dc5189373c48a5ab4958c1b9d2fde`;
a later checkout switch does not establish an API change.

## Problem and intended result

The existing composer already carries public home defaults into an image; this
proposal does not claim that capability is missing. The requested next step is
to use the engine's artifact foundation for home configuration while preserving
writable-home review and activation. The first experiment is one niri main-file
baseline, produced by the ordinary image composition command and consumed by the
ordinary installed home workflow. The single-file limit is a coordinator-proposed
first slice accepted for planning, not a detailed limit authored by the owner.

There is no measured incident frequency or promised speed improvement. The value
to demonstrate is concrete reuse: identical public config bytes have the same
engine source-object identity across releases; composition materializes its
baseline from the verified object; the installed consumer verifies that same
identity without a build store. Existing engine source import, deduplication,
verification, roots and transfer are reused instead of creating a second home
artifact store/protocol. Existing merge/publication/activation code remains in
charge. If this cannot be delivered with the bounded additions below, reconsider
the design rather than turning this into an OS-format or home-manager rewrite.

## What exists today

| Boundary | Inspected contract |
|---|---|
| Source and image defaults | [`source.rs::materialize`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot/source.rs) reads exact validated Git blobs from a frozen plan and emits existing baseline paths plus `source.json`. [`system.rs::definition`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot/system.rs) currently turns each into `SystemContent::Bytes`; home defaults are already present. It reserves `/usr/share/sysroot` and `kedra-source:` from external contributions. |
| Generic composition | Engine [`system.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot-engine/system.rs) resolves typed files and hashes their bytes in `SystemPlan`; export writes the payload and binds it into the context. `SystemDefinition.outputs` requires matching runtime-foundation outputs: a plain `src-*` object cannot be inserted there. |
| Source artifacts | Engine [`store.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot-engine/store.rs) has source import, verify/closure, roots, bundle transfer and GC. [`tree.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot-engine/tree.rs) computes canonical `sysroot-engine-tree-v1` identity from entry path/type/executable flag/size/hash and scans references. Public bounded single-file production/verification from bytes is not currently exposed. |
| Installed home authority | [`home/text.rs::installed`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot/home/text.rs) reads root-owned `source.json`, requires one ordinary-file niri home-baseline entry and verifies `/usr/share/sysroot/home/default/.config/niri/config.kdl`. It returns the existing schema1 `Baseline`. |
| Review and activation | [`text/transition.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot/home/text/transition.rs) uses committed source/history for advisory preview. [`text/activation.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot/home/text/activation.rs) separately requires installed/source equality, validates niri in its include context, journals replacement and waits for a fresh reload event before accepting a baseline. |
| System/native authority | [`context.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot-engine/context.rs) verifies exported artifacts; [`native.rs`](https://github.com/Reidond/kedra/blob/e908bb281c3dc5189373c48a5ab4958c1b9d2fde/usr/src/kedra/crates/sysroot-engine/native.rs) performs closed native transformations, including initial skeleton creation. Neither is an existing-home activation API. |

[Architecture](../../usr/src/kedra/docs/ARCHITECTURE.md#writable-home),
[TEXT-REVIEW](../../usr/src/kedra/docs/TEXT-REVIEW.md),
[HOME-REVIEW](../../usr/src/kedra/docs/HOME-REVIEW.md) and the
[engine proposal](../nix-recreation/proposal.md) are the durable boundaries.
Existing dated CLI/native/signed A/B/A results cover their existing behavior;
they do not qualify this new producer/consumer integration.

## Scope and state

The source maintainer continues the normal image build. The desktop owner uses
existing adoption, status, source preview, installed activation and recovery
commands. There is **no new normal-user prepare command, receipt/store flag,
desktop cache, daemon, library or dependency**. The live
`.config/niri/config.kdl` remains an ordinary writable file.

Accepted baseline B, live L, next baseline N, selection S, local policy I,
published source receipts P and recovery J remain independent. Only committed
public N goes through the build store. Merged live candidates, selections before
publication, private checkpoints, includes and app/credential data never do.
Schema1 records, record names, plan domains and journals stay unchanged.

Exclude other dotfiles/Noctalia artifact production, include transactions,
package references in niri, cloud caches, arbitrary activation scripts, automatic
GC policy, production rollout and additional architectures. The first producer
is the existing `qemu-arm64` system composition adapter; legacy image assembly
and other targets retain their current home behavior until separately planned.

## Proposed pipeline and trust contract

1. During existing `sysroot system compose`, the Kedra adapter selects the single niri baseline from the already frozen source plan. Preserve source provenance checks and read the same committed blob once. Use a narrow engine operation to import this one regular non-executable `config.kdl` from bounded bytes into the existing build store. Reuse source-object admission, verification and pinning; require zero references, images and derivation. No builder or live-home input is involved. If the source plan declares no niri baseline, preserve existing composition behavior and emit no record; existing home commands still refuse a missing installed baseline. Do not make niri mandatory for otherwise valid composition merely to exercise this slice.
2. Add only the minimal generic single-file artifact primitives in the existing engine crate: canonical identity calculation, admission through the existing source-store machinery, and verification/readback of a one-file source receipt against bytes. Reuse the actual tree encoder/reference rules; do not duplicate the hash algorithm in home code. Names here describe proposed operations, not existing public APIs. Return verified public bytes from the admitted object to the adapter, and use those bytes for the ordinary baseline `SystemFile`.
3. The adapter also emits one additive root-owned0644 `SystemFile` at `/usr/share/sysroot/home-artifacts.json` in the already-reserved source namespace. Proposed closed record: schema1 and one `niri` field containing the engine `ObjectReceipt`. The code fixes member `config.kdl` and its ordinary installed baseline destination; no supplied destination or extra source/target/version/hash fields duplicate `source.json`. Bound the record to4096 bytes; reject duplicate/unknown fields. The receipt must be a genuine verified source object with empty references/runtime/derivation, not a fabricated runtime output.
4. `system plan` computes the same expected file/record bytes using the shared pure identity calculation, with no source import/pinning. `system compose` additionally performs actual store admission/readback and requires agreement before exporting. The niri source object is a build input, not a runtime output: leave `SystemDefinition.outputs`, catalog contribution receipts, `SystemComposition` schema and native definitions unchanged. Existing resolved-file/payload hashing covers the new record and unchanged baseline; resulting composition identity naturally changes. No additional store-tree copy is installed; existing skeleton-copy behavior remains unchanged.
5. Extend the existing `installed(niri)` loader: perform today's root-owned source-manifest/baseline checks first. If the additive record is present, read it safely and require the engine's single-file receipt verification over the already-read baseline bytes. Its receipt/tree/object identity and closed shape must match; malformed, unsafe, unsupported or mismatched records refuse, never fall back. The source manifest continues to supply target/revision/layer authority. Return the same old `Baseline` bytes and provenance.
6. If the record is genuinely absent, use today's verified legacy baseline path. This is explicit old-image compatibility, not a claim that absence proves an image predates the feature. Broken links/unreadable entries are not absence. New-producer E2E requires record presence whenever a niri baseline is declared; runtime absence does not bypass the existing source verification or grant new authority. Old CLIs ignore the additive file and continue using the preserved ordinary baseline/source manifest.
7. Existing `activate-plan`, `apply`, adoption and installed-baseline assessment use the extended loader without options. Source-only `home file plan --repo` remains advisory and keeps its existing Git/history path: an image artifact does not yet exist for an unbuilt source preview. Apply still requires installed/source equality, current plan/live/state identity and native validation/reload. No additional Git read or artifact-specific reimplementation of reconciliation is introduced. No root helper, engine profile or native step may apply an existing home.
8. The emitted file and record are ordinary signed image payload. Signature/deployment authority remains the existing verified OCI/installed-source chain; an engine receipt is content identity, not a signature or authorization. Home does not independently promote arbitrary build-store objects into installed trust. A source-only build, untrusted image or valid receipt by itself cannot authorize home apply.
9. After export, image/context bytes and the existing home state/checkpoints are self-contained. Build-store pins protect active preparation; explicit normal build-store GC may remove artifacts later without affecting installed use, prior images, rollback or J. Do not introduce per-user roots or infer reference counts from the engine's root set. Interrupted import/export follows existing store/context recovery and must not publish a successful context containing only half the baseline/record pair. No new store access or lock ordering is added to the desktop.

### Format tradeoff and smallest sound alternative

This does add **one image-owned record format and a generic engine API surface**;
it is not metadata-free. It does not alter the signed release envelope,
`source.json`, composition/native schemas or persisted home state, and does not
weaken the runtime-foundation requirement on `outputs`. The record lives under
existing reserved contribution protection and is covered by existing context
hashing/signing. Preserve ordinary paths/modes for old images/CLIs.

A source object cannot honestly be added to current runtime outputs without a
larger format/closure change. That expansion is rejected for this slice. The
smaller alternative is simply to keep today's `SystemContent::Bytes` and source
hash check, optionally measuring its adequacy: it avoids the additive record but
provides **no new shared object producer/installed-consumer boundary**. If the
new API/record cost outweighs demonstrated reuse, choose that explicit deferral;
do not retain a manual user preparation workflow just to exercise an API.

## Acceptance criteria

All are proposed requirements, not observed passes. Existing niri text limits
remain128KiB/8192 lines, UTF-8/LF/final-newline and safe file ownership/type.

- **AC-01 — Real shared production.** Given frozen committed source with a declared niri baseline and the retained build store, when ordinary system composition runs, then the actual baseline passes through genuine engine source admission/readback, only that public file enters the artifact, and unchanged bytes reuse the same identity across source revisions. The existing baseline path/mode/content and source manifest remain equivalent. A plan alone is not a produced artifact; an unreadable/missing declared blob, ambiguous baseline or unsupported content refuses. A source plan with no niri baseline preserves existing composition behavior without an artifact record; that absence never authorizes deleting a live file.
- **AC-02 — Existing image transport.** Given the new baseline/record, when the existing context export, verification and image pipeline runs, then both are covered by actual resolved-file/payload identity and reach the final image. No new runtime output alias, contribution receipt claim or duplicate store-tree payload is added. Changed record/baseline bytes invalidate the expected context/image evidence; interrupted export cannot be reported successful.
- **AC-03 — Strict installed authority and compatibility.** Given a signed installed image, when existing home commands load N, then a present record must verify against the root-owned baseline and existing source manifest; a genuinely absent record uses the existing verified legacy path. Wrong schema/object/shape/owner/link/mode/content refuses without fallback. A valid record cannot override installed target/source/layer checks or unsigned/wrong-authority deployment refusal. Old CLI plus new image and new CLI plus old image retain working baseline loading.
- **AC-04 — Independent review and publication.** Given distinct S/I/P and a later live edit, when the ordinary preview/installed-acceptance/export sequence runs, then current reanchor/conflict/local-policy semantics remain: later L is unstaged, changed exact-local values return to review, selected export writes exactly S to the correct source layer and recording publication does not accept N. Unrelated Git index/worktree changes survive; source-only plan IDs cannot authorize installed apply; no conflict markers enter live files.
- **AC-05 — Stored-state and rollback.** Given genuine old schema1 records/history or a pending journal, when either supported old/new CLI reads or recovers them, then record names, baseline bytes, domains and schema remain compatible without migration/reset. Signed A→B→A still requires explicit home reconciliation while preserving S/I/P and writable data. Legacy source-layout provenance remains readable; unknown/corrupt/orphaned state refuses. Resume of B while A is installed is not promised.
- **AC-06 — Native apply and recovery.** Given valid/stale native plans, relative includes, validation/reload failure or actual interrupted apply, when existing apply/recover runs, then validation retains the managed main file's parent context, acceptance requires a new successful niri load event, and resume/abort/keep-current preserve their existing later-edit/checkpoint rules. Failure never advances B. Build-store loss/GC cannot prevent installed loading or J recovery; includes remain outside this transaction.
- **AC-07 — Privacy and lifecycle.** Given synthetic canaries in live/include/private data and a separately frozen public source, when composition and home workflows run, then only committed public baseline bytes enter engine objects/image records. Neither full L nor merged candidates/checkpoints enter store/source Git/public evidence. The live file remains writable and unlinked to immutable storage. Build import/pinning/GC and interrupted publication preserve their own references/sentinels independently of home state and privileged deployment.
- **AC-08 — Scope and evidence.** Given the unchanged public CLI, when the owner follows normal niri/Noctalia/caller-home workflows, then no prepare/store flags, desktop cache or new deployment action is required. Completion needs real composition artifact production, final-image consumption, old/new compatibility, native and signed disposable rollback evidence on exact sources. D1–D5 acceptance remains a prerequisite; unavailable/skipped cases are not passes.

## Planned files and risks

| Files | Proposed responsibility | Task |
|---|---|---|
| Engine `store.rs`, `tree.rs`, `lib.rs` as needed within `usr/src/kedra/crates/sysroot-engine/` | Narrow single-file source production/readback/verification using current canonical identity and admission | H2 |
| `usr/src/kedra/crates/sysroot/system.rs`; new flat `home_artifact.rs` and module declaration in `main.rs` | Existing composition produces the artifact/record; the small shared Kedra module owns its closed record type and fixed paths/member, preventing producer/consumer drift | H3 |
| `usr/src/kedra/crates/sysroot/home/text.rs` | Strict optional installed-record verification using the shared type and unchanged destination constant, returning old `Baseline`; no new home CLI flags/state format | H4 |
| Existing public composition/home E2E targets, sanctioned niri probe/native scenarios and signed home-transition VM workflow | Actual pipeline, compatibility, privacy, GC independence and native rollback cases | H5, H6 |
| Operational home/system documentation, STATUS/worklog and relevant first-party skills/manifests after implementation | Record exact behavior, legacy absence rule, source limits and observed evidence | H7 |

No edit to home merge/export/activation algorithms, release authority, native
steps or composition format is intended. If inspection finds one required,
revise the plan before broadening implementation. The original planning task changed only these three plan files. The owner
authorized the implementation and mapped qualification on 2026-10-06.

| Risk | Likelihood / impact | Control |
|---|---|---|
| Record confused with signature or legacy absence silently weakens trust | Medium / High | Existing source authority always checked; strict present-record refusal, explicit absence policy and cross-version cases (AC-03) |
| Shared pipeline captures private state or changes source-selected bytes | Medium / High | Frozen public source only, actual store readback and canary/nonempty-artifact checks (AC-01, AC-07) |
| Canonical tree identity implemented twice or runtime-output rule weakened | Medium / High | One engine implementation; source objects remain build inputs, unchanged outputs/schema checks (AC-01, AC-02) |
| Old state/rollback/recovery becomes artifact-store dependent | Medium / High | Unchanged state/journal and ordinary baseline paths; producer removed before actual home/recovery (AC-05, AC-06) |
| Extra record adds cost without useful pipeline reuse | Medium / Medium | No user workflow/cache; demonstrate actual producer/readback/consumer, otherwise defer (AC-08) |

Planning review fixed the earlier manual-artifact proposal: user preparation,
desktop-store flags/cache, new journal pointers and duplicated source metadata
are removed. Acceptance/task/case coverage is reconciled; all implementation
rows remain planned. Outstanding implementation prerequisites are the accepted
D1–D5 source and exact supported old/new image/CLI pair. The additive-record
tradeoff above remains an explicit design decision for plan approval, not an
already shipped capability or authorization to start implementation.

## Execution record (2026-10-06)

H1–H4 are implemented using merged main b60bcb0 and the existing APIs. Engine
store.rs exports single-file prediction/admission/readback/verification; tree.rs
uses one shared canonical encoder. The adapter keeps preflight ahead of store
opening, predicts the record, then admits/readbacks and checks exact agreement.
Installed loading preserves every Baseline/State/Journal field and activation
domain. No new dependency, product CLI option, output kind or composition schema.

H5/H6 local evidence is in worklog WL-20261006-04. Actual composition, same-byte
reuse across shared/target sources, two context-member tamper refusals, genuine
GC, strict installed loading, retained old CLI readback and native killed-CLI
recovery pass. An owned APFS sparse image produces real export ENOSPC; releasing
only its filler permits the same-identity retry. The signed transition fixture
now consumes a native ARM composer-produced receipt and independently builds
the retained b60bcb0 old CLI. That hosted signed A/B/A gate remains pending.

Execution adjustments: the signed desktop VM consumes the platform-independent
receipt produced by its native ARM predecessor job; this adds test coverage, not
a production desktop producer. Current Fedora Noctalia5.2.1 blocks normal home
qualification. Its exact candidate native validation/full export pass; a narrow
exact-version compatibility qualification passes the full13/13 container suite, retaining persisted
APP_VERSION5.0.1 and unknown-version refusal. The retained old CLI exercises niri
and schema1 state; Noctalia capture in the VM uses the new qualified CLI where the
old CLI cannot adopt that newer runtime. No production rollout follows this work.
