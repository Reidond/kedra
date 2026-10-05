# Future verification cases

Status: **all not-run**. Conditions cite [requirements](requirements.md); steps
below add concrete workflow inputs rather than duplicate acceptance prose.
One owning level per case. Standard compiler checks supplement these cases;
no unit/model/mock/doctest or repository-source assertion is permitted.

| Case | Conditions / technique | Owning level and method | Real inputs and actions | Expected result and preservation | Task owner |
|---|---|---|---|---|---|
| TC-01 — Author edit | AC-01, NFR-01, NFR-04; equivalence partitions valid/invalid author | Public CLI E2E, automated | Compile a generated independent author against the pinned public API, emit twice, change one package module/script, emit again using an unchanged engine binary; then compile a Rust type error. | Stable admitted inputs produce equal semantic bytes; a changed recipe changes its plan; compiler failure accepts nothing and supplies a source location. Engine hash stays unchanged (AC-01, NFR-01/03). | T1, T2 |
| TC-02 — Selection rules | AC-03, AC-07; decision table below | Public CLI E2E, automated | Emit/resolve shared plus target-specific sets using each decision row; include real declared package exports and explicit target inputs. | Correct selected set or specific conflict/unknown-target refusal with both origins; no store/build action on rejected intent (AC-03/07). | T2, T4 |
| TC-03 — Source and dependency build | AC-01, AC-04, AC-07; equivalence partitions | Public CLI E2E, automated | Admit genuine jq/SQLite sources, build library+shell, transfer runtime closure to an empty store and execute SQLite; independently rebuild; repeat with corrupt archive/tree, missing patch, cycle or unavailable library. | Actual execution and runtime library use; expected recipe/result comparisons; each invalid partition refuses at its named gate and preserves the original runnable selection (AC-04/07, NFR-03). Nonempty closure required. | T2, T7 |
| TC-04 — Fedora observation | AC-05, NFR-03; state transitions fresh→resolved→candidate, then drift | Manual real resolver/candidate workflow | Resolve actual reviewed Fedora base and system/compiler roles, build matching foundations; separately change available package material or disable a required repository in an owned fixture. | Matching complete RPM material passes; actual changed material/repository failure refuses candidate acceptance with recorded reason; old retained foundation remains usable (AC-05). | T4 |
| TC-05 — Mixed installed result | AC-02, AC-05, AC-07; provider partitions | Sanctioned installed-system container scenario, automated | Build an actual ARM candidate containing a Fedora program and source jq/SQLite, with explicit aliases and a separate compiler role; run programs and check the selected shared library. | Requested Fedora package is actually installed; engine commands use intended store outputs and actual library; compiler-only package does not leak into the runtime selection; no global binary replacement (AC-02/05/07). | T4, T5, T7 |
| TC-06 — Executable-author isolation | AC-06, NFR-02/03; adversarial partitions and numeric boundaries | Manual real-process workflow | Generated harmless author/build-script fixtures attempt to read an unmounted sentinel, reach a test network endpoint, use a daemon socket, emit beyond8MiB or hang beyond60s; record child exits and independently inspect sentinel/resources. | Access attempts cannot reach protected inputs; actual limits terminate/refuse at the expected boundary, children reaped, no accepted partial plan. Test1MiB diagnostic cap and proposed compile bounds separately; do not infer sandboxing from `--frozen` (AC-06, NFR-02/03). | T3, T7 |
| TC-07 — Generic release contribution | AC-08; valid/tampered material partitions | Public release CLI E2E, automated | Author a third simple real source package plus existing ones, generate its typed exposure and real candidate; alter source inventory, plan hash, author hash, policy, alias or output binding one at a time. | New package works without central-name edits; material mismatch refuses before signer eligibility, accepted candidate unchanged (AC-08, NFR-03). Production keys never enter fixture. | T5, T7 |
| TC-08 — Source compatibility | AC-09, NFR-03; format state transitions | Public CLI E2E, automated | Resolve retained pre-layout/current-list commits and a real committed DSL fixture; exercise mixed-authority, unknown version, absent prepared receipt and invalid-new-plan cases. | Correct reader or precise refusal; no fallback/mixed union, existing Git/index/store selections and old-source provenance preserved (AC-09). Real source resolution is product behavior, not a scanner of repository files. | T1, T6 |
| TC-09 — Emission lifecycle | AC-10, NFR-02/03; state transitions and boundary values | Manual real-process workflow | Interrupt an actual emitting process before publication, race two emitters for the same fresh destination, and test empty/8MiB/over-limit plan output with a valid existing plan sentinel. | Only a complete valid plan can win; incomplete output fails, collision does not overwrite, prior plan unchanged and owned processes/resources settled (AC-10, NFR-02/03). | T3, T7 |
| TC-10 — Independent consumer | AC-06, AC-10, NFR-01; consumer/policy partitions | Public CLI E2E, automated | Separately compile a second author with its own namespace/policy and real small executable; build/run through unchanged generic engine and data-only CLI, then use the first consumer's denied policy. | Actual program output with no Kedra target/key dependency; denied scope refuses; plan/verify does not execute author code; semantic readback matches selected target (AC-06/10, NFR-01). | T1, T3, T7 |
| TC-11 — Target cutover | AC-05, AC-09, NFR-04; target partitions and transition | Manual actual release-candidate workflow | For desktop and qemu-arm64, compare legacy/DSL requests and observed actual installed package material, run existing target-appropriate container/boot checks, then exercise a no-change repeat. | Both existing targets retain intended behavior; ARM source packages work; unsupported x86 source builds refuse explicitly; unchanged input behavior and legacy rollback-source readability preserved (AC-05/09). No workstation install or production signing. | T6, T7 |

## Selection decision table for TC-02

| Inputs | Required decision |
|---|---|
| Same Fedora include repeated identically | One semantic include, retain origins. |
| Include and remove same RPM | Refuse conflicting intent. |
| Explicit named replacement of an existing selection | Replace that exact selection, subject to independent base requirements. |
| Replacement points to no existing selection | Refuse missing origin. |
| Removal violates independent required base packages | Refuse without relaxing target policy. |
| Different source recipes under the same identity | Refuse conflicting definition. |
| Fedora jq plus source jq, distinct explicit exposure | Allow separate filesystem/store identities. |
| Two packages claim the same exported command alias | Refuse ambiguity. |
| Unknown target, unresolved role or cross-platform reference | Refuse before execution. |

No test may count a skipped Docker/VM case or an empty program/closure selection
as a pass. Negative cases must first establish valid admitted inputs and then
record the specific intended rejection, not merely any nonzero exit.
