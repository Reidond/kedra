# Design

Status: proposed. All APIs, paths marked new, and command additions below describe
future implementation. They are not executable examples of the current CLI.

## 1. Authoring surface and file ownership

Extend the existing `sysroot-catalog` crate with generic authoring types and
lowering. Keep `sysroot-engine` responsible for its existing graph/store behavior.
Add one small flat workspace binary crate, `kedra-packages`, justified as the
independently compiled owner of Kedra's definitions. The CLI must not statically
embed every recipe or template from that crate. Another project can author its
own crate against the same public API; a second package registry is unnecessary.

Proposed layout within `usr/src/kedra/crates/`:

```text
sysroot-catalog/
  lib.rs              existing public model and policy
  author.rs           new ergonomic constructors, semantic validation and lowering
kedra-packages/
  Cargo.toml          explicit main.rs; one member of the existing workspace
  main.rs             bounded plan emitter
  lib.rs              package-set declarations and target selection
  packages/jq.rs
  packages/sqlite.rs
  sets/desktop.rs
  sets/compiler.rs
  build/jq.sh
  build/sqlite-library.sh
  build/sqlite-shell.sh
  pins.json            reviewed URLs/archive hashes/tree identities as data
```

Templates stay ordinary files under `image/catalog/templates/`; the frozen author
input inventory binds their bytes. `include_str!` may embed fixed scripts in the
author executable, but no definition is embedded into `sysroot` merely for
convenience. Modifying a recipe can recompile the small author program; an existing
compatible engine/CLI binary must then accept its serialized output unchanged.

Use functions/builders first. A small `macro_rules!` convenience macro may be
added only if it preserves the same types/errors; procedural macros and a custom
parser are unnecessary for the first release. This is compatible with the owner's
request for a custom Rust DSL and the repository's no-new-language contract.

### Illustrative surface, not compiled code

```rust,ignore
pub fn desktop() -> PackageSet {
    package_set("kedra")
        .fedora(FedoraSet::system().include(["niri", "firefox", "pipewire"]))
        .compiler("c-tools", FedoraSet::compiler().include(["gcc", "make"]))
        .source(jq::package())
        .source(sqlite::library())
        .source(sqlite::shell())
        .for_target(Target::QemuArm64, arm_additions())
        .expose("jq", output("jq", "bin/jq"))
        .expose("sqlite", output("sqlite", "bin/sqlite3"))
}

pub fn package() -> SourcePackage {
    source_package("jq", "1.8.2")
        .source(archive_pin("jq-source"))
        .builder("c-tools")
        .runtime("system")
        .script(include_str!("../build/jq.sh"))
        .program("bin/jq")
}
```

`Target` above belongs to the Kedra author crate; the generic API receives an
explicit validated target descriptor. It must not choose a target from the build
machine's `cfg!`, environment or CPU. The SQLite shell declares a typed runtime
reference to its library node; a build-only tool uses a distinct dependency
constructor. A script is executable recipe content, not a trusted declarative
value: it runs only later under the existing package executor restrictions.

Distinguish selectable packages from internal library/tool recipe nodes in the
author model. The SQLite library has a typed output but no invented executable
entrypoint. Lower it into the shell package's dependency graph; only selectable
program packages need the current `Catalog::Package` program field. Shared nodes
retain the same derivation identity when referenced by multiple package roots.
Do not broaden the legacy catalog's runnable-package contract to represent them.

Authors can review emitted summaries and pins, add a package module, select it
in a set and qualify it without altering engine algorithms. Source-version edits
and pin changes are an explicit paired review; the resolver never silently
replaces a mismatched pin. Patches are ordered, separately hashed inputs applied
to disposable source copies. Their resulting recipe/source identity is recorded.

## 2. Intent before resolved identities

Do not require a builder image digest before the DSL can describe the packages
needed to build that image. Emission produces **PackageIntent v1**, with symbolic
roles (`system`, `c-tools`), source pins, target package sets, dependency graph,
typed exposure/template references and an explicit format version. It performs
no DNF transaction or engine build.

