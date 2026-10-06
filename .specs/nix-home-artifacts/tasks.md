# Planned tasks — composition-owned niri artifact

Implementation authorized on 2026-10-06. **H1–H7 are completed for this slice.** Requirements/design are in
[spec.md](spec.md), cases in [test-plan.md](test-plan.md). The owner merged D1–D5
source; unresolved prerequisite runtime qualification remains explicit.

| Task | Work and mechanism | Depends on | Test requirements | State |
|---|---|---|---|---|
| H1 | Reconcile explicitly accepted D1–D5 source, current source/composition types and supported old/new image/CLI pairs. Confirm the additive record fits existing reserved-path, payload and signed-image contracts; preserve current unresolved delivery gates. | D1–D5 acceptance; implementation authorization | TC-01, TC-02, TC-04, TC-06 | Completed |
| H2 | Add narrow single-file source identity/admission/readback/verification operations within existing engine `store.rs`/`tree.rs`/`lib.rs` as needed. Reuse canonical tree encoding, source admission/roots and no-follow checks; add no library, home-specific engine type, arbitrary writer or runtime-output exception. | H1 | TC-01, TC-03, TC-09 | Completed |
| H3 | Extend existing `sysroot/system.rs` adapter to route only the frozen public niri baseline through genuine source-object production. Feed verified stored bytes to the ordinary baseline file and generate the closed additive image record in the reserved namespace. Add flat `home_artifact.rs` and its `main.rs` module declaration for the shared Kedra record/fixed paths. Keep pure plan material equivalent to actual compose material, but distinguish prediction from admitted artifact. Leave catalog contribution outputs/receipts and composition/native schemas unchanged. | H2 | TC-01, TC-02, TC-08, TC-09 | Completed |
| H4 | Extend `home/text.rs::installed` to validate the optional record over the already-read trusted baseline using the shared record type and unchanged destination constant. Preserve absence behavior for old images, strict refusal when present, old Baseline/State/Journal bytes, existing CLI and source/installed comparison. No desktop engine store or merge/activation rewrite. | H2, H3 | TC-03, TC-04, TC-05, TC-06, TC-07 | Completed |
| H5 | Extend existing public system/context/home E2E targets with actual composer/store/Git workflows. Establish genuine artifact production, same-byte reuse, exported context identity, present-record refusals and independent selected publication/privacy. Exercise actual engine GC and bounded import/export failures; do not assert source text or fabricate successful producer receipts. | H3, H4 | TC-01, TC-02, TC-03, TC-04, TC-08, TC-09 | Completed; mapped local and signed gates pass |
| H6 | Extend sanctioned niri container and existing signed home-transition VM workflows. Prove native apply/recovery with producer/store absent, cross-version state/image compatibility and signed A→B→A. Use actual observed interruption boundaries, relative includes and fresh reload events; capture visible native results through `kedra-lab`. | H4; H5 prerequisites pass; admitted runtime window | TC-04, TC-05, TC-06, TC-07, TC-08 | Completed; signed A/B/A and cross-version state pass37477670375 |
| H7 | Update TEXT-REVIEW, SYSTEM/ARCHITECTURE where relevant, STATUS/worklog and durable first-party skills only with observed results. If skills change, bump both plugin manifests. Record legacy absence semantics, source scope, unchanged public workflow, additive-format cost and every unqualified boundary. | H5, H6; standard source gates | TC-01–TC-09 | Completed; operational docs and exact source/signed evidence recorded |

H2 is enablement. H3 plus H4 must demonstrate actual producer and installed
consumer reuse before this slice is delivered. Run H1→H2→H3→H4 in order; prepare
cases alongside their owning changes, then serialize resource-intensive H5/H6
work. The owner requested implementation/completion; no production rollout or workstation application is authorized.

Do not widen `SystemDefinition.outputs` to accept source objects or add a new
`SystemContent` variant/manifest schema merely to finish H3. If the current
record-as-ordinary-file integration cannot satisfy the contract, revise the plan
and its format tradeoff first. No temporary manual prepare/receipt/store flags
should become a user-visible substitute for the composition pipeline.

Completion requires actual mapped case results on exact inputs, ordinary Cargo
format/Clippy/build and relevant existing public CLI/container/VM gates. No
unit/model/mock/doctests, source scanner or replacement runner is permitted.
Keep state and ordinary baseline paths backward-compatible so rollback needs no
migration. Record protected production execution separately; disposable signed
qualification never authorizes deployment to the workstation.
