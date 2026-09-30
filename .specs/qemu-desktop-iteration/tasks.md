# Implementation tasks

Status: implementation authorized by the owner on 2026-09-28 ("do it").
The native GPU and container workflows are implemented and locally exercised.
The complete qualification matrix is not closed; see the per-task outcomes and
[delivery review](review.md). Requirements and design were approved on 2026-09-28.
Sources: [requirements](requirements.md), [design](design.md),
[cases](test-cases.md), [verification plan](test-plan.md).

## Phases and dependencies

| Phase | Tasks and dependencies | Exit evidence |
|---|---|---|
| 1. Prove the native graphics path | T01 → T02 → T03 → T04 | Fully locked runtime; one disposable native GPU desktop, input and usable capture; no UTM dependency |
| 2. Make retained iteration fast | T05 → T06; T02/T03 also required for VM sync; T07 depends on T06 | Conflict-safe sync, meaningful screenshots and retained VM lifecycle |
| 3. Complete coverage and retire UTM tooling | T08 depends on T05/T07; T09 depends on T03; T10 depends on T04/T06/T07/T08/T09; T11 depends on T07/T08; T12 depends on all | Functional gates, measured targets, preserved boot coverage, documented owner workflow |

Independent container work in T05–T08 may proceed if the GPU experiment is blocked.
Their VM-dependent checks remain not-run; that does not permit T10 retirement.
This is a dependency graph, not authorization to create parallel agents or work
concurrently in a shared checkout. Phase-one and phase-two quality gates are
standard review plus relevant compiler/linter/end-to-end checks. Phase three gets
the full post-task review and the verification plan's complete required gates.

All paths below are relative to `usr/src/kedra/`, except explicit root paths.
No task modifies registered owner VMs or personal tools/configuration. The owner
subsequently directed replacement of the signed `utm` identity, so T10 includes
the reviewed `qemu-arm64` target/repository/authority migration. No unit/model/mock
tests, doctests, source assertions or repository scanners are introduced.
No task independently authorizes external tool installation, commit, push or CI dispatch.

## T01 — Lock and prepare the private native runtime

- **Size / dependencies:** L; none after packet approval.
- **Mechanism:** remove UTM runtime dependence through the approved pinned QEMU,
  ANGLE, epoxy and VirGL source set, with an explicit one-time preparation command.
- **Files:** new `tests/container/qemu/inputs.json`, `prepare-runtime.py` and
  runtime license/build metadata assets; `tests/container/lab.rs`, `lib.rs`,
  new `vm.rs`, and `Cargo.toml` only as needed for dispatch.
- **Acceptance:** `tools check` is read-only; lock the full selected ANGLE DEPS/CIPD,
  swtpm/libtpms and firmware closure before building. Report exact missing native
  tools for owner authorization. Build through uv and existing native tools in
  a private prefix; verify hashes, dylib closure, Hypervisor entitlement and the
  firmware's Microsoft CA 2023 enrollment. Interrupted preparation cannot select
  a partial runtime. Preserve host installations and never execute upstream
  update/Pipenv/sudo scripts.
- **Test requirements:** TC-01, TC-02, TC-28.
- **Status:** [x] Implemented and exercised — dependency locks, private build/package
  replay, hashes, dylib closure, Hypervisor signature and CA2023 firmware inspection
  pass. The rebuilt 218 MiB external runtime passes unchanged replay and all
  prerequisite/reusability checks. A real interrupted private preparation exits
  143 and incomplete pins exit 1 without selecting a runtime or receipt; the
  selected runtime hash remains unchanged. See review for the upstream WFI fix.

## T02 — Prepare reusable VM disks and instance-specific guest access

- **Size / dependencies:** L; T01.
- **Mechanism:** pay OS/disk provisioning cost once, while keeping each writable
  instance and its credentials independent.
- **Files:** `tests/container/image.rs`, `localbuild.rs`, `builder.rs`, `vm.rs`;
  new VM fixture Containerfile, seed service and guest transport helper under
  `tests/container/qemu/` / `lab/`; reuse installer builder operations as appropriate.
