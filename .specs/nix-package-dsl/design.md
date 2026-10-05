# Architecture and migration

Status: specification only. The owner approved a standalone language implemented
in Rust and inline-first package definitions on 2026-10-05. This revision replaces
the previous Rust author-crate design; it does not implement any new frontend.
The syntax/resource contract is maintained once in [language.md](language.md).

## 1. Thin language frontend, existing execution engine

Extend `sysroot-catalog` with a bounded lexer/parser, source spans, type checking,
package-template expansion, formatting and checked lowering. Use a small explicit
recursive-descent parser for the closed v1 grammar; no interpreter VM, procedural
macro platform or general parser/plugin service. Revisit a parser dependency only
with a concrete diagnostic/maintenance justification. Reuse existing Rust models
and serde serialization rather than creating a second build executor.

Proposed file ownership:

```text
usr/src/kedra/crates/sysroot-catalog/
  lib.rs                       existing public catalog/policy API
  intent.rs                    common versioned typed intent and lowering
  language/{lexer,parser,check,format,lower}.rs
usr/src/kedra/image/packages/
  catalog.kedra                explicit entry and imports
  jq.kedra                     metadata, source pin, inline build and exports
  sqlite.kedra                 library/shell declarations and inline builds
  sets/desktop.kedra
  sets/compiler.kedra
  packages.lock.json           reviewed source/resource pins as data
  resources/                   optional large/independently maintained assets
```

All crate entrypoints remain flat. The owner inputs are outside engine Rust
source. No `kedra-packages` author binary, per-package Cargo crate or required
`.sh` sidecar is added. `sysroot` uses the library frontend; package edits leave
frontend/engine executable hashes unchanged. Rust remains a supported secondary
programmatic API to the same validated intent, not a runtime escape from `.kedra`.

The owner approval overrides the earlier no-new-language restriction for this
specific future frontend. T1/T8 must reconcile AGENTS/architecture and affected
skills narrowly during implementation; do not generalize the exception into a new
configuration language for every subsystem or a new deployment backend.

## 2. Pure intent before image/source resolution

The frontend consumes an explicitly admitted source-root snapshot, entry module,
lockfile, target and independent policy. Its loader reads only requested modules
and resources beneath that root using bounded safe file operations. It never
executes modules, shell strings, Rust, DNF, downloads or arbitrary environment
lookups. Import cycles, recursive package expansion, unknown fields/types and
conflicting declarations fail before a buildable graph can be emitted.

Emit PackageIntent v1 plus a bounded ResourceEntry inventory. Builder/foundation
roles remain symbolic until requirements of reachable selected packages, libraries
and source-preparation nodes are collected. Resolve those roles independently,
then bind exact source objects/images into PackageResolution v1 and existing
Catalog/BuildGraph/SystemDefinition values. This breaks the circular dependency
between declaring compiler RPMs and knowing the resulting compiler image digest.
Incomplete resolution cannot execute. Every generated node counts toward the
engine's existing 256-node graph limit; no hidden second graph is exempt.

| Record | Meaning |
|---|---|
| Frontend receipt | Frozen source revision, entry/import/resource/lock inventories, frontend binary/version, language/normalization version and target/policy digests. |
| Semantic intent | Canonical selected declarations, symbolic roles, exact commands/resources, ordered dependencies/patches and typed contributions. No timestamp or absolute checkout path. |
| Resolution | Intent hash, independently approved policy, exact base/builder/runtime images, complete RPM/compiler material and admitted source/resource identities. |
| Realized output | Existing derivation identity plus actual content hashes/closure; never inferred from a display name/version. |

Normalize set-like inputs by identity and retain meaningful sequence order. File
locations and source bytes are audit provenance; diagnostics are not hidden build
inputs. Formatting-only edits preserving decoded payloads keep semantic recipes,
although changed Git/source provenance can still trigger a candidate under the
existing release policy. Do not promise no new OS release for every comment edit.

## 3. Inline resources and build lowering

All inline/external resource content lowers to one representation: normalized
relative path, allowed mode, exact bytes/content hash and origin. Large inline
scripts must not be inserted as `/bin/sh -c` argument strings: the engine has a
32 KiB argument bound. Admit a canonical resource tree as an ordinary source
object, then invoke `/bin/sh -eu` with a typed input path to its script file.
`src` and `out` are reserved, explicitly bound environment values; authors cannot
override them. Other environment values are declared literals or typed paths.
No host environment inheritance is implied by shell syntax.

The resolver computes resource source-object IDs itself from the independently
accepted frozen intent/bytes, then includes those exact IDs in its per-invocation
input policy. An unverified producer cannot add its claimed resource IDs to a
standing allowlist. Wrong resource bytes, input revision or scope refuse before
store admission; policy never comes from the bundle being admitted.

Use separate resource sets per phase: build/preparation content enters the build
derivation, while post-build configuration bodies/bindings enter the system
contribution. A configuration-only template edit changes the contribution identity
without forcing an unrelated binary rebuild or creating a self-output cycle.
Both sets use the same ResourceEntry validation/content rules.

