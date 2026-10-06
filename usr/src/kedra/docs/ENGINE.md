# Build and environment engine

[`sysroot-engine`](../crates/sysroot-engine/README.md) is the reusable
ordinary-user Rust library for immutable source objects, deterministic build
graphs, isolated builds, runtime closures, retained profiles, static composition
and recovery. The CLI exposes `sysroot build`, `store`, `run`, `profile`,
`develop` and the package-facing `catalog` commands.

The engine has no package language parser and no Kedra target table. It accepts
typed `BuildGraph`, `BuildNode`, `Input`, `Argument` and `SourceFile` values. The
separate `sysroot-catalog::language` frontend parses and checks `.kedra`, then
lowers accepted intent and exact image-role resolution to those types. This is the
narrow owner-approved package language exception: author shell becomes an
admitted `build.sh` resource and runs only inside the existing isolated builder.
There is no resident daemon, Nix executable, host shell fallback or general
configuration interpreter.

The legacy typed API remains independently usable. Strict serde JSON `Catalog`
records and `Catalog::resolve` keep their own explicit reader and policy; CLI
`--catalog` selects that path. The built-in jq/SQLite Rust catalog is retained
during migration. At the committed-source boundary, old revisions use the
package-list reader while a valid `package-inputs.json` selects the `.kedra`
reader. Selection is versioned and exclusive; a failed new reader never falls
back to a legacy format.

The committed source discriminator now selects `.kedra` definitions; the old lists
are removed. The schema-1 catalog is retained immutable JSON for old callers.
Local Fedora builds/rebuilds, independent API consumption, preparation and the
installed catalog workflow pass. Fresh both-target release qualification remains
separate from these passes; no production image is published by this PR. See [package authoring](PACKAGES.md) and
[current status](STATUS.md).

## Execution and integrity

The current execution backend is a local Unix Docker socket and native aarch64
Linux. Source and image admission are explicit. Derived output paths are
input-addressed and independently realized content is hashed separately. A
rebuild compares the prior winner and preserves divergent evidence instead of
replacing it.

Builders see read-only inputs and private output backing at
`/usr/lib/sysroot/store/<output-id>`. Root filesystem and network are restricted;
host home, credentials and the Docker socket are not mounted. The exact builder
image is the coarse build userspace. This does not promise mathematical
reproducibility across host kernels, CPUs or all ambient details.

Language resources use the same engine contract as direct Rust consumers.
`source_identity` accepts at most 4,096 safe ASCII paths and 32 MiB of bytes,
binds 0644/0755 semantics, rejects case/file-directory collisions, and returns a
canonical `src-...` identity. Package build scripts are file inputs, avoiding the
engine's 32 KiB argument bound. Patch/overlay preparation is a separate graph node
with an immutable original input and a fixed admitted driver.

`Store::observe_foundation` runs fixed image-local commands through the journaled
executor and records Fedora 44 `/usr/lib/os-release` plus sorted RPM rows containing
name, epoch, version, release, architecture, header SHA-256 and payload SHA-256.
The language adapter compares authored requests to this actual material before
resource admission/build. DNF resolution remains in isolated foundation/compiler
builders; it never targets the workstation.

Bundle import requires an independently expected whole-bundle hash, validates
allowlisted members and complete references, then registers dependencies first.
Runtime image archives are retained and transferred. Profiles publish complete
staged generations and use atomic index replacement. One store lock spans
management and execution; unknown/corrupt state and uncertain cleanup refuse and
preserve recovery evidence.

Authenticated local-bundle substitution uses consumer-selected cache keys,
scope, policy revision and recipe. `ManagedSnapshot` leases protect composition,
native and catalog packet temporary state; `sysroot system recover` collects only
recognized abandoned snapshots. These mechanisms grant neither OS signing nor
installed helper authority.

## Boundaries

The first backend supports native ARM, one runtime image per closure, local Unix
Docker endpoints and Docker 29 OCI save archives. Native x86_64 source builds,
remote endpoints/caches, nixpkgs or `.nix` compatibility and an alternate OS
backend remain unqualified. The `.kedra` Kedra policy currently refuses selected
source packages for `desktop`; desktop may still select Fedora foundation
requests. Complete kernel, boot, SELinux, installation and release signing use
separate workflows.

Engine/package rollback does not roll back systemd service state, databases,
writable home, credentials, bootc deployments or registry tags. A bundle hash is
content integrity, not publisher identity. Store tree hashing does not attest
arbitrary ownership, SELinux labels or file capabilities.

Use the [library guide](../crates/sysroot-engine/README.md) for concrete APIs and
CLI examples, [SYSTEM](SYSTEM.md) for composition, [PACKAGES](PACKAGES.md) for
authoring and cutover, and [STATUS](STATUS.md) for checks actually run. The
[engine specification](../../../../.specs/nix-engine/requirements.md) and
[package-language specification](../../../../.specs/nix-package-dsl/README.md)
define intended acceptance; they are not execution evidence. The preserved
[recreation research](../../../../.specs/nix-recreation/README.md) remains design
reference rather than a substitute for current runtime results.
