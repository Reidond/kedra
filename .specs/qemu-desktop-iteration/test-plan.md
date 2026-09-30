# Verification and implementation gates

Status: implementation packet approved by the owner on 2026-09-28; reconciled with
[tasks](tasks.md) and [cases](test-cases.md). Native GPU and container workflows
are locally exercised; complete qualification remains open as recorded below.
Requirements and design are approved. Date: 2026-09-28; inspected base: `a3a39a4`.

## Scope and ownership

The plan qualifies the Mac-native QEMU desktop, retained iteration, incremental
sync/capture, and the existing Testcontainers harness. It preserves installer and
boot-level CI coverage while removing UTM tooling dependencies. It does not test
new signing identities, owner accounts, physical hardware or excluded graphics
backends. [test-cases.md](test-cases.md) contains the 28 grounded cases and
deliberate exclusions; their expected results are not duplicated here.

| Owning level | Cases | Purpose and existing facility |
|---|---|---|
| Host CLI E2E | TC-01, TC-02, TC-21, TC-28 (4) | Public preparation/media commands and real filesystem/process effects; manual execution or existing CLI E2E facilities |
| Container E2E | TC-10, TC-11, TC-12, TC-14, TC-15, TC-16, TC-17, TC-18, TC-19, TC-20, TC-25 (11) | Existing Cargo/Testcontainers harness, actual systemd/session/apps and visual inspection |
| Local QEMU VM | TC-03, TC-04, TC-05, TC-06, TC-07, TC-08, TC-09, TC-13, TC-22, TC-23, TC-26, TC-27 (12) | Native GPU, real input, retained disk/firmware state and manual installation on this Mac |
| CI QEMU VM | TC-24 (1 parameterized workflow case) | Existing Linux VM workflows after authorized publication, preserving their assertions |

These 4 + 11 + 12 + 1 cases total 28 with one owning level each. An automated
case can use a native test/probe already supported by the harness, but no second
runner or service-specific mocking layer is introduced. Manual visual checks
include the agent opening the captured images and showing them to the owner.

## Environments and test data

| Need | Provisioning and constraint |
|---|---|
| Native runtime | Approved pinned source set plus fully frozen dependency/firmware closure from T01; private workspace prefix; missing host tools require owner authorization |
| Manual VM | M2 Pro / macOS native HVF; default 6 vCPU, 4 GiB (installer 8 GiB), 2560×1600 scale 2; generated instance seed, private sockets and known-hosts file |
| Container system | Existing native Docker API endpoint and Testcontainers image stages, with named execution IDs; no disruption of unrelated workloads |
| Engine-loss faults | Separate real disposable Docker engine with private storage/endpoint; stop/restart only that engine; no fake Engine API or interruption of shared OrbStack |
| Config and malformed scenario inputs | Generated supported data in an owned disposable source snapshot/account; preserve the shared checkout and native application state |
| Foreign-resource controls | Another generated VM directory/process and retained container, identifiable before/after; no inspection or capture of real owner home/vault contents |
| Installer | Exact reviewed signed ARM image/media, generated empty/sentinel disks, synthetic account/passphrase; preserve original media |
| CI evidence | Exact implementation source and run/attempt URLs; local authorization does not imply push or dispatch authorization |
| Artifacts | Default macOS runtime/build/image cache under `~/Library/Caches/kedra/qemu`; per-checkout instance state under `~/.local/share/kedra/lab/<checkout-key>`; ignored reports elsewhere as recorded by receipts. Never place durable VM assets under Cargo `target/` or commit raw logs/screenshots/keys. |

Images are pinned once per execution and the same resulting image is used for
functional checks and timing observations. When tooling layers are applied, record
their added RPM inventory and unchanged image-under-test package set. A full
build, an overlay and a published image are distinct evidence scopes.

## Execution sequences

Shared expensive setup is reused where safe; mutable automated fixtures remain
isolated. Negative variants begin from a known-good fixture and disturb only
their named dependency. A missing or unreachable negative precondition is not a pass.

