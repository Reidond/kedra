# First engine design

Authorized direction: [requirements](requirements.md), owner implementation request
2026-10-01. Base `b224d5711e857f7dbcaabf7ed42870916525800c`; no commit/push.

## Components and concrete choices

- One `sysroot-engine` library at `usr/src/kedra/crates/sysroot-engine`, explicit
  flat `lib.rs`, separate model/tree/store/bundle/profile/executor modules. Existing
  serde/serde_json/sha2/tar workspace dependencies; standard `File::lock` avoids
  another locking dependency. CLI integration is `sysroot/engine.rs`.
- Canonical logical prefix **`/usr/lib/sysroot/store`**. Physical private store is
  an explicit CLI `--store` path; it is never mounted wholesale. Individual input
  object `data/` directories mount read-only at their logical object paths. A
  private output directory mounts writable at its final predicted output path.
- Initial backend: Docker CLI, native aarch64 Linux, `--pull never`, exact locally
  admitted image identity. Current retained builder ID:
  `sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546`
  (Rust1.98.1, Debian GCC14.2/glibc2.41). Retained runtime ID:
  `sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`
  (Ubuntu24.04, linux/arm64). These are observed retained image identities,
  not invented registry digests or a universal libc compatibility assertion.
- Image admission uses `docker image inspect` and `docker image save` of the exact
  identity. Record platform, actual immutable image identity, archive size/hash.
  The runtime archive travels with exports. Before use, verify its store bytes,
  load only when needed, inspect the exact loaded identity/platform, and reject
  image-declared volumes or unsupported configuration that could add hidden state.
- Builders use a nonzero UID matching private staging ownership, read-only root,
  no network/capabilities, no-new-privileges, private PID/IPC context, bounded PIDs,
  memory/tmpfs, fixed environment and workdir. Explicit argv is never shell
  interpolated by the engine; a declared source script may invoke its own shell.
  Runtime uses the same isolation and declared runtime foundation/closure mounts.

## Public values and identity

`BuildGraph` schema1: ordered root names plus a BTreeMap of named `BuildNode`s.
Each node carries builder/runtime image IDs, source/object inputs, dependency-node
inputs, typed argv/env and declared runtime dependency names. `Argument` is a
bounded sequence of literal/input-path/output-path segments; substitution produces
argv/env after IDs are resolved and preserves artifact references. No arbitrary
host paths are accepted in a graph.

Resolve the static DAG before execution. `ResolvedBuildSpec` contains resolved
input object IDs, sorted explicit runtime object IDs, builder/runtime identity,
platform, fixed prefix/policy/schema and typed command fields. A domain-separated
SHA-256 of deterministic serde JSON/BTree structures identifies a derivation and
its sole `out-<hex>` path. Source IDs are `src-<canonical-tree-hash>`. Image IDs are
validated exact `sha256:<hex>` values. Wire types deny unknown fields and validate
again at public entry points; constructors/serde cannot bypass path/size limits.

Tree hash is a versioned unambiguous binary framing of sorted relative entries,
their type, regular bytes/executable bit and symlink target. Names/targets are
UTF-8 and bounded; traversal/control characters/escaping links, hardlinks and
special files refuse. Sources are explicit selected caller paths, never a whole
home/vault/agent snapshot. Snapshot metadata is normalized; mtime/owner do not
participate. OS metadata is outside this first format.

## Storage, execution and roots

Store layout: marker + lock, immutable `objects/<id>/{receipt.json,data/}`, immutable
`images/<digest>/{receipt.json,image.tar}`, management roots/profiles and private
`transactions/`. Marker, lock and management files are owner-only. Unknown state,
symlinked management paths, wrong ownership/modes or unsafe root paths refuse.
Physical path spelling is excluded from artifact identity but Docker bind syntax
requires rejecting separators/control characters that change mount parsing.

Hold one exclusive store lock for the entire command, including run/build,
transfer and GC. A writer stages data/receipt, verifies canonical bytes/references,
flushes files/directories, then renames the complete object directory and flushes
its parent. A forced rebuild never removes the winner. Source/result/image roots
are persisted before returning; profile generations are retained permanently until
an explicit prune policy is added.

Executor operations write a journal before container creation with operation
nonce, store token, daemon identity, container name and owned labels. Capture
stdout/stderr with bounded draining readers and enforce deadline. Verify owned
container identity/labels, remove it before hashing/sealing (close descendant
writes), and remove the journal only after complete cleanup. A crash leaves a
journal; Store opening must not silently discard it. Explicit recovery checks
daemon/token/labels and stops only the owned operation before deleting staging.
Unavailable daemon or uncertain ownership refuses and preserves evidence.