The adapter resolves image roles and admitted sources, producing **PackageResolution
v1**. A checked lowering step binds exact images/source objects and emits existing
`Catalog`, `BuildGraph` and system contributions. Incomplete resolution is not a
buildable graph. The new data envelope is outside the existing release signer
schema and becomes hash-bound input material under the existing trust contract.

Identity distinctions:

| Record | Meaning and required binding |
|---|---|
| Author input receipt | Frozen Git revision, complete admitted crate/dependency/script/template/pin inventory, Cargo.lock/toolchain and author binary hashes. |
| Semantic intent | Canonical target, package selections, sources, roles, recipe commands and references; excludes timestamps and absolute checkout paths. |
| Resolution | Semantic-intent hash, independent policy digest, exact base/compiler/runtime images, actual RPM inventories and admitted source objects. |
| Realized output | Existing recipe identity plus actual output content hashes and runtime closure; never inferred solely from a package name/version. |

Normalize sets by stable keys and reject duplicates with unequal values. Preserve
ordered argv, shell bytes and patch order. Locations used for diagnostics are
review provenance, not a hidden build input; package module/script bytes still
participate in the author receipt. A changed author receipt requires preflight
reassessment even when semantic intent is unchanged, while no-change decisions
must not be driven by timestamps or fresh diagnostic IDs.

## 3. Fedora adapter

Fedora declarations are typed requests for the existing foundation builder. They
are not ordinary `BuildNode` objects and do not expose an arbitrary DNF argument
escape hatch. First release supports package names, explicit remove intents,
named sets and target selection. Repository configuration, weak-dependency and
upgrade policy are explicit adapter inputs initially matching the reviewed
Fedora 44 `fedora`/`updates` behavior. Repository additions, COPR, module streams,
groups, arbitrary provides expressions and per-package version ranges are deferred.

The adapter's independent `TargetPackagePolicy` carries required base package
names alongside repository and target permissions. T1 derives its initial values
from the existing reviewed boot/health requirements; author declarations cannot
relax them. It is passed separately from PackageIntent and hashed into resolution.
An intentional platform-policy change is reviewed separately from a package edit.

For each target/system/compiler role:

1. Freeze source and the independently approved exact Fedora base/repository policy.
2. Resolve shared plus selected-target declarations. Include/remove collision
   fails unless an explicit replacement names the exact prior intent. A removal
   cannot waive the platform's independently maintained required base packages.
3. Run the existing isolated upgrade/install/remove sequence with required
   repositories available and signature enforcement enabled. No `skip-broken`,
   `skip-unavailable` or implicit `allowerasing`; verify the final required set.
4. Record the complete installed seven-column RPM inventory, including name,
   epoch/version/release/architecture and header/payload hashes, plus resolver
   version, repository policy and base identity. Capture actual compiler tool
   material as the existing pipeline does.
5. Rebuild/accept a candidate only if its complete material matches preflight.
   If repositories moved, fail and require a new explicit resolution; never
   rewrite the accepted record to fit the candidate.

This first release preserves observed-material verification. A record alone is
not a promise that upstream servers retain old RPMs forever. Exact offline replay
requires retained RPM payloads or a retained exact foundation image; otherwise
the old resolution becomes unavailable and a refresh is needed. A new RPM mirror,
retention service or home package installer is not part of this project.

Pin both system and compiler roles; a compiler RPM refresh must affect dependent
engine recipes even if the application source/version is unchanged. Runtime
Fedora requirements must resolve in the chosen runtime foundation, and build-only
ones in the builder. Engine runtime closures still require a compatible common
foundation. An author cannot request live package installation on the workstation.

## 4. Compilation/evaluation is executable code

Rust type checking does not make author code pure or harmless. Cargo build scripts
and dependencies may execute code during compilation. Prepare reviewed Cargo
dependencies separately using the pinned lockfile, then compile and run authoring
inside a disposable native build environment with network disabled, fixed declared
environment, read-only admitted source/dependencies, private bounded output/scratch,
no host home/keys/agent sockets/daemon socket/devices, and no production signing
authority. Use the existing sanctioned container/lab infrastructure, not a second
general runner. The host controller may own Docker; the author process cannot.

