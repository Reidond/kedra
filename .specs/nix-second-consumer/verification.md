# D4 verification plan and evidence

Status: **not-run** for all runtime and compile cases. Prepared 2026-10-02 during
D1 source freeze; the three new Rust files are not registered by the active crate.
Passing rustfmt and API source inspection do not change this status.

Scope and expected results derive from [requirements](requirements.md); runtime
design and risk bands are in [design](design.md). This is an external-process E2E
workflow using actual library consumers, native compiler, container runtime,
filesystem/store and profile operations. Unit/model/mock tests, repository text
assertions and a new runner are excluded by repository policy.

## Cases and ownership

| Case | Condition / technique | Owning level / method | Expected result and guard | Task owner | Result |
|---|---|---|---|---|---|
| TC-01 | External API reuse; equivalence partitions (Fieldkit, Observatory). | E2E / automated | Both distinct external Cargo workspaces compile and executables perform real lifecycle calls (AC-01). Compilation error fails; no fallback to Kedra CLI. | T1, T2, T5, T6 | not-run |
| TC-02 | Authorization and validation; decision table below. | E2E / automated | Exact policy/lookup diagnostics and absent new store for each rejection; valid project declaration later builds (AC-02). | T1, T3, T6 | not-run |
| TC-03 | Native calculation and reuse; state transition v1→repeat→v2 plus independent Observatory. | E2E / automated | Fieldkit totals 43 then 57, Observatory 20; two objects built/reused; changed source changes output ID (AC-03). Runtime library and data are actually consumed. | T2, T3, T6 | not-run |
| TC-04 | Producer absence; state transition producer→export→receiver→producer unavailable. | E2E / automated | Two-object/runtime-image closure excludes source/compiler; expected bundle hash agrees; receiver executes after source deletion and producer rename (AC-04). | T4, T6 | not-run |
| TC-05 | Generation selection; state transition v1→v2→v1. | E2E / automated | Real output 43→57→43 and two retained generations; other same-named profile unchanged (AC-05). | T4, T6 | not-run |
| TC-06 | Collection ownership; state transition pin→unpin→collect. | E2E / automated | Exact orphan removed; both profile generations still run; Observatory receipt/index/sentinel and total 20 unchanged (AC-06). | T4, T6 | not-run |
| TC-07 | Independent expectation; equivalence partition valid graph with unavailable daemon. | E2E / automated | Pure resolve emits authorized namespace/recipe and expected root equal to actual build root (AC-07). No producer-derived expected spec. | T1, T3, T6 | not-run |
| TC-08 | Operating contract; manual inspection and standard tools. | Manual / manual | Existing bounds, only owned source/resource changes, standard checks and exact-source record; no unobserved completion claim (NFR-01–NFR-03). | T5, T6, T7 | not-run |
| TC-09 | Future cache producer authority; decision table to be finalized by D5 owner. | E2E / accepted-unverified in D4 | Local resolved root spec and independently chosen namespace/package scope; cross-project signer/scope rejection before admission (AC-07; D5 design contract). | T8 / D5 owner | not-run; outside D4 |

TC-02 decision table runs for each independent consumer. Every rejection uses
valid surrounding metadata; otherwise a generic parse failure could mask a policy
failure. Existing source identities are explicit preconditions.

| Catalog selection | Policy variation | Expected outcome |
|---|---|---|
| Defined `report` | Exact own namespace/package/source/images | Allowed, pure resolution then native build. |
| Defined `report` | Other project's namespace/policy | Catalog policy denied; destination absent. |
| Defined `report` | Only package allowlist excludes `report` | Catalog policy denied; destination absent. |
| Defined `report` | Only builder allowlist contains runtime image | Catalog policy denied; destination absent. |
| Defined `report` | Only runtime allowlist contains builder image | Catalog policy denied; destination absent. |
| Defined `report` | Only source allowlist contains other project's source | Catalog policy denied; destination absent. |
| Undefined `missing` | Package `missing` is explicitly allowed | Unknown package diagnostic; destination absent. |

## Execution gate and command

Entry: D3 is committed/registered; D4 owns its branch; parent registers
`#[path = "reuse.rs"] mod reuse;` in the existing `e2e_engine.rs`; installed pinned
toolchain and offline dependencies are available; exact existing BUILDER/RUNTIME
images can be admitted on the native ARM daemon; no concurrent heavy job owns the
resource slot. The existing engine fixture provides command deadlines and cleanup.

Planned command, **not executed during preparation**:

