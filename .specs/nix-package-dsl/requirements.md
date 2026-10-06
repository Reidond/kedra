# Requirements

Status: owner approved the standalone language and inline-first authoring direction
on 2026-10-05; on 2026-10-06 the owner explicitly requests finishing the spec and
implementing the language and migration. Acceptance remains evidence-based.

## Problem and actors

Built-in recipe edits currently change compiled Rust and embedded shell strings;
Fedora selections live in separate lists. Authors need one readable package format
that can carry most build details without Rust compilation or a collection of
mandatory sidecar scripts. The generic engine must remain reusable. Current
main has two compiled owner packages (jq and SQLite), three build nodes and three
compiled configuration templates. This inventory was inspected on 2026-10-06 at
`b60bcb0`; no authoring-frequency or productivity measurement is available. The
owner's requested editable authoring flow, rather than an invented incident rate,
motivates this work. Existing external Catalog JSON already supports independent
Rust consumers; preserve that capability instead of rebuilding the engine.

Actors: package author, owner selecting policy/targets, language frontend,
resolver/build processes, release verifier, and independent consumer. Recipe
contents and consumer authorization remain independent inputs.

## Scope and explicit decision change

Include a small versioned `.kedra` language with typed declarations, parameterized
package/library templates, explicit local imports, package sets, target selection,
Fedora system/compiler requirements, pinned sources and inline build content.
Include parser/type checker, formatter, source diagnostics, bounded expansion,
canonical intent, resource materialization, contribution/release integration and
legacy-source migration. Rust implements the frontend and retains a programmatic
API to the same intent; ordinary package authors do not write or compile Rust.

The owner expressly revises the former embedded-Rust/no-new-language choice for
this feature. Implementation must update the applicable architecture/agent/skill
wording narrowly at cutover; this spec does not silently change those global rules.

Exclude Nix syntax/nixpkgs compatibility, general functions/loops/recursion, dynamic
plugins/FFI/Rust escapes, arbitrary evaluation-time IO, Fedora source rebuilding,
third-party repositories, live DNF operations, general home management, new release
or deployment authority, cross-compilation, automatic version discovery, network
cache/mirror services and an IDE language server in v1. Fedora desktop behavior
must survive migration; source-build execution remains qualified native ARM only.

## Acceptance scenarios

### AC-01 — Edit a package without compiling Rust

Given compatible installed frontend/engine binaries and an admitted `.kedra` catalog,
when an author edits a package's inline build block and compiles its plan, then the
new recipe is visible and buildable without rebuilding those binaries or compiling
an author crate. Invalid syntax or types produce file/line/column diagnostics and
no accepted plan. Imports and declarations never execute shell or Rust.

### AC-02 — Select Fedora and source packages together

Given a set selecting Firefox through Fedora and jq/SQLite through source recipes,
when its resolved candidate is built, then the Fedora package is present and the
selected source-built programs execute from their recorded store outputs, including
SQLite's separate library. Missing/unauthorized inputs fail before acceptance.
No implicit replacement of `/usr/bin` occurs.

### AC-03 — Compose sets and targets deterministically

Given shared and target-specific sets, when a declared target is selected, then
includes/removals/exports are deterministic and retain their origins. Conflicting
include/remove requests, unequal definitions of one identity, unknown targets and
duplicate exports fail with both origins. Identical repeated requests may deduplicate.
Explicit Fedora-request replacement must identify an existing request; absent matches, protected
base removals and dependency removal violating the final required set refuse.

### AC-04 — Pin all source and recipe inputs

Given reviewed source pins, when acquisition/admission runs, then archive bytes
and normalized source-tree identity are verified. Inline or external scripts,
patch order, source files/modes, declared environment, compiler/runtime roles
and dependencies participate in the build recipe. Post-build templates bind the
system-contribution identity separately. Wrong bytes, unsafe archives, absent
files, ambiguous normalization and changed pins refuse without replacing a valid
result. Untracked personal files are never implicit inputs; local directory sources
are explicit and disabled in production policy by default.

### AC-05 — Resolve Fedora intent into observed material

Given a frozen plan, reviewed exact Fedora base and approved repositories, when
resolution runs in an isolated image, then complete system/compiler RPM material
is recorded and compared with the candidate. Solver choices and dependencies are
observed rather than guessed from names. Missing repositories, signature failure,
missing required packages or material drift fail closed. Unchanged deterministic
resolved inputs preserve the current no-change behavior.

### AC-06 — Separate planning from execution and authority

Given package files and independent policy, when parsing/planning runs, then no
recipe command, imported executable, environment probe, network request or Cargo
compilation occurs. Explicit frozen input loading is the only file access the
frontend needs. Sources, repositories, targets and builder/runtime permissions
are independently checked before their effectful operation. Later source preparation
and shell builds use the existing isolated/offline/nonroot boundaries; DNF uses
its separately owned disposable environment. Denied operations grant no deployment
or signing authority and preserve accepted state.

### AC-07 — Type dependencies and exports correctly