`source = files` produces an admitted immutable source tree. An archive source
retains its independent archive/tree pins. If overlays or patches are selected,
add an explicit source-preparation BuildNode using the same exact approved builder:
verified raw source and the resource tree are read-only inputs; the node creates
an owned writable copy inside its output, applies exact ordered patches and
validated overlays, then publishes a verified prepared tree. The package's real
build node receives that prepared source read-only. No admitted original source
is edited, and failed preparation cannot become an accepted source object.

The preparation driver is fixed, versioned, hash-bound implementation material,
not a user-selected host executable. Patch commands use explicit arguments and
the declared builder's tools. Refuse unsafe paths/link ancestors, absent replacement
targets, duplicate/colliding paths and fuzzy/partial application. Preparation
uses the existing engine/public workflow rather than a new privileged runner.
Language-phase file reads do not perform patches or generate source trees on the
host; materialization belongs to the explicit resource admission/build phase.

Inline templates become typed contributions with literal bodies and checked
bindings. Existing verified input-path/reference and reserved image-path rules
remain authoritative. References to the package's own output are allowed only
in post-build configuration; a source-preparation reference to a future output
would create a dependency cycle and must refuse. Credentials and personal home
content stay outside admitted recipes/resources.

## 4. Fedora adapter and independent policy

A Fedora declaration is a request to the existing isolated foundation/compiler
builder, not an engine package object or an arbitrary DNF command. First release
supports package names, explicit removes, named sets and target selection. Initial
repository/weak-dependency/upgrade policy matches reviewed Fedora 44 fedora/updates
behavior; COPR, third-party repositories, groups, streams and version-range syntax
are deferred. Requested package existence and the final required set are verified.

Independent TargetPackagePolicy binds allowed targets/repos and required base
package names. T1 derives initial required names from reviewed boot/health contracts;
authored remove/replace directives cannot relax them. Include/remove collisions
fail unless an explicit replacement names an existing request, subject to policy.
Fedora and source jq can coexist, but exports/PATH/service references must select
unambiguously and cannot overwrite `/usr/bin` or other protected base files.

For each selected role: freeze source/base/policy; resolve requirements; perform
the existing isolated upgrade/install/remove policy with required repositories
and signature checks; record the complete seven-column RPM inventory plus resolver
and compiler material; compare candidate material against preflight. Missing repo,
solver failure, signature error or drift refuses. No skip-broken, skip-unavailable,
implicit allowerasing or post-hoc rewriting of expected material.

Compiler-only requirements affect the builder role and dependent recipe identity;
runtime requirements affect the selected foundation. BuildGraph closures retain
one compatible runtime foundation. No package operation targets the workstation.
Current source-build support remains native ARM; both targets still require
Fedora migration parity, without borrowing ARM source-build qualification for x86.

A resolution record cannot guarantee old RPM availability from moving servers.
Exact replay needs retained RPM payloads or an exact retained foundation image;
otherwise refuse and require explicit refreshed resolution. No RPM mirror or
retention service is part of this language. The implementation does not replace
DNF's solver or trust package names as proof of actual installed bytes.

## 5. CLI, limits and compatibility

Proposed public check/fmt/plan operations accept an explicit `.kedra` entry and
admitted input root, with policy/target/lockfile as applicable. Preserve legacy
`sysroot catalog ... --catalog` JSON operations through their explicit existing
reader. A new file extension or data payload never causes implicit shell/Cargo
execution. Formatter writes only explicitly selected files, and `fmt --check`
compares formatting without executing builds. Reader and writer modes remain
clear; no auto-format side effect during build or verify.

Enforce NFR-02 independently at load, parse, expansion, resource and output bounds.
Check aggregate size before repeated copies/expansion can exhaust memory. Use the
ordinary CLI's owned-child/resource-lifecycle support for a bounded parse/emit
process; no background daemon/general test runner. Non-executable grammar reduces
attack surface but does not excuse parser recursion, allocation or path checks.
Collect fixed reason codes and relative source locations without echoing raw
literal content. A timeout/refusal is not retried automatically.

Emit into a new private destination and publish a complete receipt/plan using
existing locking/no-replace/fsync semantics. Concurrent creation cannot replace an
accepted plan; interruptions preserve recovery evidence and old artifacts. Plan
validation checks all resources and versions, not just outer JSON syntax.

Unknown language majors or mixed module versions refuse. Existing old source and
Catalog readers retain their versioned contracts; there is no try-new-then-fallback
heuristic. Language/tool/normalization versions participate in provenance and
semantic contracts; migration never silently reinterprets literal content.

## 6. Contribution, release inputs and migration

Generalize catalog contribution to consume verified selected material and policy
rather than a compiled jq/SQLite pair. Template bodies can originate inline but
must lower through the same validated typed configuration model. A third real
source package must work without central-name edits. Preserve protected path,
alias, foundation and provenance checks.

