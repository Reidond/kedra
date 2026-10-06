# Implementation tasks

Status: **implementation authorized on 2026-10-06; in progress**. The language
contract now defines concrete fields, lock/policy/discriminator schemas against
delivered main b60bcb0. One owner serializes Git and heavy work.
No workers or workloads are dispatched by this spec.

| Task | Files / mechanism and author-visible result | Depends on | Test requirements | State |
|---|---|---|---|---|
| T1 — Freeze language and policy contracts | Reinspect delivered engine/catalog/source/release APIs; freeze language v1, intent/resolution envelopes, independent target policy, resource decoding and format discriminator. Scope the approved exception in architecture/AGENTS guidance. | Delivery baseline reviewed | TC-01, TC-02, TC-08, TC-13 | contracts reviewed; qualification pending |
| T2 — Parser, types, formatter and owner definitions | `sysroot-catalog/intent.rs`, `language/*.rs`; new `image/packages/*.kedra`, sets and lockfile. Authors edit inline recipes without compiling Rust; diagnostic locations and formatter preserve content. | T1 | TC-01, TC-02, TC-12, TC-13 | in-progress |
| T3 — Input admission and resource lowering | Catalog CLI/root loader and existing engine APIs; materialize canonical source/resource trees, source-preparation nodes and script-file execution with current argument bounds. Keep original inputs immutable and operations bounded. | T2 | TC-03, TC-06, TC-09, TC-12, TC-14 | in-progress |
| T4 — Fedora role resolution | `source.rs`, `system.rs`, `assemble.sh`, image/catalog compiler scripts and refresh; collect build/runtime requirements before freezing exact images, preserve required base policy and complete RPM material comparison. | T1, T2 | TC-02, TC-04, TC-05 | in-progress |
| T5 — Contribution and release material | `catalog.rs`, template lowering, `material.py`, `compose.py`, existing public workflow inputs. Derive selected packages/config from independently verified frontend/resources instead of central jq/SQLite names. | T3, T4 | TC-05, TC-07, TC-12 | in-progress |
| T6 — Inline-first cutover | Versioned entry descriptor; replace current list authority and embedded Rust owner recipes only after parity. Migrate jq/SQLite and a real inline C package with in-file build logic; retain old-source readers. | T4, T5 | TC-08, TC-11, TC-12 | implemented; runtime qualification in progress |
| T7 — Consumer and qualification | Extend existing public CLI E2E/container/manual workflows. Exercise `.kedra` and direct Rust API consumers against identical intent; real builds, resource identity/refusal, dependencies and installed behavior. | T3–T6 | TC-03 through TC-14 (runtime/evidence) | in-progress |
| T8 — Documentation and review | Update package/engine/system/architecture/status guides, AGENTS and applicable repository skills with the narrow approved language decision; bump both plugin manifests if skills change. Record actual evidence and review implementation PRs. | T7 | TC-01 through TC-14 (evidence readback) | in-progress |

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

## Execution notes (2026-10-06)

The owner explicitly authorizes implementation. Main is reconciled at b60bcb0;
frontend checkpoint8dd43b4 is published and passes both architecture source CI.
Actual work and failures are in worklog WL-20261006-PKGDSL-03/04/05.
Generated public frontend/format/refusal/import and committed-source compatibility
workflows pass their recorded subsets. The initial C transport proof precedes
the actual Fedora guard and cannot count as Fedora resolution. No task or broad
case group is declared runtime-complete from that partial evidence.

Implementation choices: AST/intent types remain in language/check.rs rather than
a redundant intent.rs; both `.kedra` and direct Rust consumers share those types
and checked lowering. The packet emission option writes one atomically published
tar under the existing managed snapshot lifecycle; existing system recover handles
its closed Catalog purpose. Resource tree identity uses the existing engine tree
serialization, component ordering and an explicit8MiB metadata-path budget.
Compiler roles collect requests before binding images, and actual builds observe
Fedora44 RPMs before resource admission. Current public names are catalog check,
fmt, pins, plan/resolve/build with explicit language inputs, and plan --output.
The explicit descriptor now selects .kedra; all four authoritative lists are
removed. Legacy schema1 recipes remain immutable JSON. Local native Fedora
pilots and installed catalog checks pass; final fresh both-target material and CI
readback remain separate qualification work. See WL-20261006-PKGDSL-06/07/08.
