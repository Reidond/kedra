# Kedra language v1

Status: proposed language contract following the owner's approved direction.
Nothing here is implemented. `.kedra` files are the primary authoring format;
Rust implements parsing, checking and lowering. Most normal package definitions
keep their build logic and small resources in one file.

## 1. Files and a mixed package set

Proposed owner inputs live under `usr/src/kedra/image/packages/`, separate from
engine source. `catalog.kedra` explicitly imports individual package/set files;
there is no directory auto-discovery, network import or host-dependent search path.
A reviewed `packages.lock.json` pins archive/tree identities and explicit external
inputs. The existing release resolved-input material records selected Fedora base,
RPM and compiler images. Lockfiles are data, not executable language extensions.

```text
language 1;
namespace "kedra";

import { jq } from "./jq.kedra";
import { sqlite } from "./sqlite.kedra";

foundation system {
    fedora = 44;
    packages = ["niri", "firefox", "pipewire"];
}

builder c_tools {
    base = system;
    packages = ["gcc", "make", "autoconf", "automake", "libtool"];
}

set desktop {
    packages = [
        jq(builder: c_tools, runtime: system),
        sqlite(builder: c_tools, runtime: system),
    ];
}

target "qemu-arm64" {
    foundation = system;
    use = [desktop];
}
```

A `set` groups source-package instances and may add `fedora`/`remove` name lists
for the selected target foundation. `use` combines named sets. A `builder` uses an
explicit foundation as its base and adds compiler RPM requirements without adding
those requirements to the runtime foundation. Package `build_requires` accumulate
into its selected builder before image resolution; `runtime_requires` accumulate
into its runtime foundation. Only reachable selected declarations contribute.
Compiler role cycles and attempts to augment an already frozen exact-image binding
refuse rather than changing its identity behind the caller's back.

The entry module requires one namespace directive; imported definition modules
have none and inherit the entry namespace. Namespace selects names, not permission:
the independent consumer policy must explicitly allow it. A Fedora include/remove
override uses `replace = [replacement { origin = "sets/desktop.kedra#desktop";
name = "firefox"; action = "remove"; }];` in a set/target. Actions are exactly
`include` or `remove`; origin names the existing declaration/request. Replacements
are simultaneous and unique per original request; chains or multiple replacements
of one request refuse. Source-recipe identity conflicts always refuse in v1.

The first examples are deliberately small subsets, not complete replacements for
the existing Fedora lists. Migration must preserve all current selections and
independent required base packages, including target-specific additions.

## 2. Source package with embedded build logic

```text
language 1;

package jq(builder: Builder, runtime: Foundation) {
    version = "1.8.2";
    summary = "JSON processor";
    license = "MIT and bundled notices";

    source = archive {
        url = "https://github.com/jqlang/jq/releases/download/jq-1.8.2/jq-1.8.2.tar.gz";
        pin = "jq-source";
    };

    build_requires = [fedora("gcc"), fedora("make")];
    env = { SOURCE_DATE_EPOCH = "0"; };

    build = shell """
        set -eu
        scratch="$out/.work"
        mkdir -p "$scratch"
        cp -R "$src"/. "$scratch/"
        chmod -R u+w "$scratch"
        find -P "$scratch" -exec touch -h -d "@$SOURCE_DATE_EPOCH" -- {} +
        cd "$scratch"
        ./configure --prefix="$out" --disable-docs --disable-maintainer-mode --disable-shared --enable-static --with-oniguruma=builtin
        make -j2
        mkdir -p "$out/bin" "$out/share/licenses/jq"
        cp jq "$out/bin/jq"
        cp COPYING "$out/share/licenses/jq/COPYING"
        cp vendor/oniguruma/COPYING "$out/share/licenses/jq/ONIGURUMA"
        cd "$out"
        rm -rf "$scratch"
    """;

    export command "jq" = "bin/jq";
}
```

This is illustrative language syntax. Migration copies the complete reviewed
recipe and flags, including path-prefix mapping, rather than treating a shortened
example as already qualified. The jq scratch/timestamp behavior reflects current
observed noexec/generated-file-order constraints. Package build logic remains
inline even when it consists of ordinary shell invoking upstream configure/Make.
The language does not rewrite third-party build systems into language primitives.

## 3. Small sources, generated files, patches and templates

An entirely inline small program is supported:

```text
language 1;

package hello(builder: Builder, runtime: Foundation) {
    version = "1.0";
    summary = "Small inline-source example";
    license = "MIT";
    build_requires = [fedora("gcc")];

    source = files {
        "hello.c" = text """
            #include <stdio.h>
            int main(void) {
                puts("Hello from Kedra!");
                return 0;
            }
        """;
    };

    build = shell """
        mkdir -p "$out/bin"
        gcc -O2 "$src/hello.c" -o "$out/bin/hello"
    """;

    export command "hello" = "bin/hello";
}
```

