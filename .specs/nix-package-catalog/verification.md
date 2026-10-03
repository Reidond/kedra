# D3 evidence and remaining gates

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
