---
name: kedra-research
description: Qualify Kedra features through real end-to-end workflows, container scenarios and disposable VM checks.
---

# Qualification

Read usr/src/kedra/docs/ARCHITECTURE.md and usr/src/kedra/docs/STATUS.md, then choose the smallest real workflow covering the changed feature. Existing tests live under usr/src/kedra/tests/ and .github/workflows/test-*.yml.

Use real CLI/process/Git, RPM transactions, native applications, container scenarios and disposable VMs. Include wrong authority/target, stale inputs, interruption and recovery where relevant. No unit/model/mock/doctests, source assertions or repository scanners; the container harness is the one sanctioned runner (AGENTS.md, owner decision 2026-09-27).

Choose the layer by what the behavior needs:
- **Host CLI.** Cargo `e2e_*` and the Python release checks, for source, release and CLI logic.
- **Containers.** usr/src/kedra/tests/container, run through `cargo test -p kedra-container-tests --test container`, for installed-system behavior without a kernel boot: files, units, the real session, portals, keyring, home review, apps and screenshots. Add YAML scenarios for linear checks, native tests (native.rs) for control flow, and guest probes (lab/probes) for in-guest logic.
- **VMs.** For Secure Boot, SELinux enforcement, VT/greetd password login and PAM, bootc switch/update/rollback, the installer, and Xwayland/Qt keyboard-driven dialogs.

Exported system contexts use `kedra-lab replay --image composition:<dir>
--composition-identity <independent-id>`. Static replay loads/builds/runs with no
pulls/network before any separate networked lab-tools adaptation. The composition
System-profile native unit case runs the static image directly. Keep the same
private artifact/cache root for cache reuse: a tag without its independent binding
is refused. Actual producer-removal ELF/library replay, large config, unit and
pre-build journal interruption/retry pass (2026-10-01); later publication windows
and independent-daemon results are recorded separately in native qualification.
Boot/signature deployment remains a separate gate.
Evidence: `.specs/nix-context-replay/verification.md`. No new runner/root authority.

Fresh empty mutual-TLS Docker29.8.1 replay and same-binding cache repeat pass
(2026-10-01). Four actual static publication SIGKILL windows also pass: temporary
image/pre-ID rebuild, then exact-ID resume after journal ID, binding and final
tag publication. Capture actual state before killing; do not reconstruct it or
count a missed window. SIGKILL leaves the private verification snapshot because
Drop cannot run; qualification removes only its exact recorded process-owned
temporary copy after exit, preserving journal/tag/binding for public retry.
Evidence: `.specs/nix-native-artifacts/verification.md`. Native-cache interruption,
kernel boot/signing and power-loss/ENOSPC remain separate gates.

Native generation uses `kedra-lab derive-plan`/`derive` with separate composition
and derivation identities. Installed checks use the existing container test's
`KEDRA_LAB_NATIVE_PLAN`/`KEDRA_LAB_DERIVATION_IDENTITY`, composition source and
`native_artifacts` filter. Actual fresh-user GLib default/explicit dconf override,
unit start/mask/default, initial skeleton/preserved existing home and generated
initramfs content pass (2026-10-01). Start the fixture's existing systemd user bus;
Fedora's reviewed foundation lacks dbus-run-session, so do not add an RPM merely
to satisfy that fixture assumption. Evidence: `.specs/nix-native-artifacts/verification.md`.
Dracut's `/root` symlink handling required a temporary empty `/var/roothome` only
for the exact dangling foundation link; remove only the directory created for
that operation. Its diagnostic about unavailable build-container syslog remains
visible; require actual success and independent content readback. Module names
can use kernel-equivalent hyphens/underscores, while exact kernel path and module
file suffix remain mandatory. These are content/workflow checks, not kernel boot.

Container adaptations and limits are listed in the harness README. Examples: no SELinux labels, a shared kernel (per-UID limits apply across containers), and bootc images hard-linked to their ostree objects. A passing container scenario never qualifies boot, firmware or hardware.

Harness additions measured 2026-09-28 (M2 Pro, Docker 29.4.0): `--test-threads 2`
runs two fresh isolated containers at once; all 13 ARM cases passed in 111.60 s
in one run. Default remains one worker. `KEDRA_LAB_ORDER=reverse` also passed.
The first interrupt preserves partial reports and attempts bounded owned cleanup;
a second signal force-exits. An active-probe cancellation left no test containers
from its execution while another retained lab stayed usable. Preserve the report's
`interrupted` and `selected_count` fields when interpreting a partial run. An empty
target-filtered selection explicitly gives no coverage and prepares no image.
These are local results, not current-source CI or native GPU qualification.

