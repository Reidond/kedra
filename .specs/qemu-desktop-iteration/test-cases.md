# End-to-end and manual verification cases

Status: approved with the implementation packet on 2026-09-28.
Current outcomes are recorded in [test-plan.md](test-plan.md); unlisted variants
are recorded in test-plan.md. Neither a mapped case nor a historical pass qualifies the
implementation. Sources: approved [requirements](requirements.md) and
[design](design.md), read 2026-09-28 at base `a3a39a4`.

There are 28 parameterized cases covering all five use cases and eight NFRs.
Inputs within a case are enumerated below; each selected variant must be reported
separately. No expected value is ungrounded. Task ownership is in [tasks](tasks.md).

## Levels, methods and common fixture rules

- **Host CLI E2E:** actual public CLI/build/media commands and filesystem/process
  effects. No isolated function, API-model or source-string tests.
- **Container E2E:** the existing Cargo/Testcontainers harness and actual systemd,
  desktop and application processes, plus manual inspection of its screenshots.
- **Local QEMU VM:** the native Mac VM and its public lab commands; manual graphics
  and input verification are required.
- **CI QEMU VM:** existing Linux VM workflows, retaining their boot-level assertions.

Every TC has one level. Method `manual` includes running real CLI commands and
inspecting their results; `automated` uses existing allowed E2E facilities;
`observability` measures real workflows without flaky timing assertions in CI.
Project policy excludes units, isolated models/APIs, mocks, doctests and scanners
at every level. Container cases do not prove boot, GPU, SELinux or VT/PAM behavior.

Preconditions: exact source/runtime/image receipts, generated lab accounts only,
owned artifact directories, explicit instance/execution names and healthy baseline
dependencies before introducing one fault. Missing prerequisites mean blocked or
not-run. Every negative case must identify its intended cause; an unrelated
failure is not a pass. Fault fixtures and scratch repos never modify the shared
checkout, owner VM data or personal authentication.

## C-01 — Runtime availability and real acceleration (UC-01, NFR-06)

Technique: decision table. Risk: high likelihood / high impact, from design's
native graphics compatibility risk. Host preflight owns dependency refusal;
the local VM owns actual graphics. Linux/container rendering is excluded because
it cannot prove this Mac's native display path.

| Case | Level / method | Preconditions and selected input/action | Expected result and failure postcondition |
|---|---|---|---|
| TC-01 | Host CLI E2E / manual | Run read-only runtime check for each mutually exclusive state: complete candidate; missing QEMU; missing firmware/TPM; incompatible display capability; changed runtime hash. Use private runtime copies for invalid inputs. | Complete state reports ready-for-qualification, not GPU-passed. Each bad state reports its actual missing/incompatible item and makes no install/download/start attempt; existing runtimes remain unchanged (UC-01 exceptions, NFR-06; design “Native graphics runtime”). |
| TC-02 | Host CLI E2E / manual | With authorized prerequisites, prepare locked inputs once, prepare unchanged inputs again, then interrupt a fresh private preparation and supply an incomplete pin set in a private copy. | First preparation records the complete selected closure and verified firmware/runtime hashes; repeat reuses compatible output; interrupted/unlocked preparation cannot publish a selectable receipt (UC-01, NFR-08; design “Native graphics runtime”). No execution of mutable upstream installers. |
| TC-03 | Local QEMU VM / manual | Start the prepared native candidate; show a visible animated app and deliberate UI state. Collect niri/app renderer evidence, Secure Boot/lockdown, SELinux, real input and compositor screenshot. | Actual HVF and host-GPU rendering, enforced boot checks, working keyboard/pointer and captured content matching the same live desktop are established together. Device enumeration alone cannot pass (UC-01). Failed renderer/input/capture leaves qualification failed with owned cleanup. |
| TC-04 | Local QEMU VM / manual | Exercise accelerated startup with unavailable required graphics capability, and separately select the explicit software/debug profile. Trigger an owned startup failure after one child process exists. | Accelerated mode refuses instead of silently using software/TCG; debug mode identifies software and cannot issue accelerated evidence; failed startup reaps only its own children (UC-01 exceptions, NFR-07; design “QEMU lifecycle”). |

