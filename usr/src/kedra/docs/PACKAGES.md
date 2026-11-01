# System and package declarations

Kedra declares everything an image contains in `.kedra` files under `system/`:
packages, image files, the home baseline and managed home files, units, image
steps and target metadata ([system declarations](#system-declarations)). There
are two kinds of package. Fedora RPM requests define
the bootc foundation. Source packages build into immutable engine objects and
contribute selected commands, libraries and configuration to the composed image.

| Kind | Declared with | Materialized | Delivered |
|---|---|---|---|
| Fedora foundation packages | `foundation`, `builder`, `set` and `target` declarations | Resolved once per release run into an [RPM lock](#fedora-rpm-lock) of fixed-output store objects; the foundation image and the engine's roots install offline from it | Inside the signed Kedra image, through bootc |
| Source packages | `package` and `library` declarations | Built from exact builder roots into immutable objects below `/usr/lib/sysroot/store`, in the engine's bubblewrap sandbox ([ENGINE](ENGINE.md)) | The selected runtime closures and typed configuration enter composition ([SYSTEM](SYSTEM.md)) |

A machine receives a complete signed OCI image: nothing runs `dnf install` or
builds a package on a deployed system. Engine profiles and package rollback do not
roll back bootc deployments, home, databases or credentials.

## Language v1

This section is the normative contract of the `.kedra` language, the one narrow
language exception the owner approved on 2026-10-06.

### Scope and authority

- Rust (`sysroot-catalog::language`) parses, checks and lowers `.kedra` files as
  data into the existing `sysroot-catalog` and `sysroot-engine` types. Parsing,
  checking, formatting and planning make no network access, compile no Rust, run
  no author code, shell or package build, and write only what a command names
  (the files given to `fmt`, a new `plan --output` archive).
- Inline author shell is opaque bytes until it runs as `build.sh` inside an
  admitted, isolated engine builder ([execution
  boundary](#resource-and-execution-boundary)). The language never evaluates it.
- The language declares packages, sets, Fedora requests, image files, the home
  baseline and managed home files, unit operations, and each target's metadata,
  image steps and foundation edits (owner decision 2026-10-31, widening the
  2026-10-06 package scope). It stays data: it does not replace typed system
  composition, and grants no deployment, installation or signing authority. Installed helpers and production signers
  never parse `.kedra` or run recipes.
- It is not Nix: no Nix syntax or nixpkgs compatibility, no Fedora source
  rebuilds, no third-party repositories, no cross-compilation.

### Versions

Every module begins with `language 1;`. The parser accepts only major version 1 and
refuses any other with an upgrade diagnostic, so every imported module has the
entry's version. Nothing falls back from `.kedra` to Rust, shell, JSON or package
lists. An incompatible change needs a new major version and an explicitly selected
reader. The [source descriptor](#source-selection), the lockfile and the target
policy each carry `schema_version: 1`.

### What the language can express

Six declaration kinds, each a block of `field = value;` assignments:

| Declaration | Required fields | Optional fields |
|---|---|---|
| `foundation NAME` | `fedora` (must be `44`), `packages` (RPM names) | `remove` (RPM names) |
| `builder NAME` | `base` (a foundation), `packages` (compiler-only RPM names) | none |
| `set NAME` | none | `packages` (package invocations), `fedora` and `remove` (RPM names), `use` (sets), `replace` (replacements), `files`, `home`, `systemd` (without `default_target`) ([system declarations](#system-declarations)) |
| `target "NAME"` | `foundation` | everything a set takes, `architecture` (`"aarch64"` or `"x86_64"`), `image`, `hardware`, `os_release`, `hide_autostart` and the [image steps](#image-steps) `glib_schemas`, `systemd`, `skel`, `initramfs` |
| `package NAME(b: Builder, f: Foundation)` | `version`, `summary`, `license`, `source`, `build` | `builder`, `runtime`, `build_requires`, `runtime_requires`, `build_deps`, `runtime_deps`, `env`, `files`, `replace_files`, `patches`, `timeout`, `replaces`; `export` and `config` statements |
| `library NAME(b: Builder, f: Foundation)` | as `package` | as `package` |

Values are JSON-escaped strings, unsigned integers (at most `u64`), `true` and
`false`, lists, `{ name = value; }` maps, references to declarations or
parameters, package invocations such as `hello(builder: c_tools, runtime: system)`,
and these closed constructors and built-ins:

| Form | Meaning |
|---|---|
| `archive { url = "https://…"; pin = "NAME"; }` | A pinned archive; `url` must equal that pin's URL in the lockfile. Nothing is fetched while parsing. |
| `by_architecture { aarch64 = archive {…}; x86_64 = archive {…}; }` | One reviewed archive per RPM architecture. The target's `architecture` selects one; every archive named must still be pinned. |
| `files { "relative/path" = CONTENT; }` | A declared source tree. |
| `text LITERAL`, `file("./path")`, `executable(CONTENT)` | File content: an inline literal, an admitted file inside the catalog root, or either with mode 0755 instead of 0644. |
| `shell LITERAL`, `script(CONTENT)` | A build script, inline or from content. |
| `patch { strip = N; contents = CONTENT; }` | One unified-diff patch, applied in list order. |
| `replacement { origin = "…"; name = "…"; action = "include" \| "remove"; }` | Changes an existing Fedora request ([selection](#selection)). |
| `template { body = text …; bindings = { … }; }` | A configuration file with named `{{binding}}` placeholders. |
| `fedora("name")` | A Fedora requirement in `build_requires` or `runtime_requires`. |
| `path(REFERENCE, "relative")` | A typed path into `self` (configuration only) or a declared dependency `deps.NAME`. |
| `source_revision` | The independently verified source commit; configuration templates only. |
| `systemd { … }`, `initramfs { … }` | Target image steps; a set's `systemd` adds unit operations. |
| `managed { content = file("./…"); check = [...]; reload = [...]; }` | A home file `sysroot home` manages ([HOME](HOME.md)). |
| `os_release { name = "…"; pretty_name = "…"; }` | The names the foundation's os-release gives the image. |

Packages publish outputs with `export command "NAME" = "relative/path";`,
`export libexec`, `export library` and `export files`, and configuration with
`config "/absolute/path" = template { … };`.

### What the language cannot express

No functions, closures, loops, recursion, arithmetic, conditionals, `eval`, string
concatenation or interpolation, environment or host lookup, IO built-ins, macros,
plugins or FFI. No directory discovery, remote imports or search paths. No
arbitrary DNF arguments, package groups, version ranges, COPR or other
repositories (the target policy admits exactly `fedora` and `updates`), no Fedora
release other than 44, no Git sources. Unknown declarations, fields, constructors
and types are refused with a diagnostic, never interpreted. Shell text such as
`$src`, `$out` or `$(command)` stays literal bytes.

### Modules, imports and namespace

The entry module (`catalog.kedra`) contains exactly one `namespace "NAME";`
directive, which must equal the target policy's namespace; imported modules have
none and inherit it. A namespace selects names, not permission: the independent
policies still decide what is allowed. `import { a, b } from "./other.kedra";`
names explicit declarations from a path relative to the importing module, inside
the admitted catalog root. Imports are not re-exported, an imported name cannot
shadow a local declaration, and import cycles are refused.

Identifiers are ASCII letters, digits and `_` (not starting with a digit), at most
128 bytes; targets and other hyphenated names are quoted strings. `//` starts a
comment. Every assignment ends with `;`. Duplicate fields, declarations, imports,
exports and configuration paths are refused. Every declaration is checked, also
unselected ones; only those reachable from the selected target contribute.

### Packages and libraries

- A package or library has exactly one `Builder` and one `Foundation` parameter.
  Invocations pass both by name; positional or mixed arguments are refused. The
  optional `builder =` and `runtime =` fields pick the role parameters when they
  are named differently. Parameters cannot shadow a declaration or a reserved name
  (`self`, `deps`, `src`, `out`, `source_revision` and the built-ins).
- The runtime role must be the target's foundation, and the builder's `base` must be
  that same foundation: a target has one runtime foundation.
- `build_requires` add to the builder role only; `runtime_requires` add to the
  target's foundation requests. When a package uses `files`, `replace_files` or
  `patches`, the checker adds `coreutils`, `findutils` and `python3` (and `patch`
  for patches) to its builder role.
- `build_deps` and `runtime_deps` map aliases to package invocations. Both are
  build inputs; runtime dependencies also stay in the runtime closure. An alias
  appears once across both maps. Recursive packages and dependency cycles are
  refused. A package invoked again with the same roles is the same instance; other
  roles are refused.
- `env` values are literals (at most 4,096 bytes) or `path(deps.NAME, "…")`. `src`,
  `out`, `HOME`, `PATH`, `TMPDIR`, `source` and `resources` are reserved. `self` and
  `source_revision` are not available while building, since they would form a
  cycle.
- `timeout` is 1 to 3,600 seconds, default 600.
- Export names are RPM-style names and paths are safe output-relative paths;
  `command` and `libexec` paths use only `A-Za-z0-9+._-/` because a launcher runs
  them verbatim. A package needs a `command` or `libexec` export. A library has
  neither, exports a `library` or `files` path, and cannot be selected as a target
  package. `replaces` names only the package's own command exports, once each
  ([exports and PATH](#exports-and-path)).
- A `config` path is absolute and must be an allowed system configuration path. Its
  body is `text`; every `{{name}}` must have a binding and every binding must be
  used. A binding is a literal, `path(self, "…")`, `path(deps.NAME, "…")`, a path
  into another declaration the contribution includes, or `source_revision`. There
  is no escaping and no expression syntax: authors quote for the destination
  format. After the build, the template becomes typed `SystemFile` content (mode
  0644) with its paths as typed store references.

### Selection

Selecting a target collects its foundation's `packages` and `remove`, then each
`use`d set once (depth first), then the target's own `fedora`, `remove` and
`packages`. Every request remembers its origin, `<module>#<declaration>`.
Replacements apply together, each to an existing request with that exact origin
and name; a missing request, a chained or a duplicate replacement is refused. A
name both included and removed is refused with both origins. The final requests
must contain every `required_packages` name of the target policy, and two
selected packages exporting the same name are refused. Kedra's source plan also
refuses removing the foundation's own boot packages (`bootc`, `glibc`, `rpm`,
`dnf5`, `kernel`, `kernel-core*`, `kernel-modules*`, `shim`, `shim-*`, `grub2`,
`grub2-*`).

### Image steps

A target declares the image steps that `sysroot derive` applies after composition
([NATIVE](NATIVE.md)): `glib_schemas` and `skel` are booleans; `systemd { enable;
disable; mask; default_target; }` takes lists of concrete unit names and an
optional default target; `initramfs { modules; drivers; }` takes the dracut
modules to add and require (mandatory) and optional kernel drivers. The checker
applies the engine's step rules (one operation per unit, concrete unit names,
valid module and driver names). The language allows a target without steps;
Kedra's source plan requires all four for each of its targets, an initramfs with
at least `bootc`, `ostree`, `crypt`, `dm` and `rootfs-block`, and for `qemu-arm64`
also the `qemu` module and the `virtio_dma_buf`, `virtio_gpu` and `virtio_input`
drivers. The units of the sets a target uses join its own `systemd` step (the
same operation twice is one; conflicting operations are refused), and a target
whose managed home files declare a `check` gets a `home_checks` step that runs
those checks on the image baseline. `sysroot source plan` records the checked
steps as `native_steps` in the source manifest; they are part of the target's
package intent and its hash.

### System declarations

Sets and targets declare what the image contains besides packages:

```kedra
set login {
    files = {
        "/etc/greetd/config.toml" = text """
            [terminal]
            vt = 1
            """;
        "/usr/libexec/kedra-session" = executable(text """
            #!/bin/sh
            exec /usr/bin/niri-session
            """);
        "/usr/share/doc/kedra/START-HERE.md" = file("./doc/START-HERE.md");
    };
    home = {
        ".config/niri/config.kdl" = managed {
            content = file("./home/niri/config.kdl");
            check = ["/usr/bin/niri", "validate", "--config", "{file}"];
        };
    };
    systemd = systemd { enable = ["greetd.service"]; };
}
```

- `files` maps absolute image paths under `/etc` or `/usr` to content: inline
  `text`, an admitted `file("./…")`, or either wrapped in `executable(…)` for mode
  0755. `/etc/skel`, `/usr/etc`, `/usr/src` and `/usr/share/sysroot` are refused;
  Kedra's source plan also refuses credential paths (`/etc/shadow`, private SSH
  host keys) and the generated `/usr/share/kedra/home-files.toml`.
- `home` maps home-relative paths to content (seeding new accounts) or to
  `managed { content; check; reload; }`. A managed file's content must be a
  `file("./…")` resource, which `sysroot home capture` writes back to. `check`
  and `reload` are argv lists with an absolute program; `{file}` in `check` is
  the file to check. Private home paths are refused as for resources.
- A path declared twice, by the target or any set it uses, is refused with both
  origins: modules compose; they never override each other. A target's own
  hardware or test settings belong in a set only that target uses.
- `image` and `hardware` give the target's published repository and hardware
  status; Kedra's source plan requires both, the release table's architecture and
  repository, and builds the manifest's target record from them.
- `os_release { name; pretty_name; }` and `hide_autostart = { "entry.desktop" =
  ["niri"]; }` are foundation edits of RPM-owned files: `image/assemble.sh`
  reads them from the source manifest after the RPMs are installed. Values are
  printable text without quotes, backslashes or expansion characters.

The source plan materializes these into the payload (home files below
`/usr/share/sysroot/home/default/`), generates `/usr/share/kedra/home-files.toml`
from the managed entries, and records each file's source: the resource it came
from, or the module of an inline literal, with the declaring set as `origin`.

### Literals, resources and identity

Modules are UTF-8 with LF line ends; CRLF or NUL is refused with a conversion
hint, never silently rewritten. A triple-quoted literal starts with `"""` and a
line feed and ends with a line holding only spaces and `"""`; those spaces are the
indentation prefix, removed from every non-empty line (a line lacking it is
refused), and every line keeps its LF. Only `;`, `,`, `]`, `}`, `)` or
whitespace may follow the closing `"""`. A payload containing `"""` must use an
ordinary escaped string. Literals have no interpolation or escape processing.

Declared resource paths are unique relative ASCII POSIX paths without empty, `.`,
`..` or `.git` components, backslashes, `:` or control characters, and without
case-insensitive collisions. Paths through `.ssh`, `.codex`, `.claude` or `.cache`,
files named `auth.json`, `credentials.json` or `.credentials.json`, and content
containing a private-key marker are refused. Modes are 0644 or 0755 only; no
symlinks, devices or ownership. `file("./path")` resolves relative to its module
and must stay inside the catalog root.

The decoded path, mode and bytes of each resource determine its identity. A
formatting-only edit keeps every recipe identity (it still changes the source
commit and its recorded input hashes); changing a script, patch, file or mode
changes the recipe. File timestamps never count. `sysroot catalog fmt` sets a
canonical layout, keeps comments and every decoded literal byte, and refuses a
result whose tokens differ from the input.

### Limits

At most 128 modules, 8 MiB per module, 32 MiB in total, 4,096 declared resources,
nesting depth 64, 262,144 tokens or values, 256 package instances and an 8 MiB
intent; the lowered graph must fit the engine's 256-node, 8 MiB limits. Diagnostics
give a reason code, the catalog-relative file, line, column and related locations,
and never echo literal contents.

The CLI runs parsing and planning in an owned data-only child process with a
60-second deadline, a 512 MiB address-space limit on Linux (macOS: resident memory
sampled every 100 ms) and a 1 MiB diagnostic cap; on timeout or excess it kills
and reaps the child, and the child exits when its owner dies. It opens modules,
locks and resource files without following symlinks, as single-link ordinary
files, and checks that they did not change during admission. Direct Rust
consumers of the library supply their own input admission and process limits.

### Lock and target policy

`packages.lock.json` is strict JSON: `schema_version: 1` and `sources`, a map from
pin name to `{url, sha256, object}`. `url` is HTTPS and must equal the
declaration's; `sha256` is the archive digest; `object` is the reviewed `src-`
identity of the normalized tree. At most 256 pins; two pins with one object must
agree on URL and digest. Inline and file resources need no pins: their identities
are computed from the admitted bytes. Nothing updates the lockfile during
planning or building ([updating a pin](#updating-a-pin)).

The target policy (`image/package-policy.json`) is separate strict JSON:
`schema_version: 1`, `namespace`, `targets`, `repositories` (exactly `fedora` and
`updates`) and `required_packages`. Input data cannot grant itself a policy. Exact
builder roots, runtime roots, packages and source objects are allowed by a second,
independently selected catalog policy (`--policy`).

## Source selection

The committed `usr/src/kedra/image/package-inputs.json` names the language and
its catalog root, `system/`:

```json
{
  "schema_version": 1,
  "format": "kedra",
  "entry": "system/catalog.kedra",
  "lock": "system/packages.lock.json"
}
```

The source plan reads the descriptor, the target policy, the lock and every
imported module and resource as raw committed blobs, with no checkout, Git filter,
recipe or network access. A source without the descriptor is refused; there is
no package-list reader and no other source of image files. Kedra's source
plan also requires the policy's namespace `kedra`, exactly the targets `desktop`
and `qemu-arm64`, and the required base packages (`niri`, `noctalia`, `greetd`,
`tuigreet`, `NetworkManager`, `pipewire`, `wireplumber`, `polkit`, `gnome-keyring`,
`gnome-keyring-pam`). It records the frontend inputs, their hashes, the builder
requests, the selected packages and the intent hash (`package_frontend`) with
`native_steps` in the source manifest, `/usr/share/sysroot/source.json`, and
for each image file its destination, mode, hash, source file and declaring set.

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

Each engine root records its actual seven-column RPM inventory, observed in the
sandbox when it was made. `sysroot catalog build` and `contribute` refuse a
runtime root that lacks a requested
Fedora package or holds a requested removal, and a builder root that lacks its
compiler requests or the foundation's requests. Package names and pins do not
prove installed bytes; the observed inventory does.

To rebuild locally from an image's lock:

```sh
sysroot catalog lock extract --image <local image> --output rpm-lock.json
sysroot catalog lock fetch --store /private/rpm-store --lock rpm-lock.json
sysroot catalog lock repo --store /private/rpm-store --lock rpm-lock.json \
  --foundation --output <context>/rpms
```

Then build the foundation context as the release does: `image/Containerfile` with
`image/catalog/install-rpms.sh` next to it, on the lock's `base`. Packages need no
image build: `sysroot store add-root --store <store> --image <base> --lock
rpm-lock.json` installs the lock's foundation role offline over the retained base's
root in the engine sandbox (the runtime root a graph's `runtime_root` names), and
`--builder <role>` installs that builder role over it in turn (a `builder_root`).
`fetch` downloads from the recorded mirror, then from Koji, and admits only the
locked bytes. `kedra-lab build` resolves a lock with the host `sysroot` and
installs from it in the same way.

## Public tooling

Commands that read the language require an explicit input root, entry, target,
lock and target policy. The formatter accepts only the files it is given.
Data-producing commands print JSON on stdout and failures on stderr.

```sh
sysroot catalog check \
  --input-root system \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json

sysroot catalog fmt --check system/*.kedra system/packages/*.kedra
sysroot catalog fmt system/catalog.kedra

sysroot catalog pins \
  --input-root system \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json

sysroot catalog plan --package codex --policy /private/catalog-policy.json \
  --input-root system \
  --entry catalog.kedra --target qemu-arm64 \
  --lock packages.lock.json \
  --target-policy usr/src/kedra/image/package-policy.json \
  --resolution /private/exact-roots.json --output /private/codex-plan.tar
```

`check`, `pins` and graph planning touch no store or builder. `fmt` writes only
the named files; `--check` writes nothing. `plan --output` publishes a new tar
archive atomically and refuses an existing destination; the archive holds
`graph.json`, the admitted resource objects and `frontend.json`. `resolve` prints
the selected metadata and identities. `build` adds `--store` and executes the
resolved graph after checking the RPM inventories the roots recorded
([RPM lock](#fedora-rpm-lock)); with `--cache`/`--trusted` it substitutes recorded
closures first ([binary cache](ENGINE.md#binary-cache)). `contribute` requires
already realized outputs, canonical independently hashed release input material,
an exact lowercase source commit and a new private output directory
([SYSTEM](SYSTEM.md#plan-and-compose)).

Exact roots are resolved separately from the authored requests:

```json
{
  "runtime": "root-<64 lowercase hex digits>",
  "builders": {
    "catalog.kedra#tools": "root-<64 lowercase hex digits>"
  }
}
```

In a release both are made from the RPM lock over its exact Fedora base, so a
package's identity changes only with the base digest, the locked RPMs or the
package's own inputs, never with the per-run Kedra foundation image. `contribute`
and `sysroot system compose` accept the outputs only over a foundation whose RPM
inventory equals the runtime root's exactly.

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

No silent shadowing: in the engine sandbox, `sysroot catalog build` probes the
runtime root (the foundation's locked RPMs) and `contribute` the complete
foundation image for each exported command name in `/usr/local/bin`,
`/usr/local/sbin`, `/usr/bin`, `/usr/sbin`, `/bin` and `/sbin`. An export the
foundation already provides is refused unless its package declares it:

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
in the foundation role, so the builder role installs nothing):

| Package | Source per architecture | Exports | Runtime libraries |
|---|---|---|---|
| `codex` 0.153.4 (`codex.kedra`) | `codex-package-<arch>-unknown-linux-musl.tar.gz` | `libexec "codex" = "bin/codex"` | `codex_licenses` (license, notices, bubblewrap COPYING, zsh patch, corresponding source re-packed from the pinned commit), `codex_components` (ripgrep, PCRE2, zsh, ratatui licenses committed in `packages/notices/codex/`) |
| `bitwarden` 2026.8.0 (`bitwarden.kedra`) | `bitwarden_2026.8.0_<arm64\|x64>.tar.gz` | `command "bitwarden" = "bitwarden"` and `/usr/share/applications/com.bitwarden.desktop.desktop` | `bitwarden_licenses` (three license files and the client source re-packed from the pinned commit) |

The Codex build checks that the binary runs and reports the declared version; the
Bitwarden build checks with `ldd` that the runtime, `desktop_proxy` and the native
module resolve every library in the builder root (the locked foundation RPMs
plus build tools), and declares the Electron libraries it needs as `runtime_requires`. The
desktop entry's `Exec=` and `Icon=` are typed paths of the exact output. Its
upstream launcher resolves its own directory through `readlink -f`. Both
architectures use the official tarball, not an RPM; package scripts and setuid bits
do not exist in a tarball and cannot enter the store.

jq, SQLite and hello are not shipped. They remain the language's source-build
fixtures in `crates/sysroot/tests/fixtures/catalog`, which also shows a deliberate
`replaces` of Fedora's jq.

## Updating a pin

The SHA-256 pin is the build-time trust: the release downloads each archive and
admits only those bytes. Provenance is verified once, when a pin is added or
bumped, on a Linux machine:

```sh
# Codex: the release workflow's Sigstore signatures on codex, its code-mode host
# and bwrap, with the pinned Cosign 3.1.3, for each architecture.
uv run usr/src/kedra/image/packages/verify-codex.py --version 0.153.4 --architecture aarch64
# Then record the verified bytes (SHA-256 and src- object) in system/packages.lock.json.
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

## Resource and execution boundary

Every inline or file resource lowers to `SourceFile { bytes, executable }`, and
`source_identity` computes the canonical `src-...` identity of each resource tree
from its relative paths, modes and bytes. The resolver computes these identities
itself and allows them only for that invocation; a producer cannot add claimed
identities to a standing policy. The build script becomes the resource `build.sh`,
run as `/bin/sh -eu <typed path>` with `src` and `out` bound as typed store paths,
rather than as a command-line argument (the engine bounds arguments at 32 KiB).
An inline source tree becomes its own admitted source object.

Patches and overlays add a separate preparation node in the same builder. It
receives the original source and the recipe resources read-only, runs the fixed
`prepare.py` that ships inside `sysroot-catalog` with the builder's own
`/usr/bin/python3`, and emits a new prepared tree that the package build consumes.
It applies patches in order with `patch --batch --forward --fuzz=0` (any offset or
fuzz fails), then `files` additions and `replace_files` replacements in path order.
A patch must name one existing ordinary file per header pair, and its `strip`
must leave a valid path. An addition must not exist yet; a replacement must
already be an ordinary single-link file; a path cannot be in both maps. The
admitted original is never changed, nothing falls back to host tools, and a failed
preparation publishes nothing.

Release source acquisition is likewise separate from author shell. The reviewed
archive helper (`image/catalog/prepare-sources.py`) downloads only HTTPS pins,
bounds size (256 MiB) and time (600 s), verifies the SHA-256, extracts a bounded
(1 GiB) gzip/tar tree with restricted member types, and admits it through the
ordinary store workflow. An archive with a single top-level directory (a source
release) contributes that directory; a flat archive (a prebuilt package with
several top-level entries) contributes the extraction itself. This is public
pre-signing work; production signers receive authenticated candidate material and
never fetch sources or run recipes.

See the [engine API](../../../../crates/sysroot-engine/README.md), [composition](SYSTEM.md),
[architecture](ARCHITECTURE.md) and [current status](STATUS.md).