| Sequence | Cases in order / level | Setup, purpose and cleanup |
|---|---|---|
| TS-01 — dependency and preparation gate | TC-01 → TC-02 / Host CLI E2E | Check prerequisites before installing anything; after explicit authorization, prepare once and test interrupted/invalid private variants. Preserve only completed runtime receipts. |
| TS-02 — graphics feasibility | TC-03 → TC-27 → TC-04 → TC-05 / Local QEMU VM | Use a fresh development disk and real native window. Measure presentation when supported; record its allowed instrumentation exception otherwise. Negative startup variants get separate instances. No UTM source retirement before this gate passes. |
| TS-03 — retained instance control | TC-06 → TC-08 → TC-07 → TC-09 / Local QEMU VM | Two generated instances and sentinel process; verify identity/access, then refusal/cleanup paths. Stop/remove only explicitly owned disposable instances. |
| TS-04 — container hot iteration | TC-10 → TC-11 → TC-12 → TC-14 / Container E2E | Retained lab from an immutable image; take before/after screenshots. Recover or preserve a failed transaction for diagnosis, then remove only its lab. |
| TS-05 — native hot iteration and independence | TC-13 → TC-23 / Local QEMU VM | Reopen a qualified generated VM, apply one valid and one invalid change through real transport with UTM closed. Inspect screenshot and native runtime receipts, then cleanly stop. |
| TS-06 — harness completion | TC-15 → TC-16 → TC-17 → TC-19 → TC-18 → TC-20 / Container E2E | Run positive coverage before adversarial lifecycle exercises. Engine-loss variants use their separate engine; recover exact owned IDs after restoration. A failed teardown remains reported. |

Standalone cases: TC-21 (signed media), TC-22 (fresh manual installation), TC-24
(exact-source CI), TC-25/26 (warm-loop measurements), and TC-28 (boundary/evidence audit).
They need not run again merely because another independent phase completed.

## Performance measurements

All timing thresholds are the approved NFR targets, not general CI timeout values.
Collect five real samples for each operation, preserve raw durations, and report
median/maximum. Record host contention without stopping other workloads. The first
tool build, image pull/build and disk construction are separately labeled cold
costs, never omitted from the overall handoff.

| Requirement | Method | Observation and exact target |
|---|---|---|
| NFR-01 | Observability, TC-25/26 | Public edit→validated apply→ready desktop→saved screenshot; median <=5 s, maximum <=10 s in each environment |
| NFR-02 | Observability, TC-25/26 | Public capture through saved PNG; median <=2 s, maximum <=5 s |
| NFR-03 | Observability, TC-25/26 | Cached container start to usable session median <=20 s; prepared QEMU disk to usable session median <=45 s; generated auto-login is not real PAM-login evidence |
| NFR-04 | Observability, TC-25 | Existing applicable ARM suite median <=120 s after image preparation; selected desktop case median <=15 s; deliberate failure probes reported separately |
| NFR-05 | Observability, TC-27 | 2560×1600 scale 2, five 30 s active observations, target 60 Hz and >=95% of presented-frame intervals <=33.3 ms. Actual presentation timestamps required. Only missing instrumentation permits accepted-unverified status. |
| NFR-06 | Manual, TC-01/23/28 | Review actual authorized operations and foreign sentinels; zero automatic installs, production pushes, raw host-device/bootloader changes or owner-VM modifications |
| NFR-07 | End-to-end/manual, TC-07/09/17/18/19/20/28 | Finite design deadlines and no cross-execution cleanup; preserve original errors and exact-ID recovery |
| NFR-08 | End-to-end/manual artifact inspection, TC-02/13/14/18/23/28 | Every produced result has actual source/target/runtime/rendering/display/timing scope; inspect nonempty evidence from real runs |

No new benchmark framework is needed. Instrument existing commands with phase
durations. Missing a target remains a failed target until optimized or explicitly
accepted by the owner; a functional pass does not erase it.

## Entry and exit gates

**Entry:** approved requirements/design and this reconciled implementation packet;
actual required tools available with any necessary installation authorization;
an immutable source/image identity for the run; safe disposable fixtures. Current
specification edits do not meet any runtime entry gate by themselves.

**Phase 1:** T01–T04 pass their functional native runtime/graphics checks. No UTM
retirement on a software fallback, blank screenshot, absent input or build-only
result. Full dependency/firmware pins must exist before the candidate is built.
NFR-05 may remain unverified only under its explicit instrumentation exception.

**Phase 2:** T05–T07 pass valid/invalid/conflict/recovery/capture checks; retained
VM identity and isolation checks pass. Container-only improvements can be delivered
truthfully if the GPU path is blocked, without claiming the overall replacement done.

**Final:** all required functional cases pass on the resulting source, NFR targets
are measured and met (or a deviation is explicitly accepted), standard code gates
pass, and exact-source CI evidence is observed where required. Any missing runtime,
approval, failed case, unmeasured timing or unavailable CI is separately recorded
as `blocked`, `fail` or `not-run`. The only predefined accepted-unverified outcome
is NFR-05 without supported presentation instrumentation.

