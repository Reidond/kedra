# Independent build/store engine

`sysroot-engine` is an ordinary-user Rust library for declared native Linux builds,
immutable objects, runtime closure transfer, retained environments and typed system
composition. It does not invoke Nix. The Kedra CLI is one adapter; the public API
contains no Kedra target table, production key, repository address or package
language parser.

Builds and runtime commands execute in an unprivileged bubblewrap sandbox on native
aarch64 Linux, inside store root filesystems; no container daemon is involved.
Planning/data types remain separate from Unix filesystem/execution code. Native
Darwin builds, Windows execution, x86_64 source-build qualification, remote
endpoints/caches, `.nix`/nixpkgs compatibility and whole-OS construction remain
separate work.

## Direct typed API

Direct Rust consumers author `BuildGraph`, `BuildNode`, `Input` and `Argument`.
Planning is pure: it validates the complete graph and resolves the requested
dependency closure without opening a store, running a sandbox or capturing
filesystem state.

```rust,ignore
use std::collections::BTreeMap;
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input, plan};

let graph = BuildGraph {
    schema: 1,
    nodes: BTreeMap::from([("application".into(), BuildNode {
        builder_root: "root-<exact-builder-root-id>".into(),
        runtime_root: "root-<exact-runtime-root-id>".into(),
        inputs: BTreeMap::from([("source".into(), Input::Object(
            "src-<exact-source-id>".into(),
        ))]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::input("source", "build.sh"),
            Argument::output(""),
        ],
        env: BTreeMap::new(),
        runtime_inputs: vec![],
        timeout_seconds: 120,
    })]),
};
let planned = plan(&graph, "application")?;
# Ok::<_, sysroot_engine::Error>(planned)
```

Use `SourceFile` and `source_identity` when authoring an ordinary declared resource
tree. Identity binds canonical safe relative paths, executable bits and exact
bytes. The API accepts at most 4,096 files and 32 MiB of resource bytes; it rejects
empty trees, unsafe/non-ASCII or case-colliding paths, file/directory collisions,
depth over 64 and metadata paths over the 8 MiB JSON budget.

```rust,ignore
use std::collections::BTreeMap;
use sysroot_engine::{SourceFile, source_identity};

let files = BTreeMap::from([("build.sh".into(), SourceFile {
    executable: false,
    bytes: b"mkdir -p \"$out/bin\"\n".to_vec(),
})]);
let source_object = source_identity(&files)?;
# Ok::<_, sysroot_engine::Error>(source_object)
```

## Package frontend and retained catalog API

The owner-approved `.kedra` frontend lives in `sysroot-catalog::language`, outside
this engine. Its public Rust path is explicit and typed:

```rust,ignore
use std::collections::BTreeMap;
use sysroot_catalog::{Policy, language::{Images, Lockfile, TargetPolicy, compile, lower}};

let modules: BTreeMap<String, String> = load_reviewed_modules();
let lock: Lockfile = load_reviewed_lock();
let target_policy: TargetPolicy = load_reviewed_target_policy();
let intent = compile(&modules, "catalog.kedra", "qemu-arm64", lock, &target_policy)?;

let external_resources: BTreeMap<String, Vec<u8>> = load_reviewed_resources();
let images: Images = load_independently_selected_images();
let catalog_policy: Policy = load_independent_catalog_policy();
let lowered = lower(&intent, &external_resources, &images, &catalog_policy)?;
# Ok::<_, Box<dyn std::error::Error>>(lowered)
```

`parse`, `format`, `compile`, `lower`, `Intent`, `Images`, `Lockfile`,
`TargetPolicy`, `Content`, `Binding` and the intent records are public. The library
functions consume caller-supplied bytes; direct consumers must provide their own
bounded, stable, non-private input admission. The CLI adapter supplies the
repository's no-symlink/single-link/recheck rules. Neither path executes author
code during parsing/checking/lowering.

`lower` requires exact builder roots and a runtime root already allowed by
the independent `sysroot_catalog::Policy`. It emits a strict legacy-compatible
`Catalog`, canonical resource trees and symbolic-to-graph node names. Build scripts
become `build.sh` resources invoked through typed arguments. Inline source files
become source objects. Patches and overlays add a separate preparation node with
the fixed admitted `prepare.py`; the original source remains read-only.

