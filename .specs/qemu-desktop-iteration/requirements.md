# Fast QEMU and container desktop iteration

Status: approved by the owner on 2026-09-28. No replacement implemented.
Date: 2026-09-28. Inspected base: `a3a39a4a8a1d7777f02ab68d80ad4f39abf40c87`.

## Owner amendment — 2026-09-28

The owner requires completing the remaining graphics/audio/frame/installer/fault/CI
qualification and explicitly reverses the earlier preservation decision. Replace
the signed `utm` identity with `qemu-arm64` and retire old Kedra UTM VMs. The
current implementation review is a partial baseline, not acceptance. Historical
source/run evidence keeps its original names.

## Problem and desired result

The owner wants to remove UTM from the development workflow, manually exercise the
complete Kedra desktop in a fast GPU-rendered QEMU window, and give the agent a
complete Testcontainers workflow with visible desktop changes and quick feedback.
The first delivery targets this Apple Silicon Mac (confirmed by the owner).

Current source already supplies container scenarios, native end-to-end checks,
desktop screenshots and retained sessions. Extending that implementation is the
starting point. A second container runner or a replacement scenario language is
unnecessary. The missing result is a qualified UTM-independent graphical VM plus
a reliable, measured edit/sync/inspect loop across both environments.

There is no measured count of slow iterations or current VM latency. The owner
reports the need for faster iteration; proposed timing budgets below are targets,
not observed performance or an invented service-level guarantee.

## Evidence and assumptions

