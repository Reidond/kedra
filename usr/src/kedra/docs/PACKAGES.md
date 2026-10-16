# Package declarations and delivery

Kedra has two kinds of package input. Fedora RPM requests define the bootc
foundation. Source packages build into immutable engine objects and contribute
selected commands, libraries and configuration to the composed image.

| Kind | Authored form | Materialization | Delivery |
|---|---|---|---|
| Fedora foundation packages | Current `.kedra` inputs: `foundation`, `set` and `target` declarations in `.kedra` | Resolved once per release run into an RPM lock of fixed-output store objects; the image build and builder roots install offline from it | Complete signed Kedra image through bootc |
| Source packages | Versioned `.kedra` `package`/`library` declarations, or the retained typed Rust/JSON `Catalog` API | Exact builder roots produce immutable objects below `/usr/lib/sysroot/store` in the engine's bubblewrap sandbox | Selected runtime closure and typed configuration enter composition |

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
and target list authority. The old schema-1 JSON catalog was removed on
2026-10-14. Fedora C/API/build/rebuild,
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

Since 2026-10-16 the shipped targets select the official Codex and Bitwarden
Desktop packages ([shipped packages](#shipped-packages)) instead of jq, SQLite and
hello. Those three remain the language's source-build fixtures in
`crates/sysroot/tests/fixtures/catalog` (E2E `fixture_catalog_keeps_the_former_demo_packages`):
Fedora's foundation already ships jq, so a second jq ahead of it on PATH had no
product purpose, and shipping it would need `replaces`, which the fixture jq
declares instead.

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

## Fedora RPM lock

The committed `packages.lock.json` holds only the author's source pins. Fedora
RPMs are locked per release run instead: Fedora repositories are not snapshotted,
the nightly run is what notices an `updates` push, and CI cannot commit under the
local sign-off policy. A committed RPM lock would also be stale for the next run.

`sysroot catalog lock resolve` reads the selected `.kedra` foundation, target and
builder sets and resolves them once against the exact Fedora 44 base, in
disposable containers on a native Docker engine (upgrade, install, remove; each
builder role over the resolved foundation). Every downloaded RPM becomes a
fixed-output engine store object. The canonical JSON lock records:

- target, architecture, base image, repositories and the SHA-256 of the selected
  intent (`package_frontend.intent_sha256` of the source plan);
- per RPM file: name, epoch, version, release, architecture, size, SHA-256, header
  and payload SHA-256, source RPM, repository, the Koji signed-copy URL, the
  mirror URL this run used, and the predicted `src-` object;
- per role (`foundation`, each builder): requests, removals, the files installed
  over its parent, and its complete seven-column installed inventory.

Koji's `packages/<srpm>/<version>/<release>/<arch>/<file>` is the unsigned build
output, with another SHA-256 than mirrors serve. The durable URL is the signed copy
`packages/<srpm>/<version>/<release>/data/signed/<key id>/<arch>/<file>`, which is
byte-identical to the mirrors. Koji can prune signed copies of untagged builds;
fetching then fails rather than admitting other bytes.

The image Containerfile and the catalog builder bind-mount one role's offline
repository and install it with every repository disabled and Fedora signature
checks on (`image/catalog/install-rpms.sh`). The result must equal the role's
locked inventory. Each image carries its lock at `/usr/share/sysroot/rpm-lock.json`;
its signed resolved inputs record `artifacts["rpm-lock"]`, the SHA-256 of the lock
pins (the lock without mirror URLs). A run is a no-change when the source, the base
and these pins equal the signed stable image's.

To rebuild locally from an image's lock:

```sh
sysroot catalog lock extract --image <local image> --output rpm-lock.json
sysroot catalog lock fetch --store /private/rpm-store --lock rpm-lock.json
sysroot catalog lock repo --store /private/rpm-store --lock rpm-lock.json \
  --foundation --output <context>/rpms
```

Then build the foundation context as the release does: `image/Containerfile` with
`image/catalog/install-rpms.sh` next to it, on the lock's `base`. A builder role
needs no image build: `sysroot store add-root --store <store> --image <foundation>
--lock rpm-lock.json --builder <role>` installs it offline over the foundation's
root in the engine sandbox and prints the root a graph's `builder_root` names.
`fetch` downloads from the recorded mirror, then from Koji, and admits only the
locked bytes. `kedra-lab build` resolves a lock with the host `sysroot` and
installs from it in the same way.

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
    architecture = "x86_64";
    foundation = system;
    packages = [hello(builder: c_tools, runtime: system)];
    glib_schemas = true;
    systemd = systemd {
        enable = ["greetd.service", "NetworkManager.service"];
        default_target = "graphical.target";
    };
    skel = true;
    initramfs = initramfs {
        modules = ["bootc", "ostree", "crypt", "dm", "rootfs-block"];
    };
}

target "qemu-arm64" {
    architecture = "aarch64";
    foundation = system;
    fedora = ["qemu-guest-agent", "egl-utils"];
    packages = [hello(builder: c_tools, runtime: system)];
    initramfs = initramfs {
        modules = ["bootc", "ostree", "crypt", "dm", "rootfs-block", "qemu"];
        drivers = ["virtio_gpu", "virtio_input"];
    };
}
```

A target also declares its image steps, which `sysroot derive` applies after
composition ([native artifacts](NATIVE.md)): `glib_schemas` and `skel` are
booleans; `systemd` takes `enable`, `disable` and `mask` lists of concrete units
and an optional `default_target`; `initramfs` takes the dracut `modules` to add
and require and optional kernel `drivers`. The checker applies the engine's step
rules (one operation per unit, concrete unit names, module and driver names).
`sysroot source plan` records the checked steps as `native_steps` in the source
manifest; they are part of the target's package intent and its hash. Both targets
may select source packages; each builds natively on its own architecture.
`architecture` (`aarch64` or `x86_64`) is the target's RPM architecture: it selects
`by_architecture` sources, `sysroot catalog build` and `contribute` refuse it on an
engine of another architecture, and for a target in `image/release/targets.json`
it must be the table's.

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
the strict `packages.lock.json`. A prebuilt upstream package that differs per
architecture names one reviewed archive per architecture; the target's
`architecture` selects one, the others still need their pins:

```text
source = by_architecture {
    aarch64 = archive { url = "https://…/codex-package-aarch64-unknown-linux-musl.tar.gz"; pin = "codex-aarch64"; };
    x86_64 = archive { url = "https://…/codex-package-x86_64-unknown-linux-musl.tar.gz"; pin = "codex-x86_64"; };
};
```
 `files` creates a source tree; optional `files`,
`replace_files` and ordered `patches` prepare a copy before the real build. A
source path cannot be both added and replaced. `build_deps` are build inputs;
`runtime_deps` are build inputs retained in the runtime closure. `env` values are
literals or typed dependency paths. `src`, `out`, `HOME`, `PATH` and `TMPDIR` are
reserved.

`export command`, `export libexec`, `export library` and `export files` name safe
output-relative paths. A package needs a command or libexec export; a library can
export neither. A `config` template becomes typed `SystemFile` content after the build.
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
    "catalog.kedra#tools": "root-<64 lowercase hex digits>"
  }
}
```

## Exports and PATH

`sysroot catalog contribute` gives every command export a typed launcher,
`/usr/share/kedra/catalog-bin/<name>`, that `exec`s the exact store path of the
output (`#!/bin/sh` + `exec "<store>/<path>" "$@"`); the program still sees its real
store path. `/etc/profile.d/kedra-catalog.sh` and
`/usr/lib/environment.d/60-kedra-catalog.conf` put that one directory first on PATH.
Output directories themselves never go on PATH, so only declared exports become
commands. A `libexec` export is a private executable for another program: its
launcher is `/usr/share/kedra/catalog-libexec/<name>`, never on PATH (`sysroot
codex` uses it). `/usr/share/kedra/catalog.json` (schema 2) lists `commands` and
`libexec` with their package, output path, launcher and, for commands, `replaces`.
Global RPM paths are never replaced.

No silent shadowing: `sysroot catalog build` and `contribute` probe the exact
foundation (in the engine sandbox) for each exported command name in
`/usr/local/bin`, `/usr/local/sbin`, `/usr/bin`, `/usr/sbin`, `/bin` and `/sbin`. An
export the foundation already provides is refused unless its package declares it:

```text
export command "jq" = "bin/jq";
replaces = ["jq"];
```

`replaces` may name only the package's own command exports, and naming a command
the foundation does not have is refused too, so a stale declaration does not
survive a foundation change.

## Shipped packages

Both targets select two prebuilt official packages, unchanged, built with the
`tools` builder (coreutils, findutils, glibc-common, grep, gzip, tar; all already
in the foundation, so the role installs nothing):

| Package | Source per architecture | Exports | Runtime libraries |
|---|---|---|---|
| `codex` 0.153.4 (`codex.kedra`) | `codex-package-<arch>-unknown-linux-musl.tar.gz` | `libexec "codex" = "bin/codex"` | `codex_licenses` (license, notices, bubblewrap COPYING, zsh patch, corresponding source re-packed from the pinned commit), `codex_components` (ripgrep, PCRE2, zsh, ratatui licenses committed in `packages/notices/codex/`) |
| `bitwarden` 2026.8.0 (`bitwarden.kedra`) | `bitwarden_2026.8.0_<arm64\|x64>.tar.gz` | `command "bitwarden" = "bitwarden"` and `/usr/share/applications/com.bitwarden.desktop.desktop` | `bitwarden_licenses` (three license files and the client source re-packed from the pinned commit) |

The Codex build checks that the binary runs and reports the declared version; the
Bitwarden build checks with `ldd` that the runtime, `desktop_proxy` and the native
module resolve every library in the builder root (the foundation plus build
tools), and declares the Electron libraries it needs as `runtime_requires`. The
desktop entry's `Exec=` and `Icon=` are typed paths of the exact output. Its
upstream launcher resolves its own directory through `readlink -f`. The x86_64
image now uses the official x64 tarball like arm64, not the RPM; package scripts
and setuid bits never existed in the tarball and cannot enter the store.

## Updating a pin

The SHA-256 pin is the build-time trust: the release downloads each archive and
admits only those bytes. Provenance is verified once, when a pin is added or
bumped, on a Linux machine:

```sh
# Codex: the release workflow's Sigstore signatures on codex, its code-mode host
# and bwrap, with the pinned Cosign 3.1.3, for each architecture.
uv run usr/src/kedra/image/packages/verify-codex.py --version 0.153.4 --architecture aarch64
# Then record the verified bytes (SHA-256 and src- object) in packages.lock.json.
uv run usr/src/kedra/image/packages/pin.py --name codex-aarch64 \
  --url https://github.com/openai/codex/releases/download/rust-v0.153.4/codex-package-aarch64-unknown-linux-musl.tar.gz \
  --sha256 <printed sha256> --sysroot target/release/sysroot
```

`pin.py` extracts the archive with the release's own `image/catalog/prepare-sources.py`
and admits the tree with `sysroot store add-source`, so the recorded object is the
one the build reproduces. Bitwarden publishes no signatures: compare its archive
with the GitHub release asset digest (`gh api repos/bitwarden/clients/releases/tags/<tag>`)
and pass it as `--sha256`. Update the `.kedra` URL and version in the same change.
Verifying at build time instead would put Cosign, its Sigstore trust root and
network access into every release build for no added protection: the pin already
binds the build to the verified bytes, and a mismatch fails the download.

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
archive helper downloads only HTTPS pins, verifies size (256 MiB), deadline (600 s)
and SHA-256, extracts a bounded (1 GiB) gzip/tar tree with restricted member types,
and then admits the resulting tree through the ordinary store workflow. An archive
with a single top-level directory (a source release) contributes that directory;
a flat archive (a prebuilt package with several top-level entries) contributes
the extraction itself. This code is
public pre-signing work; production signers receive authenticated candidate
material and never fetch sources or run recipes.

At deployment time the machine receives a complete signed OCI image, not live
`dnf install` or package builds. Engine profiles and package rollback do not roll
back bootc deployments, home, databases or credentials.

See the [language contract](../../../../.specs/nix-package-dsl/language.md),
[engine API](../crates/sysroot-engine/README.md), [composition](SYSTEM.md),
[architecture](ARCHITECTURE.md) and [actual status](STATUS.md).
