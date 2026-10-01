# Implementation tasks and file ownership

State: first engine implemented and verified with declared limits, owner-authorized 2026-10-01. The requirements/design and
CLI contract passed Astra pre-implementation review after five concrete mechanism
corrections; review is not behavior evidence. Test planning ran independently
against the same requirements/design, then its cases were reconciled here.

| Task | Mechanism / recovered capability | Owned files | Verification | State |
|---|---|---|---|---|
| T01 | Validated typed graph and deterministic resolution, separate from IO | New engine model/lib; parent workspace registration | TC-01/02/06/18/20/21 | implemented; observed cases pass |
| T02 | Owner-private immutable source/tree/image admission with content validation | New engine tree/store/image modules | TC-03/15/16/18/19/21 | implemented; full adverse matrix not-run |
| T03 | Bounded native Docker execution, final output path, independent rebuild and crash cleanup | New engine executor/build modules | TC-04/05/08/13/14/17/21/22 | implemented; observed cases pass |
| T04 | Closed runtime transfer, manifest allowlist, expected digest and dependency-first admission | New engine bundle/closure/store modules | TC-07/08/09/15/18/19/22 | implemented; observed cases pass |
| T05 | Atomic retained profiles, declared runtime develop/run, protected roots and explicit GC | New engine profile/root/GC modules | TC-10/11/12/13/16/22 | implemented; power-loss/ENOSPC not-run |
| T06 | Public ordinary-user sysroot adapters with preserved existing commands/status | Parent sysroot engine.rs/main.rs/Cargo.toml, status inventory/docs | TC-01–20 | implemented; observed cases pass |
| T07 | Generated real C software and external Rust authoring consumer through actual CLI | Astra-owned sysroot/tests/e2e_engine.rs; parent target registration | TC-01–20 (actual coverage recorded after run) | ten cases observed across full and targeted runs |
| T08 | Standard compiler/lint/build gates, critical failure review and truthful operational documentation | Parent workspace gates/worklog/STATUS/README/skills as needed | TC-21/22 plus ordinary existing checks | passed with explicit not-run limits |

The core owner writes only `usr/src/kedra/crates/sysroot-engine/`; parent writes
existing CLI/workspace/spec/status files. The E2E owner writes only its new test
file. No parallel global suite or shared index/commit operation runs. Parent
performs final gates once writers finish; a check over an evolving tree is not
final evidence. No worker writes shared AI tracking, worklog or project status.

Every TC-01–22 has an owning task above. Cases can share a public lifecycle while
retaining distinct expected outcomes. Manual fault cases not executed must stay
not-run. No fabricated requirement IDs or unit/source-scan test levels are added.

## Review corrections applied before implementation

Uniform runtime foundation; forced rebuild of all needed nodes with comparison;
bounded manifest-derived extraction and metadata symlinks; dependency-first import
with durable roots/recovery; explicit profile runtime/scratch definition for
`develop`. Concrete policy limits are in design.md; they are chosen first-version
defaults, not unmeasured performance claims.

## Authorization and remaining phases

The owner's "ok let's implement this" authorizes work within the researched
direction. Spec choices concretize its first useful slice without repeated approval
requests. Signed remote cache, Nix frontend compatibility, broader architectures
and full OS composition remain declared subsequent phases; no current engine
gate is inferred from existing bootc qualification. This spec is retained and
updated by status, never deleted as cleanup.

## Outcome and remaining work

[Verification](verification.md) is the case-level evidence authority. Implemented does not mean every TC variant passed: TC16/19 full matrices, TC22 power-loss/ENOSPC and cold independent-daemon load are not-run. Shared bundle memoization is source-reviewed; the shared-source DAG case proves store traversal/GC only. The first private engine slice is complete; general declarative OS composition, signed caches, further architectures and Nix compatibility remain separate phases.