Given build-only tools, runtime libraries and package templates, when a selected
program is transferred/run, then its runtime closure is complete and build-only
inputs stay outside it unless actually referenced. Cycles, recursive instantiation,
unbound image roles, wrong-platform references and absent outputs refuse. Internal
libraries have typed outputs, not invented executable entrypoints. A Fedora
requirement is resolved for the appropriate image role, not an engine output.
Fedora jq and source jq may coexist only through unambiguous explicit exposure.

### AC-08 — Bind definitions to contribution and release inputs

Given verified resolved language material with a newly selected package, when
contribution/preflight run, then outputs/templates follow the selected plan rather
than a jq/SQLite whitelist. Frozen sources, frontend identity, language version,
input/resource inventory, pins, policy, resolution and actual results remain bound
to candidate/release material. Stale/tampered content or references refuse before
signing eligibility. The production signer executes no frontend or recipe code
and retains its existing trust schema/authority.

### AC-09 — Migrate with one authoritative format

Given retained legacy and new-language revisions, when each is resolved, then an
explicit format discriminator selects exactly one path. Old revisions keep their
semantics; mixed-authority lists/language inputs and unknown versions refuse.
Parity of package intent and actual installed behavior on both existing targets
is required before cutover. Failed new-format preparation cannot fall back to
legacy lists. Old signed image provenance stays readable; no live home/data changes.

### AC-10 — Reuse the same intent from an independent consumer

Given a second `.kedra` catalog and an independent Rust consumer of the typed API,
when they describe equivalent admitted inputs and execute their real package
workflow, then both use the same semantic intent/lowering without Kedra release
keys or targets embedded in the generic layer. Invalid scope refuses. Concurrent
or interrupted emission leaves no accepted partial plan and preserves old outputs.
Data-only validation never invokes an author executable supplied by the plan.

### AC-11 — Keep ordinary package builds inside package files

Given the migrated jq/SQLite definitions and a small generated C program, when each
is built, then all authored shell steps and small build resources are contained in
its `.kedra` definition, with no mandatory `.sh`, patch, template or Rust sidecar.
Inline source files, ordered patches, generated text, declared environment and
build/runtime dependencies work in a real build. An optional external reference
resolves only within admitted inputs and cannot silently substitute different bytes.
This is an authoring default and concrete pilot requirement, not an invented
percentage of all future packages or a requirement to embed upstream tarballs.

### AC-12 — Preserve literal content and file identity

Given inline text/shell/patch/template blocks or their admitted external equivalent,
when formatted, lowered and materialized, then exact defined payload bytes/modes
are preserved and participate in content identity. Formatting outside a literal
cannot change build behavior; changing literal bytes or file mode changes its
resource identity. Path traversal, absolute paths, duplicate/case-colliding paths,
symlink ancestors, unsafe modes and implicit overlay replacement refuse. Shell
variables are not expanded by the `.kedra` frontend.

### AC-13 — Provide bounded, versioned language tooling

Given admitted v1 modules, when check/format/plan runs, then deterministic diagnostics
identify syntax/type/import/version errors and cycles. Formatting is idempotent
and preserves literal payloads. Imports cannot escape the catalog root, follow
links or resolve by network/search path. Unknown versions, excessive depth/size,
expansion cycles or plan-limit exhaustion refuse without partial acceptance.
No plugin, general eval, ambient environment lookup or recursive function exists.

## Nonfunctional contracts

- **NFR-01 / identity:** identical admitted inputs produce identical semantic plan
  and resource bytes. Sets normalize by stable identity; argv, patches and build
  phases retain order. Formatting-only source changes affect audit provenance but
  not semantic recipe identity if payloads are unchanged. This does not prove
  bit-for-bit reproducibility of all resulting OS images.
- **NFR-02 / bounds:** proposed defaults are at most 128 imported modules, 8 MiB per
  admitted input, 32 MiB total admitted frontend input, 4096 declared resources
  and 64 nesting levels. These resource limits exclude the separately bounded
  upstream archive tree, which follows existing engine admission.
  The emitted intent/lowered graph respects existing 8 MiB data and 256-node graph
  limits, with at most 256 selectable package entries. Parse/expand/emit has a
  60-second deadline, 512 MiB process ceiling and 1 MiB diagnostic cap. No automatic
  retry or silent truncation; generated preparation nodes count toward limits.
  Existing per-build limits remain separate. These are policies to qualify, not
  measured performance claims.
- **NFR-03 / preservation:** refusal/interruption preserves accepted plans,
  unrelated objects, profiles, home and OS deployment. Diagnostics share source
  locations and fixed reasons without echoing arbitrary literal contents. Existing
  process ownership, cleanup and per-invocation provenance contracts apply.
- **NFR-04 / scope:** Rust edition 2024, pinned tools, one Cargo.lock, flat crate
  entrypoints and small justified dependencies. Only public CLI E2E, installed
  scenarios/manual real processes and standard compiler/linter checks; no unit,
  model, mock, doctest or repository source/layout scanner.

## Future implementation completion

AC-01 through AC-13 and NFR-01 through NFR-04 need actual evidence. Migrate the
inline-first pilots, add another source package without central-name edits, prove
both target migrations and independent consumption, and retain explicit production
publication boundaries. This PR completes the specification revision only.