## C-02 — Retained manual desktop and safe lifecycle (UC-02, NFR-07)

Technique: state transition. Risk: medium likelihood / high impact for firmware
identity; low likelihood / high impact for fixture transport, as scored in design.
The real VM owns these checks; container session startup is a different workflow.

The required transition set is new→running, running→stopped, stopped→running,
running→failed/cleaned, and stopped→removed. Refused transitions are a second start,
removal while running, and control of foreign/unknown state. TC-06/07/09 partition
these transitions rather than repeating the happy path at several levels.

| Case | Level / method | Preconditions and selected input/action | Expected result and failure postcondition |
|---|---|---|---|
| TC-05 | Local QEMU VM / manual | Qualified desktop; exercise GTK 3/4, Qt 5/6 and Xwayland applications, file choosers, panels, scrolling, window movement, resize, 1×/2× scale and guest audio. Use logged-out synthetic state. | Each named toolkit/input operation is usable in the native window; screenshots and actual display state agree. Failed capabilities are recorded individually, not hidden by startup success (UC-02). Microphone/real account tests are excluded. |
| TC-06 | Local QEMU VM / manual | Write a generated marker in instance A, stop and reopen it, then create B from the same immutable base. Inspect instance identities and receipts. | A retains its marker and disk/firmware/TPM identity; B starts with fresh writable state and credentials; shared base stays immutable. Prepared VM operations work with Docker unavailable to that invocation (UC-02; design “Disk preparation” and “Commands”). |
| TC-07 | Local QEMU VM / manual | Against owned fixtures, attempt: second start; removal while running; wrong runtime/base receipt; unsupported state schema; symlink escape; stale PID pointing to an unrelated generated sentinel process. | Every invalid transition refuses for the intended reason; no state/disk repair is silently performed and the sentinel remains alive. Failure does not modify another instance (UC-02 exceptions, NFR-07; design “QEMU lifecycle”). |
| TC-08 | Local QEMU VM / manual | Reach a healthy generated guest; execute through its valid key, then use wrong pinned host key, another instance's client key and an incorrect instance token, one at a time. Inspect generated seed/private permissions and logs. | Valid access reaches the correct account/instance; each wrong authority is refused distinctly with no requested operation performed. No personal SSH config/agent is used, no production guest-agent restriction is relaxed and generated secrets do not enter exported evidence (UC-02; design “Disk preparation and guest access”). |
| TC-09 | Local QEMU VM / manual | Keep B running while A undergoes clean shutdown, a nonresponsive guest shutdown, explicit forced stop and owned QEMU child failure in separate attempts. Remove A only after stopped. | Clean stop preserves state; nonresponsive stop fails after the design's 30 s grace without implicit force; explicit force adds at most 10 s. Failed children are reaped, stopped data is recoverable, B and shared bases remain untouched (UC-02, NFR-07). |

## C-03 — Correct visible incremental changes (UC-03, NFR-01/02/08)

Technique: state transition for apply/recovery; the decision table below partitions
valid, invalid, conflicting and unsupported input. Risk: medium likelihood / high
impact from design's live-sync risk. Detailed sync/recovery checks are owned by
containers; TC-13 is a deliberately shallow VM transport/SELinux integration
check, not a second copy of every sync edge case.

