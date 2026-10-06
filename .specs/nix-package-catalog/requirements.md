# D3: real Rust package catalog

Authorized continuation of the [delivery charter](../../usr/src/kedra/docs/NIX-DELIVERY.md)
and [five-PR topology](../nix-delivery/topology.md). This specification records the
already prepared implementation and its remaining acceptance work; it does not
claim a specification preceded that preparation. Owning branch is
`codex/nix-package-catalog`, immediately above D2. Private compiler validation used
D2 `deee81332a32180c702b323b38fb727031a50e21`; adoption and runtime remain pending.

## Outcome and scope

An ordinary caller can select real jq and SQLite packages through reusable Rust
definitions and explicit policy, build them offline, retain/transfer their runtime
closure and deliver them through D2's existing Fedora candidate composer. Installed
users can select those commands, and a system service can retain SQLite data across
restarts. Fedora RPM ownership, signed bootc delivery and writable home remain intact.
The existing generic engine has demonstrated fixture programs; this slice supplies
maintained application definitions and a concrete installed-use path.

Actors are the ordinary package author, the public candidate builder and a user of
the composed qemu-arm64 image. The production signer retains its existing separate
authority. The generic catalog API must also support D4's independent definitions;
D4 owns qualification of that second consumer.

## Acceptance criteria

- **AC-01 — Exact sources:** given the pinned upstream jq 1.8.2 and SQLite 3.53.4
  archives, when verified and normalized as documented, then admission produces their
  recorded canonical source objects. Different bytes cannot silently retain the
  trusted package identity. Download failure or mismatched hashes stop admission.
- **AC-02 — Effective policy:** given an independently selected Policy, when a
  package is resolved, then the namespace, package, every builder/runtime image
  and every source object are authorized before opening a store. Wrong
  namespace/package/image/source or a hidden
  undeclared input refuses. All nodes in the selected graph are checked.
- **AC-03 — Reusable authoring:** given a Rust-authored Catalog, when plan/resolve
  is requested, then it deterministically emits existing engine graph/output
  identities without Docker or realization. Namespace scopes names; it does not change the canonical
  store prefix or grant installed authority.
  An invalid graph refuses before realization.
- **AC-04 — Acquirable compiler:** given D2's reviewed immutable native Fedora 44
  base, when preflight runs, then it resolves compiler RPMs using Fedora/updates
  with GPG checking.
  Only a changed build creates the compiler image. Actual compiler RPM material
  must equal preflight before that exact native image is retained and used.
- **AC-05 — Real applications:** given admitted sources and exact compiler/runtime
  images, when built, then the existing nonroot, network-disabled executor produces
  jq 1.8.2 and the SQLite 3.53.4 CLI plus its separate shared-library node. SQL JSON output processed
  by jq, including regular expressions, has the expected application result; the
  SQLite loader actually selects the declared library. jq output must be ELF;
  missing dependencies or failed compilation cannot publish a successful output.
- **AC-06 — Independent reproduction:** given valid prior outputs, when repeated
  normally, then they are reused; when explicitly rebuilt, then every required node
  is independently built and compared by actual bytes. A divergent result must
  preserve the existing winner and report failure.
- **AC-07 — Complete transfer:** given digest-bound export/import into a fresh
  store, when producer availability is removed, then both applications execute with the
  retained foundation and SQLite library. Undeclared dependencies or corruption
  refuse through the existing engine; no source/builder download occurs at run time.
- **AC-08 — Reviewed contribution:** given built outputs matching the selected
  recipes/foundation and canonical preflight material, when invoked, then the author
  emits a typed SystemDefinition and D2-compatible receipt. Wrong pins/author/material/foundation
  or missing objects refuse; an existing output directory is never overwritten.
- **AC-09 — Installed selection:** given the exact D3 composed image, when a login
  shell or systemd user service starts, then it selects declared store commands through its
  PATH configuration. Existing Fedora `/usr/bin/jq` remains RPM-owned and functional;
  no FHS executable/library replacement or global library-path override is required.
- **AC-10 — Persistent service:** given an installed
  `kedra-catalog-history.service`, when manually started, then it records actual
  SQLite version/source revision in its StateDirectory database. When restarted,
  then it adds a row while preserving earlier rows and a generated user-data row.
  No automatic service enablement is added.
- **AC-11 — Deterministic delivery inputs:** given source pins, author binary/source
  recipes, compiler policy and compiler RPM material, when preflight runs, then
  these participate in D2's no-change comparison. Realized builder/output/foundation identities and contribution
  receipts remain result evidence. Unchanged inputs build/publish no image; changed
  inputs flow through existing candidate verification and isolated signing rules.

## Constraints and exclusions

- **NFR-01:** native aarch64 Linux package execution and qemu-arm64 Fedora 44 delivery
  only; Unix controllers use an explicit private store and local Docker endpoint.
  Keep `/usr/lib/sysroot/store`, source FHS layout and existing image/helper trust.
- **NFR-02:** no workstation package install, home capture/apply, global agent
  configuration, privilege expansion or new language/daemon/downloader service.
  Release downloads are fixed, bounded workflow operations; package builds are offline.
- **NFR-03:** use only public CLI/manual E2E and the sanctioned installed harness.
  No unit/model/mock tests, source scanners or alternative runners. Execute heavy
  work one at a time within a declared runtime budget and pin controller artifacts.
  Preparation source acquisition is bounded to 512 MiB and its private Cargo target
  to 3 GiB; application/image execution uses the delivery lead's separate budget.

Native x86/Darwin application execution, arbitrary nixpkgs compatibility, a full
RPM migration, authenticated remote substitution (D5), independent-consumer
lifecycle proof (D4), production merging/signing and new OS lifecycle proof (D2)
are excluded. Compiler success is not application, installation or boot evidence.
AC-11 defines intended production behavior; branch qualification covers its real
material inputs and candidate path. Live protected-main no-change/signing
observations remain explicitly post-merge work under the delivery charter.

## Owner format steering — preparation follow-up

- **AC-12 — One native content source:** given the catalog's shell, environment
  and systemd text, when edited, then each lives in one ordinary native-format
  file under the development image tree. Rust embeds that file and derives its
  install destination from the same mirrored relative path; it does not duplicate
  the body or put raw placeholders into shared root payload. Four closed catalog
  tokens bind through typed input segments or the validated source revision.
  Runtime PATH expansion remains literal. Unknown/malformed catalog tokens refuse.
- **AC-13 — Selected metadata and complete provenance:** when authoring a
  contribution, inventory versions come from the resolved selected packages.
  Template bytes, path metadata and binding definitions participate in existing
  pins/author/preflight identity. Committed-source or foundation collisions remain
  refusals; this format change grants no replacement authority.