Use a named local tag for BuildKit FROM when the stage resolves to a bare image ID;
`Docker::pin_local` verifies that owned tag against the exact ID and the report
retains the ID. A bare `sha256:...` was treated as a Docker Hub image name and
failed before provisioning (WL-20260928-04; `usr/src/kedra/tests/container/image.rs`).

Image stages for local runs are `stable`, `run-*`, `sha256:*`, `builds:*`, `ref:*`, or `build[:rev]`, with an optional working-tree overlay. Local candidates are unsigned and never published or installed over the workstation. Disposable test derivatives may use generated fixture keys solely for normal signature-admission tests. Published OS images are built and signed only in Actions; local on-demand owner ISO builds consume reviewed signed images. Local CLI E2E uses generated fixtures; never enroll real home, use vault content or production signing keys as fixtures.

Derived QEMU qualification (2026-10-02) uses `vm image` with the complete
composition/native identity tuple. Preserve declared artifact bytes after DNF
fixture triggers and independently bind booted reference/manifest/native tuple/
trust to host image.json. Actual signed BIB plus cold/warm Secure Boot/lockdown,
enforcing SELinux and Metal desktop pass; AVC notices remain recorded.
Source: `.specs/nix-derived-boot/verification.md`. Image source, disk recipe and
diagnostic script can have separate revisions; never relabel a retained image
as newer committed source. Hash the pinned controller before/after each call.

Inherited sigpolicy rejects an unsigned fixture. Keep target production trust
bytes unchanged; use a separate generated-key buildroot and two exact-image-ID
rules for ordinary/BIB stores, requiring the unique signed reference. BIB's
ID-only source cannot match a repository-only rule. The actual pinned builder
supports `--build-container NAME@MANIFEST_DIGEST`; a bare config ID can parse as
a registry name. Strict default-policy unsigned/wrong-key refusals and allowed
admission precede install. Keys stay in tmpfs and are removed before BIB; do not
save/load signatures afterward. Podman5.8.7/Skopeo1.22.3 producer normalization
changes manifest digest while preserving config/native bytes; record both.
The disk builder lacks cmp/Python/OpenSSL CLI; use its verified existing tools,
and include new scripts in the closed Docker context. Preserve observed absent
default-VM PIDs rather than claiming a running default. Disposable admission is
not production enrollment/update authority. No generic guest root transport.

Record source/run/attempt, exact artifacts, versions and expected/actual result. Separate pass/fail/not-run/blocked, build/boot/install/healthy and physical hardware. Link Actions evidence from usr/src/kedra/docs/STATUS.md and worklog.md; do not commit raw logs, screenshots or research reports. Historical evidence is retained in Git history.

Update the relevant operational skill when a durable failure or version boundary is learned. Documentation is not test evidence; an untested integration stays unqualified.

Native preparation facts (2026-09-28; `tests/container/qemu`): keep host Python
build dependencies in the uv project pinned to 3.12. `uv run --with` temporary
interpreter paths can disappear between Meson setup and install; the persistent
uv project prevents that. virt-firmware 26.8 on Python 3.13 pulled crypt-r, which
failed on macOS without crypt.h; Python 3.12 uses its existing crypt implementation.
The QEMU fixture preserves DRM and SELinux instead of inheriting container unit
skips. Native disk export verifies both engine identity and immutable image ID
before mutations, avoiding a Docker CLI-context mismatch with Testcontainers.

Measured 2026-09-30 on the Mac/Noctalia 5.2 fixture: hot sync uses an ephemeral
private 0700 bare Git repository and independent indexes instead of copying every
checkout file. Preserve owner global ignores only for read-only path listing;
sanitize private Git configuration, use literal paths, refuse unsafe file types/
hardlinks, require matching content trees/HEAD/path sets, and remove ephemeral
objects. Full builds retain materialized snapshots; the production source archiver
continues layout, rootfs, target and private-key validation. Exact old/new commit
and payload SHA equality plus real refusal/concurrency/interruption workflows pass.
Five visibly changed native sync/capture samples pass median 4.453 s/max 5.136 s;
phase metrics distinguish source, transport and capture. Measure after unrelated
host load finishes and preserve contended samples as diagnostics.