- **Acceptance:** factor base/payload preparation from container adaptations;
  cache disks by complete compatible inputs; record provenance. The VM layer
  preserves native DRM, enforcing SELinux and base RPM versions. Generate fresh
  client/server keys and seed per instance, pin SSH host identity, disable personal
  SSH config/agents and scope sudo access to the fixture-only helper. Lab access
  is absent from production images and signed installer media.
- **Test requirements:** TC-02, TC-06, TC-08, TC-28. Transport execution depends
  on T03; do not mark its qualification complete before that run.
- **Status:** [x] Implemented and exercised — raw-to-sparse-QCOW2 preparation,
  canonical Docker/Podman identity check, cache receipt, native security and
  per-instance generated pinned keys. Runtime/base disk are restored outside Cargo
  output. Two real instances retained separate writable state; cross-key SSH denied.

## T03 — Own QEMU processes and retained instance lifecycle

- **Size / dependencies:** L; T02.
- **Mechanism:** a named VM reopens directly, with bounded control and no UTM GUI,
  Apple Events, shared fixed ports or PID-only termination.
- **Files:** `tests/container/lab.rs`, `vm.rs`, the standard-library `qemu/vm.py` controller,
  `lib.rs`, `Cargo.toml` and root `Cargo.lock` for justified existing-library use only.
- **Acceptance:** implement native up/status/exec/logs/down/remove, private QMP/TPM
  sockets, instance lock and process-start identity. Preserve disk/VARS/TPM across
  clean restarts. Report port/running-state/schema/runtime/ownership failures
  accurately; no automatic software fallback. Graceful and explicit forced stop
  follow design deadlines. Already-prepared VM operations do not connect to Docker.
- **Test requirements:** TC-06, TC-07, TC-08, TC-09, TC-28.
- **Status:** [x] Implemented and exercised — retained lifecycle, QMP/process identity,
  private locks, Docker-independent operations and historical five-start evidence.
  The restored public CLI additionally refuses mutable-file hard links, TPM-tree
  links, log links and a predictable pending-file link against a private clone,
  preserving foreign data and the running default. Nonresponsive 10-second and
  literal grace variants both pass: the latter reports accepted poweroff after
  31.336 s without implicit force, verifies a 45-second unit ignores TERM, and
  explicit force completes in 1.487 s preserving disk/VARS/foreign sentinel before
  public removal. Its default was stopped; running-default evidence comes from the
  separate 10-second variant.

## T04 — Qualify native graphics, input and actual captured content

- **Size / dependencies:** M; T03.
- **Mechanism:** prove the complete visible rendering path before relying on it.
- **Files:** `tests/container/vm.rs`, guest probes and runtime diagnostic receipts;
  `tests/container/README.md` qualification instructions. Reuse existing capture code.
- **Acceptance:** run the UC-01 gate on this Mac with Secure Boot, enforcing SELinux,
  GPU-backed niri/app rendering, real input and captured visible content. Exercise
  toolkit/resize/scale/audio behavior and software/failed-start negative paths.
  Candidate incompatibility is a fail/blocker, not permission to retire UTM or
  silently change requirements. Save runtime/source evidence in ignored artifacts.
- **Test requirements:** TC-03, TC-04, TC-05, TC-27. Presentation timing is
  measured at this gate when instrumentation exists; its explicit exception
  remains available. The complete sync-based independence case belongs to T10.
- **Status:** [x] Core GPU path and all six GTK3 Wayland/Xwayland, libadwaita,
  Qt5 and Qt6 chooser workflows pass, with captures viewed. Audible audio passed
  twice with owner confirmation; physical `abc` and Command+Enter pass. Five
  visible 30 s EGL/SHM presentation runs meet the interval target for guest Virtual-1.
  Physical scrolling/window movement and resize to 1920 pass; unlocked scale
  1/1.5/2 screenshots were independently viewed, and reopening QEMU restored actual
  2560×1600 scale 2. Native remains GPU-only; software diagnostics stay in Testcontainers.

## T05 — Separate source archiving from Linux compilation

- **Size / dependencies:** M; independent of the GPU gate.
- **Mechanism:** home-only sync uses a prepared, current host-native archiver and
  the existing source resolver, without invoking the Linux binary builder.
