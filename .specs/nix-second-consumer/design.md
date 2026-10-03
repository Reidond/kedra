# D4 design

Status: prepared, unregistered, not compiled or executed. See
[requirements](requirements.md) and [verification](verification.md).

## Shared interface

Frozen D3 source inspected on 2026-10-02: `sysroot-catalog/lib.rs` exposes
`Catalog { namespace, packages }`, `Package { version, summary, license, sources,
recipe }`, `Recipe { graph, root, program }`, and `Policy { namespace, packages,
builder_images, runtime_images, source_objects }`. `Catalog::resolve` returns
`ResolvedPackage { namespace, package, version, recipe, plan }` without accessing
files, a store or a daemon. All categories use exact allowlists.

The selected package declares one `SourceOrigin::Local { description }` with the
admitted `src-` identity. Both graph nodes use that source; the report depends on
the support node through `Input::Node`. This matches D3's requirements: every
Object input anywhere in the selected recipe graph must be an allowed declared
source; `out-` Object inputs are forbidden; declared/used source sets must agree.
No new API or policy parser is needed.

Each generated consumer has its own Cargo `[workspace]`, explicit `main.rs`,
project/program constants and a `definitions.rs` module copied from the reusable
fixture. Dependencies are engine, catalog, serde and serde_json. Both builds use
one private target cache with distinct package/binary names. Standard Cargo is
invoked offline; missing cached dependencies fail instead of installing tooling.

The adapter offers ordinary explicit operations: declare, resolve, admit,
admit-source, build, run, profile switch/read/run/rollback, export/import,
closure, unpin, unpin-image, collect and verify. The file representation is the
existing serde catalog/policy model, not an added configuration language.
`build` resolves the graph and policy before `Store::create`; explicit source/image
admission is a separate owner operation. Profile/import/GC operations retain
existing ordinary-owner Store authority and do not acquire catalog publisher or
installed OS authority. They are not a new sandbox against the store owner.

## Actual workflow

Two namespaces use short package `report`, but different admitted sources,
programs and source allowlists. Fieldkit builds `libreadings.so` plus runtime data
as one output, then links `bin/field-report` to that output using an exact store
RPATH and compiled data path. Observatory builds `bin/station-report` with its own
data and arithmetic. Results and versions are defined by AC-03.

The E2E child module reuses `e2e_engine`'s bounded process capture, generated
fixture lifetime and exact retained image IDs. All lifecycle execution uses the
compiled external executables. Pure resolve also runs with a nonexistent Docker
endpoint. Its planned root is compared with the real build result (AC-07).

Fieldkit v1/v2 closures transfer into a new receiver, their original source
directories are deleted, and the producer store is renamed unavailable. The
receiver then exercises `development` v1/v2/rollback and collection. Observatory's
same-named profile lives in a different store and is checked through actual
readback and execution before and after collection. The unchanged physical
logical-prefix contract remains `/usr/lib/sysroot/store`; project namespaces do
not alter derivation addressing or become security identities.

Refusal cases mutate valid policy documents one category at a time and require
the catalog-policy diagnostic plus an absent destination. The undefined-package
case explicitly allowlists the missing name so it reaches catalog lookup rather
than being mistaken for a package-policy test. Neither generic nonzero exit nor
an ignored test listing is sufficient evidence.

## D5 interface boundary

The prepared D5 `verify_cache_receipt` accepts an independently selected
`CachePolicy` and `ResolvedBuildSpec`. A future consumer first calls
`Catalog::resolve`, then selects `resolved.plan.specs[resolved.recipe.root]` as
the expected recipe. That local expected value must never be replaced with the
signed receipt's own derivation or inferred merely from its claimed output ID.

Catalog policy and cache policy serve different purposes. The former authorizes
package names, exact inputs and images. D5's independent scope/key/revision/time
policy authenticates a producer and signed bundle. The catalog namespace is NOT
present in `ResolvedBuildSpec`; callers must explicitly bind the intended
namespace/package to the independently chosen cache-policy scope. A matching
recipe alone cannot establish publisher or project authority. Dedicated generated
cache keys and separate Fieldkit/Observatory scopes must exercise cross-policy
refusal on D5; production signing keys remain outside this workflow.

D5's opaque verified receipt must be bound to complete staged bundle validation
before import admission/pinning. Ordinary expected-hash `Store::import` in D4
does not claim authenticated substitution. Generic bounded signature verification
may later live in `sysroot_core::signature` without consuming Kedra release target
tables, installed trust or privileges. No D5 module/API is wired by D4 preparation.

## Files and ownership

Prepared files owned by this worker:

- `usr/src/kedra/crates/sysroot/tests/reuse.rs`
- `usr/src/kedra/crates/sysroot/tests/fixtures/reuse_consumer.rs`
- `usr/src/kedra/crates/sysroot/tests/fixtures/reuse_definitions.rs`
- `.specs/nix-second-consumer/{requirements,design,tasks,verification}.md`

Parent's later D4 integration adds only the child registration in
`sysroot/tests/e2e_engine.rs`, then owning-layer docs/worklog/status/skill changes
as needed. It consumes D3's already registered crate. No existing files change
during D1 freeze; Git/index/branch and heavy-resource ownership stay with parent.

## Alternatives, risks and rollback

Extending the old graph-emitting author alone was rejected because it leaves
lifecycle ownership in Kedra's CLI. Copying Store internals or introducing a
second policy type would not prove shared interface reuse. Rewriting Kedranix
would expand scope before this small compiled consumer is qualified.

| Risk | Likelihood / impact | Handling |
|---|---|---|
| D3 API drifts before D4 | Medium / medium | Reinspect the frozen predecessor; compile external consumers on D4. |
| Generated crates depend on cached Cargo packages/images | Medium / medium | Offline build and exact image admission fail explicitly; never install tools silently. |
| A generic refusal hides the wrong failure path | Medium / high | Require policy-specific/unknown-package diagnostic and absent destination, followed by valid builds. |
| Profile GC appears isolated without exercising another live project | Medium / high | Preserve same-named live Observatory profile and receipt/index/sentinel; execute it after GC. |
| Catalog policy confused with authenticated cache or OS authority | Medium / high | Explicit separate D5 scope/spec/key policy; no D4 signing claim. |
| Failed command leaves generated resources | Low / medium | Existing deadlines/cleanup and retained failure fixture; inspect actual owned resources after execution. |
| D4 reads as tested while still unwired | Medium / high | Every case remains not-run and preparation is distinguished from qualification. |

D4 changes no engine persistence format or deployed state. Reverting its module
registration removes the new E2E case; fixture files/specs can remain dormant.
Never delete owner stores or shared Docker images as rollback or cleanup.
