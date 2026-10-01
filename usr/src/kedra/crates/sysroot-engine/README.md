# Independent build/store engine

`sysroot-engine` is an ordinary-user Rust library for declared native Linux builds,
immutable objects, runtime closure transfer and environment generations. It does
not invoke Nix. The Kedra CLI is one adapter; the public API contains no Kedra
target table, production key or repository address.

The first backend uses a local Unix Docker socket and native aarch64 Linux.
Planning/data types remain separate from Unix filesystem/execution code. Native
Darwin builds, Windows execution, x86_64 qualification, Nix-language/nixpkgs
compatibility, signed remote substitution and whole-OS construction are later work.
The typed `SystemDefinition` / `SystemFile` API now exports static native config
and verified runtime closures over a retained Fedora foundation; see
[system composition](../../docs/SYSTEM.md) for scope and actual qualification.

## Authoring

Use the public `BuildGraph`, `BuildNode`, `Input` and `Argument` types. Serialize
them with serde_json for the CLI. This data format has no imports, evaluation
language or dynamic realization during planning.

```rust,ignore
use std::collections::BTreeMap;
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input};

fn graph(source: String, builder: String, runtime: String) -> BuildGraph {
    BuildGraph {
        schema: 1,
        nodes: BTreeMap::from([("application".into(), BuildNode {
            builder_image: builder,
            runtime_image: runtime,
            inputs: BTreeMap::from([("source".into(), Input::Object(source))]),
            argv: vec![Argument::literal("/bin/sh"),
                       Argument::input("source", "build.sh"),
                       Argument::output("")],
            env: BTreeMap::new(),
            runtime_inputs: vec![],
            timeout_seconds: 120,
        })]),
    }
}
```

The source ID comes from source admission; image identities come from exact
locally admitted image archives. A node dependency uses `Input::Node(name)`.
`runtime_inputs` lists input aliases explicitly required at runtime; conservative
scanning also records known references. Planning validates the complete graph and
resolves the requested dependency closure without Docker or filesystem capture.

## CLI workflow

Build the workspace CLI, then use explicit private store and selected source paths:

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

Placeholder identities above must be replaced with actual admission/result IDs.
Image admission never pulls or installs tools. Runtime image archives are retained
and transferred; the compiler image and build-only sources need not be part of a
runtime export. Every output in a runtime closure must use the same foundation.
`develop` runs a command with private scratch and the selected declared closure;
it does not mount the host working directory or offer an interactive host shell.

## Integrity and recovery

Source objects use canonical tree hashes. Derived output paths are input-addressed,
with separate realized-content hashes. Independent rebuild compares the old
winner; divergence preserves it and retains private alternate evidence.

Builders see read-only input objects and private output backing at
`/usr/lib/sysroot/store/OUTPUT_ID`. Root filesystem and network are restricted;
no host home, credentials or daemon socket are mounted. The declared image is
the coarse userspace foundation. The host kernel, CPU and ambient process details
are not a universal mathematical purity/reproducibility guarantee.

Bundle import requires an independently expected whole-bundle hash, validates
allowlisted members and complete references, then registers dependencies first.
Image archives are checked against their exact OCI root and selected native
manifest/config/layer graph. The initial archive protocol is Docker29 OCI save;
classic Docker-only archives refuse. Other-platform branches may be absent;
only the selected ARM image is required. Docker performs decompression/DiffID
verification when loading. A bundle hash does not supply publisher or OS authority.

One store lock spans management and execution; the Store is movable but cannot
be concurrently shared. Execution journals bind an owned container to a frozen
Unix endpoint and daemon identity. GC retires unreferenced objects into journaled
tombstones before removing bytes. Initial profiles publish a complete staged
directory; existing selection uses atomic index replacement. Unknown/corrupt
state or uncertain cleanup refuses and preserves recovery evidence. Store GC
does not delete Docker images, GHCR objects or bootc deployments.

The private owner and Docker daemon are trusted. These commands never grant
installed root-helper or release-signing authority. Package tree hashing does not
attest arbitrary owners, SELinux labels or file capabilities. Source admission
requires an explicitly selected directory; exclude credentials before capture.

## Verification

The public E2E target uses generated Rust authoring and real C executable/shared
library builds, fresh receiver stores, profiles, tampered artifacts, owned
interruption/recovery and independent sentinels. Docker cases are explicitly
ignored by default; an ignored listing is not a behavior pass.

```sh
cargo test -p sysroot --test e2e_engine --locked -- --include-ignored --test-threads 1 --nocapture
```

This requires the retained pinned builder/runtime images recorded in the
[implementation design](../../../../../.specs/nix-engine/design.md). Actual
results and remaining fault variants belong in the implementation verification
record and project status. No OS boot/installer qualification follows from this
controller workflow.
