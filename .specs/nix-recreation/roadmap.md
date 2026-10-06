# Implementation path and qualification gates

Proposed on 2026-10-01. Every implementation/behavior case below is **not-run**.
This document is a development sequence, not an implemented task list or a timing
estimate. Research and preserved-document integrity are reported separately.
Kedra is the only present target; future reuse is demonstrated with a generated
second project, not a modification of the Kedranix reference.

## Delivery sequence

| Slice | Concrete outcome | Evidence required before advancing |
|---|---|---|
| 0. Contract experiment | Choose canonical prefix, artifact format, native Linux sandbox/bootstrap and initial frontend boundary | Real compiled binary with runtime library executes in the intended namespace, copies to a fresh namespace, and survives image export without path rewriting; ordinary host content is unavailable |
| 1. Useful independent engine | One Rust crate and `sysroot build`, with immutable source/import handling, canonical `BuildSpec`, local DAG realization and output receipts | Real package A depends on real package/library B; changing B changes A's identity, old/new versions coexist, a warm request reuses a validated object, and source lock omissions refuse before building |
| 2. Closures and environments | `closure`, `copy`, `run`, `develop`, durable profiles and coordinated GC | An empty receiver runs a copied closure offline; an environment runs actual compiler/tool commands; profile rollback runs the old executable; concurrent use/import/build remains rooted during collection |
| 3. Cache and shared API | Independent content verification, authorized substitution, pinning and an externally usable library boundary | A fresh disposable namespace consumes a trusted cached closure; corrupt/wrong-platform/wrong-prefix/incomplete-reference receipts fail; two independent clean builders compare actual output bytes; a generated second project uses the same core without Kedra release constants |
| 4. Declarative Kedra compiler | Reviewed Rust contributions lower current target/source/native formats through typed merge/provenance into `SystemPlan`; static files/units/packages become derivations | Public CLI planning shows source-level conflicts; config changes rebuild the appropriate outputs; real systemd services/config run in the sanctioned harness; no custom user configuration language or NixOS module compatibility is implied |
| 5. OS release integration | Locked Fedora/bootstrap material and new outputs composed into the existing OCI pipeline | Exact-candidate installed-system/container checks; observed Actions validation, isolated signing and strict readback; native QEMU fresh encrypted install, visible login, correct target trust, forward update and retained rollback |
| 6. Catalog and broader reuse | Required Kedra desktop packages/tools are modeled with supported runtime closures; a second personal consumer can pin the engine and supply its own catalog | Native desktop screenshots and actual applications, EGL/renderer identity, portal/audio/input behavior; independent target release and consumer-specific trust; reproducibility claims tied to retained artifact/toolchain bytes |
| 7. Optional full OS backend | Replace bootc only if the owner selects a store-composed OS model | Separate installer, kernel/initramfs/boot-loader, Secure Boot/SELinux, encrypted storage, mutable-state, rollback and offline-recovery qualification; deliberate architecture/status updates |

The minimal useful option is slices 0–2 on one native Linux architecture and one
output per derivation, with an explicit pinned runtime foundation. Start with
aarch64 Linux; qualify x86_64 before either architecture enters new OS release
integration. Remote cache substitution can follow independently of the first
local workflow. The larger catalog, general modules and optional OS backend do
not have to exist to get a useful build/environment engine.

Slices 0–3 are useful even if bootc stays the permanent Kedra activation backend.
Slices 4–6 provide declarative OS composition backed by the new engine. A full
source-built package ecosystem or NixOS replacement is larger than the first
engine milestone. A successful library build is not evidence for slices 4–7.

## First vertical slice

Use retained immutable aarch64 Linux builder inputs first, then repeat the
qualified workflow on x86_64. Each derivation initially has exactly one output;
enabling multiple outputs requires its own coordination/publication/recovery gate.
Build a small real source package with a real runtime library dependency and two
observable versions. Its executable should identify the version and exercise the
library, so closure copying and rollback prove user-visible behavior. Compiler,
linker, headers and shell are declared bootstrap inputs; no live network install
occurs inside the build. The experiment may use generated source fixtures and
public CLI invocations in disposable Linux namespaces.

Record actual source/toolchain digests, build/host platform, canonical prefix,
sandbox policy, resulting receipts and executable output. Run the receiver
without the producer's filesystem and with network disabled. Include every
required userspace dependency: either store-contained loader/libc/libraries or an
explicit digest-bound runtime root transferred as part of the closure. Missing
or mismatched runtime roots must refuse. The receiver may provide kernel/syscalls,
but its undeclared `/usr` and loader cannot make the example pass. Contrast repeated
realization (cache reuse) with independent realization (byte comparison). These
are different gates.

