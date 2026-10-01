# Independent Rust engine — first usable implementation

State: implementation authorized by the owner on 2026-10-01, following the
[research proposal](../nix-recreation/proposal.md). This concretizes its minimal
slices 0–2; it does not authorize publishing or replacing the current OS backend.
Kedra is the target; Kedranix remains read-only reference. The owner instruction
to implement provides authorization for this specification and work within the
accepted proposal; no repeated stage approval is requested.

## Outcome

A reusable flat-source Rust crate and public `sysroot` commands must build real
software in a declared native Linux namespace, retain immutable outputs, transfer
the complete runtime closure to a new store, execute it offline, and select/roll
back profile generations. This is an independent engine: it invokes no Nix.
Current signed-image/home/helper behavior remains separate.

## Acceptance conditions

- **AC-01 — Independent planning:** a validated Rust `BuildGraph` produces stable
  derivation/output identities without Docker execution, filesystem capture,
  network fetch or credentials. Version, platform, canonical prefix, builder and
  runtime foundation identities, source/input references, argv/env and named output
  participate. Unknown schemas, duplicate names, missing graph inputs and cycles
  refuse. One output per derivation is supported.
- **AC-02 — Native build:** public CLI source/image admission plus build compiles
  a real C shared library and an executable that calls it. Inputs are read-only,
  private staging is mounted at the final logical output path, the builder is
  non-root, network is absent, host home/socket/credentials are not mounted.
  A failed/timed-out builder never registers valid output.
- **AC-03 — Immutability and reproduction:** identical realization validates and
  reuses existing content; source/environment/input changes yield different IDs.
  Forced independent rebuild compares real canonical bytes; divergent content
  refuses to overwrite the valid winner. Invalid stored bytes/receipts refuse.
- **AC-04 — Runtime closure:** actual output references, explicit runtime inputs
  and a digest-bound runtime image are retained; build-only sources/compiler image
  need not be exported. The exported bundle binds exact metadata, references and
  runtime image bytes. Import requires an independently expected whole-bundle
  SHA-256 and validates every member before publication. Unknown entries, traversal,
  unsafe nodes/links, missing references, wrong digests and duplicate IDs refuse.
- **AC-05 — Offline execution:** a fresh receiver store imports that bundle and
  executes the program with network disabled and no producer mounts. A missing,
  altered or platform-mismatched runtime foundation refuses. An image cached in
  the daemon cannot substitute for missing/corrupt store runtime evidence.
- **AC-06 — Profiles:** two observable versions coexist. Profile selection is a
  durable atomic metadata replacement; retained generations remain roots. Public
  profile run/rollback executes the corresponding old/new binary. This does not
  claim service/data/boot rollback. `develop` offers command execution in the
  selected declared environment; no native Darwin package realization is promised.
- **AC-07 — Serialization and recovery:** one exclusive standard filesystem lock
  serializes management and run/build use. Image/object/profile commits are
  durable atomic directory/file publication. An unfinished execution journal blocks
  normal use until explicit recovery verifies the same daemon and owned labels,
  stops/removes that operation's container, and removes private staging safely.
  Unknown/corrupt journals refuse; no arbitrary foreign process/container cleanup.
- **AC-08 — Roots and collection:** source/image admission and requested results
  are protected before returning. Explicit pin/unpin and dry-run/explicit GC retain
  profiles, runtime references and images; GC never operates outside engine-owned
  storage or GHCR/bootc deployment stores. Use is serialized against GC. Missing
  root/reference state refuses rather than silently collecting dependencies.
- **AC-09 — Reusable API:** a generated Rust authoring consumer uses the public
  crate types/API, without Kedra target constants, release keys or repository URLs.
  JSON is a bounded serialization/CLI transport for those types, not a new
  expression/configuration language. Typed paths and references survive lowering.
- **AC-10 — CLI compatibility:** existing commands keep their contracts. Engine
  diagnostics go to stderr; machine-readable operation results go to stdout.
  Child runtime stdout/status remain observable. Docker unavailability produces
  an actionable refusal; there is no automatic tool install/pull or weaker backend.

## Resource and trust requirements

- **NFR-01:** safe Rust, edition2024/toolchain1.98.1/workspace lints; no new daemon,
  root builder, custom Git engine, custom test runner or unit/mock/doctests.
- **NFR-02:** bounded JSON (8MiB), graph (256 nodes), arguments/environment (256),
  output files/bytes and process logs; builder/runtime timeouts are explicit and
  enforced. No unbounded subprocess capture. Unknown wire fields/schemas refuse.
- **NFR-03:** the initial store is private to its Unix owner and has a schema/prefix
  marker. Existing store paths and receipts are checked before use. Local owner
  and Docker daemon are trust boundaries; this is not a root/multi-user cache or
  protection against a compromised daemon/owner. Imported bundles are explicitly
  selected by expected digest, not authenticated third-party build provenance.
- **NFR-04:** canonical tree data includes sorted names, regular bytes/executable
  flag, directories and supported symlink targets. Hardlinks/special nodes refuse;
  arbitrary ownership/xattrs/SELinux metadata are not package-tree attestations.
  Store/OS metadata remain separate as researched.

## Deliberate later work

Signed remote substitution, general Nix-language/nixpkgs compatibility, parallel
multi-output realization, cross compilation/native Darwin, general OS module
composition and deployment backend replacement remain subsequent slices. Initial
execution is native aarch64 Linux; x86_64 must be qualified separately. No new
engine result inherits existing OS boot/signature qualification.
