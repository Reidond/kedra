# Independent build/store engine

`sysroot-engine` is an ordinary-user Rust library for declared native Linux builds,
immutable objects, runtime closure transfer, retained environments and typed system
composition. It does not invoke Nix. The Kedra CLI is one adapter; the public API
contains no Kedra target table, production key, repository address or package
language parser.

The first backend uses a local Unix Docker socket and native aarch64 Linux.
Planning/data types remain separate from Unix filesystem/execution code. Native
Darwin builds, Windows execution, x86_64 source-build qualification, remote
endpoints/caches, `.nix`/nixpkgs compatibility and whole-OS construction remain
separate work.

## Direct typed API

Direct Rust consumers author `BuildGraph`, `BuildNode`, `Input` and `Argument`.
Planning is pure: it validates the complete graph and resolves the requested
dependency closure without opening Docker or capturing filesystem state.

```rust,ignore
use std::collections::BTreeMap;
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input, plan};

let graph = BuildGraph {
    schema: 1,
    nodes: BTreeMap::from([("application".into(), BuildNode {
        builder_image: "sha256:<exact-builder-id>".into(),
        runtime_image: "sha256:<exact-runtime-id>".into(),
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

`lower` requires exact builder roles and a runtime foundation already allowed by
the independent `sysroot_catalog::Policy`. It emits a strict legacy-compatible
`Catalog`, canonical resource trees and symbolic-to-graph node names. Build scripts
become `build.sh` resources invoked through typed arguments. Inline source files
become source objects. Patches and overlays add a separate preparation node with
the fixed admitted `prepare.py`; the original source remains read-only.

The pre-existing catalog API remains supported independently:
`sysroot_catalog::{Catalog, Package, Recipe, Source, Policy}` is strict serde data,
and `Catalog::resolve` checks namespace, package, image and source allowlists before
returning a `ResolvedPackage`. CLI `--catalog FILE` selects this reader. The built-in
jq/SQLite Rust recipes also remain during migration. There is no implicit
new-language-to-legacy fallback.

PR38 now selects `.kedra` through the explicit source discriminator and removes
list authority. Local Fedora build/rebuild, independent API and installed catalog
checks pass. Both-target fresh release qualification is recorded separately; no
production publication is implied. See [package declarations](../../docs/PACKAGES.md) and
[status](../../docs/STATUS.md).

## CLI workflow

Admit exact sources and images into a private store before building a graph:

```sh
sysroot store init --store STORE
sysroot store add-source --store STORE --source SELECTED_SOURCE_DIRECTORY
sysroot store add-image --store STORE --image sha256:EXACT_BUILDER_ID
sysroot store add-image --store STORE --image sha256:EXACT_RUNTIME_ID
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application --dry-run
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application
sysroot run --store STORE --object OUTPUT_ID --program bin/program
sysroot build --store STORE --plan GENERATED_GRAPH.json --root application --rebuild
sysroot store export --store STORE --object OUTPUT_ID --output closure.tar
sysroot store init --store RECEIVER
sysroot store import --store RECEIVER --bundle closure.tar --expected-sha256 EXPECTED_SHA256
sysroot profile switch --store RECEIVER --name development --object OUTPUT_ID --program bin/program
sysroot profile run --store RECEIVER --name development
sysroot profile rollback --store RECEIVER --name development
sysroot develop --store RECEIVER --name development --program /bin/sh -- -c 'printf hello'
sysroot store gc --store STORE
sysroot store gc --store STORE --delete
sysroot store recover --store STORE
```

Placeholder identities must be replaced with actual admission/result IDs. Image
admission never pulls or installs tools. Runtime image archives are retained and
transferred; compiler images and build-only sources need not enter a runtime
export. Every output in one runtime closure must use the same foundation.
`develop` runs a command with private scratch and the declared closure; it does
not mount the host working directory or offer an interactive host shell.

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
artifacts, owned interruption/recovery and independent sentinels. Its Docker cases
are ignored by default; listing an ignored case is not a behavior pass.

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
`/usr/lib/sysroot/store/OUTPUT_ID`. Root filesystem and network are restricted;
host home, credentials and daemon socket are not mounted. The declared image is
the coarse userspace foundation. Host kernel, CPU and ambient process details are
not a universal purity/reproducibility guarantee.

Bundle import requires an independently expected whole-bundle hash, validates
allowlisted members and complete references, then registers dependencies first.
Image archives are checked against their exact OCI root and selected native
manifest/config/layer graph. The initial archive protocol is Docker 29 OCI save;
classic Docker-only archives refuse. Docker performs decompression/DiffID checks
when loading. A bundle hash supplies no publisher or OS authority.

One store lock spans management and execution; the store is movable but cannot be
concurrently shared. Execution journals bind an owned container to a frozen Unix
endpoint and daemon identity. GC retires unreferenced objects into journaled
tombstones before removing bytes. Initial profiles publish a complete staged
directory; existing selection uses atomic index replacement. Unknown/corrupt
state or uncertain cleanup refuses and preserves recovery evidence. Store GC does
not delete Docker images, GHCR objects or bootc deployments.

The private owner and Docker daemon are trusted. These APIs never grant installed
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

## Authenticated cache and temporary snapshots

`verify_cache_receipt` returns `VerifiedCacheReceipt` only after exact P-256
signature, independently selected key/scope/policy revision, validity and resolved
recipe checks. `Store::substitute` copies and validates the complete bundle,
rechecks authority/expiry, then publishes an import journal or admits objects and
images. The expected recipe and policy come from the consumer, never the cache.
Transport is an explicitly selected local bundle; there is no network cache daemon.

`sysroot store substitute` exposes the same boundary with explicit `--plan`,
`--root`, `--scope`, `--cache-policy`, `--receipt`, `--signature`, `--public-key`
and `--bundle` inputs. Independently compiled consumers use their own resolved
catalog recipe rather than trusting recipe metadata from the cache.

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
