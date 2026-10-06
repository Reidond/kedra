# Future verification cases

Status: **qualification in progress; broad groups not yet complete**. Conditions cite [requirements](requirements.md) and the
normative [language contract](language.md). Cases add executable workflow details;
they do not duplicate acceptance prose. All cases use one owning level. No unit,
model, mock, doctest or repository source/layout scanner is permitted.

| Case | Conditions / technique | Owning level and method | Real inputs/actions | Expected result and preservation | Tasks |
|---|---|---|---|---|---|
| TC-01 — Edit and plan | AC-01, NFR-01/04; valid/invalid language partitions | Public CLI E2E, automated | Check/plan a generated admitted `.kedra` entry twice, edit an inline shell block and plan again with identical frontend/engine executables; introduce syntax/type errors. | Stable input gives equal semantics; edited script changes recipe; invalid input reports source location and accepts nothing; no Cargo invocation (AC-01, NFR-01/03). | T1, T2 |
| TC-02 — Selection rules | AC-03/07; decision table below | Public CLI E2E, automated | Resolve shared/target sets with each row, explicit image roles and real package declarations. | Correct selection or intended conflict/role/target refusal with origins; no store/build action on rejected intent (AC-03/07). | T1, T2, T4 |
| TC-03 — Source/library build | AC-04/07; source/dependency partitions | Public CLI E2E, automated | Admit real jq/SQLite source, build inline library/shell recipes, transfer to an empty receiver and execute SQLite; independently rebuild. Try corrupted archive/tree, absent patch/tool, cycle or library. | Actual command/library use and rebuild comparison; specific intended refusals preserve original runnable output, nonempty closure required (AC-04/07, NFR-03). | T3, T7 |
| TC-04 — Fedora material | AC-05; fresh→resolved→candidate→drift transitions | Manual real resolver/candidate workflow | Resolve approved real Fedora system/compiler roles; build matching foundations; separately change repository package material or make a required repo unavailable in a fixture. | Full observed material matches or candidate refuses with the correct reason; retained valid foundation preserved (AC-05, NFR-03). | T4 |
| TC-05 — Mixed installed result | AC-02/05/07; provider/role partitions | Sanctioned container scenario, automated | Build an actual ARM candidate with a Fedora program plus source jq/SQLite; execute them and inspect the selected library. Choose a compiler-only requirement absent from runtime base and not required by other runtime selections. | Fedora package installed; engine programs use exact store paths/library; that compiler-only requirement does not leak into runtime; no global replacement (AC-02/05/07). | T4, T5, T7 |
| TC-06 — No effects while planning | AC-06, NFR-02/03; adversarial input partitions | Public CLI E2E, automated | Put harmless sentinel-write/network command text inside shell blocks; run check/format/plan without executing a build; try undeclared imports, env()/eval()/Rust escapes and denied consumer policy. | Literal commands remain data and sentinel unchanged; unsupported/denied inputs refuse at named gates; plan requires neither network nor Docker; no accepted partial output (AC-06, NFR-03). | T3, T7 |
| TC-07 — General contribution | AC-08; valid/tampered material partitions | Public release CLI E2E, automated | Contribute a third real package with inline configuration; alter frontend/plan/resource/source/policy/version/alias binding one at a time. | New package runs without central-name edits; each mismatch refuses before signing eligibility, accepted candidate preserved (AC-08, NFR-03). No production keys. | T5, T7 |
| TC-08 — Source compatibility | AC-09; version/format transitions | Public CLI E2E, automated | Resolve retained old-layout/list sources and a committed new-format fixture; test mixed authority, unknown version, absent prepared material and invalid new input. | Correct reader or precise refusal, no fallback/union; Git/index, old provenance and accepted selections preserved (AC-09). This exercises real source-resolution behavior, not a repository scanner. | T1, T6 |
| TC-09 — Emission lifecycle | AC-10, NFR-02/03; actual state transitions | Manual real-process workflow | Interrupt an actual frontend emission, race two emitters at one new destination and trigger bounded timeout/resource refusal while retaining a valid plan sentinel. | No partial accepted plan or overwritten winner; actual children reaped, owned scratch settled and prior plan intact (AC-10, NFR-02/03). | T3, T7 |
| TC-10 — Independent frontend/API consumer | AC-06/10, NFR-01; consumer partitions | Public CLI E2E, automated | Build/run a real program from an independent `.kedra` catalog and separately compiled Rust API consumer describing equivalent input, then apply the wrong namespace/policy. | Same semantic intent/lowering and actual output without embedded Kedra targets/keys; denied scope refuses; no implicit author executable evaluation (AC-06/10, NFR-01). | T7 |
| TC-11 — Two-target migration | AC-05/09, NFR-04; target partitions/transitions | Manual real candidate workflow | Compare old/new requests and actual material for desktop and qemu-arm64, run existing appropriate container/boot workflows and no-change repeat. | Intended installed behavior and legacy readability preserved; ARM source packages work, unsupported x86 builds refuse; no workstation install or production publication (AC-05/09). | T6, T7 |
| TC-12 — Self-contained package pilot | AC-11/12; content-kind partitions | Public CLI E2E, automated | Build inline jq/SQLite and inline C source; exercise inline patch, generated header/env and typed config template in a generated real package. For one resource also use an admitted external equivalent. Run resulting program, inspect exact content/modes and compare admitted resource IDs. | No required script/Rust sidecars for pilots; real expected output, original archive source unchanged; equivalent bytes/modes match semantic resources; scripts execute from file inputs rather than oversized argv (AC-11/12). | T2, T3, T5, T6, T7 |
| TC-13 — Parser and formatter limits | AC-13, NFR-01/02; ordered boundaries and invalid grammar partitions | Public CLI E2E, automated | Run real check/fmt/plan on generated modules at/over depth, module, bytes, resource and graph bounds; cycles/mixed versions/duplicate names/import escapes; format literals containing shell variables/backslashes/empty lines twice. | Exact accepted boundaries or bounded specific refusal, no side effects; second format unchanged and decoded literal payload hashes identical (AC-12/13, NFR-01/02). Formatter/parser byte tests exercise product output, not repository text. | T1, T2, T7 |
| TC-14 — Resource identity and refusal | AC-04/12, NFR-03; equivalence/state partitions | Public CLI E2E, automated | Change script/patch/text/template/mode individually; build real preparation nodes with unsafe path/link/duplicate/implicit overwrite/fuzzy patch inputs, keeping original and valid-result sentinels. Include a script over32KiB but within input limits. | Material changes affect identities; unsafe input refuses at the intended stage with sources/winner intact; long valid script builds without relaxing engine argument bounds (AC-04/12, NFR-03). | T3, T7 |