| Case | Level / method | Preconditions and selected input/action | Expected result and failure postcondition |
|---|---|---|---|
| TC-10 | Container E2E / automated | Retained healthy session and prepared archiver; generated supported home configuration transitions unchanged→modified→added→managed deletion→unchanged. Invoke public sync for each and read effective runtime state. | Only intended managed paths change; deleted managed inputs disappear without deleting unrelated files; native reload/readiness succeeds; no-op performs no writes/restart. No compiler, image pull, RPM transaction or reboot occurs in the hot path (UC-03; design “Incremental sync”). |
| TC-11 | Container E2E / automated | Start from valid sync. Parameterized inputs: invalid niri config; invalid Noctalia config; guest-side managed-file edit; GUI override masking requested setting; mismatched target; package/rootfs change; stale archiver. Neutralize all other faults. | Invalid config preserves usable state; guest edits are preserved with conflict diagnosis; masking override is reported and preserved; target/build changes require the appropriate rebuild/recreate; stale archiver requires preparation without implicit compilation (UC-03 exceptions; design “Incremental sync”). |
| TC-12 | Container E2E / manual | Interrupt a real sync helper after its transaction/backup state exists, then rerun sync. Repeat with an additional guest-side edit before recovery and with two simultaneous sync commands. Use generated slow/large valid input only if needed to reach the transition. | Interrupted apply is recovered or explicitly recoverable; a newer guest edit is never overwritten by rollback; concurrent sync is serialized/refused. Unreachable interruption is inconclusive, not pass. No partial apply is reported successful (UC-03 failure postconditions). |
| TC-13 | Local QEMU VM / manual | Qualified retained VM; perform one valid home edit with `sync --shot`, then one invalid edit through the actual SSH transport. Inspect actual visible change and SELinux labels/readiness. | Valid edit reaches this VM, reloads and appears in its screenshot; invalid edit preserves the prior desktop. Renderer/display/source receipt identifies this instance. This proves VM wiring, while TC-10/11/12 own detailed sync decisions (UC-03, NFR-08). |
| TC-14 | Container E2E / manual | Capture baseline, open a distinctive panel, capture again, and inspect both PNGs plus receipt. Repeat with a deliberately failed capture on an owned session. | PNG contents correspond to the intended desktop states, dimensions and current source; stale/empty/failed capture is reported as failure, not a valid preview. Evidence distinguishes software rendering and timing scope (UC-03, UC-04, NFR-08). |

## C-04 — Existing harness coverage and failure behavior (UC-04, NFR-07/08)

Technique: decision table for selection/validation; state transition for ordering,
concurrency, startup, cancellation, deadlines and cleanup. Risk: medium likelihood /
medium impact, from design's resource-leak risk. All cases belong to the existing
container harness; native GPU and firmware tests are intentionally excluded.

| Case | Level / method | Preconditions and selected input/action | Expected result and failure postcondition |
|---|---|---|---|
| TC-15 | Container E2E / automated | Run the current-source native-target suite against a prepared image on ARM and x86 CI. Record the nonempty applicable case list before execution. | Every applicable existing image/session/Noctalia/portal/keyring/agent/Bitwarden/home/GTK workflow remains covered and passes with its actual assertions, screenshots and report. Earlier-source results are not substituted (UC-04). |
| TC-16 | Container E2E / manual | Public harness modes: list; one selected case; unmatched filter; selection inapplicable to target; malformed YAML/unknown action in a disposable fixture checkout; one valid scenario with an intentionally wrong expected wallpaper. | List/empty/refused inputs create no test environments; one-case filtering creates only that case's environment; bad input fails before provisioning; wrong expectation fails with expected/actual despite successful command execution. Zero cases are no coverage, not full-suite success (UC-04; design “Visual evidence, timing and harness completion”). |
| TC-17 | Container E2E / automated | Run the same applicable case set forward and reverse; separately run two execution IDs with a retained lab beside them. Verify nonempty executed case lists. | Order does not change outcomes; fixtures/caches do not corrupt or leak across executions; each cleanup leaves the other execution and retained lab usable (UC-04, NFR-07). |
| TC-18 | Container E2E / manual | Send SIGINT during actual startup and during an actual running case; exercise second-signal termination separately. Keep another owned sentinel execution running. | Scheduling/waits stop, completed results remain readable, original cancellation is visible and owned cleanup/watchdog paths work within bounded deadlines. Other execution remains usable. A signal delivered after completion does not qualify cancellation (UC-04, NFR-07/08; design cancellation contract). |
| TC-19 | Container E2E / automated | In a healthy session, run a bounded read-only eventually observation with an intentionally impossible expectation and explicit deadline; keep a positive observation as a control. | Positive control succeeds; negative observation times out with final expected/actual evidence, stops polling and preserves diagnostics/cleanup. No state-changing action is retried (UC-04, NFR-07; existing scenario semantics retained by design). |
| TC-20 | Container E2E / manual | On a separate disposable real Docker engine, provoke partial session startup failure; separately make that engine unavailable during removal, after a genuine case failure. Restore that private engine for cleanup. | Startup failure removes only created resources. Teardown failure remains alongside the original failure and respects the 30 s diagnostic/cleanup bounds; restoring the engine permits exact-ID cleanup. Shared OrbStack and unrelated workloads are never stopped (UC-04, NFR-07; design deadlines/ownership). |