Use `cargo --frozen` in that prepared environment; it is a dependency/network
constraint, not a sandbox. In-tree author crates have no custom build.rs or
procedural macro extension point initially; dependencies still receive the same
isolation. Host-target conditionals cannot silently choose the deployment target.
Bound compile/evaluation/output as NFR-02 and kill/reap only owned children on
timeout. Validate the full output as data; success exit alone is insufficient.

Emit to a new private directory. Write a single complete plan and receipt, fsync
and publish through existing no-replace/locking primitives. A failed evaluation
or concurrent destination collision leaves an old accepted plan intact. Repeat
evaluation in identical admitted environments must match semantic bytes before
acceptance. Raw diagnostics are bounded private evidence; shared errors use fixed
stage, package, source location and reason codes without arbitrary secret output.

No general-purpose "pure Rust" verifier is proposed. Restricting inputs and
comparing evaluations is the practical boundary, with nondeterminism still a
qualification concern. Never execute authoring code in the privileged helper,
production signer or during ordinary data-only plan/verify operations.

## 5. CLI, contribution and release integration

Keep existing public `sysroot catalog list/plan/resolve/build --catalog ...`
behavior for legacy Catalog data. Add a separately explicit author command path
(proposed `sysroot catalog author`) that prepares the sanctioned evaluation; no
implicit Rust execution occurs because a JSON input happens to contain a path.
Expose a data-only intent validation/summary and a separate resolution operation;
command spelling is a usability decision, but the three phases cannot collapse
into a hidden side effect. Explicit source acquisition happens only in the
authorized resolver, before offline package building.

Generalize `catalog contribute` to accept verified resolved author material and
independent policy. Derive outputs and allowed template references from a bounded
validated inventory instead of `['jq', 'sqlite']`. Version the new envelope;
preserve the legacy schema reader and reject unknown fields/versions. Templates
keep typed input references and existing reserved-path/alias checks. Replacing
Fedora global binaries is forbidden; exposing a store binary via an explicit
profile/PATH/service reference is supported. Two packages cannot claim one alias.

Replace the closed package-name expectations in release material with structural
validation plus exact frozen author-input/plan/resolution hashes. This does not
mean accepting producer-supplied authority or arbitrary source paths. Preflight
must independently verify the source inventory, executable and policy before
building. Candidate checks verify actual output bytes, RPM material and closures.
The protected signer still verifies existing source/rank/target/image identity
without running the author, building Cargo or gaining a new key environment.

## 6. Incremental migration and compatibility

Retain old parsers/source-layout resolution for old commits. New source revisions
select the author format through a small explicit versioned input descriptor
under the existing image development tree. The descriptor supplies data about the
author crate and package-set entry, not executable command strings or permissions.
The independently approved caller admits it. DSL presence alone is not activation.

The migration must update the closed source resolver to understand that format,
the required input inventory and the new crate location. Reject mixed-authority
new revisions rather than unioning DSL with editable lists. Temporary generated
legacy-compatible lists may exist only in private build context, are marked as
derived, and cannot be checked-in authoritative inputs. A resolver does not execute
Rust merely to inspect a historical Git commit; it consumes the independently
prepared receipt/material for the new format or reports that preparation is needed.

First compare legacy and proposed intent on both targets without changing the
release route. Then exercise both actual foundation builds and the ARM catalog
path. Convert the checked-in shared/target lists and built-in jq/SQLite definitions
in one intentional source cutover; do not leave a second mutable copy. Desktop
keeps its existing Fedora path; unsupported source-built execution on x86 must
refuse rather than borrow ARM results. Retained old commits stay resolvable and
their signed image identities/home provenance remain unchanged.

Rollback of an authoring deployment means choosing the previous source/image via
existing reviewed mechanisms. It does not downgrade the workspace lockfile in
place, erase stores, reinterpret unknown schemas or rewind live home/databases.
The separate home-artifact plan may consume a future verified baseline; this DSL
adds no home-apply authority and no dependency on that future feature.