Release preflight binds frozen source, the complete frontend/resource inventory,
frontend binary/version, intent/resolution/pins/policy and actual outputs. Replace
the package-name whitelist with bounded structural validation and independently
selected expected inputs. The production signer still receives authenticated
material under the existing trust/schema/rank rules; it does not parse `.kedra`,
execute recipes or compile Rust. A frontend output cannot grant signing authority.

Use a versioned source-input descriptor pointing to an admitted catalog entry,
not arbitrary executable commands. New revisions have exactly one authoritative
package format. Reject mixed DSL/list authority, while old committed source stays
readable. Derived compatibility lists may exist only in private build context,
never as a second editable checked-in truth. Source inspection can report that
prepared material is needed; it cannot run code merely to inspect an old commit.

First compare legacy and proposed requests and actual foundation material for
both targets. Then migrate jq/SQLite and a real small inline-source program,
including library closure and installed behavior. Their package build logic and
small resources must be inline; keeping legacy `.sh` wrappers would fail the
approved authoring outcome. A justified large external asset still uses a pinned
admitted reference. Cut over lists/embedded owner recipes in one reviewed change
only after parity. Preserve old readers and retained signed source/home provenance.

Rollback selects an older reviewed source/image using current mechanisms; it does
not mutate the workspace lockfile, erase stores or rewind home/databases. The
separate home-artifact plan can consume a future baseline and gains no dependency
or authority from this frontend. Current D1–D5 qualification remains separate.

## 7. Affected files and minimal implementation

| Area | Future work |
|---|---|
| `crates/sysroot-catalog/{lib.rs,recipes.rs,intent.rs,language/*.rs}` | Shared intent, lexer/parser/checker/formatter/lowering; legacy Catalog support; move owner definitions out of Rust. |
| `image/packages/` (new) | `.kedra` package/set files and lockfile; inline build/source/patch/template content by default. |
| `crates/sysroot/{catalog.rs,source.rs,system.rs}` | CLI frontend, admitted input loading, version discriminator, generic contribution and diagnostics. |
| `image/{packages.list,remove.list,targets/*/packages.list}` | Retire as new-format authority only after parity; retain old readers. |
| `image/catalog/{Containerfile,builder.sh,templates/}` | Resolve compiler requirements; migrate small owner templates inline when authored by a package, retaining equivalent typed contributions. |
| `image/{Containerfile,assemble.sh,release/refresh.py,release/material.py,release/compose.py}` | Verified requests and frontend/resource/material bindings, unchanged privilege scope. |
| `.github/workflows/{check.yml,release-target.yml}`, existing CLI/release E2E and container fixtures | Parse/fmt/materialize/build/candidate workflows, signer isolation and real migration coverage. |
| `AGENTS.md`, architecture and relevant first-party skills/manifests, PACKAGES/ENGINE/SYSTEM/STATUS and worklog | Scope the explicitly approved language exception; document actual syntax/support and qualification. No broad policy rewrite. |

No per-package Rust compilation, new core engine, RPM solver, plugin host, LSP,
permanent service or cross-compilation framework is needed for v1. The generic
library can be called by other Rust consumers without executing a language parser
inside a signer or installed privileged helper.

## 8. Risks and deliberate choices

| Risk | Likelihood / impact | Mitigation and case owner |
|---|---|---|
| Parser/import input escapes root or exhausts resources | Medium / high | Closed grammar, bounded loader/expansion and explicit source admission; TC-06/13. |
| Inline formatting changes shell/patch bytes | High / high | Normative decoding, literal-preserving formatter and actual content identity; TC-12/14. |
| Source overlays/patches escape or corrupt input | Medium / high | Separate owned preparation node, safe paths and original-source preservation; TC-12/14. |
| Moving Fedora repositories defeat stale resolution | High / high | Complete material comparison and explicit refresh; TC-04. |
| Compiler requirements depend on their own image ID | Medium / high | Symbolic intent, collect role requirements, then exact lowering; TC-02/03. |
| DSL and lists silently diverge | High / high | Explicit discriminator and real two-target parity; TC-08/11. |
| Generalized contribution weakens trust | Medium / high | Independently selected expected material and structural validation; TC-07. |
| Language grows into arbitrary runtime programming | Medium / medium | Versioned small grammar, no loops/FFI/eval or ambient IO. |
| Trust/recovery scope expands silently | Medium / high | Preserve existing keys/deployment authority; explicit non-coverage. |
| New frontend delays existing delivery | High / medium | Spec-only PR; no implementation added to current delivery gates. |

## 9. Primary references and limits

The existing source evidence is pinned in README.md. External documentation was
checked 2026-10-05 for the initial spec; it establishes upstream behavior, not a
runtime claim for this language:

- [DNF5 install](https://dnf5.readthedocs.io/en/latest/commands/install.8.html): dependency resolution remains DNF's job; explicit-request repository selection alone is not a complete dependency repository allowlist.
- [DNF5 configuration](https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html): solver/dependency/signature settings must be explicit adapter inputs.
- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html): executing Rust author crates would require compilation-time isolation. The approved `.kedra` path avoids author Cargo compilation entirely; this remains relevant to trusted frontend development and independent Rust consumers, not an implicit per-package step.
