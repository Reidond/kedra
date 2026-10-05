# Future implementation tasks

Status: **all not-started**. This PR creates the plan only. Implementation needs
separate authorization after review and reconciliation with the delivered engine
stack. One owner serializes Git and heavyweight container/VM work. No worker or
background task is dispatched by this spec.

| Task | Files / mechanism and user-visible result | Depends on | Test requirements | State |
|---|---|---|---|---|
| T1 — Freeze contracts | Reinspect delivered catalog, source, engine and release APIs; settle v1 intent/resolution schemas, independent target package policy, format discriminator and diagnostics. Make unknown/mixed data refuse explicitly. | Delivery baseline reviewed | TC-01, TC-02, TC-08, TC-10 | not-started |
| T2 — Author API and owner definitions | Extend `sysroot-catalog/lib.rs`, add `author.rs`; add flat `kedra-packages` binary/library, per-package modules, scripts, sets and pins; root Cargo manifests/lockfile. Authors edit package files without changing engine algorithms. | T1 | TC-01, TC-02, TC-03 | not-started |
| T3 — Bounded author execution | Explicit CLI author entry and existing sanctioned container integration; compile/evaluate with admitted dependencies, bounded IO/resources, separate policy and safe output publication. Data-only operations remain data-only. | T2 | TC-06, TC-09, TC-10 | not-started |
| T4 — Fedora resolution and lowering | `source.rs`, `system.rs`, `assemble.sh`, image Containerfile, catalog compiler scripts and `refresh.py`; resolve role-specific requirements before binding exact engine graphs. Preserve actual RPM comparison and target restrictions. | T1, T2 | TC-02, TC-04, TC-05 | not-started |
| T5 — General contribution and release material | `catalog.rs`, templates, `material.py`, `compose.py`, public build/check workflow inputs. Derive selected packages/references from verified material, removing the special jq/SQLite whitelist without weakening signer trust. | T3, T4 | TC-05, TC-07 | not-started |
| T6 — Deliberate source cutover | Versioned source-input descriptor; remove current shared/target lists and embedded owner recipes as authority only after parity. Retain old-source readers. No implicit new-to-old fallback. | T4, T5 | TC-08, TC-11 | not-started |
| T7 — Consumer and qualification | Extend existing public CLI E2E and sanctioned container/manual workflows. Build real programs, test dependency transfer, run candidate checks, preserve failed attempts and exact provenance. | T3–T6 | TC-03, TC-05, TC-06, TC-07, TC-09, TC-10, TC-11 | not-started |
| T8 — Documentation and publication review | Update PACKAGES/ENGINE/SYSTEM/STATUS, relevant first-party skills and both plugin manifests if needed, plus worklog. Review separate implementation PRs; no automatic merge/release. | T7 | TC-01 through TC-11 (evidence readback) | not-started |

T1→T2→T3/T4→T5→T6→T7→T8 is dependency order, not a promise of parallel execution
or a duration estimate. Existing release/VM/runtime owners control exclusive
resources. The current delivery's fault tests and installer repair remain owned
by their existing work; this initiative cannot substitute its own prerequisites
for those results.

Suggested later review slices are author model/evaluator, Fedora integration,
contribution/release migration, then qualified cutover. Exact branch topology is
chosen at implementation start against the then-current base. The current PR
contains no such code and has no merge-train dependency on drafts33–37.

Every affected file family in design section7 has an owner above. Runtime evidence
belongs to T7, even when the same existing E2E workflow exercises several cases.
Any added requirement reopens this task-to-case reconciliation before implementation.