## 7. Affected files and implementation boundaries

| Area | Expected change, after separate implementation authorization |
|---|---|
| `crates/sysroot-catalog/{lib.rs,recipes.rs,author.rs}` | Add generic author model/lowering; move owner recipes out; retain legacy model/policy. |
| `crates/kedra-packages/` (new), root Cargo manifests/lockfile | Flat small author binary with package modules, scripts, sets and pins; reuse dependencies. |
| `crates/sysroot/{catalog.rs,source.rs,system.rs}` | Explicit author/resolution inputs, format discrimination, generic contribution and diagnostic origins. |
| `image/{packages.list,remove.list,targets/*/packages.list}` | Remove from new-format authority only at verified cutover; old source reader retained. |
| `image/catalog/{Containerfile,builder.sh,templates/}` | Bind declared compiler-role material and declared exposure templates. |
| `image/{Containerfile,assemble.sh,release/refresh.py,release/material.py,release/compose.py}` | Consume verified resolved requests/inventory without broadening assembly privilege. |
| `.github/workflows/{check.yml,release-target.yml}` and current VM/container fixtures | Prepare/verify author material before use; preserve signer isolation and real workflow coverage. |
| Existing CLI `e2e_catalog`, `e2e_engine`, `e2e_source`, release CLI workflows and container harness | Extend actual author-to-installed-program workflows; no new test runner. |
| `docs/{PACKAGES,ENGINE,SYSTEM,STATUS}.md`, applicable repository skills, worklog | Explain commands, migration, evidence and limits; bump both plugin manifests if a skill changes. |

No core store rewrite, custom RPM solver, permanent service, UI or generic plugin
loader is required. The smallest useful release covers existing jq/SQLite, one
additional simple source package, and all current Fedora selections on both
targets; target-matrix expansion is a separate qualification.

## 8. Risks and deliberate choices

| Risk | Likelihood / impact | Mitigation and coverage owner |
|---|---|---|
| Author/dependency code accesses secrets or daemon | Medium / high | Compile and evaluate under the same bounded isolation; TC-06. |
| Moving Fedora repositories defeat stale resolution | High / high | Compare full material, refuse drift, explicit refresh; TC-04. No availability guarantee. |
| Compiler-role circularity prevents lowering | Medium / high | Symbolic intent first, resolve images before exact BuildGraph; TC-02/03. |
| DSL and lists silently diverge | High / high | Explicit format discriminator, mixed-input refusal, real parity; TC-08. |
| Generic contribution weakens source/trust checks | Medium / high | Structural inventory plus independent pins/policy/source verification; TC-07. |
| Third-party trust or recovery guarantees expand silently | Medium / high | Preserve current repository/key/activation boundaries and explicit non-coverage; TC-06/07. |
| Macro framework obscures simple recipes | Medium / medium | Ordinary Rust constructors/modules first; no proc-macro language. |
| Builds are claimed reproducible from hashing alone | Medium / high | Separate recipe/result identities and actual rebuild evidence; TC-03/05. |
| Future feature scope delays delivery | High / medium | Specification-only PR, no home feature, resolver/mirror service or cross-compile expansion. |

## 9. Primary technical references

Checked 2026-10-05; these establish external behavior, not Kedra implementation:

- [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html): compilation may execute build scripts, so evaluation isolation alone is insufficient.
- [Cargo build options](https://doc.rust-lang.org/cargo/commands/cargo-build.html): locked/frozen/offline flags constrain dependency resolution; separate OS isolation remains necessary.
- [DNF5 install](https://dnf5.readthedocs.io/en/latest/commands/install.8.html): package resolution includes dependencies; repository selection for explicit requests alone does not constrain all dependency sources. Do not replace the complete repository allowlist with `--from-repo` alone.
- [DNF5 configuration](https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html): solver, dependency and signature/repository behavior must be explicit inputs. This spec preserves existing reviewed adapter behavior until a separately verified change.
