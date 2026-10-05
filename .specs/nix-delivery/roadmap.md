# Outcomes and gates

All five implementation outcomes are authorized. Five drafts are the reviewable
delivery surface; merging/protected production execution is separate.

Delivery checkpoint (2026-10-05): D1–D5 are drafts
[33](https://github.com/Reidond/kedra/pull/33),
[34](https://github.com/Reidond/kedra/pull/34),
[35](https://github.com/Reidond/kedra/pull/35),
[36](https://github.com/Reidond/kedra/pull/36),
[37](https://github.com/Reidond/kedra/pull/37), respectively. D4 subsequently passes its complete native consumer case on
frozen owning source d591d2e (relevant Rust/Cargo unchanged after the cascade).
D1/D3/D4 local qualification passes in its recorded scope. D2 local installation,
signed A/B/rollback, exact-candidate13/13 container checks and genuine production
contribution with nine refusals now pass. Four hosted ARM runs pass refusal/shutdown
but time out during installation. Corrected-classifier c01ee2f/run37330502213
attempt2 also fails at7201.573s after entering the bootc deployment path;
the retained evidence cannot identify the subsequent operation or root cause.
X86 updates/rollback and ARM refusal/cleanup pass. D2–D5 are aligned and
published as drafts; exact heads ad75aa6/948d3aa/7171e4e/5e94e8b pass both
architecture workspace checks. Eighteen redundant cascade-triggered runtime
runs were cancelled and confer no new qualification. No further speculative
VM run is proposed. D5 ordinary
signed-cache/rebuild/refusal and several genuine lease/ENOSPC cases pass, while
native fault qualification remains blocked and incomplete. Publication alone
does not complete an outcome. Exact current evidence is in the owning
verification records and `usr/src/kedra/docs/STATUS.md`.

1. Derived image: compose current committed Kedra, closed graphical native plan,
   exact derived→fixture→Podman→disk→boot provenance; verify actual booted kernel/
   initramfs, Secure Boot/lockdown, enforcing SELinux, real desktop and warm restart.
2. Release integration: ARM foundation mode defers only native transforms; use
   deterministic recipes/artifacts for no-change; bind actual outputs in final
   signed material; preserve signer schema/key/source isolation. Disposable normal
   trust/media/VM workflows prove install, A→B→A and writable data/home preservation.
3. Catalog: real jq/SQLite sources with observed canonical tree pins, useful
   CLI/library dependency, nonroot offline build/rebuild, transfer and installed
   command/service behavior. No automatic global FHS binary replacement.
4. Reuse: second compiled consumer calls generic library/catalog, own definitions
   and effective image/cache policy; equal short names in two stores, isolated
   profiles/rollback/GC and producer-absent closure execution.
5. Cache/recovery: signatures bind exact recipe/root/bundle/platform/purpose/scope
   before admission; preserve unauthorized/tampered state. Registered temporary
   artifact leases skip active/stopped users and collect exact abandoned copies.
   Pin every controller invocation for four real native publication kills/retries;
   actual bounded-filesystem ENOSPC/retry with valid/foreign sentinels preserved.

Standard gates per owning source layer: pinned Cargo fmt/Clippy/public CLI E2E/
release build, applicable sanctioned container/VM workflows and uv/Ruff/legacy
release CLI checks. Source/doc inspection is not execution coverage. First runtime
failure narrows/revises the implementation; never relax validation/security.

Risk/gate precision:

- qemu-arm64 first; existing desktop release continues unchanged. Native x86
  package/system behavior requires its own future qualification.
- Fixture DNF can regenerate boot/schema material: actual receipt checks must
  detect it rather than borrow native generation proof from an earlier image.
- Signed OCI output, generated initramfs, firmware signatures and cache producer
  authorization are different identities/authorities.
- Actual D1 disk installation rejects an unsigned fixture under inherited
  signature enforcement. Add a scoped generated-key fixture signing producer
  and strict normal consumer admission before BIB; preserve production policy/
  key/identity and enforcement. BIB's ID-only source needs its exact scratch-store
  scope plus exact signed reference, rather than a repository-only policy match.
- Native system identity binds inputs; nondeterministic realized initramfs bytes
  must not defeat no-change preflight. Preserve actual output hashes separately.
- Protect storage: one heavyweight VM/import/build at a time, explicit private
  bounds and exact cleanup; never fill the workstation/retained daemon.
- Main-only production signer/current-source/registry checks cannot be proven by
  a branch's disposable fixture. Record live production publication not-run until
  reviewed changes are on main; no branch bypass or merge merely to pass a gate.

Falsifiers: incompatible boot/security halts release-path adoption; an undeclared
runtime dependency halts package admission; any unauthorized post-import visible
object refutes cache design; cleanup of a locked/foreign snapshot refutes leases.
Resolve/fix before publishing that layer as qualified.

Power loss and Docker layer-storage exhaustion are distinct from tmpfs metadata
ENOSPC. Full Nix language/nixpkgs, optional OS-backend replacement, future hardware
and owner vault/account workflows remain outside this five-PR scope.

The owner requested a [niri home-artifact follow-up plan](https://github.com/Reidond/kedra/blob/codex/nix-home-artifacts-plan/.specs/nix-home-artifacts/spec.md)
on 2026-10-05. It proposes an engine-produced baseline in the existing image
pipeline and consumption through existing home commands. It is planning only;
its documents are reviewed in a separate PR above D5. No implementation is
included here, and the plan does not close current delivery gates.