The following additional package fields are optional, typed forms of the same
resource contract, not evaluation-time execution:

```text
files = files {
    "generated/config.h" = text """
        #define FEATURE_ENABLED 1
    """;
};

patches = [patch {
    strip = 1;
    contents = text """
        --- a/config.h
        +++ b/config.h
        @@ -1 +1 @@
        -#define DEFAULT_LIMIT 10
        +#define DEFAULT_LIMIT 20
    """;
}];

config "/etc/profile.d/hello.sh" = template {
    body = text """
        export PATH="{{hello_bin}}:$PATH"
    """;
    bindings = { hello_bin = path(self, "bin"); };
};
```

`files` adds new source paths; replacing an existing source path uses a separately
named `replace_files` map and requires that exact path already exist as an ordinary
file. It never follows a source symlink or permits a silent overwrite. Apply source
patches in listed order, then overlays in canonical path order; reject a path
selected by both add and replace maps. The patch tool must be present in the exact
builder (e.g. an explicit `fedora("patch")` requirement); it cannot fall back to a
host executable. Unified text patches only in v1: no binary patch, symlink change,
rename or ambiguous path header; use exact, noninteractive application with zero
fuzz and preserve failure. A failed preparation never mutates admitted source.

Templates substitute only named, typed bindings. Missing/extra bindings and bad
paths refuse; no general string expressions or automatic shell escaping. Authors
quote for the destination syntax; path bindings inherit the existing safe store
path grammar and final native configuration validation. `self` is available only
for post-build system contributions; it is forbidden in source preparation/build
inputs because it would create a dependency cycle. `config` is serialized typed
system content, not a write to the running host; existing reserved-path, provenance
and replacement policies still apply.

Text/binary assets too large or independently maintained for inline use may be
referenced through `file("./resources/asset")`. Build scripts may use
`build = script(file("./build.sh"));`. Paths must resolve to admitted, tracked,
ordinary files within the catalog root; the content/mode hash is bound before use.
External references and inline literals lower through the same ResourceEntry
representation. Build-heavy package pilots must not use external script files as
an easy default. A reference to upstream source inside a pinned archive is normal
build input, not an external sidecar exception.

## 4. Grammar and static types

Core grammar outline (the schema table below restricts legal fields/constructors):

```text
module      := 'language' UINT ';' (namespace | import | declaration)*
namespace   := 'namespace' STRING ';'
import      := 'import' '{' IDENT (',' IDENT)* '}' 'from' STRING ';'
declaration := ('foundation' | 'builder' | 'set') IDENT block
             | 'target' STRING block
             | ('package' | 'library') IDENT parameters block
parameters  := '(' (IDENT ':' TYPE (',' IDENT ':' TYPE)*)? ')'
TYPE        := 'Builder' | 'Foundation'
block       := '{' (assignment | export | config)* '}'
assignment  := (IDENT | STRING) '=' value ';'
export      := 'export' ('command' | 'library' | 'files') STRING '=' STRING ';'
config      := 'config' STRING '=' 'template' block ';'
value       := STRING | UINT | 'true' | 'false' | reference
             | '[' (value (',' value)* ','?)? ']'
             | block | IDENT block | ('text' | 'shell') (RAW | STRING)
             | reference '(' arguments? ')'
reference   := IDENT ('.' IDENT)*
arguments   := value (',' value)* | IDENT ':' value (',' IDENT ':' value)*
```

Identifiers are ASCII letters/underscore followed by letters/digits/underscore;
hyphenated external names use strings. Comments use `//` outside strings. All
assignments require semicolons. Quoted strings use JSON escapes; no implicit
coercions, concatenation, interpolation, overloads or ambient variable lookup.
Only the closed built-ins below and declared package/library templates are callable.
Named template arguments are required and checked exactly. Every v1 package/library
must bind its `builder` and `runtime` fields to explicit parameters of Builder and
Foundation types; the example parameter names provide those default field bindings.
No implicit current-target or host-image fallback exists. Mixed named/positional
arguments refuse.

