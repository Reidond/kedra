# Design

Keep generic composition in flat `sysroot-engine/system.rs` and the Kedra adapter
in `sysroot/system.rs`. Use existing dependencies, store lock, descriptor/tree
verification, final logical prefix and bounded journaled executor. The adapter
uses source resolution/materialization from a frozen plan, not a second HEAD read.

Typed file contributions contain provenance, priority, explicit prior-owner
replacement and bytes or typed template content. Deterministic resolution rejects
ambiguous duplicates and prefix collisions. Selected output aliases resolve to
verified objects; all runtime foundations equal the independently supplied exact
foundation. Native config is emitted as ordinary files, not executable authority.

RPM inventory uses the existing release material columns NAME, EPOCHNUM, VERSION,
RELEASE, ARCH, SHA256HEADER and PAYLOADSHA256. The observation has a canonical hash.
Kedra requires Fedora 44, native ARM and desired/removed package membership.
Foundation observation cannot be supplied as an unchecked caller claim.

Context publication stages privately beside a new destination, fsyncs complete
artifacts, and publishes atomically. `payload.tar` contains resolved config and
verified runtime object trees with root ownership and canonical modes/timestamps;
foundation archive retains its verified receipt. `Containerfile` binds the exact
foundation and adds the payload without RUN/DNF. `composition.json` binds source,
target, provenance, foundation/package evidence, object receipts and artifact hashes.
No existing signed source/resolved-input schemas or installed helper are changed.

The plan/compose identity is input-addressed over the canonical resolved plan,
including actual file bytes/modes, verified tree receipts and retained foundation
material. The export manifest records resulting artifact hashes separately;
embedding the plan identity in payload provenance creates no self-hash cycle.

Evidence: [NixOS static closure/module research](../nix-recreation/findings/O2-os.md),
[store/recovery research](../nix-recreation/findings/O1-store.md), current
`source.rs`, `image/release/material.py` and the container harness README. The
research archive's source IDs/citations retain original document/page provenance.
Docker29.4 BuildKit resolves a bare image ID in FROM as a registry name; use an
explicit digest-derived local foundation tag, load the retained archive and
verify its image ID before tagging/building. Context export alone is not cold
daemon replay qualification.

An already assembled foundation carries generated artifacts. Updating a GLib
override, initramfs input or unit does not automatically regenerate schemas,
initramfs or enablement. This slice exports directly consumed config and typed
references; those derived transformations remain later gates. Foundation-owned
executable passthrough, if implemented, must independently match exact bytes and
mode and be explicit in the plan; it must never overwrite executable/library ABI.

Parent owns Git/stack and shared docs. Astra engine implementation owns engine
composition files; adapter implementation owns sysroot/system.rs, source.rs and
main.rs; verification agent owns e2e_system.rs. Do not run suites until writers
finish. No agent commits, pushes, changes branch or writes shared worklog.

Failure boundaries: untrusted typed IR is bounded/strictly parsed; config cannot
overwrite a foundation executable/library or engine store namespace; corrupt
objects/image archives are rejected before export; successful context publication
does not imply bootable, signed, installed or healthy. An interrupted observation
uses existing executor recovery. Output publication leaves no selectable partial
context. Later unsigned installed-system qualification must use the sanctioned
container harness, with no new test runner.
