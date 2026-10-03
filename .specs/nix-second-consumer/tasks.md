# D4 tasks

Status: preparation only. Source follows the agreed D3 API; no compiler/runtime
result has been observed. Parent owns integration, gates, shared metadata and PR.

| Task | Dependencies | Work | Test requirements | State |
|---|---|---|---|---|
| T1 | D3 API source available | Match typed declarations, independent effective policy and resolved-plan access. | TC-01, TC-02, TC-07 | Prepared; source inspection only. |
| T2 | T1 | Generate two isolated external Cargo consumers with shared lifecycle adapter and separate project definitions/inputs. | TC-01, TC-03 | Source prepared; compilation not-run. |
| T3 | T2 | Add native library/data/report, repeat reuse, v1/v2 and policy refusal workflows. | TC-02, TC-03, TC-07 | Source prepared; execution not-run. |
| T4 | T3 | Add producer-absent transfer, receiver profile transitions and isolated GC assertions. | TC-04, TC-05, TC-06 | Source prepared; execution not-run. |
| T5 | D3 committed; T1–T4 reviewed | Parent creates D4 branch above D3, registers child module, checks current inherited helpers/API, then formats/lints/builds. | TC-01, TC-08 | Pending owning layer. |
| T6 | T5; serialized resource slot | Parent pins actual source/executables and runs explicit ignored external-consumer E2E with retained ARM images. Record every case result and owned cleanup. | TC-01–TC-08 | Not-run. |
| T7 | T6 | Parent completes standard workspace/release checks and final review; updates shared docs/status/worklog, commits and submits separate D4 draft via existing stack. | TC-08 | Not-run; no publication claim. |
| T8 | D5 owner scope | D5 maps independently resolved root spec and explicit namespace/package scope into its independent cache policy; qualify generated-key cross-project refusal. | TC-09 | Future D5 integration; not D4 execution. |

Next preparation action: parent reviews these files without wiring them into D1.
Next implementation action: T5 only after D3 is committed and the resource/source
freeze allows D4 work. No task requires editing Kedranix or installing tooling.

Preparation assessment (2026-10-02): exact catalog field/type matching passes
ordinary source inspection. Corrected the refusal fixture to distinguish unknown
package lookup from package-policy denial. Added pure resolve output to carry
the independently computed plan, useful for D5 without importing its unfinished
cache module. Formatting passes for the three Rust files; no behavioral claim.

The requested four-document shape is used with the complete case table and
planned commands in `verification.md`; no parallel duplicate test-plan document.
Repository restrictions take precedence over generic skill suggestions for unit
tests, repository scanners, additional agent work, speculative tooling or spec
deletion. Existing user authorization covers this preparation scope.