The completed handoff includes the private runtime setup cost, everyday commands,
actual desktop screenshots, cold/warm timings and recovery commands. It must state
that container rendering remains software on this Mac and distinguish lab auto-login
from boot/PAM/security qualification. No automatic publication accompanies handoff.

## Standard code gates during implementation

Use the narrow relevant checks after each edit, then the required final checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
cargo test -p kedra-container-tests --test container --locked
uv run --with ruff==0.16.9 ruff check .
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
uv run usr/src/kedra/tests/cli/release-material.py --workdir target/release-material
```

Use existing pinned tools; if a required tool is missing, follow the host-tool
authorization boundary. Add normal syntax/format checks appropriate to changed
scripts/workflows. Empty Linux-only Cargo targets on macOS do not qualify Linux
behavior; use the existing Linux CI for that coverage. No source-text tests or
custom check runners. Repeat successful checks only after relevant changes or
new failures justify it.

## Current verification state — 2026-09-30

PR #23 merged to main as `bafd1884a569d4890e768c72e335514d824bbf1f`.
The local checkout contains later installer/stopped-lab work plus uncommitted
Noctalia 5.2 and qemu-arm64 initramfs corrections.
Outcomes apply only to the observed variants below; a partial result
never qualifies a full parameterized case. The owner later deleted the local Rust
`target/` tree, including the ignored QEMU runtime, disks, screenshots and raw
reports. Results below therefore distinguish historical observations recorded in
WL-20260928-08 from reproducible exact-source Actions evidence. No replacement OS
has yet been signed or published.

| Cases | Actual result and remaining variants |
|---|---|
| TC-01/02 | pass: private locked runtime preparation/package replay, signatures/firmware/receipt checks, missing binary/changed firmware refusal and native disk construction. The rebuilt 218 MiB runtime outside Cargo output passes every prerequisite/reusability check. Unchanged replay passes; a real interrupted private preparation exits 143 and incomplete private pins exit 1 for missing ANGLE without selecting a runtime/receipt; the current runtime hash matches its original state. |
| TC-03 | pass for actual niri + Wayland EGL Metal rendering, Cocoa presentation with visible EGL/SHM clients, basic/physical input, visible edit/capture, Secure Boot and SELinux. Presentation timing scope is in TC-27. |
| TC-04 | pass: failed QEMU startup leaves no owned QEMU/TPM and another VM stays running. No automatic software fallback. Explicit software/debug profile not implemented. |
| TC-05 | pass: native keyboard/pointer, physical `abc` and Command+Enter, scrolling, window movement, QMP Super+Return, audible audio confirmed twice, and all six GTK3 Wayland/Xwayland, libadwaita, Qt5 and Qt6 chooser workflows pass. Resize to 1920 works. Unlocked scale-1/1.5/2 Noctalia Settings captures were independently viewed with valid geometry; reopening QEMU restores actual 2560×1600 scale 2. |
| TC-06 | pass: retained disk/firmware/TPM, five distinct restart boot IDs, fresh second instance, separate home/keys/ports, prepared operations with unavailable Docker endpoint. |
| TC-07 | pass for observed refusal matrix: live detach, unsupported state, changed runtime receipt, stale foreign PID, linked disk/instance parent, missing QEMU/TPM, changed firmware and a non-emulator executable refuse without touching generated foreign sentinels. The restored public CLI also refuses mutable-file hard links, TPM-tree links, log links and a predictable pending-file link against a private clone, preserving foreign data and the running default. Interrupted preparation remains TC-01/02 coverage. |
| TC-08 | pass: correct pinned SSH access, cross-instance/wrong client key, wrong pinned host key and wrong instance token were denied; no requested guest marker was created. |
| TC-09 | pass: clean/force stop, second-instance preservation and failed-start cleanup pass. The 10-second nonresponsive variant refuses without force and preserves the running default. The literal grace variant reports accepted poweroff after 31.336 s without implicit force; a 45-second ordinary unit ignores TERM, explicit force completes in 1.487 s preserving disk/VARS/foreign sentinel, and public remove passes. Its default was stopped before/after with stale PIDs, so it makes no running-default claim. Empty/whitespace selectors exit 1 safely. |
| TC-10/11/12 | pass for container apply/add/delete/no-op, invalid KDL/TOML, target/rootfs/stale archiver refusal, guest/GUI edits, concurrent lock refusal and real interrupted-write recovery preserving newer edits. |
| TC-13/14 | pass: native valid/invalid config workflow, actual changed screenshot, source/display receipts; container capture/receipts and failure/partial reports. Fresh unlocked software-rendered Settings capture `final-container-desktop-1790748508-70906.png` was viewed healthy. |
| TC-15 | pass for merged source: main container CI passes ARM/x86 and five stable-tag worktree-overlay runs pass 13/13. Current uncommitted Noctalia 5.2 worktree run `1790751520-28479` also passes 13/13 with two workers in 109.92 s/no cleanup failure, but exact-source CI is pending. Earlier deleted-Quay-input and no-overlay stale-provenance failures remain recorded. |
| TC-16 | pass for observed variants: offline listing/empty selection, filtered execution, malformed YAML/unknown action refusal before provisioning, and a deliberate wrong-wallpaper expectation preserving the actual value and report. |
| TC-17 | pass for forward/reverse ordering, two workers, colliding-prefix long names and retained-lab preservation. Independent concurrent Bitwarden `1790709593-88410` and desktop `1790709593-88411` executions both pass, clean up independently and leave the default retained lab healthy. |
| TC-18 | pass for active eventually cancellation, startup kill/SIGINT and forced second SIGINT. Reports remain durable, exact-execution cleanup succeeds after engine recovery, and the retained lab stays healthy. |
| TC-19 | pass for positive suite observations and genuine impossible-deadline probes: 6 s observation capped by 2 s eventually deadline, and 500 ms command deadline. |
| TC-20 | pass: a separate disposable Docker daemon was stopped only after a real observation failure; the original failure, bounded diagnostics and teardown failure were preserved, then exact cleanup succeeded after that daemon resumed. Shared OrbStack was not stopped. |
| TC-21 | pass: public replay builds/verifies the new signed qemu-arm64 ISO after fixing a stale mount-root path. Private APFS copies cover changed ISO byte/checksum, mismatched SHA256SUMS/manifest, wrong desktop target, changed image-digest identity and existing output; all refuse with exit 1. Valid verification exits 0, corrupt public installer preflight creates no instance/QEMU, original ISO/hash/manifest/checksum inode/mtime/content remain unchanged, and private copies were removed. Evidence: `~/.local/state/kedra/evidence/2026-09-30/media-refusal-matrix.json`. The old UTM ISO remains historical and is correctly refused. |
| TC-22 | partial: fresh encrypted installation completes with only the 96 GiB disk selected; original ISO/unselected sentinel hashes and stopped detach pass. ISO-free Secure Boot/GRUB reaches LUKS, but Cocoa stays inactive because its initramfs lacks `virtio_gpu`, `virtio_input` and `virtio_dma_buf`. Unsigned sanctioned candidate `3c744556…` contains regenerated 7.2.7 initrd `3f37ce…` with those drivers/dependencies plus generic crypt/TPM/bootc/OSTree/EROFS/overlay support, mode 0644/nlink 1; 12 lint checks pass with pre-existing warnings only. Corrected signed media and healthy installed boot are not yet qualified. |
| TC-23 | pass: standalone retained native start/exec/sync/shot/stop uses a private runtime with no UTM libraries/Apple Events. UTM source/identity are retired; six owned prototypes were removed, no registered owner VM was found, old packages return API 404 and the old signing environment is absent. |
| TC-24 | pass for merged `bafd1884`: main workflows, verified stable and exact same-SHA ARM retry pass. Follow-up workspace checks pass at `1e157f4` and ARM at `4e0f4fc`; local Noctalia 5.2 and initramfs corrections pass targeted/full local gates but remain uncommitted with CI pending. A key-free exact-candidate full-harness signer gate is implemented without changing no-change behavior; its first execution is not-run. |
| TC-25/26 | pass: filtered/container startup, five stable suites, five restored native full-CLI starts and five captures meet their targets as recorded below. |
| TC-27 | pass for guest Virtual-1 scope: five visible native-Cocoa EGL/SHM observations after 5 s warmups run 30 s each at 81.967, 91.126, 89.334, 81.634 and 90.500 presentations/s (median 89.334/min 81.634); every p95 interval is 16.667 ms. This does not qualify a physical monitor or other hosts. |
| TC-28 | partial: instance/client-key/disk isolation, foreign sentinels, original ISO and unselected installer-sentinel preservation, no raw host disk/registered owner VM changes, signed publication and complete UTM resource/credential retirement pass. Healthy installed end-state and pending local-fix publication remain open. |

| Requirement | Measured result (seconds unless stated) |
|---|---|
| NFR-01 native edit + capture | 4.676 / 8.959 — pass; samples 6.488, 8.959, 4.428, 4.676, 4.582 |
| NFR-01 container edit + capture | 5.000 / 9.177 — at median boundary; samples in qemu-final-timings.json |
| NFR-02 native capture | 0.875 / 0.893 — pass; restored samples 0.869, 0.874, 0.875, 0.883, 0.893 |
| NFR-02 container capture | 0.331 / 0.467 — pass |
| NFR-03 native warm start | 19.239 / 21.253 — pass; restored full-CLI samples 19.239, 19.348, 18.461, 21.253, 18.533; readiness median 16.437; five distinct boot IDs and Metal each |
| NFR-03 current-target cached container start | 4.091 / 10.125 — pass; complete public `up` samples 10.125, 4.087, 4.438, 4.016, 4.091; clean cached base `6288…`, no overlay, excludes one-time source/overlay preparation; observed stdout, no separate raw timing file |
| NFR-04 full ARM suite, two workers | 85.610 / 124.800 — median passes; samples 63.29, 106.68, 71.67, 85.61, 124.80 |
| NFR-04 stable worktree-overlay suite | 65.563 / 72.787 — pass; five 13/13 wall samples 58.959, 66.322, 64.764, 65.563, 72.787; tag `6f95044a24e38c69` |
| NFR-04 complete test command | 91.030 / 133.366; includes Cargo/preparation |
| NFR-04 current-target filtered case | 6.680 / 7.714 — pass; scenario samples 6.680, 7.714, 6.578, 7.181, 6.256; complete command wall median 7.038 |
| NFR-05 presentation | pass for guest Virtual-1: five 30 s rates 81.967, 91.126, 89.334, 81.634, 90.500 presentations/s; median 89.334/min 81.634; every p95 interval 16.667 ms; physical monitors and other hosts unqualified |
| NFR-06/07/08 scope, lifecycle, evidence | Observed variants above pass; missing fault variants stay unqualified |

Optimized filtered receipts are runs `1790706424-99511`, `1790706431-99651`,
`1790706439-99793`, `1790706446-99900` and `1790706454-153`. The scenario durations
come from their durable reports/JUnit output; command-wall median was recorded by
the invoking measurement. Post-change full two-worker run `1790706590-5077` passed
12/13. `native::home_review_cycle` failed in 19.037 s because image source revision
`48ebd03f…` was absent from the checkout (`fatal: bad object`); cleanup itself had
no failure.
The corrected worktree-overlay rerun `1790706944-44808` then passes 13/13 with two
workers in 60.07 s. The no-overlay refusal remains a valid recorded failure for
its stale-provenance scope rather than being relabelled.
Five stable-tag repetitions are reports `1790708814`, `1790708873`, `1790708939`,
`1790709025` and `1790709090`, with the source/tag receipt outside Cargo output.

The native loop, warm-start and quiet-CPU results (2.9% over 10.007 s) were
recorded before local artifacts were deleted. Earlier native loop regression to
6.009 s median was reduced to 4.676 s by batching screenshot metadata without
dropping provenance. Source was restored after every generated edit. Current-target
filtered execution and complete public cached container startup now pass after
optimizing the actual software-rendered desktop startup. The startup scope excludes
one-time source/overlay preparation; source-sensitive native home coverage requires
the working-tree overlay. The deleted native JSON files cannot serve as final
retained artifacts.

Recorded earlier run IDs and fault results are retained in worklog WL-20260928-07/08.
The restored 218 MiB runtime/disk live outside Cargo output. Current native evidence
includes five Metal warm starts/captures, audio, all required input interactions,
unlocked scaling/resize/restore, five presentation observations and refusal
artifacts. The new current fixture image `6303cf1f…` is prepared but not yet booted
or hot-synced. Exact-source CI artifacts remain available in the linked Actions runs.

## Reconciliation and remaining decisions

- Cases → tasks: all 28 have one implementing owner in tasks.md; no unowned cases.
- Tasks → cases: all 12 tasks have explicit TC references; supporting tasks may
  cite a shared case without creating another owning test level.
- Use-case/NFR citations are grounded in the approved documents. Detailed hot-sync
  edges belong to containers; the separate VM smoke case verifies transport/native
  integration. Runtime-dependent observations cannot pass from an empty case list.
- Risk pairs and exclusions are inherited from design/test-cases; no second risk
  scoring system is introduced. No unresolved expected values remain.

Prerequisites and implementation are authorized and completed for the delivered
local workflows. The full packet remains partially qualified: outstanding cases
above need corrected signed media plus a healthy ISO-free graphical boot,
new-fixture hot-sync, first execution of the exact-candidate presigner gate and
publication/CI of the Noctalia/initramfs follow-ups. TC09, fresh encrypted
installation/sentinel preservation, main CI and signed replacement publication
pass. No further approval is needed for ordinary local iteration; owner-account
tests retain their separate boundaries.
