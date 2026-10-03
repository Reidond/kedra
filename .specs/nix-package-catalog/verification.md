# D3 evidence and remaining gates

Current outcome (2026-10-03): the full native package workflow passes on exact
source57a47cf. Installed native::catalog/PATH/service and the production BuildKit
compiler path remain separate pending gates. Earlier preparation and failure
entries below retain their original scopes.

Recorded 2026-10-02 by Codex catalog worker. D3 is prepared, privately type/lint
checked and not yet adopted or runtime-qualified. User authorization covers the
owning delivery; there is no new approval dependency in this record.

## Actual preparation evidence

| Check | Outcome | Scope/evidence |
|---|---|---|
| Real upstream archive acquisition and checksum verification | pass | jq 1.8.2 and SQLite 3.53.4, bounded public downloads; values below |
| Canonical source admission | pass | Existing pre-D3 sysroot executable SHA256 `ef97bfa0addf27d3162afc4eb6eba8cb8fd3abd525370998c189974f5c2b57b4`; source-only private store, no images |
| Prepared Rust/Python/workflow syntax | pass | Rustfmt, Ruff, Ruby/Psych YAML parse and Bash syntax; no source-scan tests |
| Private workspace fmt | pass | Explicit installed Rust 1.98.1, exact D2 base plus frozen D3 changes |
| Private workspace/all-target Cargo check | pass | `--offline --locked`, final D2 check log, 5.38s recorded by Cargo |
| Private workspace/all-target Clippy | pass | `--offline --locked -- -D warnings`, final D2 log, 5.40s |
| Binary linking, public catalog CLI/E2E and release build | not-run | Cargo check/Clippy do not establish executable behavior |
| Compiler image/RPM equality, application builds/rebuild/transfer | not-run | No Docker/image work by catalog preparer |
| D2 contribution consume and installed PATH/persistent service | not-run | Native case compiled; never executed |
| Owning-source CI, draft PR and live production delivery | not-run | Lead owns adoption/publication; protected main authority stays separate |

The private validation source was initially archived from D1 712927e, then
reconciled against exact final D2
`deee81332a32180c702b323b38fb727031a50e21`. Only refresh.py required a merge of
D2's later resolve-base operation; all final controller/local-fixture files were
preserved. No shared tracked source or Git state was changed by this preparation.

Frozen local evidence lives under `target/nix-delivery/d3-validation/`:

- `d3-only.patch`, 17 implementation paths, SHA256
  `6a03a209ee007edfffeca4026dfd617fbf7c7e24251a322cda9a54405cb70bd9`.
- `d3-validated-manifest.json`, SHA256
  `e1ecea508c9cba966e60bedb46df6d4941ac4c5475eb352ccdb8666df9eee2ce`.
- `final/` contains frozen D3 source; `evidence/cargo-check-final-D2.log`,
  `cargo-clippy-final-D2.log` and `d2-final-reconciliation.json` identify checks.
- Build target was 319 MiB at handoff, below its 3 GiB limit; incremental disabled,
  jobs 2. Later workers reuse the mutable source/target, so use frozen final/ and
  the manifest when identifying this D3 result. These local artifacts are not CI.

Earlier failures are retained: inherited Rust 1.94 refused the workspace's minimum
version; explicit installed 1.98.1 corrected the invocation. Initial D3 SHA2 digest
LowerHex formatting did not compile under sha2 0.11; validated CLI/E2E copies now
format individual bytes. The earlier unregistered main-checkout copies were left
untouched; the delivery lead must adopt the validated versions, not those copies.

## Source inventory

Source preparation removed the enclosing archive directory, expanded hardlinks
into identical independent files, and preserved executable bits. The engine then
computed canonical source identities. jq has one such documentation hardlink.
No source bytes were edited. Full acquisition JSON and the admitted source-only
store are retained under `target/nix-delivery/catalog-inputs` (52 MiB observed).

