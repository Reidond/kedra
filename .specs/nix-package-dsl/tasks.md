# Future implementation tasks

Status: **implementation authorized on 2026-10-06; in progress**. The language
contract now defines concrete fields, lock/policy/discriminator schemas against
delivered mainb60bcb0. One owner serializes Git and heavy work.
No workers or workloads are dispatched by this spec.

| Task | Files / mechanism and author-visible result | Depends on | Test requirements | State |
|---|---|---|---|---|
| T1 — Freeze language and policy contracts | Reinspect delivered engine/catalog/source/release APIs; freeze language v1, intent/resolution envelopes, independent target policy, resource decoding and format discriminator. Scope the approved exception in architecture/AGENTS guidance. | Delivery baseline reviewed | TC-01, TC-02, TC-08, TC-13 | not-started |
| T2 — Parser, types, formatter and owner definitions | `sysroot-catalog/intent.rs`, `language/*.rs`; new `image/packages/*.kedra`, sets and lockfile. Authors edit inline recipes without compiling Rust; diagnostic locations and formatter preserve content. | T1 | TC-01, TC-02, TC-12, TC-13 | not-started |
| T3 — Input admission and resource lowering | Catalog CLI/root loader and existing engine APIs; materialize canonical source/resource trees, source-preparation nodes and script-file execution with current argument bounds. Keep original inputs immutable and operations bounded. | T2 | TC-03, TC-06, TC-09, TC-12, TC-14 | not-started |
| T4 — Fedora role resolution | `source.rs`, `system.rs`, `assemble.sh`, image/catalog compiler scripts and refresh; collect build/runtime requirements before freezing exact images, preserve required base policy and complete RPM material comparison. | T1, T2 | TC-02, TC-04, TC-05 | not-started |
| T5 — Contribution and release material | `catalog.rs`, template lowering, `material.py`, `compose.py`, existing public workflow inputs. Derive selected packages/config from independently verified frontend/resources instead of central jq/SQLite names. | T3, T4 | TC-05, TC-07, TC-12 | not-started |
| T6 — Inline-first cutover | Versioned entry descriptor; replace current list authority and embedded Rust owner recipes only after parity. Migrate jq/SQLite and a real inline C package with in-file build logic; retain old-source readers. | T4, T5 | TC-08, TC-11, TC-12 | not-started |
| T7 — Consumer and qualification | Extend existing public CLI E2E/container/manual workflows. Exercise `.kedra` and direct Rust API consumers against identical intent; real builds, resource identity/refusal, dependencies and installed behavior. | T3–T6 | TC-03 through TC-14 (runtime/evidence) | not-started |
| T8 — Documentation and review | Update package/engine/system/architecture/status guides, AGENTS and applicable repository skills with the narrow approved language decision; bump both plugin manifests if skills change. Record actual evidence and review implementation PRs. | T7 | TC-01 through TC-14 (evidence readback) | not-started |

T1→T2→T3/T4→T5→T6→T7→T8 describes dependencies, not a promised concurrency or
completion estimate. No new Cargo author crate or engine rewrite is planned.
A parser dependency, if justified later, requires the ordinary small-dependency
review and existing workspace lockfile update, not per-package Cargo files.

Suggested implementation review slices: language/intent/tooling, inline resources
and actual package builds, Fedora/contribution integration, then qualified migration.
Choose branch topology against the eventual baseline; this spec PR remains separate
from drafts33–37 and cannot waive their unfinished tests.

Every affected file family in design section7 has an owner above. One case has one
owning level even when several tasks contribute. Any syntax or inline-content
amendment reopens task/case reconciliation, especially TC-12/13/14.