- **Files:** `tests/container/builder.rs`, `image.rs`, `lab.rs`, new `lab_sync.rs`.
  Change the production source resolver only if a concrete compatibility defect
  is found; do not reimplement its layout, target or credential rules.
- **Acceptance:** prepare the archiver once; reject a stale archiver with an
  actionable prepare step. Snapshot the actual tree, resolve its target and produce
  the payload. Separate archive/binary caches and protect shared cache publication
  against concurrent executions. Hot sync invokes no compiler/pull/RPM transaction.
- **Test requirements:** TC-10, TC-11, TC-17, TC-25.
- **Status:** [x] Container implementation verified — prepared native archiver, stale-input refusal, no hot rebuild, isolated content-addressed build objects and locked Linux build/copy. Final builder/session smoke passes; see review.

## T06 — Apply validated incremental home changes

- **Size / dependencies:** L; T05; T02/T03 for the VM transport.
- **Mechanism:** shared sync planning computes additions/changes/deletions; the
  real environment validates and reloads only affected supported components.
- **Files:** `tests/container/lab_sync.rs`, `lab.rs`, `session.rs`, `vm.rs`, and
  the guest lab helper. This is lab-account tooling, not production `sysroot home`.
- **Acceptance:** stage/validate the prospective config, recheck hashes under a
  lock, preserve guest edits/overrides, apply managed deletions, and record backups
  before mutation. Confirm effective readiness; recover an interrupted apply
  without overwriting newer guest changes. Refuse package/rootfs/target mismatches.
  No-op sync performs no writes or desktop restart. Support `sync --shot`.
- **Test requirements:** TC-10, TC-11, TC-12, TC-13.
- **Status:** [x] Implemented and exercised in both transports — container add/delete/no-op, validation, conflicts/overrides, interrupted-write/newer-edit recovery and locking; native valid/invalid edits and Docker-independent sync/capture pass.

## T07 — Produce trustworthy visual receipts and phase timings

- **Size / dependencies:** M; T06.
- **Mechanism:** use compositor screenshots and explicit timing/provenance so the
  agent can inspect the applied desktop and distinguish cold work from iteration.
- **Files:** new `tests/container/capture.rs`; `session.rs`, `vm.rs`, `lab.rs`,
  `report.rs`, `main.rs` and relevant guest probes.
- **Acceptance:** verify capture content/dimensions in real runs; report renderer,
  source, target, display and timing scope. Separate firmware diagnostics from
  desktop capture. Make partial reports durable after each test. Exclude generated
  secrets before evidence export. No PNG-header-only GPU pass.
- **Test requirements:** TC-13, TC-14, TC-18, TC-25, TC-26, TC-28.
- **Status:** [x] Implemented and exercised — container/native screenshots carry actual display/source receipts; reports survive interruption. Captured content was viewed. Readiness and captures deliberately do not automatically grant full GPU qualification.

## T08 — Complete the existing container harness lifecycle checks

- **Size / dependencies:** L; T05/T07.
- **Mechanism:** preserve current installed-system scenarios while fixing measured
  filtering, cancellation, cleanup, ordering or isolation gaps in that same harness.
- **Files:** `tests/container/{main,environment,docker,report,scenario,native}.rs`,
  existing YAML scenarios/setups/probes, and `.github/workflows/test-container.yml`
  only where resulting evidence/upload configuration needs adjustment.
- **Acceptance:** no provisioning for empty applicable selection; strict public
  input validation; durable failure reports; deterministic reversed execution;
  interruptible Docker waits; bounded cleanup preserving original errors. Exercise
  two executions beside a retained lab. Use an isolated disposable Docker engine
  for engine-loss tests, never interrupt the owner's shared OrbStack engine.
  Keep one-scenario concurrency unless measured evidence justifies a later change.