## Selection decision table for TC-02

| Input | Required decision |
|---|---|
| Same Fedora include repeated identically | Deduplicate semantic request, keep origins. |
| Include/remove same RPM without explicit replacement | Refuse conflict. |
| Explicit named Fedora request replacement | Replace exactly that request, subject to independent base policy. |
| Replacement names absent origin/request | Refuse missing reference. |
| Removal violates required base package policy | Refuse, never relax policy. |
| Unequal source recipes under one identity | Refuse conflict; select one explicitly rather than guessing. |
| Fedora and source jq with distinct explicit exposure | Allow distinct filesystem/store identities. |
| Two packages export the same command alias | Refuse ambiguity. |
| Unknown target, unbound role, recursive template, bad-platform dependency | Refuse before realization. |

## Content decision table for TC-12/13/14

| Input | Required result |
|---|---|
| Inline resource and admitted file have identical path/mode/decoded bytes | Equal semantic resource identity; distinct audit origin is retained. |
| Outer syntax formatting only | Same literal/recipe semantics; source provenance may differ. |
| Embedded script, patch order, text or mode changed | Changed relevant resource/recipe identity. |
| `$src`, `$(...)`, backslashes in raw text/shell | Preserved while planning; shell interpretation only during build. |
| Missing/extra template binding or premature self reference | Refuse with location, no arbitrary interpolation. |
| Existing source path added without explicit replacement | Refuse; no silent overwrite. |
| Absolute/traversing/colliding path or link ancestor | Refuse; external and original-source sentinels unchanged. |
| Mixed language versions, unsafe import or unsupported executable syntax | Refuse at frontend boundary, not fall back. |

Each negative case first establishes valid prerequisites so an unrelated failure
cannot count. Skipped Docker/VM work, empty selected programs or missed process
interruption points are not passes. Runtime-script isolation remains the existing
engine contract; no new host-side script execution is introduced for convenience.
