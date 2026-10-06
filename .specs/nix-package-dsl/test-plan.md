# Verification management

Status: **qualification in progress**. Case groups remain not-run or partial until
every stated outcome has evidence. Current executable subsets and failures are
recorded in worklog WL-20261006-PKGDSL-03 through09; source checks do not establish
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
| AC-01 | TC-01 | pass — CLI edit/format/plan and real build |
| AC-02 | TC-05 | pass — installed programs/library and Fedora Firefox |
| AC-03 | TC-02 | pass — selection/replacement/refusal workflows |
| AC-04 | TC-03, TC-14 | pass — admitted archive/tree and prepared builds; extended unsafe archive partitions not-run |
| AC-05 | TC-04, TC-05, TC-11 | pass — both native full-RPM parity, role observation and no-change material repeat |
| AC-06 | TC-06, TC-10 | pass — pure CLI/no-effects and explicit policy refusal |
| AC-07 | TC-02, TC-03, TC-05 | pass — separate library/runtime closure and compiler-only RPM absence |
| AC-08 | TC-07 | pass — third C package/contribution and pins/author/intent/RPM tamper refusals |
| AC-09 | TC-08, TC-11 | pass — old/new committed readers and native target parity |
| AC-10 | TC-09, TC-10 | pass — independent API/output reuse and actual emission race/recovery |
| AC-11 | TC-12 | pass — inline jq/SQLite/C, generated header, environment and patch builds |
| AC-12 | TC-12, TC-13, TC-14 | pass — inline/external graph equality, long script and exact preparation; extended mode/link partitions not-run |
| AC-13 | TC-13 | pass — public syntax/version/import/format and boundary workflows |
| NFR-01 | TC-01, TC-10, TC-13; automated public workflows | pass — repeat intent, graph/resources, material and rebuilt outputs |
| NFR-02 | TC-13 automated boundaries; TC-09 manual real timeout/lifecycle | pass — byte/module/resource/graph and actual process deadline/memory refusal; exact nesting/diagnostic ceiling partitions partial |
| NFR-03 | Preservation readback throughout TC-01 through TC-14 | pass — recorded winner/accepted-result preservation and owned recovery |
| NFR-04 | Standard Cargo gates and manual scope review | pass — pinned standard tooling and E2E/manual-only scope |

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

## Observed evidence and remaining scope (2026-10-06)

- Both architectures pass Check37511817585 and37511811654 at2d44c7e.
- Native migration in container run37511811578 passes identical old/new requests
  and complete seven-column RPM rows for desktop and ARM from the same exact base.
- The same run records desktop11/12 and ARM12/13 installed cases passing; only
  native::home_review_cycle refuses the already unqualified Noctalia5.2.1.
  Reports are uninterrupted, with no cleanup failures.
- Local installed native::catalog passes on exact imageb44e6c3b55a3: all aliases/C,
  jq/SQLite/separate library/PATH, query pipeline and persistent user service.
- Real independently compiled API plus C build/rebuild/transfer/preparation and
  exact patch over a fuzz-offset.c filename pass; an actually shifted GNU patch
  refuses and the earlier runnable output survives (final-inline-native.log).
- Full source/binary/pins material generation repeats unchanged, and contribution
  verifies1435 actual foundation RPM rows plus complete compiler-role material.
  Agent archives there are public byte-identity fixtures, not an agent delivery pass.
- Real70,099-byte script builds/runs; an admitted external C file and inline C
  produce equal graphs. Compiler autoconf exists while runtime autoconf is absent;
  Firefox remains installed.
- Actual two-emitter race, observed166,912-byte killed writer, recognized snapshot
  recovery and byte-identical retry preserve the winner. Actual60-second stopped
  child and memory-excess refusals reap/settle their owned process and emit no plan.

Remaining broader partitions are explicit: exhaustive archive/link/mode/diagnostic
limit and repository-signature outage fault injection are not new passes here.
Existing source-admission/release refusal workflows remain applicable in their
recorded scopes. Full protected release, native x86 source execution and existing
installer/native fault gates remain separate. The new legacy home fixture is
prepared through the real public source CLI with equal A/B requests; its corrected
hosted VM run is pending. Worklog09 records final-source checks and actual failures.

Final code6f1269d passes both architecture Check37517872017/37517864795,
workspace/focused E2E, Clippy/Ruff, release compilation and public release
interoperability/material checks. Corrected hosted home run37517864796 is still
in progress; the local source-preparation pass is not called a VM boot pass.