| Construct | Static meaning / restrictions |
|---|---|
| `foundation` | Fedora release, package/remove name sets and explicit policy-compatible image role. |
| `builder` | Base Foundation and compiler package requests. No arbitrary command field. |
| `set` | Explicit `packages`, `fedora`, `remove`, `use`, `replace` collections; cycles refuse. |
| `target` | Independently allowed target name, foundation, sets, packages and explicit request replacements. Command exposure comes from selected exports. |
| `package` / `library` | Bounded named template returning a program package or internal library recipe. Required source, build and metadata; build/runtime roles are explicit parameters. |
| `archive { url; pin; }` | Pinned source descriptor; no fetch while parsing. Git source fetching is deferred in v1. |
| `files`, `text`, `executable(...)`, `file(...)` | Resource tree/content values; files default 0644, explicit executable 0755 only. |
| `shell RAW`, `script(resource)` | Build command content; never executable during planning. |
| `patch { strip; contents; }` | Ordered source preparation input; strip must be nonnegative and smaller than the component count of every affected path, which remains validated after stripping. |
| `replacement { origin; name; action; }` | Override of an existing Fedora request; missing/conflicting origin/action refuses and required base policy still applies. |
| `fedora("name")` | Typed image requirement, allowed in build/runtime requirement lists; no arbitrary DNF arguments. |
| `path(reference, "relative")` | Typed path to a declared dependency/output; lowers to input/output segments, never interpolates a host path. |
| `template { body; bindings; }` | Named substitution into a contribution or generated file; bindings must be available at that phase. |

Library declarations may export libraries/headers/files and have no fake executable
entrypoint. `build_deps` and `runtime_deps` are name-to-instantiated-recipe maps;
the latter contribute to closure retention. `deps.name` refers to such an input,
while builder/runtime refer to template parameters. Recursion/cycles are illegal.
The global import namespace exports only named declarations; unknown/duplicate
exports and shadowed identifiers refuse. Imported modules cannot see the caller's
locals; dependencies enter through explicit parameters. Templates can refer to
imports and their own parameters, never the host target or environment.

V1 does not include general functions, closures, loops, arithmetic, conditionals,
eval, environment lookup, IO built-ins, macros, plugins or an FFI escape hatch.
Target blocks and named sets provide explicit variation. Unsupported features
produce diagnostics rather than being interpreted as shell or Rust code.

## 5. Exact inline-content semantics

Input modules are UTF-8 with LF newlines; CRLF is rejected with a conversion
instruction rather than silently changing source/patch bytes. Literals use a
triple-quote delimiter. The opening delimiter is followed by LF and the closing
delimiter stands alone except for its statement punctuation. Its leading spaces
form the indentation prefix: remove that prefix from each nonempty body line,
reject lines lacking it, remove the opening LF and retain each body line's ending
LF. Empty lines remain LF. Tabs in the indentation prefix refuse. Payload data
containing a delimiter uses an ordinary escaped string/resource reference in v1;
there is no second raw-string dialect.

Raw `text` and `shell` payloads have no language interpolation or escape processing:
`$src`, `$out`, `$(command)` and backslashes stay literal until the isolated shell
runs. A normal quoted string uses the stated JSON escapes to represent exact
content needing escapes or no terminal newline. `template` substitution is a separate explicit
operation over a text value; the formatter may move the enclosing syntax only
when the decoded payload stays byte-identical. It must not reformat shell/C/patch
contents, reorder patches/commands or alter file modes.

Declared resource names use unique ASCII relative POSIX paths, rejecting absolute paths,
NUL, empty/dot/dot-dot components, backslashes, case-fold collisions, duplicate
names and symlink/hardlink source aliases. Modes are 0644 or 0755; no device, FIFO,
setuid or ownership declaration. External files are opened from the admitted
root without following symlinks and rechecked for content/metadata stability.
The independent root admission excludes credentials and personal state before
capture; the language does not scan a home directory to discover resources.
Upstream archive trees retain the existing engine's source-admission contract;
the ASCII restriction is on newly declared resource paths, not a rewrite of
already-qualified source-tree semantics.

Source/modules and their original bytes are audit inputs. Canonical decoded
resource path/mode/content records determine semantic resource identity. Thus a
formatting-only change can change source provenance while retaining an identical
recipe, whereas changing an embedded script, patch, file or mode changes identity.
Filesystem mtimes never substitute for content identity.

## 6. Tooling and compatibility

Proposed CLI surface: `sysroot catalog check`, `fmt`, and `plan --entry catalog.kedra
--target ... --lock packages.lock.json`, with explicit input-root/policy arguments.
Names are proposed. `fmt` writes only explicitly selected files; `--check` emits
status without writes, and no formatter runs automatically during plan/build.
Diagnostics contain reason code, catalog-relative file, line/column and related
locations; default shared diagnostics do not echo arbitrary literal contents.

Every module starts with the supported major language version. Imported versions
must match; unknown majors fail with an upgrade diagnostic. Formatter/parser
versions and resource normalization rules are included in the frontend identity.
No parser fallback to Rust, shell, legacy JSON or package lists is allowed.
Unsupported old input formats keep explicit legacy readers at the source boundary.
