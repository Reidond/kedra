# Requirements

Status: proposed, not implemented. Scope is package authoring and the adapters
needed to consume it; specification approval is separate from implementation.

## Problem and actors

Today a built-in package change edits compiled Rust plus shell strings; Fedora
intent is maintained separately in line lists. Adding a catalog package also
encounters a fixed release inventory. The author should be able to change a
recipe without editing engine algorithms or a central package-name whitelist.

Actors are the package author, the owner selecting policy/targets, the isolated
author/resolver/build processes, the release verifier and a second independent
consumer. Author intent and consumer authorization are independent inputs.

## Scope

Include: ergonomic typed Rust authoring, one module per source package, adjacent
scripts/patches, named package sets, shared/target composition, Fedora system and
compiler package requirements, source pins, dependency roles, bounded evaluation,
canonical data output, general catalog contribution, release provenance, old
source compatibility and E2E/manual migration evidence.

Exclude: Nix syntax/nixpkgs compatibility, a new text configuration language,
Fedora source rebuilding, arbitrary third-party repositories, automatic live DNF
operations, general home management, new OS deployment/signing authority,
cross-compilation, automatic upstream version discovery, a network cache service,
and a framework for arbitrary shell hooks. No production or workstation action
is authorized by this spec. Existing x86 Fedora image behavior must be preserved;
source-build execution remains within the qualified native ARM backend.

## Acceptance scenarios

### AC-01 — Edit a package without editing the engine

Given a generated owner-approved author crate and existing engine/catalog binaries,
when an author changes jq's package module or adjacent build script and emits a
plan, then the changed recipe is visible in the plan and can be built using the
unchanged engine binary. Changing definitions recompiles the author program only.
Invalid Rust fails with native file/line diagnostics and produces no accepted plan.

### AC-02 — Select Fedora and source packages together

Given one package set selecting Firefox through Fedora and jq/SQLite through source
recipes, when its resolved candidate is built, then the actual Fedora package is
present and the selected source-built programs execute from their recorded store
outputs, including SQLite's separate library. Missing or unauthorized inputs fail
before a candidate becomes accepted. No implicit replacement of `/usr/bin` occurs.

### AC-03 — Compose targets without order-dependent overrides

Given shared and target-specific package sets, when a declared target is selected,
then includes, removals and source exports are explicit, deterministic and carry
origin information. Conflicting include/remove requests, two different definitions
of one package identity, unknown targets and duplicate export names fail with both
origins. Identical repeated declarations may deduplicate. An explicit replacement
must name the declaration it replaces; missing matches fail. Protected base package
removals and dependency removals that violate the final required set fail.

### AC-04 — Pin source bytes and all recipe inputs

Given reviewed archive pins, when sources are acquired and admitted, then both the
archive hash and normalized source-tree identity are checked before use. Scripts,
patches, declared environment, compiler/runtime binding and ordered dependencies
affect the resulting recipe identity. Wrong bytes, unsafe archive entries, absent
files, ambiguous normalization or changed inputs refuse without replacing a valid
existing result. Local directory input is explicit and excluded from production
release by default; untracked personal files are never an implicit source.

### AC-05 — Resolve Fedora intent into observed material

Given a frozen source plan, exact reviewed Fedora base and approved repositories,
when the resolver performs the existing upgrade/install/remove policy in an
isolated image, then the complete installed RPM material is recorded for each
system/compiler role and compared with its candidate. Solver choices and implicit
dependencies are recorded, not guessed from names. Unavailable repositories,
signature failure, missing requested names or material drift fail closed. An
unchanged deterministic resolved-input set retains the existing no-change result.

### AC-06 — Preserve authorization and execution boundaries

Given an author plan and independent consumer policy, when a plan is evaluated or
realized, then source, target, package, repository and builder/runtime permissions
are checked at their applicable boundaries. The author cannot grant its own trust.
Author compilation/evaluation has no host home, credentials, production keys or
daemon socket. Package builders retain existing nonroot/offline restrictions;
DNF runs only in the separately owned disposable foundation/compiler environment.
Denied access or malformed output produces no accepted plan or deployment action.

