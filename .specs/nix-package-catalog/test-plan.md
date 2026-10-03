# D3 verification plan

Conditions come from [requirements](requirements.md); [tasks](tasks.md) assigns
every case. Each case has one owning level: manual E2E, public CLI E2E or installed
container E2E. No unit/model/mock/source-text substitute is authorized.

| Case / technique | Owning level and concrete workflow | Expected result and condition | State |
|---|---|---|---|
| TC-01 / equivalence partitions | Manual E2E: verify each real archive checksum, normalize complete source, admit through public store CLI; mismatched archive/tree must not be relabeled | Exact recorded pins; wrong bytes refuse (AC-01) | local positive acquisition pass with prior binary; new workflow/negative not-run |
| TC-02 / decision table | Public CLI E2E: vary namespace, selected package, builder, runtime and source allowlists independently; include hidden source; use absent store path | Each denied case exits before store creation/access; no stdout result (AC-02) | not-run |
| TC-03 / state transition | Public CLI E2E: list/pins/resolve/plan twice from same reviewed inputs; no image admission | Stable metadata/graph/IDs without daemon realization (AC-03) | not-run |
| TC-04 / end-to-end data flow | Public CLI E2E: real nonroot offline builds; run SQLite data query into jq transformation/regex; capture loader initialization of exact library | Expected real JSON, jq 1.8.2/SQLite 3.53.4 and declared library/ELF, on selected Fedora runtime (AC-05, NFR-01) | not-run |
| TC-05 / state transition | Public CLI E2E: build, cache repeat, then forced independent rebuild of all graph nodes | Reuse distinct from reproduction; actual bytes equal or preserved winner plus divergence failure (AC-06) | not-run |
| TC-06 / state transition | Public CLI E2E: export both closures, import into fresh store using expected digest, remove producer, execute SQL→jq and profile | Runtime remains complete without producer/source/builder download (AC-07) | not-run |
| TC-07 / decision table | Manual E2E: resolve actual compiler RPMs, build exact native image, independently read back contents and retention; exercise mismatched material | Native Fedora/GPG policy and exact compiler RPM equality before use; mismatch refuses (AC-04) | not-run |
| TC-08 / decision table | Manual E2E of real D2 material constructor: unchanged source/recipe/tool/RPM inputs versus one changed input; retain actual comparison inputs | Stable equality for unchanged, unequal changed inputs, runtime IDs excluded; actual main-only workflow skip/signing remains separate (AC-11) | not-run |
| TC-09 / valid-invalid pairs | Public CLI E2E: author actual built packages; wrong pin/material, absent output, other foundation or existing destination; then real D2 consume | Exact closed receipt/definition/closure binding; invalid input preserves prior state (AC-08) | not-run |
| TC-10 / installed state transition | Installed container E2E: login shell and systemd user service resolve exact inventory paths; real SQL stdin→jq regex; run absolute RPM jq | Correct selected commands and unchanged functional RPM path, without FHS/library override (AC-09) | not-run |
| TC-11 / state transition | Installed container E2E: manually start history unit, insert generated user row, stop/start and query same database | Additional observation, previous rows/user data preserved and successful service result (AC-10) | not-run |
| TC-12 / compiler and execution contracts | Manual standard gates on adopted source; retain executable/source hashes and invoke the existing case owners without duplicating assertions | Required format/Clippy/E2E/release/legacy checks pass on owning source (NFR-03) | private compiler subset pass; owning-source/runtime not-run |

The TC-02 decision table uses one permitted tuple, then independently denies each
of namespace, package, builder image, runtime image and source object; a sixth
negative introduces an object absent from declared sources. TC-07 compares equal
versus different actual compiler RPM material. TC-08 compares unchanged inputs,
changed source pin, changed author/recipe, changed compiler RPMs and changed
realization-only receipt data. The last must leave deterministic material equal.
These are explicit partitions, not a claim that the prepared cases execute every
possible combination.

NFR-01 uses actual platform/endpoint/installed observations in TC-04/TC-07/TC-10.
NFR-02 uses manual authority/mount review of the real execution and its receipts,
alongside TC-04/TC-09/TC-10; do not substitute source-string assertions. NFR-03 uses
TC-12 and recorded resource measurements. Lower-level isolated tests are excluded
by owner policy; the installed level is necessary for PATH, systemd and persistence.

CLI cases use `sysroot/tests/e2e_catalog.rs`. TC-04–TC-06 and the positive/wrong-pin
part of TC-09 are prepared in its ignored real-application workflow; missing
runtime partitions must be executed through public workflows during T8. Do not
claim the complete negative matrix from two ordinary cases. TC-10/TC-11 use
`catalog_tests.rs` registered as `native::catalog` for the exact derived candidate.
Ordinary unrelated fixture images are deliberately excluded. D2's existing
configuration-path refusal cases supply the inherited FHS boundary; D3 does not
recreate source assertions for it.

Entry: adopt reviewed D3 above final D2, build and pin actual tool binaries, acquire
compiler plus exact fresh Fedora foundation, and admit verified source trees.
When using an outer daemon from a controller, mount the store/TMPDIR path identically
in both namespaces. Use explicit endpoint/owned directories; no credentials/home
mounts. Run heavy image/test work serially within the delivery storage floor.

Run ordinary `cargo test -p sysroot --test e2e_catalog --locked`, then explicitly
include the ignored workflow with real `KEDRA_CATALOG_BUILDER`,
`KEDRA_CATALOG_RUNTIME`, `KEDRA_CATALOG_JQ_SOURCE`,
`KEDRA_CATALOG_SQLITE_SOURCE` and pinned `KEDRA_ENGINE_E2E_BINARY`. Use
`--include-ignored --test-threads 1 --nocapture`; a listing/ignored result is no pass.
Select the exact D3 derivation in the existing container harness for `native::catalog`.

Exit: complete declared E2E/manual cases or explicitly retain a blocker; required
owning-source checks pass; actual artifacts and failures are recorded in
[verification](verification.md). Source/format/compiler evidence never qualifies
runtime. TC-08's protected-main workflow observation is explicitly deferred to
post-merge production observation, not a branch bypass or a claimed local pass.
Full Nix compatibility and x86 are unqualified high-consequence scope extensions;
production publication, D4 lifecycle and D5 cache/recovery have separate owners
and gates rather than duplicate coverage here.

## Native-template follow-up

**TC-13 / valid-invalid binding partitions — public CLI/manual E2E, T5 with T4/T6
integration:** after compiling the owning source, run the existing author against
real built outputs and inspect its emitted SystemDefinition/inventory/pins as
product artifacts. The three destinations must contain their native text and
typed store references, runtime PATH expansion must survive, and selected versions
must match executed programs (AC-12/AC-13). Verify actual rejection of a mismatched
template/author pin and unsupported reserved token in a disposable author build;
retain the existing composer FHS-collision refusal. This is product behavior, not
a repository/source-text scanner or isolated renderer test. Current state: not-run.
The existing installed TC-10/TC-11 remains the authoritative shell/environment.d/
systemd/persistence result. No new runner or parser-test framework is introduced.
