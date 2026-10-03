# D5 verification plan

All cases below are **not-run**. Preparation is not test evidence. Source:
[acceptance conditions](requirements.md#acceptance-conditions),
[design](design.md), [task mapping](tasks.md).

## Method and entry conditions

Only public CLI/external-library end-to-end or manual actual-process workflows.
Use equivalence classes for authorization refusals, boundaries for time/size,
and state transitions for crash/publication/recovery. Fixtures may generate inputs
and sign them; assertions inspect user-visible execution and actual persistent
state, never this repository's source/manifests/test presence.

Parent freezes integrated D5 source and records exact D4/D5 ancestry, tool versions,
runtime image and daemon identities before qualification. Build once, copy final
`sysroot`/`kedra-lab` to private single-link mode 0500 files and hash each before and
after every invocation. Stop concurrent Cargo builds/heavy daemon activity. Inputs,
keys, volumes, names and cleanup authority are generated and fixture-specific.
No production credentials, broad unlocked vault/session, registry write or live
home/workstation installation is part of any case.

## Cases

| Case / condition | Owning level | Inputs and actual actions | Required result / postcondition |
|---|---|---|---|
| TC-C1 / AC-C1 | Public CLI E2E | Build/export real catalog ELF/library closure; generated P-256 key signs exact receipt; remove producer; fresh empty consumer substitutes using independently resolved graph/root and explicit policy. Run program with real input. | Expected actual output; exact selected root/closure/image and signer; normal build reuses it without executing producer recipe. |
| TC-C2 / AC-C2 | Public CLI E2E | Independent generated key, altered payload/signature/archive, wrong purpose/scope/revision/root/recipe/platform/prefix/runtime, duplicate references, incomplete closure and conflicting preexisting winner. Time fixtures cover before-from, at-from, before-until, at-until; a large actual copy crosses expiry. | Every invalid case refuses before new object/image/root/profile publication; existing selected program still runs identically. Valid boundary cases pass. No silent local-build fallback hides a refusal. |
| TC-C3 / AC-C3 | External consumer E2E | D4 standalone Cargo project uses public catalog/engine API and its own cache scope/key; substitute/run/reuse then explicit independent rebuild. Try the first project's receipt under second project's policy. | Real custom consumer output and independent build byte comparison; other scope refuses; no installed authority lookup/privileged operation. |
| TC-C4 / AC-C4 | Public CLI/manual process | Start actual composition verification/native recipe consumption; observe durable lease and growing payload, SIGSTOP then SIGKILL one invocation. Keep another same/different identity consumer active or stopped; run recovery; resume survivor and finish. | Abandoned registered payload reclaimed; active entries reported/skipped; survivor completes with unchanged input; journal/tag recovery occurs only through public retry. |
| TC-C5 / AC-C5 | Public CLI E2E | Retained fixture leases plus foreign sentinel; introduce unknown member/schema, changed directory inode, symlink, hardlink and legacy unregistered directory. Interrupt cleanup after directory removal before lease retirement. | Unsafe entry reported/refused with all its payload and external sentinel intact; other safe entry may recover. Orphan durable lease retires safely; legacy paths are untouched. |
| TC-C6 / AC-C6 | Actual native manual workflow | Four new native transactions; capture each publication state using filesystem/image events, SIGSTOP, read actual journal/binding/tag/ID, then SIGKILL. Public retry with same independent identities and pinned executable. | Pre-ID state rebuilds; post-ID/binding/tag states revalidate and reuse exact ID, preserve binding bytes and retire only owned pending state. Missed window is not-run and must be repeated. |
| TC-C7 / AC-C7 | Actual bounded-filesystem manual workflow | Put only fixture cache/store on sized isolated tmpfs or dedicated filesystem; fill it and execute real snapshot copy, transaction/binding `.next` writes, import admission/root publication. Retain valid unrelated binding/closure and external sentinel; remove only filler, publicly recover/retry. | Actual OS ENOSPC observed, no fabricated fault return; no incomplete authorized closure/profile selection; retry succeeds and prior data/sentinel survive. Metadata allocation failure refuses/retains ambiguous state before any large unleased copy. |
| TC-C8 / AC-C8 | Evidence audit of above actual flows | Record executable hashes, exact invocation, observed state/timing, signal exit, daemon/nonce/image, disk ceiling/free floor, cleanup targets/results and sentinel readbacks per case. | Hashes identical per invocation, no unowned deletion, unchanged retained default, truthful missed/failed/blocked cases. No retrospectively assigned executable hash. |

Case techniques/methods: TC-C1/TC-C3 use actual end-to-end reuse/rebuild flows;
TC-C2 uses authorization equivalence classes and time/size boundaries; TC-C4/TC-C5
use concurrent-state and ownership decision cases; TC-C6/TC-C7 use observed state
transitions under actual process/filesystem faults; TC-C8 manually audits the
per-invocation observations. TC-C4 is owned by the manual actual-process workflow;
its row's CLI calls are the interface being exercised, not a second testing level.
No lower-level unit/model/mock coverage is planned because owner policy forbids it.

TC-C7's cache/store filesystem is bounded independently from the workstation.
Tmpfs ENOSPC does not qualify the daemon's image-layer filesystem. To claim native
BuildKit layer exhaustion, run a separate disposable daemon with a bounded data
filesystem, enough verified space for baseline layers and a recorded failure while
writing a derived layer. Preserve cached parent identity, free fixture filler and
retry normally. Never fill the retained default daemon's disk or host filesystem.

The filesystem/image-event observer is fixture code around real public processes,
not a product fault hook or new test runner. Do not reconstruct journals or use
sleep timing alone to assert a captured window. The existing Cargo E2E targets,
container harness and public `kedra-lab`/VM tools remain the execution entrypoints.

## Required source gates and result records

After integration, parent runs repository-required formatting, workspace Clippy,
ordinary CLI E2E and release build, plus the existing required uv/OpenSSL
release-interop and release-material workflows. Core release code is unchanged;
the new independent engine verifier is exercised with actual OpenSSL cache signatures.
Execute explicit ignored native/cache workflows separately; an ignored listing or
zero selected cases is not coverage. Run installed harness cases when native
material/behavior changes. Actual counts, source and retained report paths go into
a later verification record and the parent-owned worklog/status.

The earlier four static replay fault runs remain source-level evidence with their
documented per-invocation executable provenance gap. They qualify none of TC-C6.
Do not claim power-loss persistence, x86 native generation, Nix protocol
interoperability, production key eligibility or fresh native boot from these cases.
No unit, model, mock, source-string, repository-scanner or doctest additions.

Prepared executable source is in the ignored shadow tree, not yet adopted:
`tests/cache_recovery.rs` under the existing `e2e_engine` target contains a real
signed build/export/refusal/import/run/rebuild flow and a Linux-only opt-in bounded
tmpfs ENOSPC/import/recovery flow. `e2e_context` adds a killed real context copy
while another reader remains SIGSTOP-held, followed by public recovery and actual
survivor completion. Existing empty-parent assertions become public recovery
result assertions. These sources are not compiled or executed yet and do not
stand in for later native publication-window or journal-boundary faults.

Exit criteria: required cases actually pass, material failures are fixed and
rerun, owned resources are retired with preserved sentinels, exact-source gates
pass and the separate D5 draft PR is attached with its immediate D4 base. Runtime
prerequisites that cannot be met remain explicitly blocked/not-run.