| Claim | Provenance and limitation |
|---|---|
| This machine is an Apple M2 Pro with 32 GiB RAM, arm64 macOS | Read-only `sysctl` and `uname` on 2026-09-28; one host |
| Standalone QEMU, qemu-img and swtpm are absent from PATH | Read-only tool lookup, 2026-09-28; UTM is installed but its private frameworks do not establish a standalone runtime |
| OrbStack supplies Docker 29.4.0 on aarch64 | Live Docker Engine readback, 2026-09-28 |
| Containers deliberately use software rendering | `tests/container/lab/niri-nested.conf` sets `LIBGL_ALWAYS_SOFTWARE=1`; the parent compositor uses pixman |
| UTM-named CI already runs plain QEMU | `.github/workflows/test-utm-image.yml` and `tests/vm/utm-image/boot.sh`; native ARM image build followed by TCG Secure Boot boot |
| Existing harness covers the real desktop | `tests/container/{main,session,scenario,native}.rs`; current-source `cargo test ... -- --list` passes and lists 14 cases across both targets without provisioning |
| Prior container speed | Historical WL-20260927-02: 13 ARM cases in 101 s, session startup about 3–7 s, full cached up about 16 s, sync about 2 s; not remeasured for this proposal |
| Existing CI | Exact-head workspace [36326037562](https://github.com/Reidond/kedra/actions/runs/36326037562) passes; both-target container run [36323569209](https://github.com/Reidond/kedra/actions/runs/36323569209) passes at earlier `490e13a` |
| Old GPU evidence | STATUS records VirGL/Venus on the owner's UTM guest, 2026-09-26; this does not qualify a new launcher, build or display backend |
| Standalone Mac GPU feasibility | Primary sources describe compatible patches; no selected standalone build has run here yet. UC-01 is the first implementation gate |

Source paths above are relative to `usr/src/kedra/` unless rooted at `.github/`.
Ignored prior lab artifacts and unrelated Docker workloads are not evidence of
this implementation and must remain untouched.

## Actors

- **Owner:** uses the VM window for manual input, animation, resize and application checks.
- **Agent:** uses the existing lab and Cargo harness to sync scoped changes, run scenarios,
  inspect screenshots and report actual failures.
- **CI runner:** executes portable container coverage and remaining boot-level QEMU checks.

No model API or unattended agent daemon is introduced.

## Scope and compatibility

- Direct native ARM QEMU with HVF for the local VM, a GPU-capable macOS display
  backend, persistent development sessions, screenshots and bounded lifecycle commands.
- Extend `kedra-lab` and the existing Testcontainers crate; keep one container harness.
- Reuse immutable image stages and cached build inputs. Support fresh disposable
  test disks and retained development sessions without reinstalling on every edit.
- Retire the repository's UTM launcher/bundle/Apple Events dependency after the new
  path passes its acceptance checks. Preserve the ARM Secure Boot CI coverage under
  QEMU-oriented naming, together with all other necessary VM assertions.
- Preserve the signed target ID `utm`, its image repositories, public
  authority and release history. Its name is compatibility data, not a requirement
  to use the UTM app. This scope was approved with the requirements;
  a future rename would require a separate explicit migration scope before edits.
- Retain the installed UTM app, existing owner VM bundles/disks/NVRAM/TPM state and
  installer media. Removing the development dependency does not authorize deleting
  personal VMs or importing their credentials into test fixtures.
- Linux GPU hosts, x86 emulation speed, physical hardware, USB passthrough,
  owner accounts/vaults, suspend/resume and live GPU memory snapshots are outside
  this delivery. Existing Linux CI continues to run.
- No commits, pushes, production publication, workstation enrollment or raw host-disk
  changes are implied. Missing external tool installation requires explicit owner
  authorization under AGENTS.md; runtime preflight must never install silently.

## Required use cases

### UC-01 — Establish an independent accelerated QEMU runtime

Actor: Agent. Trigger: run the runtime qualification command on this Mac.
Preconditions: reviewed, immutable QEMU/graphics/firmware/TPM inputs are available;
any required external installation has been authorized.

Main success scenario:
1. Record executable paths, versions, input hashes, host CPU/OS and renderer configuration.
2. Start a disposable ARM guest with HVF and a native interactive GPU display.
3. Boot the supported Fedora/Kedra chain with Secure Boot and SELinux enforcing.
4. Obtain actual compositor and application renderer evidence identifying the host
   GPU; exercise a visible animated application in the same session.
5. Capture that displayed content, verify keyboard/pointer input, and record frame
   timing at the declared resolution. Mere Vulkan device enumeration is insufficient.

Exception flows: missing tools, incompatible libraries/firmware, unavailable HVF,
software fallback, a blank captured scanout or failed input produce a specific
failure. Do not silently select TCG or llvmpipe for the accelerated profile.
An explicitly chosen software/debug profile must identify itself as such.

Failure postconditions: no host settings changed, no owner VM opened, no GPU pass
claimed. Keep bounded diagnostics and remove only resources created by this attempt.
UTM tooling removal waits for this gate.

### UC-02 — Manually operate a retained desktop

Actor: Owner. Trigger: start a named QEMU development VM from a prepared lab image.
Preconditions: UC-01 passed for the selected runtime; generated development account
and lab-only access are separated from published OS payloads.

Main success scenario:
1. Reuse the compatible prepared base disk and create or reopen the named writable instance.
2. Show the real niri/Noctalia desktop in a native window with keyboard, pointer,
   scrolling and selectable display scale/resolution; expose usable guest audio.
3. Open GTK 3/4, Qt 5/6 and Xwayland applications; exercise file choosers, panels,
   window movement and animations manually.
4. Let the agent capture the same desktop and run bounded lab commands through an
   authenticated, instance-specific connection.
5. Stop cleanly and reopen with that instance's disk, firmware and TPM identity preserved.

Exception flows: a busy instance, stale process identity, occupied endpoint, wrong
guest identity or incompatible base disk refuses without modifying another VM.
Guest access does not remove the production guest-agent RPC block list. Host
network endpoints bind to loopback; QMP/serial use private sockets where possible.

Failure postconditions: retained data stays recoverable; failed startup does not
leave an untracked QEMU/TPM/transport process. Forced termination, when necessary,
is explicit and limited to the owned instance. Reset creates a fresh instance from
the immutable base; it never restores unsupported live GPU state.

### UC-03 — Apply a desktop edit and inspect the result

Actor: Agent or Owner. Trigger: sync a scoped working-tree desktop change to a
named running lab session, in either the container or the qualified QEMU VM.

Main success scenario:
1. Identify the session's target and source; show exactly which supported files changed.
2. Validate the prospective configuration with the actual installed applications.
3. Apply changed managed files and managed deletions within the disposable account,
   preserving unrelated files and application state.
4. Reload or restart only affected session components and confirm readiness.
5. Produce a fresh screenshot with source, environment and renderer identity plus
   elapsed time; the agent opens it and reports the visible effect.

Exception flows: invalid configuration keeps the last usable configuration;
concurrent sync is serialized or refused. A target mismatch or package/system
change outside the hot-sync contract requires an explicit rebuild/recreate path.
An application-written override that hides the edit is reported, not silently erased.

Failure postconditions: partial sync is rolled back or explicitly recoverable;
no successful preview of unapplied changes is claimed. Normal home-only changes
cause no RPM transaction, Rust rebuild, image pull, VM reboot or OS reinstall.

### UC-04 — Run automated desktop coverage and see failures

Actor: Agent or CI runner. Trigger: run a selected group or the full existing
Testcontainers suite using Cargo.

Main success scenario:
1. Validate/filter before provisioning and resolve one immutable image plus scoped overlay.
2. Start isolated systemd/session environments and execute actual installed-system workflows.
3. Retain current image/session/Noctalia/portal/keyring/agent/Bitwarden/home/GTK coverage.
4. Capture meaningful desktop checkpoints and failure screenshots, application state
   and bounded logs; emit existing JSON/JUnit evidence with source and phase timings.
5. Clean resources owned by that execution; local failed-session retention remains explicit.

Exception flows: wrong visual expectations must fail; timeout, partial startup,
cancellation and teardown failures preserve the original cause and bounded
diagnostics. Cancellation cleans only this execution's containers and processes.
Two executions and a retained manual lab must coexist without interference.

Failure postconditions: no unrelated resource is removed; cleanup failure is visible.
Reversed-order execution, cancellation, async timeout and teardown failure are
qualified through public workflows (the currently incomplete harness acceptance
cases). Do not add unit tests or source-text scanners to prove them.

### UC-05 — Replace UTM-specific tooling without losing boot coverage

Actor: Agent. Trigger: complete the switch to the qualified QEMU development workflow.

Main success scenario:
1. Replace repository-local UTM create/control instructions and commands with the
   tested QEMU lifecycle and a documented installer-media entrypoint.
2. Keep the ARM QEMU CI Secure Boot assertions, required doctor checks and SELinux
   checks, plus the existing desktop login, PAM, update/rollback and installer coverage.
3. Update affected documentation, route skills and plugin versions as applicable.
4. Confirm the normal local workflow works with UTM closed and without calling its
   executable, utmctl, Apple Events or libraries inside the UTM application bundle.

Exception flows: a missing replacement capability leaves the retirement incomplete
and reports the gate. Preserve historical evidence as historical; do not globally
rewrite worklog entries or signed target identities to make the naming uniform.

Failure postconditions: prior source remains recoverable in Git, owner VM data is
untouched and no existing security check is disabled just to make the replacement pass.

## Performance and operational requirements

These are proposed warm-loop budgets for this M2 Pro, pending measurement. Use five
repeats, record every sample and report median and maximum. Identify contention;
never silently stop unrelated workloads to improve numbers. Cold downloads, Rust
compilation, tool builds and first disk creation are measured separately.

| ID | Target / invariant | Verification |
|---|---|---|
| NFR-01 | Home edit to ready desktop plus screenshot: median <=5 s, maximum <=10 s | Timed UC-03 on both environments; include validation and screenshot transfer |
| NFR-02 | Screenshot of an already running desktop: median <=2 s, maximum <=5 s | Timed capture plus visual inspection of actual desktop content |
| NFR-03 | Cached container start to usable session <=20 s median; retained QEMU disk to usable desktop <=45 s median | Timed public startup, no rebuild/pull; report automatic session startup separately from real password login |
| NFR-04 | Existing ARM container suite <=120 s median after image preparation; selected desktop scenario <=15 s median | Timed public Cargo workflow; image prep is separately reported |
| NFR-05 | Manual GPU session targets 60 Hz at 2560x1600, scale 2, with >=95% of presented-frame intervals <=33.3 ms during a 30 s active animation | Capture supported presentation timing and inspect interaction; renderer evidence required; if instrumentation is unavailable, mark timing unverified |
| NFR-06 | Zero automatic tool installs, production pushes, raw host-disk/bootloader writes or modifications to owner VM data | Manual execution review and owned-resource before/after evidence |
| NFR-07 | Zero cross-execution cleanup; startup/test actions have finite deadlines and teardown has a bounded grace period | Public lifecycle and cancellation/failure exercises; exact deadlines selected in design |
| NFR-08 | Every reported image/desktop result names source, target, runtime/rendering mode, display and timing scope | Inspect generated evidence from actual runs; no documentation/source scanner |

Missing a speed target is a reported failed target with measured data and a
specific next optimization; it is not permission to remove checks or relabel
software rendering as GPU accelerated. Fresh automated test fixtures remain
isolated even though manual development sessions are retained.

## Graphics feasibility and alternatives

QEMU's [VirtIO GPU documentation](https://www.qemu.org/docs/master/system/devices/virtio/virtio-gpu.html)
distinguishes the software 2D path from VirGL and rutabaga acceleration. Enabling a
virtio display or HVF alone does not establish guest 3D acceleration.

The initial candidate is native QEMU/HVF with a reviewed macOS Cocoa/VirGL/ANGLE
path to Metal. Venus/Vulkan is a separately qualified capability; it cannot
replace proof that niri and the applications actually render through the GPU.
Preserve current GTK GL compatibility until real evidence supports changing it.

The [startergo implementation](https://github.com/startergo/homebrew-qemu-virgl-kosmickrisp)
documents Cocoa/ANGLE support. Its inspected [formula at 45425d8](https://github.com/startergo/homebrew-qemu-virgl-kosmickrisp/blob/45425d89012e5cc54db5bdeefba3021eeb8ea9c5/Formula/qemu.rb)
downloads mutable QEMU master and invokes pip with `--break-system-packages`.
It is a source lead, not an approved installer: do not execute that recipe. A
usable runtime needs immutable reviewed inputs, a verified dependency closure,
and project-compliant host tooling. No exact build is selected by these requirements.

The minimal delivery is one Mac QEMU backend plus the existing software-rendered
container harness. Keeping cocoa-way alone would retain software guest rendering;
it does not meet the manual GPU requirement. Reusing UTM's private frameworks
would retain the dependency the owner wants removed. Replacing QEMU with another
hypervisor or building a GPU-backed Docker host is not assumed authorized scope.

## Expected affected areas and risk boundaries

Design will assign exact files and commands after requirements approval:

- `usr/src/kedra/tests/container/`: extend lab lifecycle, bounded sync, screenshots,
  timing/provenance and existing harness failure workflows.
- `usr/src/kedra/tests/vm/` and `.github/workflows/test-utm-image.yml`: preserve
  assertions while removing UTM-specific operational naming.
- `usr/src/kedra/installer/utm/`: replace UTM-specific control, retaining the
  reviewed media-building capability through an appropriate QEMU entrypoint.
- Test/installer READMEs, root README, PLAN, ARCHITECTURE, STATUS, relevant first-party
  skills/manifests and `worklog.md`: document the actual new workflow and limits.
- Any proposed dependency additions and Cargo manifest changes require a concrete
  implementation reason. Source/release identity and signing configuration remain
  unchanged under the provisional identity-preservation scope.

| Risk | Required response |
|---|---|
| Native GPU display works but capture is blank or stale | Qualify screenshot and input on the same accelerated session before retiring UTM |
| Mac graphics patches are incompatible or costly to maintain | First prove a pinned minimal stack; keep a concrete failed/blocked result instead of a software fallback |
| Standard Mac containers do not expose the Apple GPU | Identify their software-rendered coverage honestly; use the native VM for manual acceleration checks |
| Live sync is fast but wrong/incomplete | Validate first, track managed deletions, preserve overrides, and inspect resulting pixels |
| New VM disk preparation dominates every iteration | Cache compatible immutable bases; measure first provisioning separately from reuse |
| Secure firmware/TPM replacement breaks boot identity | Each instance owns stable NVRAM/TPM data; fresh instances get fresh state; never modify owner state |
| Broad cleanup or shared builder caches race | Keep execution ownership and isolation; exercise concurrent public workflows |

## Approval and next stage

This document specifies requested outcomes and evidence, not an implementation
claim. The owner approved these requirements and the concrete design on 2026-09-28.
The [tasks](tasks.md), [cases](test-cases.md) and [verification plan](test-plan.md)
were approved for implementation on 2026-09-28. Current partial results and
remaining native-runtime gates are recorded in the test plan. No forbidden
unit/model, mock or repository-source tests are planned.