Runtime refs are explicit runtime inputs plus conservative scans across the entire
declared input closure (including serialized names/symlink targets/file bytes).
Unknown possible store references or missing referenced objects refuse. Support
self refs explicitly; non-self reference cycles refuse initially. This mechanism
is qualified for the concrete package workflow, not arbitrary compressed/dynamic
pointer encodings.

Profiles contain content-bound generation records (root output, relative program,
argv, runtime image), retained history and selected index. Write the immutable
generation then atomically replace the profile pointer/index; an interrupted
unselected generation cannot change the selected command. Rollback selects the
prior retained generation. GC takes the same management lock, validates all roots
and receipts, retains transitive runtime objects/images and profiles, and removes
unreferenced referrers before dependencies. Build-only image/source roots can be
explicitly unpinned; active use cannot race collection.

## Transfer and trust

Export a streamed tar of a versioned bundle manifest, selected output closure
receipts/data and required runtime image archives. Use bounded, normalized member
paths. Export computes the whole-bundle SHA-256. Import requires that independently
selected hash, stages privately, verifies members/receipts/trees/image archives and
reference closure before any registration, then admits/roots objects. Already
valid matching objects reuse; conflicting content refuses. Partial admission
cannot select a profile or expose an incomplete valid closure; recovery retains
or removes orphan unselected objects conservatively. No signed-cache or privileged
OS authority is derived from the bundle.

## Public CLI

`sysroot build --store S --plan GRAPH --root NAME [--rebuild]` returns result IDs,
tree hashes, reuse/independent-rebuild observations and runtime references as JSON.
`sysroot store` admits selected source/image, verifies/inspects/copies closures,
imports/exports digest-bound bundles, pins/unpins, dry-runs/executes GC and recovers
interrupted execution. `sysroot run` executes an output's relative program.
`sysroot profile` selects/lists/runs/rolls back generations. `sysroot develop`
runs a command in the selected declared environment. These are ordinary-user
development workflows; existing `update`, home, agents and root helper are untouched.

## Risks and qualification

Residual writers, archive/path escapes, corruption, interrupted registration,
concurrent GC/use, daemon-context changes and unbounded logs are substantive risks;
their failure contracts are above and real CLI cases must cover them. Owner and
daemon compromise, bitwise repeatability of arbitrary packages, full OS metadata,
hardware/security boot and remote-cache authentication are outside this slice.
The concrete C library/executable must actually load on the declared runtime;
glibc compatibility is accepted only after that observed result.

Primary APIs checked 2026-10-01:
[Docker run](https://docs.docker.com/reference/cli/docker/container/run/) and
[standard File locking](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock).
Research provenance remains [the prior reading guide](../nix-recreation/reading-guide.md).

## Pre-implementation review refinements

- One exact runtime image must satisfy every output in a transitive runtime
  closure. Reject differing foundations during graph resolution, registration,
  import and run; there is no inferred cross-image ABI compatibility.
- `--rebuild` forces every needed graph node into fresh staging, even when its
  winner exists. Compare canonical tree digest and runtime reference metadata with
  the validated winner. Equal results report independent reproduction; divergent
  results return a distinct failure, retain the winner and preserve the alternate
  private evidence. Reused dependencies are not described as independently rebuilt.
- Imports prevalidate a bounded allowlist from the bundle manifest; duplicate
  IDs/member paths, unknown members and archive-level links/devices refuse. Only
  regular files/directories are extracted; supported object symlinks are encoded
  as tree metadata, validated, and created last after safe directories/files.
  Register images/dependencies before referrers. A durable import transaction
  roots all admitted members until final result-root publication. Interrupted
  recovery conservatively roots valid admitted members, never selects a profile,
  then removes private incomplete staging; retry may reuse validated members.
- `develop` is command execution in a selected profile's declared runtime image
  and mounted closure, with a temporary `/build` work area. The command may select
  a validated absolute program from that declared image (for example `/bin/sh`)
  or its normal profile executable. No compiler-image fallback, host working
  directory, interactive Darwin shell or network access is implied.
- Concrete versioned limits: JSON8MiB; graph/input/argv/env counts256; arguments
  32KiB; nodes100000; regular file256MiB; canonical tree1GiB; image archive8GiB;
  bundle16GiB; per-stream process capture1MiB with draining readers/truncation
  indication. Builder timeout defaults120s, runtime60s, all positive and at most
  3600s; control/image-transfer commands300s. Sandbox: memory1GiB, PIDs256,
  temporary space256MiB, nonzero owner UID. These are first-version policy choices,
  not measured requirements from Nix papers. Build-affecting policy is in identity.
- Use existing `rustix` safe Unix descriptor APIs for no-follow source reads and
  directory access where standard path checks leave capture races. No unsafe code
  or new third-party runtime is introduced.

The Astra review's five mechanism gaps are resolved by these refinements. Further
implementation deviations must be recorded here and checked against requirements.