## C-05 — Installation continuity and UTM retirement (UC-05)

Technique: state transition for media/installer lifecycle; decision table for
authority and old/new runtime dependency. Risks: medium likelihood / high impact
for firmware compatibility, and high likelihood / high impact for native graphics,
matching design. Local installation owns new-launcher behavior; CI owns regression
of existing boot checks. These are distinct assertions on different runtimes.

| Case | Level / method | Preconditions and selected input/action | Expected result and failure postcondition |
|---|---|---|---|
| TC-21 | Host CLI E2E / manual | Build media from an exact reviewed signed ARM digest into a new output directory. On private copies exercise mismatched target/digest, modified ISO/manifest/checksum and existing destination. | Existing signature, identity and byte verification still apply; valid media has matching records. Every invalid input refuses before VM creation/overwrite, preserving source media and prior output (UC-05; design “Disk preparation” and “UTM retirement”). |
| TC-22 | Local QEMU VM / manual | Create installer-mode VM with verified media and generated empty disks; install into only its selected disk with a generated account and encryption, stop, detach ISO and boot again. Keep a sentinel disk untouched. | Startup reports installer window state without fake SSH readiness; exec/sync refuse installer mode. Fresh installation boots ISO-free with enforcing security/healthy desktop, preserves its firmware entry and sentinel disk, and leaves original media unchanged. Detach while running refuses (UC-02/05; design installer contract). |
| TC-23 | Local QEMU VM / manual | Close UTM; execute the complete prepared native start/exec/sync/shot/stop workflow and inspect actual process/library/runtime receipts and the documented entrypoints. | Working local workflow uses the private standalone runtime without UTM executables, app-bundle libraries or Apple Events. Signed target identity remains `utm`; owner app/VM data remain untouched. Missing replacement behavior blocks retirement (UC-05, NFR-06/08). |
| TC-24 | CI QEMU VM / automated | After authorized publication of the exact implementation source, run renamed ARM workflow plus affected existing desktop/login/PAM, update/rollback, direct-registry and home-transition workflows. | Existing positive and negative boot/security assertions still pass on that exact source, including ARM bootc/Secure Boot/SELinux/units checks. No container or historical pass substitutes for these results; missing CI authorization is not-run (UC-05). |

## Non-functional measurements and boundary audit

Technique: repeated observations for timings (not invented input boundary tests),
and decision table for owned/foreign resources and authorized/unauthorized effects.
Timing risk is medium/medium; unavailable frame instrumentation is medium/low;
fixture boundary risk is low/high, exactly as in design. Performance cases gather
all five samples before reporting median and maximum; no background workloads
are stopped for a better result.

