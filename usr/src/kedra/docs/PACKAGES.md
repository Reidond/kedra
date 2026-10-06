# Package declarations and delivery

Kedra has two kinds of package input. Fedora RPM requests define the bootc
foundation. Source packages build into immutable engine objects and contribute
selected commands, libraries and configuration to the composed image.

| Kind | Authored form | Materialization | Delivery |
|---|---|---|---|
| Fedora foundation packages | Current `.kedra` inputs: `foundation`, `set` and `target` declarations in `.kedra` | DNF resolves, installs, upgrades and removes RPMs in isolated Fedora 44 image builders | Complete signed Kedra image through bootc |
| Source packages | Versioned `.kedra` `package`/`library` declarations, or the retained typed Rust/JSON `Catalog` API | Exact builder images produce immutable objects below `/usr/lib/sysroot/store` | Selected runtime closure and typed configuration enter composition |

The `.kedra` frontend is an owner-approved, narrow language exception. Rust parses,
checks and lowers data into the existing `sysroot-catalog` and `sysroot-engine`
types. Parsing performs no network access, filesystem mutation, package build or
shell execution. Inline author shell runs only later as `build.sh` in an admitted,
isolated engine builder. Installed helpers and signers do not parse `.kedra` or
execute author recipes. This does not establish a general Kedra configuration
language or grant deployment authority.

## Current migration state

The frontend, resource lowering, typed APIs and release adapters are implemented.
`package-inputs.json` now selects the authored package catalog, replacing shared
and target list authority. The old schema-1 catalog remains immutable JSON for
compatibility rather than embedded Rust owner scripts. Fedora C/API/build/rebuild,
resource preparation, migrated jq/SQLite/library builds and the installed
catalog/alias/PATH/service workflow pass their recorded local scopes. Final
both-target native request/full-RPM parity and source CI pass at2d44c7e; protected
main-only release is not run; existing
Noctalia/installer/native failures are separate. See [status](STATUS.md) and the
[language verification plan](../../../../.specs/nix-package-dsl/test-plan.md).

The retained compiled jq/SQLite catalog has earlier real build, rebuild, transfer,
service and PATH passes in the
[catalog evidence](../../../../.specs/nix-package-catalog/verification.md), and its
genuine contribution passed the composer/exact-image checks recorded in
[D2 qualification](../../../../.specs/nix-release-composition/verification.md).
Those results remain valid for their exact sources and scope; they do not qualify
the new frontend, its migration, protected signing or a current production image.

The versioned source discriminator is deliberately explicit:

```json
{
  "schema_version": 1,
  "format": "kedra",
  "entry": "packages/catalog.kedra",
  "lock": "packages/packages.lock.json"
}
```

When a retained commit lacks that descriptor, the source resolver uses the legacy
package-list reader. A commit containing `.kedra` package inputs without the
descriptor is refused. A descriptor cannot coexist with authoritative shared,
target or removal lists, and an invalid new-format source never falls back to the
legacy reader. Retained legacy commits stay readable.

## Language v1

Every module begins with `language 1;`. The entry module alone declares the
policy-selected namespace; imports are explicit relative paths inside the admitted
catalog root. There is no directory discovery, remote import, macro, loop,
conditional, environment lookup, plugin or FFI escape hatch.

```text
language 1;
namespace "kedra";

import { hello } from "./hello.kedra";

foundation system {
    fedora = 44;
    packages = ["niri", "greetd", "NetworkManager", "pipewire"];
}

builder c_tools {
    base = system;
    packages = ["gcc", "binutils"];
}

target "desktop" {
    foundation = system;
}

target "qemu-arm64" {
    foundation = system;
    fedora = ["qemu-guest-agent", "egl-utils"];
    packages = [hello(builder: c_tools, runtime: system)];
}
```

Packages and libraries take exactly one `Builder` and one `Foundation` role.
Builders derive from the selected foundation and add compiler-only RPM requests.
`build_requires` augments the builder; `runtime_requires` augments the runtime
foundation. Sets may group source packages and Fedora requests. An explicit
`replacement` may change an existing include/remove request by exact origin and
name, but it cannot remove independently required base packages. The first-party
target policy independently restricts the namespace, targets, Fedora repositories
and required packages.

An inline source package keeps ordinary build logic next to its metadata:

```text
language 1;

package hello(builder: Builder, runtime: Foundation) {
    version = "1.0";
    summary = "Inline-source package workflow";
    license = "MIT";
    source = files {
        "hello.c" = text "#include <stdio.h>\nint main(void) { puts(\"Hello from Kedra!\"); return 0; }\n";
    };
    build = shell """
        mkdir -p "$out/bin"
        /usr/bin/gcc -O2 -ffile-prefix-map="$src"=. "$src/hello.c" -o "$out/bin/hello"
        """;
    export command "kedra-hello" = "bin/hello";
}
```

