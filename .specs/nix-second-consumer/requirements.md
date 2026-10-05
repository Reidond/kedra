# D4: independent Rust consumers

Status: preparation only, 2026-10-02. D1 is frozen; D4 source is unregistered
and has not been compiled or executed. The owner authorized this as the fourth
follow-on PR in [the topology](../nix-delivery/topology.md). This specification
records that authorized scope; it does not authorize earlier-layer edits.

## Problem and evidence

The engine's existing external Rust author emits a graph, but Kedra's CLI then
performs its lifecycle (`sysroot/tests/e2e_engine.rs`, `authoring_consumer`). That
source observation does not prove another project can use the catalog, build,
transfer, profiles and collection directly. This is a missing qualification
case, not a measured production incident; no incident frequency is claimed.
D3 now supplies the generic typed catalog and effective caller policy needed to
exercise this boundary. Reuse of Kedranix is future work; its files stay unchanged.

Actors: the ordinary owner selects explicit generated inputs and policies; the
external consumer executable calls public catalog/engine APIs; the E2E controller
starts real processes and checks resulting runtime behavior. No installed helper,
release signer, production credentials or privileged host operation participates.

## Acceptance criteria

- **AC-01 — independent compilation.** When the controller prepares Fieldkit and
  Observatory, each external Cargo workspace compiles its own executable against
  the same `sysroot-catalog` and `sysroot-engine` path dependencies, with explicit
  flat `main.rs` and separate definitions. Neither delegates lifecycle operations
  to the Kedra CLI. Build failure is a failed case, never a skipped pass.
- **AC-02 — names and effective policy.** When either executable resolves short
  package `report`, its own namespace/source/package/builder/runtime allowlists
  permit the intended recipe. Valid allowlists belonging to the other project or
  denying one required category produce `catalog policy denied` before any
  destination store exists. An allowlisted but undefined package produces the
  distinct unknown-package diagnostic. An unavailable daemon does not prevent
  successful pure resolution.
- **AC-03 — real different outputs.** When Fieldkit realizes v1, its native C
  report reads runtime data `[5,7,8]` through a separately built shared library
  applying `2*x+1` and prints `fieldkit-v1: total=43`. Repeating the build reuses
  both objects. Fieldkit v2 applies `3*x-1` and prints total `57` with a different
  output identity. Observatory reads `[1,3]`, applies `4*x+2`, and prints
  `observatory-v1: total=20` from `bin/station-report`. Source and executable
  failures must remain visible as nonzero process results.
- **AC-04 — producer-independent runtime.** When both Fieldkit versions are
  exported and imported by the external consumer, each exact-hash runtime bundle
  contains the report/library-data objects and runtime image, excluding compiler
  and admitted source objects. After the selected source directories are deleted
  and producer store renamed unavailable, both versions still execute in the
  receiver. Existing engine corruption/admission refusals stay in force.
- **AC-05 — independent profile history.** When Fieldkit's receiver selects
  v1, v2, then rollback, observed output follows 43, 57, 43. Both projects use
  short profile `development` in different stores. Observatory's selected object,
  profile index and runtime behavior remain unchanged; retained v2 remains runnable.
- **AC-06 — isolated collection.** When Fieldkit unpins an orphan and imported
  object/image roots and collects, only its orphan is removed; both generations
  remain retained by profiles. Observatory's object receipt, profile index and
  explicit sentinel remain unchanged and its executable still prints total 20.
- **AC-07 — portable resolved expectation.** When the external consumer resolves
  a catalog under its independently selected policy, the emitted
  `ResolvedPackage.plan.outputs[recipe.root]` equals the later realized root.
  The plan also exposes `specs[recipe.root]` for a future D5 cache consumer; D4
  must not obtain that expected spec from cache receipts or bundles.

## Constraints

- **NFR-01 — bounded owned execution.** Use the existing E2E process controller
  (600-second command deadline, bounded captured output), 120-second recipe
  deadlines and 60-second runtime calls. Preserve failure fixtures for diagnosis;
  clean only generated owned fixture paths on success. No performance SLA is claimed.
- **NFR-02 — scope.** This preparation writes only its new spec directory and
  the three owned unregistered files. Parent alone later registers the module,
  runs gates, updates shared status, and publishes D4 above D3. No new runner,
  unit tests, repository scanners, global installation or Kedranix mutation.
- **NFR-03 — explicit completion evidence.** Runtime cases remain `not-run` until
  executed on the owning layer. Formatting/source review is not compilation,
  execution, package delivery, signing, boot, installation or deployment evidence.

Out of scope: OS composition and signed activation; Nix language/nixpkgs;
cross-user security or adversarial owners of the same private store; authenticated
cache substitution (D5); new controller architectures; physical hardware.

These scope boundaries are owner decisions in the topology. Current API matching
is source-inspected; behavior remains unverified. Historical engine qualification
does not qualify these new consumers.