- **Test requirements:** TC-15, TC-16, TC-17, TC-18, TC-19, TC-20.
- **Status:** [x] Core harness passes — five stable current worktree-overlay suites
  pass 13/13 with median 65.563 s/max 72.787 s, alongside reverse/two-worker
  runs, empty selection, malformed input, wrong expectation, long-name isolation,
  active/startup cancellation, forced second signal, real eventual/fractional
  deadline probes and private-engine-loss recovery. Main container CI passes ARM
  and x86. Post-optimization no-overlay run
  `1790706590-5077` passed 12/13 and correctly refused absent image provenance;
  corrected worktree-overlay run `1790706944-44808` passes 13/13 with two workers
  in 60.07 s. Two simultaneous independent executions now pass with isolated
  cleanup while preserving the retained lab. Main container CI passes on both
  architectures; the earlier deleted-Quay-input failure remains recorded.

## T09 — Preserve media creation and manual installation

- **Size / dependencies:** M; T03.
- **Mechanism:** extract the working signed-media builder and attach its verified
  output to a distinct manual-installation instance type.
- **Files:** new `installer/macos/media.py`, relocated builder script/pins;
  `tests/container/vm.rs`, `lab.rs`; installer READMEs and `docs/INSTALL.md`.
- **Acceptance:** preserve exact-digest/signature/manifest/hash/new-output checks;
  create only empty owned disks, retain original ISO, detach only while stopped.
  Installer-mode startup reports window/process state, not lab readiness, and
  refuses automatic exec/sync. Qualify a fresh disposable installation and ISO-free boot.
- **Test requirements:** TC-21, TC-22, TC-28.
- **Status:** [x] Qualified — signed media generation and refusal matrix pass.
  Corrected signed image 9d6/ISO cb8578 was freshly installed encrypted on only the
  new 96 GiB target with its 64 MiB sentinel unselected. Original ISO/sentinel full hashes
  and stopped detach pass. Visible ISO-free LUKS unlock and greetd login work;
  installed doctor, Secure Boot/lockdown, enforcing SELinux, Metal, LUKS2, bootc
  exact signed image status and read-only TPM/PCR7 dry-run pass. The first missing
  initramfs-driver failure remains historical. Final graceful stop timed out and
  explicit public force was required; post-stop integrity hashes still match.

## T10 — Retire UTM-specific source and retain boot-level coverage

- **Size / dependencies:** M; T04/T06/T07/T08/T09 all qualified.
- **Mechanism:** replace UTM launch/control paths with the proven native workflow,
  migrate operational CI naming and replace the signed target identity as later
  explicitly directed by the owner.
- **Files:** `tests/vm/utm-image/` → `tests/vm/qemu-arm64/`;
  `.github/workflows/test-utm-image.yml` → `test-qemu-arm64.yml`;
  UTM-specific installer script/pins after extraction; old launcher directory removed as explicitly requested. Update inbound operational references, not historical entries.
- **Acceptance:** no normal local command depends on UTM binaries/libraries or
  Apple Events. ARM bootc/Secure Boot/SELinux/unit assertions and remaining VM
  coverage run on the resulting source. Use a fresh per-target authority and
  repository names without reusing the old key. Retire old external resources only
  after the first replacement publication is independently verified.
- **Test requirements:** TC-23, TC-24.
- **Status:** [x] Superseded source and external identity resources are removed after
  native GPU/control/sync/capture
  proof and explicit owner cleanup instruction. ARM workflow/scenarios/observer
  renamed with assertions preserved. Source, repository names and fresh authority
  now use `qemu-arm64`. The first strict signed stable publication passes and old
  packages return 404; the old signing environment is absent. Main container CI
  passes both architectures. Main ARM run 36617035132 attempt 1 retains its
  90-minute PID 1 freeze; exact same-SHA attempt 2 passes with a 5m06s boot step.

## T11 — Measure the complete warm loop

- **Size / dependencies:** M; T07/T08 and qualified VM path for its measurements.
- **Mechanism:** measure the public operations in five real repetitions, with
  cached preparation separated from runtime/iteration; optimize the observed cost.
- **Files:** existing CLI/report timing fields as necessary; ignored artifacts;
  final measured results in test README/status/worklog, not new benchmark runners.
- **Acceptance:** report every sample, median and maximum for the approved targets.
  Distinguish full existing ARM suite from deliberate failure exercises. Use actual
  frame presentation data from T04 if its runtime/display/rendering inputs remain
  identical; remeasure after relevant changes. Unavailable instrumentation leaves
  NFR-05 unverified. Do not stop unrelated workloads or remove assertions to improve timings.