The pre-existing catalog API remains supported independently:
`sysroot_catalog::{Catalog, Package, Recipe, Source, Policy}` is strict serde data,
and `Catalog::resolve` checks namespace, package, builder root, runtime root and
source allowlists before
returning a `ResolvedPackage`. CLI `--catalog FILE` selects this reader. The built-in
jq/SQLite Rust recipes also remain during migration. There is no implicit
new-language-to-legacy fallback.

PR38 now selects `.kedra` through the explicit source discriminator and removes
list authority. Local Fedora build/rebuild, independent API and installed catalog
checks pass. Both-target fresh release qualification is recorded separately; no
production publication is implied. See [package declarations](../../docs/PACKAGES.md) and
[status](../../docs/STATUS.md).

## CLI workflow

Admit exact sources and images into a private store and create the runtime and
builder roots before building a graph:

```sh
sysroot store init --store STORE
sysroot store add-source --store STORE --source SELECTED_SOURCE_DIRECTORY
sysroot store add-image --store STORE --image sha256:EXACT_BASE_ID --archive SAVED.tar
sysroot store add-root --store STORE --image sha256:EXACT_BASE_ID --lock rpm-lock.json
sysroot store add-root --store STORE --image sha256:EXACT_BASE_ID \
  --lock rpm-lock.json --builder ROLE
sysroot store add-root --store STORE --image sha256:EXACT_OTHER_IMAGE_ID
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application --dry-run
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application
sysroot run --store STORE --object OUTPUT_ID --program bin/program
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application --rebuild
sysroot store export --store STORE --object OUTPUT_ID --output closure.tar
sysroot store init --store RECEIVER
sysroot store add-image --store RECEIVER --image sha256:EXACT_BASE_ID --archive SAVED.tar
sysroot store add-root --store RECEIVER --image sha256:EXACT_BASE_ID --lock rpm-lock.json
sysroot store import --store RECEIVER --bundle closure.tar --expected-sha256 EXPECTED_SHA256
sysroot profile switch --store RECEIVER --name development --object OUTPUT_ID --program bin/program
sysroot profile run --store RECEIVER --name development
sysroot profile rollback --store RECEIVER --name development
sysroot develop --store RECEIVER --name development --program /bin/sh -- -c 'printf hello'
sysroot store gc --store STORE
sysroot store gc --store STORE --delete
sysroot store recover --store STORE
```

Placeholder identities must be replaced with actual admission/result IDs. `add-image`
without `--archive` saves the image from the local Docker engine; with `--archive`
it admits a `docker image save` archive without a daemon. `add-root` prints the
root identity a graph's `builder_root` or `runtime_root` names: an image's unpacked
root, that root plus an RPM lock's foundation role (`--lock`), or that runtime root
plus one builder role (`--builder`), installed offline (the locked RPM objects must
already be in the store, see `sysroot catalog lock fetch`). Image admission never
pulls or installs tools. Roots are not exported: a receiver creates the same
runtime root before `import`. Builder roots and build-only sources need not enter a
runtime export. Every output in one runtime closure must use the same runtime
root. `develop` runs a command with private scratch and the declared closure in
the runtime root; it does not mount the host working directory or offer an
interactive host shell.

The language-aware CLI adds pure `sysroot catalog check`, `fmt`, `pins`, `plan`
and `resolve` operations plus store-backed `build` and post-build `contribute`.
When those commands select the language reader, they require an explicit input
root, entry, target, lock and target policy; `fmt` instead operates only on its
named files. Plan/build additionally require a catalog policy and either an exact
role-resolution record or explicit builder/runtime identities. See the [package
guide](../../docs/PACKAGES.md) for complete commands and schemas.

## Verification

The established public engine E2E target uses generated Rust authoring and real C
executable/shared-library builds, fresh receiver stores, profiles, tampered
artifacts, sandbox isolation observed through `/proc`, killed-controller and
surviving-sandbox recovery and independent sentinels. Its sandbox cases are ignored
by default (they need bubblewrap with user namespaces and the retained images, which
the fixture imports from the local Docker engine); listing an ignored case is not a
behavior pass. In a container, run them with
`--security-opt seccomp=unconfined --security-opt systempaths=unconfined` (user
namespaces and a mountable `/proc`) and an init process that reaps orphans.

```sh
cargo test -p sysroot --test e2e_engine --locked -- --include-ignored --test-threads 1 --nocapture
```

The retained builder/runtime images are specified in the
[engine design](../../../../../.specs/nix-engine/design.md). Exact results and
remaining fault variants belong in the implementation verification records and
[project status](../../docs/STATUS.md). The package-language suite remains partial,
so the command above and historical engine evidence do not qualify its cutover.