| Package | Upstream source archive | Recorded identity |
|---|---|---|
| jq 1.8.2 | [Official release](https://github.com/jqlang/jq/releases/download/jq-1.8.2/jq-1.8.2.tar.gz), 1,959,950 bytes | SHA256 `71b8d6e8f5fe81f6c6d0d110e3892251f6ce76ed095abd315e26e6e1193af3af`; `src-5d3f1f89c8b6b206fc3c133f24d90d6225d73cd4c6274ec610c4cacf9f44e2d2` |
| SQLite 3.53.4 | [Official release](https://www.sqlite.org/2026/sqlite-autoconf-3530400.tar.gz), 3,283,177 bytes | Published SHA3-256 `454e45f61c6bd75b7420e7190732dea03ce6639c63ada47bbc592f67fc340338`; computed SHA256 `0e9483900e92cd5de8fd48d16bf9200145a61f7fd5be542a5ac81d8a9516eb9c`; `src-f4837e042b248ee63fd02e3f51fd6c9b9e2ffdf34bfb57450dd22a10c67f4d65` |

Source pins are acquisition evidence, not an execution claim. jq's distributed
configure/libtool source supports the intended static-internal-library output;
ELF and regex checks still require a real build. Existing retained 9d6 Fedora 44
material reports glibc 2.43, RPM jq 1.8.1 and sqlite-libs 3.51.2; that historical
foundation does not qualify D3 against D2's future fresh foundation/compiler.

## Remaining execution

All new D3 runtime acceptance cases in [test-plan](test-plan.md) are not-run.
Next: adopt/freeze the owning D3 source, build pinned tools, execute ordinary CLI
refusals and real compiler/package/rebuild/producer-absent workflows, then author
and consume the contribution and execute the exact installed harness case.
Record source/compiler/foundation/output identities, actual command outcomes,
byte comparisons, installed service evidence and failures before publication.
Keep D4/D5 and protected production observations distinct.

## Owner native-format follow-up — preparation only

Static profile/environment/service bodies now have one source in three native
files under image/catalog/templates; destinations derive from their mirrored
relative paths. Four closed catalog tokens bind typed inputs/revision while
runtime PATH expansion stays literal. Pins/preflight include template bytes and
binding metadata; inventory and installed expectations use selected versions.
Earlier artifact-directory and release-profile gate corrections are preserved.

Rustfmt on changed Rust files, Ruff on the changed preflight module and Bash
syntax for the profile template pass. No Cargo, application/image build, template
behavior, installed harness, Docker or VM execution was performed. Earlier private
Cargo/Clippy evidence remains historical and does not qualify these Rust changes.
Current adoption hashes are recorded in the accompanying template-format handoff
and frozen D3 manifest; prior artifacts are preserved under
`target/nix-delivery/d3-validation/versions/pre-catalog-templates`.

## Owning-source adoption — 2026-10-03

Adopted above D2 draft PR34 head `582049b` from exact twenty-file manifest
`f9869424508b8fc28934c29b983f270e038bb984407e07326b8bc99ee7799983`
and patch `ce23069a764d040617aea0f625bc09ceddf24208afdbf3303dc6b88a14839a7c`.
Current destination preimages matched the frozen D2 hashes; no newer D2 source
was replaced. Template-aware specs accompany the single native-text source.

Current pinned Rust 1.98.1 fmt/all-target Clippy pass. The two real public CLI
catalog planning/policy cases pass, including an independently authored catalog
and source authorization before store creation. The actual application
build/rebuild/transfer case remains ignored because it requires the serialized
native runtime; ignored is not passed. Ruff, native shell syntax and whitespace
pass. Workspace release build passes; manual released `catalog list` and `pins`
execute successfully. D3 binaries are pinned under
`target/nix-delivery/publication/d3-binaries`; CLI SHA256 is
`541d814eefe7d6c72de66e3ae79b85130fb8a8dda4bd14976010d3b0f12635e3`.
Release-material passes both target workflows and all recorded refusals in
`target/d3-publication-release-material`.

Review surface: catalog resolution is used by the public CLI and generated
independent consumer; CLI contribution feeds the existing D2 composer; pins feed
refresh material and compiler/source/template admission; the exact-native-image
release gate selects the existing container catalog case. These are inspected
static dependencies, not a proof that runtime callers are exhaustive. No extra
store prefix, native source duplication or global FHS replacement is introduced.

## Actual catalog failure and jq executable scratch — 2026-10-03

The full existing catalog case on frozen `808405e`, CLI `541d814…` and test
`54321e8…` **fails** after 162.62 seconds, actual parent wait101. Compiler/runtime
admission and the SQLite library/shell builds complete before jq's `./configure`
returns126, permission denied. The healthy guard records zero remaining owned
containers and peak owned allocation5,403,889,664 bytes. Retained failure evidence
is `target/nix-delivery/d3-catalog-e2e/test.stderr` and `test-guard.json`; that
failed source/case is not relabeled by the following fix.

A real probe with the same exact compiler image
`sha256:07c0e38400ec538e63d1f09cc6f09577cb342c210f449fa332d9eb76e24feb0d`
and engine sandbox flags observes `/build` mounted noexec and a real gcc-generated
ELF also returns126 there. Create/start/container/remove waits are0; cleanup takes
0.345 seconds. The source configure file is executable, including in the admitted
read-only source. See `actual-build-mount-probe.json` in the same evidence folder.
Invoking only `sh configure` would not fix its executable compiler probes.

The owning D3 correction moves only jq's build tree into its existing private
output binding at `$out/.work`, maps that exact logical scratch prefix out of
compiler output, installs the real ELF and licenses, then removes scratch before
successful admission. The static build flags and existing ELF-magic assertion
remain. Generic engine/input mounts and `/build` noexec are unchanged; SQLite
recipes are unchanged because they compile directly to output and do not execute
build-directory probes. Limited independent Astra review is clear on frozen diff
`280011c62c2966aa5d5f8ec45c62a334dc4034a2026f35549efed6def847a39a`.

Rust1.98.1 fmt/all-target Clippy, the two public catalog policy/planning cases and
workspace release build pass. The actual application case remains ignored in
those host gates and requires a new full native run. Released `catalog list` and
`pins` execute; new CLI SHA256 is
`24a9d4ea12a6555c7a713b2cc49dfb15c37aa172b46bcbb78021b968aeec56e0`,
new existing-case executable SHA is
`69ca9fd21426105f0df4d1cfc194fe7a8b05dc9183ec27859190f74981bc894b`,
and pins SHA is `19121ecfaac42c099dc0bfe59db54ec6781ca02e7c4e41dc1d33813ca27a4bb1`.
Mode0500 copies live in `publication/d3-jq-fix-binaries`; the earlier source,
executables, context and SQLite output metadata remain preserved. No native
rebuild/reproduction or installed-service pass is claimed before the rerun.

## Vendored generated-file timestamps — 2026-10-03

The bc34 case fails after211s, actual parent wait101, after successfully running
configure in executable output scratch. Vendored Oniguruma make then invokes
missing aclocal-1.16 even though the compiler legitimately supplies automake1.18.
The original configure.ac is11s older than aclocal.m4; admitted canonical source
reverses that ordering by approximately33ms while preserving bytes/source ID.
The actual vendored Makefile.in94/419 declares an unconditional regeneration rule;
the top-level maintainer-mode flag does not guard it. Evidence:
`d3-catalog-e2e-fixed/result.json` SHA
`d88433353e8240e629ea675685286f0ea6026ecc99fb934671e6f75de897dafd`,
`test.stderr` and `source-mtime-observation.json`. The full case stays failed.
Both SQLite output trees independently match bytes/modes across808/bc34, which
is a narrower passed reproduction observation; runtime queries remain unpassed.

The jq-only correction runs `find -P`/`touch -h` over its fresh disposable scratch
after copying and before configure, normalizing files/directories/link inodes to
the already declared SOURCE_DATE_EPOCH0. It follows no symlink target, changes no
input bytes/modes or compiler tools, and leaves engine noexec intact. Existing
findutils/coreutils provide the commands. Limited Astra review is clear on patch
`3d820dac028a1827c855ed260ac36b5029f0e9e5fa446b27421f9d976e45681f`.
Owning Rust1.98.1 fmt/Clippy, two public catalog cases and release build pass; new
full native execution remains required. CLI `eb4bf1b471fac83b5674b2a9dc23d292b8251e5bc83e82a5c32477d952a375ef`,
case binary `dfcef691bac7b98820ad653d5f93beeda40e3a000c512a3efb53f89317b0737d`
and pins `36bcc36bf8f666eb30088f091740ac2c056e3d8aad15f76c799823e02b2a9d87`
are retained in publication/d3-jq-mtime artifacts.

## Full native catalog workflow passes — 2026-10-03

The unchanged existing case `real_catalog_build_reproduce_transfer_and_query`
passes **1/1, zero failures, zero ignored** in580.87s, actual parent wait0
(guard584.524s). Exact source is `57a47cf6664632839eeaf35528e9873a26c5cb12`,
CLI `eb4bf1b471fac83b5674b2a9dc23d292b8251e5bc83e82a5c32477d952a375ef`,
E2E `dfcef691bac7b98820ad653d5f93beeda40e3a000c512a3efb53f89317b0737d`.
The source handoff, executables and selected global image identities are unchanged.

Actual assertions cover canonical sources, complete compiler/runtime archive
admission, jq1.8.2 and SQLite3.53.4 native builds, cache reuse and forced independent
byte reproduction for all three outputs, exact SQLite shared-library loading,
contribution metadata and mismatched-pins refusal, importing both closures into
an independent receiver, removing the producer before SQL/jq regex evaluation,
the expected alpha/gamma result and receiver-profile version execution.
Selected jq is `out-80330acbc48ca807fdfe591c4bb94e2ba60570bc1e1c014aa8a5baa69e0d0168`;
SQLite shell/library remain `out-51f112ad…` / `out-bf28350f…`.

The successful fixture is gone, owned containers and host process group are
absent, and selected compiler07c/runtime122 global images are preserved. Peak
charged allocation is15,826,591,744 bytes; minimum Linux/host free space is
59,967,152,128 /61,093,240,832 bytes, with the original capacity floor respected.
Later capacity samples are retained without assigning an unobserved accounting
cause. Aggregate `target/nix-delivery/d3-catalog-e2e-epoch/result.json` SHA is
`ae3dbe525592a69b4d40ad449dbbaa42c93800181320b9a757f36126612906d7`;
its linked stdout/guard, selected graphs/policy and image observations were read.

The earlier808/noexec and bc34/generated-timestamp failures remain failed with
their receipts preserved. This is a package/contribution workflow pass, not an
installed-image or signed deployment pass. Compiler acquisition used the disclosed
Podman backend with reviewed recipe/RPM material; production BuildKit equivalence
is not asserted. The separate installed native::catalog service/PATH/persistence
case, D2 media/VM and D5 cache/fault work remain outstanding.