### AC-07 — Keep typed dependency and exposure semantics

Given source packages with build-only and runtime dependencies, when a selected
package is exported/transferred, then its runtime dependencies are complete and
build-only inputs remain outside the runtime closure unless actually referenced.
Unbound image roles, cycles, wrong-platform references and missing outputs refuse.
A Fedora prerequisite means an assertion about the selected foundation, not an
engine output node. Selecting both Fedora jq and source jq is allowed only with
unambiguous explicit exposure; they never silently overwrite each other's files.

### AC-08 — Bind authoring to candidate and signed release inputs

Given a verified authored plan with a newly selected package, when contribution
and preflight run, then the selected output set and typed template references are
derived from that plan rather than a hard-coded jq/SQLite list. Frozen source,
author executable/input inventory, plan, pins, policy, resolver/tool identity and
actual outputs remain bound to the candidate and release material. Tampering,
stale author bytes or unresolved references refuse before signing eligibility.
Production signer isolation and its trust schema/authority remain unchanged.

### AC-09 — Migrate without two competing sources of truth

Given retained legacy source and a new DSL source revision, when each is resolved,
then a positive format discriminator selects exactly one path. Old revisions keep
their old semantics. A new revision containing both authoritative DSL and legacy
package lists, or an unknown format, refuses. Package-intent parity and installed
behavior for desktop and qemu-arm64 must pass before retiring current lists. No
legacy fallback may hide a new-plan failure. OS rollback retains readable old
source provenance and never modifies live home/data as a migration side effect.

### AC-10 — Independent consumption and a bounded operator workflow

Given a second Rust author crate with its own namespace and policy, when it emits,
plans, builds and runs a real program through the generic APIs, then it needs no
Kedra target table or release keys. The existing CLI can consume the prepared data
without executing author code implicitly during plan/verify/sign operations.
An interrupted or concurrent emission preserves existing plans and leaves no
accepted partial output; stdout is bounded machine data and diagnostics are stderr.

## Nonfunctional contracts

- **NFR-01 / identity:** two evaluations of the same admitted inputs must produce
  identical canonical semantic plan bytes; independently record audit provenance.
  Explicit set normalization preserves ordered command arguments and patches.
  This detects tested nondeterminism; it does not prove arbitrary Rust purity or
  byte-for-byte reproducibility of every resulting OS image.
- **NFR-02 / bounds:** the authored JSON plan is at most 8 MiB (the current catalog
  bound), with at most 256 selectable package entries. Author evaluation has a
  proposed 60-second wall limit, 512 MiB memory ceiling and 1 MiB diagnostic cap; author
  compilation has a separate proposed 15-minute/4 GiB ceiling. Both use no network
  after explicit dependency preparation and no automatic retries. These are new
  policy defaults to qualify, not observed performance claims.
- **NFR-03 / preservation:** refusal/interruption leaves previously accepted
  plans, unrelated store objects, selected profiles, live home, and OS deployment
  unchanged. Public logs contain no raw private file contents. Existing cleanup,
  locking and per-invocation source/executable identity rules apply.
- **NFR-04 / scope:** edition 2024, pinned workspace tooling, one Cargo.lock, flat
  crate entrypoints and small justified dependencies. Verification is public CLI
  E2E, installed-system scenarios or manual real-process work, plus standard Cargo
  tools; no unit/model/mock/doctests or repository text/layout scanners.

## Definition of done for future implementation

AC-01 through AC-10 and NFR-01 through NFR-04 have observed results on their named
boundaries, migration parity passes for both existing targets, at least one
additional package is contributed without central-name edits, and an independent
consumer works. Production activation remains a separately authorized gate. This
documentation PR completes only the reviewable specification, not these criteria.