| Case | Level / method | Preconditions and input/action | Expected result |
|---|---|---|---|
| TC-25 | Container E2E / observability | Prepared current-source image/archiver; measure five public cached starts, captures, edit+capture operations, selected desktop cases and existing ARM suites. Keep deliberate timeout/failure exercises outside the normal-suite speed measurement and report their cost separately. | Target medians: start <=20 s, capture <=2 s, edit+capture <=5 s, selected case <=15 s, existing ARM suite <=120 s. Capture max <=5 s and edit+capture max <=10 s. Report every sample and cold preparation separately; a miss remains a failed target (NFR-01/02/03/04). |
| TC-26 | Local QEMU VM / observability | Qualified prepared VM; measure five normal disk starts, captures and home edit+capture operations. Log generated automatic-session readiness separately from manual login. | Target medians: startup <=45 s, capture <=2 s, edit+capture <=5 s; capture max <=5 s and edit+capture max <=10 s. No rebuild/reinstall is hidden outside the timer; cold preparation is separate (NFR-01/02/03). |
| TC-27 | Local QEMU VM / observability | Qualified native GPU renderer, 2560×1600 at scale 2; obtain actual presented-frame timestamps during five 30 s active-animation observations if supported instrumentation is available. | Target 60 Hz with >=95% of presented-frame intervals <=33.3 ms; inspect interaction and report trace scope. Missing instrumentation is accepted-unverified for this quantitative target only, never proof of smoothness/acceleration. glmark2 average FPS or video frame count does not substitute (NFR-05). |
| TC-28 | Host CLI E2E / manual | Audit actual operations/artifacts with generated secrets, an unrelated sentinel process/VM directory and another retained container. Exercise owned cleanup; compare scoped before/after inventories and review exported evidence. | No automatic external installs, production pushes, raw host-device/bootloader access or owner-VM modifications; foreign sentinels survive. Generated keys/secrets are excluded from exported logs/screenshots before publication. Runtime receipts describe actual source/target/mode and scopes, without empty-population or machine-wide privacy claims (NFR-06/07/08; UC-02/04/05). |

## NFR mapping and verification methods

| Condition | Method and case ownership | Risk inherited from design |
|---|---|---|
| NFR-01 — full edit feedback time | Observability: TC-25, TC-26 (separate environments) | Warm timings: medium/medium |
| NFR-02 — capture latency | Observability: TC-25, TC-26 | Warm timings: medium/medium |
| NFR-03 — cached startup | Observability: TC-25, TC-26 | Warm timings: medium/medium |
| NFR-04 — existing suite/filter speed | Observability: TC-25 | Warm timings: medium/medium |
| NFR-05 — actual frame presentation | Observability: TC-27; accepted-unverified only if instrumentation unavailable | Instrumentation: medium/low |
| NFR-06 — host/owner scope | Manual: TC-01, TC-23, TC-28 | Fixture boundary: low/high |
| NFR-07 — isolation/deadlines | Automated/manual E2E: TC-07/09/17/18/19/20/28 | Lifecycle leaks: medium/medium |
| NFR-08 — evidence identity | Manual/E2E inspection: TC-02/13/14/18/23/28 | Capture/graphics and isolation risks from design |

## Deliberate non-coverage

| Condition | Design risk / boundary | Disposition |
|---|---|---|
| Quantitative presentation timing without supported instrument | Medium/low | NFR-05 accepted-unverified is explicitly allowed; GPU functionality still requires TC-03/04 |
| Vulkan/Venus, physical GPU passthrough and another hypervisor | Outside approved delivery; native graphics risk high/high | No claim of support; the first accepted graphics path is OpenGL via ANGLE |
| Linux GPU hosts, physical hardware, USB, real vault/accounts, suspend and live GPU snapshots | Explicit requirements scope exclusions | No qualification claims; existing Linux boot CI remains in scope through TC-24 |
| Exhaustive display sizes and every graphical application | UC-02 representative toolkit scope | TC-05 covers named toolkit families and declared display settings; no universal compatibility claim |
| Unit/model/mock/parser-only/source-layout/skill tests | Owner testing-policy exclusion | Public inputs and real workflows cover applicable behavior; none of these prohibited tests will be created |

No UC or NFR is silently untraced. No unresolved expected values. Execution
prerequisites—native tools, full pinned closure, GPU feasibility, and later CI
publication authorization—remain explicit gates, not test passes.