- **Test requirements:** TC-25, TC-26, TC-27.
- **Status:** [x] Complete warm-loop measurements pass. Five stable worktree-overlay
  suites have median 65.563 s/max 72.787 s; filtered desktop median is 6.680 s and
  cached public `up` median 4.091 s. Final Noctalia 5.2 sync+shot median is
  4.453 s/max 5.136 s; five native starts median 18.690/max 23.580 s and captures
  median 0.937/max 1.055 s. Five Virtual-1 presentation rates span 96.57–101.01/s,
  median 99.055, with p95 interval 16.667 ms and `sce_` in every run. Scope excludes
  physical monitors/other hosts.

## T12 — Final review, standard gates and owner handoff

- **Size / dependencies:** M; T01–T11, with truthful partial reporting if a gate fails.
- **Mechanism:** reconcile implementation and evidence, then provide reproducible
  commands and actual limitations instead of a scaffolded-as-complete claim.
- **Files:** root `README.md`, `AGENTS.md` only if its operations change, `worklog.md`;
  `PLAN.md`, `docs/{ARCHITECTURE,STATUS,INSTALL}.md`, test/container/installer READMEs;
  affected research/desktop/github-actions/machines/rust-workspace skills and both
  first-party plugin manifests. Keep specs through incomplete implementation.
- **Acceptance:** full post-task review; appropriate formatting, Clippy, ruff,
  release build and CLI/interop/material checks; all required case outcomes
  recorded with exact source and artifacts. Prove foreign resources and private
  data boundaries using generated sentinels. Show desktop screenshots to the owner.
  Handoff includes first setup, fast daily commands and recovery, measured costs,
  and every fail/blocked/not-run result. No commit/push without authorization.
- **Test requirements:** TC-28, plus completion review of TC-01 through TC-27.
- **Status:** [x] Delivery qualified — PR23/24/25 are merged; source CI and the
  exact-candidate presigner gate pass on both architectures. Release 36702944904
  signs and strictly publishes corrected image 9d6. All scoped functional/performance
  cases are settled in test-plan.md, including corrected encrypted graphical boot
  and final sentinel/ISO hashes. The default Metal VM and container desktop are
  restored; corrected installed VM is retained stopped and obsolete diagnostic VM
  is absent. Final documentation publication is being completed. The explicit-force
  final stop observation and physical/other-host exclusions remain visible.

## Coverage ownership

Each case has one implementing owner; prerequisite tasks may cite it too.

| Cases | Owning task |
|---|---|
| TC-01, TC-02 | T01 |
| TC-08 | T02 |
| TC-06, TC-07, TC-09 | T03 |
| TC-03, TC-04, TC-05, TC-27 | T04 |
| TC-10, TC-11, TC-12, TC-13 | T06 (T05 supplies archiving) |
| TC-14 | T07 |
| TC-15, TC-16, TC-17, TC-18, TC-19, TC-20 | T08 |
| TC-21, TC-22 | T09 |
| TC-23, TC-24 | T10 |
| TC-25, TC-26 | T11 |
| TC-28 | T12 |

## Deviations and clarifications

- 2026-09-28: clarified NFR-06's “host-disk writes” to “raw host-disk/bootloader
  writes.” The approved design necessarily creates ordinary workspace runtime,
  image and evidence files; the workstation-device boundary is unchanged.
- Task and case details do not authorize new release scope, tool installation,
  physical testing or automatic UTM app removal.
- 2026-09-28: original epoxy/VirGL source pins are unavailable from both Git and
  GitHub's commit API. Replaced with exact fetchable commits from their upstream
  `v` branches; see design and `qemu/inputs.json`. The owner subsequently authorized Meson/libslirp/json-glib and the Metal component. Native renderer evidence is recorded in review.md.

- Delivery adjustment: QMP/process/media control uses standard-library host Python
  behind the existing Rust CLI, rather than adding another Rust transport stack.
  Native lab memory defaults to 4 GiB (installer 8 GiB). VM shot combines metadata
  into one SSH request. No second harness, persistent daemon or unit tests added.