Archive sources refer to an exact URL, SHA-256 and normalized `src-...` object in
the strict `packages.lock.json`. `files` creates a source tree; optional `files`,
`replace_files` and ordered `patches` prepare a copy before the real build. A
source path cannot be both added and replaced. `build_deps` are build inputs;
`runtime_deps` are build inputs retained in the runtime closure. `env` values are
literals or typed dependency paths. `src`, `out`, `HOME`, `PATH` and `TMPDIR` are
reserved.

`export command`, `export library` and `export files` name safe output-relative
paths. A command package needs a command export; a library cannot export a
command. A `config` template becomes typed `SystemFile` content after the build.
Bindings may be literal, a typed package path or the independently verified
`source_revision`; missing, unused or phase-invalid bindings refuse.

Inline `text` and `shell` preserve decoded bytes. Triple-quoted literals require
LF input and a standalone closing delimiter; CRLF is refused. Larger reviewed
assets can use `file("./relative/path")`, and a file-backed build script can use
`script(file("./build.sh"))`. Public CLI planning runs in an owned data-only child with a 60-second deadline,
512 MiB Linux address-space ceiling, macOS resident-memory observation every
100 ms, and a 1 MiB diagnostic cap. Timeout/excess kills and reaps the child;
owner death terminates it. Direct Rust consumers provide their own process
supervision. The CLI opens modules, locks and resource files as
bounded, single-link ordinary files without following symlinks, checks that they
did not change during admission, and rejects known credential paths and private
key markers.

## Public tooling

Commands that select the language reader require explicit roots and policies.
The formatter accepts only its explicitly named files. Data-producing commands
emit JSON on stdout and failures on stderr.

```sh
sysroot catalog check \
  --input-root usr/src/kedra/image/packages \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json

sysroot catalog fmt --check usr/src/kedra/image/packages/catalog.kedra
sysroot catalog fmt usr/src/kedra/image/packages/catalog.kedra

sysroot catalog pins \
  --input-root usr/src/kedra/image/packages \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json

sysroot catalog plan --package hello --policy /private/catalog-policy.json \
  --input-root usr/src/kedra/image/packages \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json \
  --resolution /private/exact-images.json --output /private/hello-plan.tar
```

`check`, `pins` and graph planning are pure with respect to stores and builders.
`fmt` writes only the named files; `--check` writes nothing. `plan --output`
publishes a new tar packet atomically and refuses an existing destination. The
packet includes `graph.json`, admitted resource objects and `frontend.json`.
`resolve` emits selected metadata and identities. `build` adds `--store` and
executes the resolved graph after observing the exact Fedora 44 runtime and
compiler-role RPM material. `contribute` requires already realized outputs,
canonical independently hashed release input material, an exact lowercase source
commit and a new private output directory.

Exact image resolution is separate from authored requests:

```json
{
  "foundation": "sha256:<64 lowercase hex digits>",
  "builders": {
    "catalog.kedra#c_tools": "sha256:<64 lowercase hex digits>"
  }
}
```

Command exports with a different alias from their output filename receive a typed
launcher under `/usr/share/kedra/catalog-bin`. Exact-store paths for matching
command names are preserved; global RPM paths are never replaced.

The engine observes each retained image's actual seven-column RPM inventory. It
refuses a runtime missing a requested Fedora package, containing a requested
removal, or a builder missing either its compiler requests or the selected
foundation requests. Package names and pins do not prove installed bytes.

## Resource and execution boundary

Every inline or external build resource lowers to `SourceFile { bytes,
executable }`; `source_identity` computes its canonical `src-...` identity from
safe relative paths, modes and bytes. Build scripts are resources named
`build.sh`, invoked as `/bin/sh -eu <typed-resource-path>`, rather than inserted
into a command argument. Inline source files become another admitted source
object.

When patches or overlays are present, lowering adds a separate preparation node.
That node receives the original source and recipe resources read-only, runs the
fixed `prepare.py` from the selected exact builder, and emits a new prepared tree.
The package build consumes that tree. It does not mutate the admitted original or
fall back to host tools.

Release source acquisition is likewise separate from author shell. The reviewed
archive helper downloads only HTTPS pins, verifies size, deadline and SHA-256,
extracts a bounded single-root gzip/tar tree with restricted member types, and
then admits the resulting tree through the ordinary store workflow. This code is
public pre-signing work; production signers receive authenticated candidate
material and never fetch sources or run recipes.

At deployment time the machine receives a complete signed OCI image, not live
`dnf install` or package builds. Engine profiles and package rollback do not roll
back bootc deployments, home, databases or credentials.

See the [language contract](../../../../.specs/nix-package-dsl/language.md),
[engine API](../crates/sysroot-engine/README.md), [composition](SYSTEM.md),
[architecture](ARCHITECTURE.md) and [actual status](STATUS.md).
