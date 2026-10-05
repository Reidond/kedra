# Verification management

Status: **planned; nothing in this document has run**. The documentation PR is
validated by manual source/design review and standard Git diff checks only.
The cases describe future implementation acceptance.

## Entry conditions and levels

Implementation is authorized separately; the delivery baseline and schemas are
frozen; actual native builder/runtime images and source pins are available;
required resources have an owner and measured capacity. Use exact per-invocation
binaries and source identities, bounded private fixtures and preserved external
sentinels. No production keys, owner credentials or workstation install.

Use the existing public CLI E2E targets, sanctioned container harness and manual
actual-process/release workflows. No new runner, unit/model/API-isolation tests,
mock services, doctests, source-string assertions or manifest/layout scanners.
This follows the repository's owner policy rather than a generic testing pyramid.
Compiler/linter success alone cannot establish package execution or isolation.

## Traceability and methods

| Requirement | Cases | Verification status |
|---|---|---|
| AC-01 | TC-01, TC-03 | not-run |
| AC-02 | TC-05 | not-run |
| AC-03 | TC-02 | not-run |
| AC-04 | TC-03 | not-run |
| AC-05 | TC-04, TC-05, TC-11 | not-run |
| AC-06 | TC-06, TC-10 | not-run |
| AC-07 | TC-02, TC-03, TC-05 | not-run |
| AC-08 | TC-07 | not-run |
| AC-09 | TC-08, TC-11 | not-run |
| AC-10 | TC-09, TC-10 | not-run |
| NFR-01 | TC-01, TC-10; automated public workflows | not-run |
| NFR-02 | TC-06, TC-09; manual limits/process evidence | not-run |
| NFR-03 | Preservation readbacks in TC-01 through TC-11 | not-run |
| NFR-04 | Standard Cargo gates and scope review; no prohibited test level | not-run |

Every case has one owning level in test-cases.md and at least one task in tasks.md.
Cases with several task contributors still have one authoritative result. Unit,
mock and repository-text levels are excluded by owner policy; kernel VM tests
are used only where installed boot behavior requires them, not for pure author
serialization. No additional VM is required merely to test a Rust diagnostic.

## Execution sequence and exit

1. Run pinned Cargo formatting, Clippy, appropriate public CLI E2E and release
   build; run existing uv/Ruff/release interoperability/material checks where
   affected. Do not install missing tools automatically.
2. Execute author/selection/source/consumer flows TC-01/02/03/08/10, using real
   compiled author programs and actual outputs.
3. Serialize TC-04/05/07/11 heavyweight resolver/candidate workflows with the
   current runtime owner; record actual RPM material and program execution.
4. Execute isolated bounded TC-06/09 under fresh resource/ownership checks. A
   denied execution or missed interruption point is blocked/not-run, never passed
   by source inspection or a substitute case.
5. Re-read all results and retained failure records, update operational docs and
   status, and review implementation PRs before any separate merge/publication.

Exit requires every applicable AC/NFR above to pass, exact owned cleanup and
sentinel preservation, no unresolved material defect, and explicit unrun gates.
Any platform/environment prerequisite that is absent remains blocked. Exact-main
production publication is separate and cannot be inferred from branch fixtures.

## Deliberate non-coverage

| Boundary | Risk band from design | Why excluded / who owns it |
|---|---|---|
| Permanent RPM availability or offline mirror | Fedora drift: high/high | No retention service is proposed; replay requires retained artifacts or explicit re-resolution. Release operator owns availability. |
| Universal purity of arbitrary Rust | Author execution: medium/high | Isolation and repeated evaluation provide scoped evidence, not a mathematical purity claim; author/evaluator owner preserves this limit. |
| General home activation and rollback of user data | Scope expansion: high/medium | Existing home subsystem and separate home-artifact plan own it; DSL only describes package inputs. |
| Cross-compiling source packages or native x86 engine qualification | Scope expansion: high/medium | A future platform workstream must qualify it; current desktop Fedora migration still gets TC-11. |
| New third-party repository/key policy, cluster atomicity, power loss | Trust expansion: medium/high | Separate trust/recovery projects; preserve existing policies and do not claim these outcomes. |

Review note: thresholds in NFR-02 are proposed operational limits to qualify.
No elapsed-time benchmark or success claim exists merely because a limit is named.
