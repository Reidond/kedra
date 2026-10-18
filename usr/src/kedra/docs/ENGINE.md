# Build and environment engine

[`sysroot-engine`](../crates/sysroot-engine/README.md) is the reusable
ordinary-user Rust library for immutable source objects, deterministic build
graphs, isolated builds, runtime closures, retained profiles, static composition
and recovery. The CLI exposes `sysroot build`, `store`, `run`, `profile`,
`develop` and the package-facing `catalog` commands.

The engine has no package language parser and no Kedra target table. It accepts
typed `BuildGraph`, `BuildNode`, `Input`, `Argument` and `SourceFile` values. The
separate `sysroot-catalog::language` frontend parses and checks `.kedra` files,
then lowers the accepted intent and the exact builder and runtime roots to those
types; `Catalog` is only that lowered form, and `Catalog::resolve` applies the
caller's exact policy to it. Author shell becomes an admitted `build.sh` resource
and runs only inside the isolated builder ([language contract](PACKAGES.md#language-v1)).
There is no resident daemon, Nix executable, host shell fallback or general
configuration interpreter.

## Objects and identity

A store is a private directory chosen with `--store`, owned by one Unix user. Its
marker binds the store schema, the logical prefix `/usr/lib/sysroot/store` and the
owner; another owner, another schema or an unknown layout is refused. Every
command holds the store's exclusive lock for its whole run, including builds,
runs, transfer and collection, so a store can move but cannot be shared
concurrently. An unfinished transaction blocks every command except `store
recover`. Objects appear at the logical prefix only inside the sandbox; on an
installed system `/usr` is read-only, so store objects reach
`/usr/lib/sysroot/store` there only through a composed image.

- Source objects (`src-<sha256>`) are the canonical hash of an admitted tree:
  sorted relative names, entry type, executable bit, bytes and symlink targets.
  Owners, timestamps, xattrs, SELinux labels and file capabilities are not part of
  it. Hardlinks, special files and symlinks that leave the tree are refused; one
  file may hold at most 256 MiB and a tree 1 GiB.
- Outputs (`out-<sha256>`) are input-addressed: the hash of the resolved build
  spec, which binds the platform, the logical prefix, the sandbox policy, the
  builder and runtime roots, the exact input objects, typed argv and environment,
  the runtime inputs and the timeout. Each node has one output. The realized
  content hash is recorded separately.
- Roots (`root-<sha256>`) are named by their recipe: an image, optionally plus
  locked RPM roles ([execution](#execution-and-integrity)).

A graph has 1 to 256 nodes and at most 8 MiB of JSON; a node has at most 256
inputs, argv entries, environment entries and runtime inputs, arguments of at most
32 KiB and a timeout of 1 to 3,600 seconds. Store paths enter commands only as
typed input or output segments: a literal containing the store prefix is refused,
as are `HOME`, `PATH` and `TMPDIR` as node environment names. Unknown fields,
duplicate names, missing inputs and cycles are refused before anything runs, and
planning opens no store.

An output's runtime references are the store paths found in its bytes plus its
declared runtime inputs. Every reference must be an input of the node or part of
an input's closure, and a whole runtime closure must share one runtime root;
anything else is refused. A
repeated build reuses a valid output; `--rebuild` builds every needed node again
and compares content and references with the existing winner. A divergent result
keeps the winner, moves the alternate to `quarantine/` and fails (exit 78).

## Execution and integrity

Package builds, `run`, `profile run` and `develop` execute without a daemon in an
unprivileged bubblewrap (`bwrap`) sandbox on native Linux, so they work on
an installed Kedra (the foundation carries `bubblewrap`) and need no Docker socket,
which is effectively root. The engine refuses to start a sandbox as root. Each
process gets new user, PID, network, IPC, UTS and
cgroup namespaces (`--unshare-all --unshare-user --disable-userns`), runs as the
store owner with no capabilities and `no_new_privs`, `--die-with-parent`,
`--new-session`, a cleared environment (`HOME=/build`, `TMPDIR=/tmp`,
`PATH=/usr/bin:/bin`, `LANG`/`LC_ALL=C`, `TZ=UTC` plus the node's declared
variables) and the node's timeout. It sees only a read-only root filesystem,
private tmpfs `/tmp` and `/build`, its `/dev` and `/proc`, the read-only store
inputs at `/usr/lib/sysroot/store/<id>` and, for a build, the one writable output
directory there. No host home, credentials, sockets or other host paths are
mounted. The sandbox cannot bound memory or process count (bubblewrap has no
cgroup control). The derivation policy string `v2:bwrap:...` is part of every
output identity.

Roots are store root filesystems (`rootfs/root-<sha256>`), input-addressed by a
recipe and reused by identity:

- an image root is the store's archive of an exact image, unpacked without a
  daemon (layers verified against their uncompressed digests, whiteouts applied,
  device nodes skipped, set-ID bits cleared, every entry owned by the store owner
  and owner-readable). Foundation observation creates it on first use;
- an RPM root is an image root plus roles of a [Fedora RPM lock](#fedora-rpm-lock),
  one role per root over its parent root, installed offline from the lock's
  fixed-output RPM objects by the root's own `rpm` inside the sandbox. `sysroot
  store add-root --image <base> --lock <lock>` installs the foundation role over
  the lock's Fedora base (a runtime root); `--builder <role>` installs that builder
  role over the runtime root (a builder root). Every file's Fedora signature is
  checked first, and a root is refused unless its RPM inventory equals the role's
  locked inventory. A non-root `rpm` keeps file ownership instead of changing it,
  which is what lets an ordinary user install.

A graph node names its `builder_root` and its `runtime_root`; `run`, `profile
run` and `develop` execute in the output's runtime root. Both are part of the
output identity, so a package's `out-` identity follows the Fedora base digest,
the locked RPMs and its own inputs only. In a release neither is built from the
per-run Kedra foundation image, whose ID changes with every commit; a composition
accepts an output only over a foundation whose RPM inventory equals the runtime
root's exactly ([SYSTEM](SYSTEM.md#plan-and-compose)). Roots are not transferred
in bundles; a receiver creates the same input-addressed roots from the same image
archive and lock before it imports. Roots are verified by owner, mode and recipe
identity on use, not re-hashed; their receipts record the observed RPM inventory,
entry count and size. `store gc` keeps pinned roots, the runtime roots of retained
objects and profiles, the image roots of retained images and the parents of all of
these.

The engine is native: `PLATFORM` (`aarch64-linux` or `x86_64-linux`) is the
architecture `sysroot` was compiled for, and it refuses images, daemons, roots and
objects of any other. Source, image and root admission are explicit. The exact
builder root is the coarse build userspace. This does not promise bit-for-bit
reproducibility across host kernels, CPUs or all ambient details.

Every sandbox is journaled in `transactions/execution.json` with the kernel boot
and PID-namespace identity and, once started, the bubblewrap PID and start time.
`--die-with-parent` ends the sandbox when its controller is killed. `store
recover` then refuses a journal of another store, accepts a sandbox from an earlier
boot as gone, kills a surviving sandbox (matched by PID and start time) and waits
until it is gone, refuses to guess from another PID namespace of the same boot,
and removes the staging directory.

Language resources use the same engine contract as direct Rust consumers.
`source_identity` accepts at most 4,096 safe ASCII paths and 32 MiB of bytes,
binds 0644/0755 semantics, rejects case/file-directory collisions, and returns a
canonical `src-...` identity. Package build scripts are file inputs, avoiding the
engine's 32 KiB argument bound. Patch/overlay preparation is a separate graph node
with an immutable original input and a fixed admitted driver.

`Store::observe_foundation` runs fixed commands in the foundation image's root
through the journaled sandbox and records Fedora 44 `/usr/lib/os-release` plus
sorted RPM rows containing name, epoch, version, release, architecture, header
SHA-256 and payload SHA-256. Builder roots carry the same rows from admission.
The language adapter compares authored requests to this actual material before
resource admission/build.

Bundle import requires an independently expected whole-bundle hash, validates
allowlisted members and complete references, refuses unless the receiver already
holds every runtime root, then registers dependencies first. Bundles carry no
image archives. Profiles publish complete
staged generations and use atomic index replacement. GC retires unreferenced
objects through journaled tombstones before removing bytes, and never touches
Docker images, registries or bootc deployments. Unknown or corrupt state and
uncertain cleanup refuse and preserve recovery evidence.

`ManagedSnapshot` leases protect composition, native, catalog plan and formatter
temporary state in an owner-private registry (`.sysroot-snapshots-v1`). A held file
lock is the liveness signal, so a stopped reader keeps its snapshot; `sysroot
system recover --workdir <dir>` collects only recognized abandoned snapshots and
reports active and refused ones, never judging by age or PID. These mechanisms
grant neither OS signing nor installed helper authority.

## Binary cache

Realized closures can be shared through an OCI registry. The registry is never
trusted: what a consumer admits is decided by records it already trusts, the
object receipts in the installed `/usr/share/sysroot/composition.json` of a
signature-verified image. In a release that is the previous stable image (preflight
pulls it under the strict signature policy and copies the file out); on a machine it
is the booted image, which bootc verified when it was staged.

- `sysroot store push --store S --object ID --cache REPO` exports `ID`'s verified
  closure as one OCI artifact (one uncompressed tar layer holding the bundle) tagged
  `closure-<sha256>`, the hash of the closure's canonical receipts. Different
  realizations of one derivation therefore never share a tag; a tag already
  present is left as it is.
- `sysroot store substitute --store S --object ID --cache REPO --trusted FILE`
  refuses unless `FILE` holds a record for `ID` and its whole closure, derives the
  tag from those records, pulls the artifact and imports it only if every staged
  object (its tree hash recomputed from the staged bytes) equals its record and
  nothing else is present. A closure the store already holds needs no transfer.
  A refusal exits 78; a cache that is unreachable or lacks the tag exits 69.
- `sysroot build` and `sysroot catalog build` take the same `--cache`/`--trusted`:
  before building a missing node that has a record, the closure is substituted;
  a refusal or unavailable cache is recorded in `cache_misses` and the node is
  built. A substitution that leaves the store needing recovery (an import journal
  remains, or the store reports recovery required) stops the build instead; run
  `sysroot store recover` first. `--rebuild` never substitutes. A node without a record (a build-only
  intermediate the image does not carry) is built.

Transport is `skopeo`, run with a cleared environment, a private home and a
policy that accepts only the cache repository and local layouts, so neither the
user's nor the system's container signature policy applies (signatures carry no
trust here). `--cache-authfile` supplies registry credentials;
`--cache-plain-http` serves local test registries and refuses credentials. Runtime
roots are not part of an artifact: the consumer makes the same roots from the
Fedora base and RPM lock first, as for `store import`. Hits need equal identities,
so they follow the base digest and the locked RPMs ([runtime
roots](#execution-and-integrity)): a release hits when it rebuilds packages whose
base, lock roles and inputs the previous stable image already realized.

Admission does not depend on what else the registry holds, so a cache can be
pruned freely: Kedra releases keep one cache package per target and delete every
closure the newly published stable image does not record
([RELEASES](RELEASES.md#binary-cache)). A cache never bypasses the release checks:
an image is still built, tested and signed as a whole, and cache access grants no
signing authority.

## Fedora RPM lock

Fedora RPMs are input-addressed. `Store::resolve_rpms` is the only networked
package step: it runs DNF once, as root inside disposable containers of the exact
base image on a native local Docker engine, never on the host. The foundation
transaction (upgrade, install, remove) runs first. The last builder role installs
in that container itself and any earlier role in a container from one commit of
it, so every role sees the same repository metadata and an untouched foundation.
Every RPM the transactions downloaded becomes a fixed-output single-file `src-`
object: its identity is predicted from file name, size and SHA-256 before any byte
is read (`rpm_object`), and admission (`import_rpm`, `fetch_rpm`) refuses any other
bytes. A resolution of the same store removes containers an interrupted
predecessor left, which the store lock makes safe; leftover staging is ordinary
`store recover` work.

`materialize_rpms` copies exactly one role's verified objects into a new offline
repository. Images then install from it with all repositories disabled and Fedora
signature checks on (`image/catalog/install-rpms.sh`), and refuse any installed
inventory other than the locked one. `fetch_rpm` downloads with `curl` from the
recorded mirror, then from Koji's signed copy. The per-file bound is 256 MiB, the
store's tree file bound. RPM objects are pinned; unpin them to let `gc` collect
them. The lock document itself is `sysroot-catalog::rpm_lock`; see
[package delivery](PACKAGES.md#fedora-rpm-lock).

## Boundaries

The engine supports one native architecture per machine (aarch64 or x86_64;
cross-architecture builds are refused), one runtime root per closure, bubblewrap
with unprivileged user namespaces (0.8 or later) and Docker 29 OCI save archives.
Image import from a daemon (`store add-image --image`), RPM resolution and native
derivation use the local Unix-socket Docker engine (no TLS or remote endpoint);
`store add-image --archive` imports a saved archive without one. Both release
targets build through the engine ([one image pipeline](SYSTEM.md#one-image-pipeline)),
each natively on its own runner. The [binary cache](#binary-cache) has been
exercised only against a local registry; [STATUS](STATUS.md) records what has run
where.

The engine does not replace bootc. Kernel, boot loader, initramfs installation,
SELinux, encrypted installation, signed updates and rollback stay with the signed
bootc image workflow; a store-composed OS backend would need its own authority for
each of those and is not part of Kedra. There is no Nix language or nixpkgs
compatibility. Package execution stays ordinary-user and nonroot, home stays
writable, and the installed helper and the production signer keep their narrow
roles.

Engine/package rollback does not roll back systemd service state, databases,
writable home, credentials, bootc deployments or registry tags. A bundle hash is
content integrity, not publisher identity.

Use the [library guide](../crates/sysroot-engine/README.md) for concrete APIs and
CLI examples, [SYSTEM](SYSTEM.md) for composition, [PACKAGES](PACKAGES.md) for
authoring, and [STATUS](STATUS.md) for checks actually run.
