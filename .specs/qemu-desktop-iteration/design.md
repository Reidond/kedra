# QEMU and container desktop iteration design

Status: approved by the owner on 2026-09-28. Implementation not started.
Date: 2026-09-28. Base: `a3a39a4a8a1d7777f02ab68d80ad4f39abf40c87`.
Prerequisite: [requirements](requirements.md), approved by the owner on 2026-09-28.

## Decisions

Extend the existing `kedra-container-tests` crate and `kedra-lab` binary. Keep
container scenarios on Testcontainers and Cargo. Add a native QEMU command group
for retained manual VMs; do not create a second test runner or make Docker host
the graphical VM. Containers remain explicitly software-rendered on this Mac.

The local VM uses ARM64 QEMU/HVF, the Cocoa display backend, VirGL and ANGLE/Metal.
The first delivery qualifies OpenGL desktop rendering. Vulkan/Venus remains a
separate capability and is not enabled by default without matching evidence.
Keep the signed target ID `utm`, repositories, keys and release ordering unchanged.

The speed strategy is reuse: prepare tools/images once, retain the desktop,
archive home inputs without compiling Rust, transfer only changed managed files,
and capture through the guest compositor. Full images are rebuilt only when
the requested changes need them.

## Existing implementation to reuse

| Existing code | Design change |
|---|---|
| `builder.rs::snapshot`, `sysroot source archive` | Keep the real source resolver and credential exclusions; split archiving from Linux binary compilation |
| `image.rs` image stages and payload overlay | Factor image-under-test preparation from the container-specific tools layer so VMs never inherit nested/software-renderer adaptations |
| `session.rs::screenshot` and `grim` | Keep compositor capture; share PNG/provenance output with the VM transport |
| `lab.rs::sync` | Replace unconditional `builder::prepare` and whole-home writes with validated incremental lab sync |
| `environment.rs`, `docker.rs` | Keep Testcontainers ownership and Docker Engine API; add bounded cancellation/diagnostic behavior where qualification finds gaps |
| `main.rs`, `scenario.rs`, `report.rs` | Keep scenario language and native workflows, single-scenario concurrency by default; extend phase timing and durable partial results |
| `installer/utm/kedra-utm.py` ISO path | Extract its existing Linux-builder operation to a QEMU-neutral media command; retain signature checks and new-output semantics |
| `tests/vm/utm-image` | Rename operational paths and markers, retaining every boot/security assertion |

These observations are from the current source, not assumed missing features.
In particular, the current sync invokes a Linux `cargo build --release` through
`builder::prepare`, and current screenshots already use `grim` in the session.

## Native graphics runtime: first qualification gate

