# Build and environment engine

The reusable Rust library is
[`sysroot-engine`](../crates/sysroot-engine/README.md). Public commands are
`sysroot build`, `store`, `run`, `profile` and `develop`; this first implementation
provides ordinary-user package workflows on a Unix controller with a native
aarch64 Linux Docker backend.

It has explicit source/image admission, deterministic graph planning, isolated
builds, immutable output validation, independent rebuild comparison, digest-bound
runtime closure transfer, retained profile generations and explicit collection/
recovery. No Nix executable, new configuration language or resident engine daemon
is used. An explicit private store is separate from `/var/lib/sysroot` installed
management state and the current image release authorities.

Use [the library guide](../crates/sysroot-engine/README.md) for API/CLI examples,
[the implementation specification](../../../../.specs/nix-engine/requirements.md)
for scope, and [project status](STATUS.md) for actual checks. The preserved
[research/citations](../../../../.specs/nix-recreation/README.md) remain the design
reference, not a substitute for executed evidence.

The current Fedora bootc system, OS source assembly, signed OCI publication,
installer, staging/reboot and writable-home workflows continue to use their
existing contracts. This engine's package/profile rollback does not roll back
services, databases, home, credentials or an installed OS. General declarative
system composition has a [static context export](SYSTEM.md) and a separate
[native derivation stage](NATIVE.md). [PACKAGES](PACKAGES.md) explains where RPM
and engine package declarations live. The release integration is implemented in
the D2 draft; production qualification remains separate. An alternate OS backend
is later work.

First support limits: local Unix Docker endpoints, native ARM, single output per
derivation, one runtime image per closure and Docker29 OCI save archives. Native
x86_64, remote endpoints/caches, existing `.nix` expressions/nixpkgs and complete
kernel/boot/SELinux composition are not qualified by this implementation.

The D5 draft adds authenticated substitution of an explicitly supplied local
bundle, using a cache-specific key and consumer-resolved recipe/scope, plus leased
composition/native temporary snapshots and `sysroot system recover`. These APIs
are independent of OS signing and sysroot-core. The [library guide](../crates/sysroot-engine/README.md)
describes the inputs and refusal boundaries. Signed transfer, independent rebuild,
ordinary snapshot recovery and several bounded ENOSPC workflows have recorded
passes. Native publication-window retries and native metadata ENOSPC remain
unqualified; see [current status](STATUS.md) and the
[exact D5 evidence](../../../../.specs/nix-cache-recovery/verification.md).