```sh
cargo test -p sysroot --test e2e_engine --locked independent_projects_reuse_catalog_and_store_without_kedra_authority -- --ignored --test-threads 1 --nocapture
```

The command must report one actually executed passing case. Zero filtered cases
or ignored listings confer no qualification. The case itself launches standard
offline Cargo builds for both external workspaces and runs the resulting binaries.
Record real source revision, tool/image identities, durations, command result and
cleanup outcome. If failure retains generated fixtures, record their owned paths
and cause; preserve unrelated stores/images/resources.

After the focused case, parent runs the standard appropriate workspace formatting,
all-target Clippy, ordinary CLI E2E, release build and two existing release CLI
checks required by AGENTS. New GUI/kernel/image behavior is outside D4, so no new
boot result is inferred. Existing lower-layer required checks retain their own scope.

## Deliberate non-coverage

| Boundary | Risk from design | Decision |
|---|---|---|
| Authenticated cache and producer-key/scope tests | Medium / high | D5 owns TC-09; D4's bundle hash is content identity only. |
| Malicious store owner/cross-user isolation | Outside declared actor model | Owner-private Store is trusted; separate stores demonstrate management independence, not a multi-tenant sandbox. |
| New controller architecture or unavailable retained image | Medium / medium | Fail/report missing prerequisite; do not emulate or silently fetch/install. |
| Process-kill/crash matrices and lease cleanup | Existing engine/D5 ownership | D4 uses current process/transaction contract; do not duplicate D5 recovery tests. |
| Signed OS deployment, kernel boot and hardware | Outside D4 | Remain D1/D2 or existing installed-system qualification; no consumer pass qualifies them. |

## Preparation evidence and review

- `pass` — source inspection of the frozen D3 structs, exact source declarations,
  `Input::Node` dependency, all-category policy and `ResolvedPackage.recipe/plan`
  matches the prepared fixture. This is an interface assessment, not compilation.
- `pass` — rustfmt for the three owned Rust files.
- `not-run` — external Cargo compilation, focused E2E, workspace gates, D4 Git
  publication and all cache integration. No Docker/VM/resource operations occurred.
- Review correction: the earlier unknown-package attempt would have been rejected
  by policy first. The prepared case now allowlists the unknown name and requires
  the distinct lookup diagnostic. Generic refusal checks now require the catalog
  denial prefix. This prevents an unrelated failure from satisfying TC-02.
- Review correction: catalog namespace is absent from `ResolvedBuildSpec`; D5 must
  bind an independently selected cache scope explicitly, not infer it from a recipe.
- Traceability reviewed: AC-01–AC-07 and NFR-01–NFR-03 map to the rows above;
  TC-01–TC-09 map to T1–T8. All real execution remains pending.

## Owning-source checkpoint — 2026-10-03

D4 is adopted above D3 `808405e` from reconciled manifest
`6813bf1b3448b5c7dae1ec9652e1cded797196c56604ab1117d339b0acc9414b`.
The four-path delta preserves all twenty D3 implementation files and introduces
no engine/catalog API change. Rust 1.98.1 workspace fmt, all-target Clippy and
release build pass.

Two separate real external Cargo workspaces compile offline and execute their
public consumer commands directly against the generic libraries. Each resolves
its own `report` with an explicitly unavailable Docker endpoint, rejects all five
policy categories before destination-store creation, rejects the other actual
project's policy, and distinguishes an allowed-but-undefined package.
These are actual process/CLI results, not source or model assertions. Evidence:
`target/nix-delivery/publication/d4-consumers/manual-results.json`, generated
workspaces/lockfiles and their emitted declaration/resolution documents. The
synthetic image/source identities in this pure-resolution exercise are not
admitted images or application build evidence.

Fieldkit executable SHA256 is
`71e533ab71e5b5a94493cd350fc44688517030e003742e7e9fccac76f3c6a91a`;
Observatory is `a1fd53b7150a5450e1d506855bf4b9ae7fad57de90ca78267afb6af647483b32`.
TC-01 passes and TC-02's exercised pure-policy subset passes. Native compilation
of the application packages, actual totals, transfer after producer removal,
profile rollback and isolated GC remain not-run. The full existing ignored
Docker E2E still owns those remaining gates and is not claimed passed.

Static review follows the generated consumer's direct Catalog/Store calls,
fixture registration and recipe definitions. No new production library symbol or
release path is changed; dynamic/runtime callers are not established by this
source review. D5 cache authority remains separate.