Start from the upstream integrator's source set and pin each fetched revision,
recording necessary corrections, rather than using a mutable Homebrew formula or
UTM application libraries. The initial candidate is
[akihikodaki/v at cd8293b](https://github.com/akihikodaki/v/tree/cd8293b463ed9963f511a23b2b80025c65a81303),
whose Git tree was inspected on 2026-09-28:

| Component | Repository | Initial immutable revision |
|---|---|---|
| QEMU (VERSION reports 11.0.0) | `akihikodaki/qemu` | `29d25d77c771ecd9a9b901aa631d19ad4a886b77` |
| ANGLE | `akihikodaki/angle` | `b4f38bcd8f6366c11546e12849b823cdcaa134ba` |
| libepoxy | `akihikodaki/libepoxy` | `923f67e8064687aaa7aa0a4fd064c6e4782ca985` |
| VirGL renderer | `akihikodaki/virglrenderer` | `619f0a056a4c13b9e57a5d6f9de426f2bf7069db` |
| depot_tools, build-time only | Chromium depot_tools | `06b875921815b3d28e6f739b2f31138a233c53bd` |

This is a qualification candidate, not a claim that the April stack works on
macOS 27. No adoption of the integrator's `update` or `run` scripts: they use
mutable updates, Pipenv, sudo and vmnet settings outside this design.

Implementation correction, 2026-09-28: the original v snapshot's epoxy
`c9cd1717162a521315c536bb060881a0c678f125` and VirGL
`4fac4e593268bfa0bcbc18dc720d49cfbf7d9b68` cannot be fetched: Git reports “not our
ref” and GitHub's commit API returns 422. The table now pins the fetchable `v`
branch commits inspected and checked out on that date. Their compatibility is
unqualified, and the complete ANGLE dependency closure remains unlocked.

Implementation adds `tests/container/qemu/inputs.json` and a host-side
`prepare-runtime.py` with PEP 723 metadata, invoked only through uv. An explicit
`kedra-lab vm tools prepare` invokes that builder after host preflight and owner
installation authorization. `tools check` is read-only and never installs.

The builder:

1. Fetches the exact source revisions into ignored runtime build directories;
   exports the selected ANGLE DEPS Git/CIPD dependency closure with immutable IDs.
   Disables depot_tools self-update and automatic unreviewed hooks/downloads.
2. Checks existing native build dependencies. Missing dependencies produce a
   concrete installation plan and stop; no brew, pip, sudo or Xcode installation
   occurs automatically. Python build entrypoints run in a uv-managed environment.
3. Builds release ARM64 ANGLE/epoxy/VirGL and only QEMU's aarch64 system target plus
   qemu-img into a private prefix. It records compiler/SDK, dependency versions,
   source/archive hashes and configure options. Host build tools can be reused;
   dynamically loaded runtime libraries are copied with their licenses into the
   private runtime closure and relocated there, leaving host installations untouched.
4. Includes a pinned swtpm/libtpms runtime and independently sourced AAVMF Secure
   Boot firmware. For firmware, use the same Ubuntu package family as the existing
   ARM CI, extracted in a disposable container, not from `/Applications/UTM.app`.
   Freeze the exact package/archive hashes and code/VARS hashes in inputs.json
   before building the candidate; unset pins are a preparation error. Review its
   enrollment using virt-fw-vars and require Microsoft UEFI CA 2023, PK and enabled
   Secure Boot. The source closure and additional dependency pins are phase-one
   deliverables; later phases cannot run against a partially locked manifest.
5. Ad-hoc signs the local QEMU executable with its required Hypervisor entitlement
   and verifies that signature. The result is an ordinary-user local build,
   not an Apple-notarized or production-signed artifact.
6. Atomically publishes a completed runtime receipt under
   `target/kedra-lab/qemu/runtimes/<inputs-hash>/`; an incomplete build never becomes
   the selected runtime. No automatic runtime upgrade happens on VM start.

Preflight verifies HVF, the `virtio-gpu-gl-pci` device, Cocoa `gl=es`, library
resolution independent of UTM, firmware pairing and swtpm. A minimal disposable
Kedra desktop must pass UC-01 before UTM tooling is removed. A build or graphics
failure remains a failed gate; changes to the candidate are explicitly recorded.

This source-build cost is paid once, separately from iteration benchmarks.
Prebuilt third-party bottles were considered, but their dependency closure and
mutable-source recipes add ambiguity. A later verified bundle may replace source
preparation without changing the VM or lab interfaces.

## Commands and source layout

Existing container commands keep their behavior and names. QEMU dispatch occurs
before `Docker::connect`; operating an already prepared VM does not require Docker.

| Command | Behavior |
|---|---|
| `kedra-lab vm tools check` | Report runtime and firmware readiness without downloading or installing |
| `kedra-lab vm tools prepare` | Explicit one-time preparation using locked inputs and already authorized prerequisites |
| `kedra-lab vm image --image <stage>` | Prepare a bootable, cached development disk from the existing image-stage selection |
| `kedra-lab vm up --name <name> [--image <stage>]` | Start/reopen a retained instance; default 6 vCPUs, 8 GiB RAM, 2560x1600 at scale 2 |
| `kedra-lab vm sync --name <name> [--shot <label>]` | Apply supported home changes, confirm readiness, optionally capture the result |
| `kedra-lab vm shot [label] --name <name>` | Save the actual guest compositor output plus metadata |
| `kedra-lab vm exec --name <name> [--root] -- <argv>` | Execute argv through authenticated lab-only guest access, with a deadline |
| `kedra-lab vm status`, `logs` | Report source/runtime/rendering identity or collect bounded diagnostics |
| `kedra-lab vm down --name <name> [--force]` | Request clean shutdown; forced stop is explicit; disk/NVRAM/TPM remain |
| `kedra-lab vm remove --name <name>` | Remove an explicitly selected, stopped, owned disposable instance; preserve shared bases |
| `kedra-lab vm create --name <name> --iso <verified-path>` | Create a separate manual installer VM with an empty disk and verified media |
| `kedra-lab vm detach-installer --name <name>` | Detach media only from a stopped owned VM; preserve the original ISO |

`vm up` defaults to the accelerated profile and has no automatic TCG/software
fallback. An explicit debug software profile is visibly labeled and cannot
produce an accelerated qualification receipt. Names cannot contain paths and
must be unique within the QEMU namespace; ambiguous container operations continue
to require a name. Hot container sync gains the same `--shot` option.

New flat modules in the existing crate are `vm.rs` (CLI operations and disk
preparation), `qmp.rs` (bounded control protocol), `host_process.rs` (owned process
lifecycle), `lab_sync.rs` (shared sync plan) and `capture.rs` (shared artifact
receipt). `qemu/` holds immutable inputs and preparation assets; `lab/` holds
guest-only helpers. No new workspace member, persistent host daemon or plugin
framework. Prefer a concrete container/SSH transport enum to a general backend API.

## Disk preparation and guest access

Factor `image.rs` into image resolution/payload preparation and environment-specific
tool layers. `vm image` uses the base plus requested payload, adds a thin VM-only
fixture layer, then uses the already pinned arm64 bootc-image-builder in a
disposable privileged Linux container. Reuse the existing installer builder's
nested Podman approach and its ownership lock. Never nest QEMU inside Docker.

The fixture layer adds capture/renderer probes, an SSH-based lab transport and
first-boot seed handling. Keep SELinux enforcing, the real packaged Kedra session
and native DRM backend. Do not copy `niri-nested.conf`, container greetd skips,
software-renderer environment variables or container-only rtkit adaptations.
Lab tools may add packages but must not upgrade/remove the image-under-test RPMs;
reuse the existing inventory-difference guard.

Cache keys include immutable image digest, payload and binary hashes, fixture
layer, image-builder, target, architecture, disk schema and size. Shared bases are
read-only, contain no owner data and are never signed/pushed. Create per-instance
QCOW2 overlays with explicit backing format, their own VARS copy and TPM state.
Keep base files while any instance refers to them; first delivery has no automatic
base garbage collection. Failed builds leave labeled diagnostic output and never
replace a known-good base.

Each instance gets its own client key, SSH server host key, random account secret
and instance token. Generate private material in a mode-0700 instance directory,
never print it or include it in shared bases. Supply a read-only instance seed
through QEMU fw_cfg to a fixture-only first-boot service; it writes keys/accounts
with correct ownership and SELinux labels and records consumption. Bootc/shim
identity checks remain active. Development auto-login is labeled and scoped to
the disposable fixture; real VT/password/PAM qualification stays in VM CI.

The transport invokes system OpenSSH with a dedicated identity and known-hosts
file, strict host checking, no agent forwarding, `IdentityAgent=none` and no personal
SSH config. QEMU user networking binds the guest SSH forward to loopback only;
port collision during start gets at most three fresh-port attempts, each after
the failed process is reaped. Use the known guest host key, never TOFU/accept-new.

Remote operations call a fixed image-owned lab helper and pass bounded JSON/stdin
payloads; the helper executes argv without shell interpolation. Root operations
use a fixture-only sudo rule for this helper and explicit operation validation.
The helper and rule are absent from production images and installer media. The
production QEMU guest-agent RPC deny list is unchanged. No host home, vault,
source credentials or SSH-agent socket is shared with the guest.

Manual installer VMs use the verified ISO/empty disk path and require manual
account creation. They do not receive automatic lab SSH access or hot sync. This
preserves real installation behavior while the development-disk path handles
fast visual iteration. Their `up` result is only process/window started, not
authenticated desktop readiness; `exec` and `sync` refuse this instance kind.
Preboot QMP capture is diagnostic where available; unsupported accelerated
installer capture is reported rather than returning a blank image as success.

## QEMU lifecycle and I/O

Use the versioned `virt-11.0` machine for the initial QEMU 11.0 candidate,
`-accel hvf`, `-cpu host`, `virtio-gpu-gl-pci` and Cocoa `gl=es`. Check actual
device/machine support before launching. Use USB keyboard/tablet on xHCI so
firmware input works, virtio disk/network/RNG and a qualified CoreAudio-backed
audio device. Keep microphone input disabled unless explicitly requested.
Use user-mode NAT, avoiding sudo/vmnet and the previously observed DHCP issue.

Instance state stores schema version, runtime receipt, base digest, display,
resource settings, process identity, private socket paths and last lifecycle
outcome. It is written atomically under an exclusive instance lock. Operations
refuse unknown schemas, symlink escapes, foreign directories and runtime/base
mismatches. Capture the live process start identity as well as PID; never kill a
process solely because an old state file contains that PID.

QMP, TPM and serial sockets live in a short mode-0700 runtime directory owned by
the caller, with limits checked before start. QMP messages have request IDs,
bounded reads and deadlines. No TCP QMP/serial console. An ordinary-user supervisor
exists only for the lifetime of its VM and owns its QEMU/swtpm children. It is not
a persistent machine service. It handles failed starts, window closure, Ctrl-C
and child exit, and updates stopped/failed state after reaping children.

Graceful shutdown uses the authenticated guest command when available, then a
bounded QMP power request if needed. A still-running guest leaves `down` failed
and recoverable; `--force` explicitly permits QMP quit/owned-child termination.
Do not use GPU savevm/suspend snapshots. Reopen the disk normally; reset means a
new instance from the immutable base. Preserve an existing instance when new
image inputs are requested; require a new name or explicit stopped-instance removal.

## Incremental sync shared by both environments

`up` prepares the host-native `sysroot` archiver once with the pinned Cargo tools
and records its build input identity. `sync` reuses it only while those source,
Cargo and toolchain inputs match; otherwise it asks for preparation. It does not
launch a Linux builder or implicitly compile anything.

1. Snapshot the working tree using existing Git snapshot rules and run the real
   host-native `sysroot source archive`. Resolve the actual environment target,
   not an architecture guess or an independently supplied mismatching target.
2. Compare the produced home baseline against the previous sync manifest. Reject
   rootfs/package changes for hot apply; identify them as requiring an image build.
   Accept only supported niri/Noctalia home paths; do not turn this into whole-home sync.
3. Read the current managed files. A file changed inside the guest since the last
   receipt is a conflict: report it and preserve it. App-written overrides are
   reported when they hide the requested setting; never clear them wholesale.
4. Stage the prospective config tree privately in the guest. Validate niri and
   the installed Noctalia version with their native validation commands before
   replacing live files. Validate managed deletions against include/dependency rules.
5. Under a guest sync lock, recheck original hashes, write backups and a small
   lab-only transaction record, then replace changed files and remove only
   previously managed files. Apply included files before their entrypoint;
   multiple file renames are not advertised as one atomic filesystem transaction.
6. Confirm niri's reloaded state; restart Noctalia only when its affected inputs
   require it, then check its IPC/native effective values. On failure, restore
   backups only if current hashes still match the transaction's writes. A concurrent
   writer leaves explicit recovery state rather than being overwritten.
7. Commit the managed-file receipt and optionally take a screenshot. A subsequent
   sync first resolves an incomplete transaction; it never claims a partial apply passed.

This transaction belongs only to disposable lab accounts. It does not replace
`sysroot home` or broaden production home-apply authority. A no-op sync reports
zero changed files and does not restart the desktop.

## Visual evidence, timing and harness completion

Guest `grim` capture is authoritative for the desktop; QMP screendump is a separate
firmware/greeter diagnostic and is not assumed to capture GL scanout. A successful
PNG header alone is insufficient qualification: open the PNG and match a deliberate
visible change to the live accelerated desktop. Record dimensions, display scale,
renderer evidence, source and capture timestamps in a sidecar JSON receipt.

Use actual niri logs and EGL/application probes to establish the GPU path, with
negative checks for llvmpipe/softpipe/lavapipe/SwiftShader. An ANGLE library loaded
on the host or a Vulkan device name alone cannot pass UC-01. Startup confirms an
already qualified renderer; explicit requalification follows runtime changes.

Keep JSON/JUnit reports and existing scenario capture actions. Add phase durations
for image resolution/build, system boot, session readiness, execution, capture
and teardown. Flush partial test results atomically after each case so cancellation
does not erase completed evidence. Record zero applicable cases as no coverage,
and avoid provisioning for a target-filtered empty selection.

Preserve serial scenario execution until isolation evidence justifies changing
it. Support deterministic forward/reverse case order for A4. Qualify A5/A6/A7/A10
by driving real public commands and real lifecycle failures: partial startup,
SIGINT, a bounded guest wait that cannot succeed, and an unavailable removal
operation. A cleanup error must remain visible beside the original test failure.
Run two executions alongside a retained manual session and verify only owned
resources are removed. Add no unit/model/mock tests or synthetic source scanners.

Cancellation is propagated into Docker I/O and readiness waits, not merely
checked between scenarios: the first signal stops scheduling, interrupts pending
waits and starts owned-resource teardown. A second signal may terminate the
runner, leaving its flushed report and watchdog/explicit cleanup path. Keep this
execution's exact created resource IDs so concurrent runs are not selected by a
broad label or name prefix.

Reuse existing signal support where possible; a direct `signal-hook` dependency
or Tokio signal feature is justified only for reliable cancellation. A safe
process/lock facility may reuse workspace `rustix` or standard APIs. No general
scheduler, connection pool or new test framework is needed.

| Operation | Runtime deadline (distinct from speed target) |
|---|---|
| Local preflight/tool query, individual QMP request | 10 s |
| QEMU boot plus authenticated session readiness | 120 s total |
| SSH connection | 5 s per connection, within the parent deadline |
| Hot sync, including validation and reload | 30 s, then bounded recovery |
| Screenshot | 10 s |
| Default explicit guest command | 60 s; caller may select a bounded override |
| Graceful shutdown | 30 s; force adds at most 10 s |
| Container failure diagnostics/cleanup | 30 s each, without replacing original error |
| Cold native tool preparation | 90 min overall, progress reported per phase |
| Cold VM image preparation | 60 min overall, existing step limits retained if lower |

The approved NFR targets remain unchanged: edit/capture median <=5 s and max <=10 s;
capture median <=2 s and max <=5 s; container startup median <=20 s; QEMU cached-disk
startup median <=45 s; current ARM suite median <=120 s and selected desktop case
median <=15 s. Measure five samples for each, separate cold work and preserve
unrelated host workloads. The 60 Hz / 95% <=33.3 ms presentation target at
2560x1600 scale 2 needs actual presentation data, for example an available native
Metal trace. Do not infer it from glmark2 average FPS or screen recording. If no
suitable instrument is available, retain NFR-05 as unverified, as requirements allow.

## UTM retirement, files and rollback

Only retire UTM-specific source after the native graphics/capture gate passes.
Move `tests/vm/utm-image/` to `tests/vm/qemu-arm64/` and rename the workflow to
`test-qemu-arm64.yml`. Update its trigger paths, job/artifact labels and runtime
markers together. Keep the target itself `utm`; preserve bootc version, Microsoft
CA 2023, explicit shim boot entry, Secure Boot, SELinux and failed-unit assertions.
This rename does not change `.github/workflows/release.yml` or signed identity.

Extract the local media build into `installer/qemu/kedra-media.py`, with its
existing `build-in-container.sh` and pins. Remove UTM create/plist/Apple Events
control after the replacement works. Leave a short retired-entrypoint explanation
at the old README path so historical links remain meaningful; do not rewrite
completed worklog entries. No UTM app uninstallation or VM deletion occurs.

Affected existing code: `tests/container/Cargo.toml` and
`tests/container/{lib,lab,builder,image,session,main,environment,docker,report}.rs`,
container lab assets/probes and README, the ARM VM fixture/workflow, installer
tooling, `tests/README.md`, root README and relevant PLAN/ARCHITECTURE/STATUS docs.
New code/assets are the modules and qemu/installer paths above. Update the
research, desktop, github-actions, machines and rust-workspace skills where their
contracts change; bump both first-party plugin manifests. AGENTS.md needs only
the new QEMU operation/testing guidance if its current instructions become stale.
No production release table/key/registry changes, personal configuration, or
unrelated desktop defaults are in scope.

Rollback is source-level: revert the development-tool change and keep all owner
VMs/media untouched. Do not auto-downgrade new instance schemas; old tools refuse
them. Retain runtime/base receipts so a stopped development instance remains
reopenable with its exact runtime. Corrupt/incomplete states require explicit
diagnosis and never authorize broad cleanup.

## Phases, coverage ownership and risks

| Phase | Exit evidence | Requirements |
|---|---|---|
| 1. Runtime and one disposable VM | Fully locked build closure; HVF, enforced boot chain, GPU desktop, real input and visible capture on this Mac | UC-01, NFR-05/06 |
| 2. Retained VM and shared hot iteration | Lifecycle/auth/isolation failures plus five-sample timings; same desktop before/after an edit | UC-02/03, NFR-01/02/03/07/08 |
| 3. Harness completion and UTM retirement | Existing suite, order/concurrency/failure exercises, unchanged boot assertions, updated docs and reproducible owner commands | UC-04/05, NFR-04/06/07/08 |

Runtime feasibility gates retirement, not independent container improvements.
Container cases own portable installed-system behavior; local QEMU owns Mac GPU
and manual interaction; CI QEMU owns existing boot-level regressions. No container
result claims hardware, firmware, PAM login or native GPU qualification.

| Risk | Likelihood / impact | Mitigation and remaining limit |
|---|---|---|
| Pinned Mac graphics stack fails on macOS 27 | High / high | First implementation gate; no automatic software substitution or speculative retirement |
| Building ANGLE/runtime has high first-use cost | High / medium | One-time private cache, immutable receipts; report cold cost separately |
| Firmware and GPU-ready configuration do not combine under HVF | Medium / high | Boot exact candidate with Secure Boot before adopting runtime; preserve CI separately |
| Lab transport/seed escapes fixture scope | Low / high | Generated credentials, private paths, strict host keys, no personal mounts and no production payload changes |
| Sync overwrites live edits or produces partial config | Medium / high | Hash recheck, conflict refusal, validation, backups and explicit recovery |
| Warm timings miss targets | Medium / medium | Report each phase; optimize measured cost without reducing coverage |
| Failure/cancellation leaks resources | Medium / medium | Owned supervisor/container IDs, bounded teardown and real concurrent lifecycle exercises |
| Frame-presentation instrument unavailable | Medium / low | NFR-05 remains unverified; manual smoothness never becomes a quantitative claim |

Design self-review resolved the main risks of a cached binary from another source,
container-only adaptations leaking into the VM, shared SSH host keys across clones,
PID-only termination and treating an apparently valid PNG as accelerated capture.
The owner approved this design on 2026-09-28. The [tasks](tasks.md),
[cases](test-cases.md) and [verification plan](test-plan.md) are now reviewed and
reconciled and were approved for implementation on 2026-09-28. Container-side
implementation and verification are recorded in the test plan; the native runtime
remains blocked and unqualified.

## Primary references

- [Integrator's pinned source set](https://github.com/akihikodaki/v/tree/cd8293b463ed9963f511a23b2b80025c65a81303), especially `.gitmodules`, `run`, `update` and the Git tree; read only, not executed.
- [QEMU VirtIO GPU modes](https://www.qemu.org/docs/master/system/devices/virtio/virtio-gpu.html): 2D/software versus accelerated backends.
- [QEMU invocation](https://www.qemu.org/docs/master/system/invocation.html): explicit accelerators, device/display options and private control transports.
- Current repository implementation and existing ARM Secure Boot fixture at the inspected base; they define the safety and compatibility behavior to preserve.
