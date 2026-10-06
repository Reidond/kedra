# Verification management

Status: **qualification in progress**. Case groups remain not-run or partial until
every stated outcome has evidence. Current executable subsets and failures are
recorded in worklog WL-20261006-PKGDSL-03/04/05; source checks do not establish
installed migration or broad-case completion.

## Entry conditions and levels

Before implementation: owner authorization was supplied on2026-10-06; freeze the delivered baseline
and schema/language/resource contracts, admit source/pins/images and obtain an
owned resource window. Use exact per-invocation binaries, bounded fixtures and
external sentinels. No owner credentials, production keys or workstation install.

Use existing public CLI E2E targets, the sanctioned container harness and manual
real-process/release workflows. Parser/formatter acceptance runs through the
actual CLI on generated fixture input and observes behavior; no source-string
assertions against repository files. No unit/model/mock/doctest or replacement
runner. Cargo/Ruff/compiler checks remain appropriate supporting gates, not proof
of installed behavior. Do not compile arbitrary Rust while parsing `.kedra`.

## Traceability and methods

| Requirement | Cases | Status |
|---|---|---|
| AC-01 | TC-01 | not-run |
| AC-02 | TC-05 | not-run |
| AC-03 | TC-02 | not-run |
| AC-04 | TC-03, TC-14 | not-run |
| AC-05 | TC-04, TC-05, TC-11 | not-run |
| AC-06 | TC-06, TC-10 | not-run |
| AC-07 | TC-02, TC-03, TC-05 | not-run |
| AC-08 | TC-07 | not-run |
| AC-09 | TC-08, TC-11 | not-run |
| AC-10 | TC-09, TC-10 | not-run |
| AC-11 | TC-12 | not-run |
| AC-12 | TC-12, TC-13, TC-14 | not-run |
| AC-13 | TC-13 | not-run |
| NFR-01 | TC-01, TC-10, TC-13; automated public workflows | not-run |
| NFR-02 | TC-13 automated boundaries; TC-09 manual real timeout/lifecycle | not-run |
| NFR-03 | Preservation readback throughout TC-01 through TC-14 | not-run |
| NFR-04 | Standard Cargo gates and manual scope review | not-run |

Each case has one owning level and tasks in tasks.md. Multiple contributors do
not create duplicate authoritative assertions. VM boot is required only where the
migrated installed OS boundary needs it, not to prove a parser diagnostic.

## Execution order and exit

1. Standard pinned Cargo formatting/Clippy/public CLI E2E/release compilation and
   affected uv/Ruff/release interoperability/material workflows. Missing tools are
   not automatically installed.
2. Execute frontend/selection/compatibility/no-side-effect/formatter cases against
   real CLI binaries; compiler success cannot substitute for syntax semantics.
3. Execute real source/resource builds, runtime-library transfer and independent
   frontend/API workflows. Assert actual nonempty package output and script bytes.
4. Serialize Fedora/candidate/installed-target checks with the runtime owner;
   retain exact full RPM observations and all failed attempts.
5. Run bounded interruption/collision/source-preparation failure cases on generated
   private inputs. A blocked or missed case stays blocked/not-run.
6. Review all evidence, reconcile documentation, review implementation PRs, then
   separately authorize any merge or production action.

Exit requires all applicable AC/NFR cases to pass, exact source/binary and cleanup
readbacks, no material unresolved defects, and explicit remaining platform/trust
limits. The source-package pilots must use inline build/resource definitions;
passing by retaining mandatory sidecar scripts would miss AC-11. Publishing this
spec cannot qualify language execution or the current delivery's outstanding work.

## Deliberate non-coverage

| Boundary | Design risk band | Reason / owner |
|---|---|---|
| Permanent RPM availability or new mirror | Fedora drift: high/high | Requires retained artifacts or explicit new resolution; no retention service is proposed. |
| General functions, loops, recursive eval or plugin/FFI execution | Language growth: medium/medium | Outside v1; unsupported syntax refuses. The frontend owner must not smuggle it into a built-in. |
| IDE language server | Language growth: medium/medium | CLI diagnostics and formatting are required; editor protocol support can follow actual language use. |
| General home management, database rollback and power-loss guarantees | Trust/recovery expansion: medium/high | Existing home/deployment and separate qualification own these boundaries. |
| Cross-compilation/native x86 source-engine qualification | Delivery scope: high/medium | Future platform work; desktop Fedora migration still requires TC-11. |
| Third-party repositories or new signing keys | Trust/recovery expansion: medium/high | Preserve current independent allowlists and signer authority. |

Bounds in NFR-02 are proposed policy limits to qualify. No speed, sandbox quality,
fully reproducible OS build or production eligibility is inferred from naming them.
