# Five follow-on PRs

Source basis: exact PR32 head3ba7d1bbe0278e11a51bf17c93aec4d2b874874b.
User authorization: all five selected outcomes, subagents, each its own PR; use
existing gh-stack workflow. No merge/production installation authority implied.
Initial dirty-tree probes both pass; no rebase is planned. Parent owns Git/index.

| Layer | Branch | Deliverable | Class/lane | Immediate PR base | Dependency / review surface |
|---|---|---|---|---|---|
| D1 | codex/nix-derived-boot | Verified native image bridge and actual QEMU boot/security/desktop qualification | TOOLING R | codex/nix-native-artifacts | Existing composition/native API; review VM provenance/boot material. |
| D2 | codex/nix-release-composition | ARM candidate builder/release wiring and disposable signed install/update/rollback | BACKEND/TOOLING R | codex/nix-derived-boot | Uses D1's exact derived boot/provenance bridge and shared candidate build path; signer remains isolated. |
| D3 | codex/nix-package-catalog | jq/SQLite Rust catalog, CLI and actual installed delivery through candidate builder | BACKEND R | codex/nix-release-composition | Catalog implementation is preparable independently; its installed image/candidate delivery uses D2. |
| D4 | codex/nix-second-consumer | Generated independently compiled consumer, custom definitions/policy and isolated lifecycle | BACKEND R | codex/nix-package-catalog | Consumes D3's public generic catalog policy/recipe API, not Kedra release constants. |
| D5 | codex/nix-cache-recovery | Authenticated substitution, leased temporary cleanup and real native/ENOSPC recovery | BACKEND R | codex/nix-second-consumer | Applies cache authorization to D3/D4 consumers and repairs temporary artifacts used by D1/D2. |

Each row has its own draft PR. Code documentation, specs, status/worklog and
necessary local skill updates ride the owning code layer under the repository's
required continuation contract. No sixth journal-only base layer. No history
rewrites/lower-layer changes merely to aggregate fixes on top.

Declared source ownership:

- D1 worker: container vm/image bridge and qemu tools/disk/observer files.
- D2 worker, after D1 freeze: image assembly/foundation mode, release compose
  orchestration, refresh preflight and release/test workflows.
- D3 preparer: new sysroot-catalog crate, new CLI adapter and new catalog E2E;
  parent alone registers workspace/binary/test entries and candidate hooks.
- D4 preparer: new standalone consumer/definition fixtures and new reuse E2E;
  parent owns existing e2e_engine registration.
- D5 worker: cache/signature/snapshot modules, engine bundle/context integration,
  native temporary guard and dedicated E2E; shared CLI/manifest wiring by parent.
- Parent: branch operations, index/commits/PRs, shared docs/worklog/STATUS,
  module/manifest wiring, aggregate review and serialized qualification.

All rows planned; D1 branch is active. Later branches are created only when the
preceding source/verification boundary is committed. Replan if file ownership
overlaps, ARM-only integration becomes a desktop migration, a signer needs
candidate code/checkout, or the package scope requires a new privilege boundary.
At boundaries inspect git status/stack/remote head and source contents, not stale
ancestry or filesystem timestamps. Update pending briefs in the same replan.