## Integrity and recovery

Derived output paths are input-addressed, with separate realized-content hashes.
Independent rebuild compares the old winner; divergence preserves it and retains
private alternate evidence.

Builders see read-only input objects and private output backing at
`/usr/lib/sysroot/store/OUTPUT_ID` inside a bubblewrap sandbox: new user, PID,
network, IPC, UTS and cgroup namespaces, no capabilities, `no_new_privs`, a
read-only root, private `/tmp` and `/build`, a cleared environment,
`--die-with-parent` and the node timeout. Host home, credentials and sockets are
not mounted. Memory and process count are not bounded. The declared builder root is
the coarse userspace foundation. Host kernel, CPU and ambient process details are
not a universal purity/reproducibility guarantee. See the
[engine contract](../../docs/ENGINE.md#execution-and-integrity) for roots.

Bundle import requires an independently expected whole-bundle hash, validates
allowlisted members and complete references, then registers dependencies first.
Image archives are checked against their exact OCI root and selected native
manifest/config/layer graph. The archive protocol is Docker 29 OCI save; classic
Docker-only archives refuse. Unpacking a root checks every layer's uncompressed
digest. A bundle hash supplies no publisher or OS authority.

One store lock spans management and execution; the store is movable but cannot be
concurrently shared. Execution journals record the sandbox's boot, PID namespace,
PID and start time; recovery kills a surviving sandbox and removes its staging. GC
retires unreferenced objects into journaled
tombstones before removing bytes. Initial profiles publish a complete staged
directory; existing selection uses atomic index replacement. Unknown/corrupt
state or uncertain cleanup refuses and preserves recovery evidence. Store GC keeps
pinned roots and the roots of retained images; it does not delete Docker images,
GHCR objects or bootc deployments.

The private owner is trusted; the Docker daemon only for image import and RPM
resolution. These APIs never grant installed
root-helper or release-signing authority. Package tree hashing does not attest
arbitrary owners, SELinux labels or file capabilities. Exclude credentials before
source capture.

## System composition

`SystemDefinition`, `SystemFile` and `SystemContent` export static native config
and verified runtime closures over a retained Fedora foundation. `NativeDefinition`
and `NativeStep` add closed declarations for GLib compilation, systemd links,
initial account skeletons and QEMU initramfs generation. The model binds inputs and
implementation; the Kedra harness owns isolated native execution and receipt
validation. See [typed composition](../../docs/SYSTEM.md) and
[native artifacts](../../docs/NATIVE.md).

## Binary cache and temporary snapshots

`TrustedRecords::read` takes object receipts from an installed composition receipt
(`/usr/share/sysroot/composition.json`, schema 2) of an image the caller verified.
`Store::push(&BinaryCache, id)` publishes a verified closure under
`closure_tag` (the hash of its receipts); `Store::substitute(&BinaryCache,
&TrustedRecords, id)` pulls the tag the records imply and admits the closure only if
every staged object equals its record. `Store::build_with(.., Some(Substitution))`
substitutes recorded nodes before building them and records cache misses. The
registry is untrusted transport (`skopeo` with a private policy and home); see
[the binary cache](../../docs/ENGINE.md#binary-cache).

```sh
sysroot store push --store STORE --object OUTPUT_ID --cache ghcr.io/OWNER/CACHE
sysroot store substitute --store RECEIVER --object OUTPUT_ID --cache ghcr.io/OWNER/CACHE \
  --trusted /usr/share/sysroot/composition.json
sysroot build --store RECEIVER --plan GRAPH.json --root application \
  --cache ghcr.io/OWNER/CACHE --trusted /usr/share/sysroot/composition.json
```

Composition, native and catalog packet contexts use `ManagedSnapshot` leases in a
versioned owner-private registry. Readers retain a file lock, including while
stopped; callers explicitly finish after consumers exit. `sysroot system recover
--workdir PATH` collects only recognized abandoned snapshots and reports active or
refused entries. It does not delete unknown paths by age or PID.

Recorded D5 passes cover signed transfer, producer-independent execution/rebuild,
stopped-reader snapshot recovery and several bounded Linux ENOSPC cases. Native
publication-window retries and native transaction/binding ENOSPC remain
unqualified. See [D5 evidence](../../../../../.specs/nix-cache-recovery/verification.md)
and [actual status](../../docs/STATUS.md). No OS boot, installer or production
publication claim follows from an engine workflow.