Do not implement a full evaluator, distributed scheduler, universal dependency
solver, remote fleet service or new CI server just to run this slice. Keep the
dependency graph complete before realization; prohibit import-from-derivation
initially. If Nix expression compatibility becomes mandatory, replan the frontend
work explicitly rather than implying a small parser satisfies it.

## End-to-end and manual case matrix

| Case | User-visible workflow and failure check | Method / status |
|---|---|---|
| E01 | Lock inputs; plan without build/network/undeclared host access; reject absent locks and option cycles/conflicts with provenance | Public CLI in disposable Linux namespace; not-run |
| E02 | Build and execute a real package with its real dependency; prove input changes, platform and prefix changes produce distinct identities | CLI plus actual compiler/linker/runtime; not-run |
| E03 | Repeat realization without rebuilding; independently rebuild in a fresh environment; detect same identity/different bytes without overwrite | CLI receipts, real build processes and content comparison; not-run |
| E04 | Copy/export/import complete closure into an empty offline receiver and execute; reject missing references, traversal, unsafe links, special files and tampered bytes | CLI plus real filesystem/import archives; not-run |
| E05 | Enter development environment and run a compiler and tool; private host file/network credential access fails | Actual shell/tool processes inside qualified sandbox; not-run |
| E06 | Create two profile generations, select new, run it, roll back and run old; retention and explicit pruning are observable | Public CLI with executable behavior; not-run |
| E07 | Interrupt building/import/index commit/profile selection; simulate storage exhaustion on a disposable volume; resume safely without half-visible objects | Real process termination/filesystem limits/public recovery; not-run |
| E08 | Run/use/build/import while collecting; referenced outputs and active leases survive; explicit root removal allows unused objects to be collected | Concurrent public workflows, real processes and store; not-run |
| E09 | Import trusted binary result; reject unauthorized producer, wrong recipe/platform/prefix/references, stale policy and signed-but-altered bytes | Disposable signing fixtures and independent verifier; not-run |
| E10 | Consume the core from a generated second project with different outputs, package catalog and authority policy | Actual compiled Rust consumer plus public CLI; not-run |
| E11 | Merge shared/target configuration; start real service A/B, change config, verify native state/health and failed-activation recovery | Existing Testcontainers/systemd/nested-session scenarios; not-run |
| E12 | Compose native target OS, publish through exact-candidate gates, install fresh encrypted VM, boot, update, roll back and recover offline with persistent data intact | Existing Actions, sanctioned container harness and retained QEMU VM; not-run |
| E13 | Validate niri/Noctalia and actual desktop applications, GPU/scanout, portals, audio/keyring/input, `/etc` and home reconciliation | `kedra-lab up`, `sync`, `shot`; show screenshots; VM for excluded container behaviors; not-run |
| E14 | Replace OS backend, if selected, without losing encrypted installation, firmware trust, enforcing SELinux, correct module/initramfs or retained boot recovery | Separate native disposable-VM/hardware qualification; not-run |

Versioned fixture scenarios belong to the existing container harness and ordinary
public-CLI E2E tests. No unit/model/mock/doctests, source-string assertions,
repository-layout scanners, custom check runner or new xtask are proposed. Do not
test enrollment, formatting, home apply or bootc switching on the workstation.

## Standard implementation checks

When Rust implementation begins, use the pinned Rust 1.98.1 workspace and existing
format, Clippy, release-build and public CLI E2E checks. Use the sanctioned
installed-system harness for changes that it can host. Run Python scripts only
through uv, with the existing pinned Ruff checks. Boot/security/installer and
desktop cases must use the relevant retained QEMU workflows rather than container
results presented as boot qualification.

This research changes documents and stores supplied PDFs only, so compiler,
Clippy, package build, container and VM cases were not invoked to validate it.
Archive checksums establish preserved bytes only. CI on the inspected base is
prior source evidence, not a pass for a new engine.

## Risk-driven replan triggers

- If a tool requires dynamic or self-referential output addressing, Nix-specific
  module recursion or import-from-derivation, decide whether to implement the
  feature or keep that tool outside the supported first catalog.
- If the canonical store prefix cannot support both image export and execution,
  settle distinct namespaces and identities before adopting a binary-cache format.
- If pinned bootstrap/RPM material cannot be retained and replayed offline,
  narrow the reproducibility claim and implement retention before declaring it.
- If reused Rust components require a different API/license/runtime model,
  compare an isolated prototype with a minimal independent core before adopting.
- If changing bootc or home semantics becomes necessary, revise the durable Kedra
  architecture and requalify those workflows as an explicit slice.

Next concrete implementation artifact: detailed slice-0/1 requirements and design
with actual chosen prefix/bootstrap/interface, followed by the first behavior E2E
scenario. The current proposal leaves those product decisions visible for review.
