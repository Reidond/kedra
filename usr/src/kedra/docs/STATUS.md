# Verified status

## Noctalia5.2.1 follow-up (2026-10-07)

PR38 is merged at `caa87aca7d1fe88a94cbf9e57e5237dc68bd3ae3`. The owner
requests a follow-up for the Noctalia compatibility and corrected home VM gates.
The exact native5.2.1 output and unchanged three-field types pass prequalification
on retained Fedora44 package5.2.1-1.fc44. The two closed gates now admit only this
additional runtime, retaining persisted projection identity5.0.1 and unknown
version refusal. The full actual home-review cycle passes in21.96s with no
interruption or cleanup failure. Full target and signed A/B/A checks remain
in-progress/not-run at this checkpoint. See worklog HOMEQUAL-01/02.

## Package language delivery (2026-10-06)

The owner requests completing [PR38](https://github.com/Reidond/kedra/pull/38)'s
[specification](../../../../.specs/nix-package-dsl/README.md) and implementing its
standalone Rust `.kedra` frontend, inline resources and package/release migration.
The branch implements the parser/type checker/formatter, bounded process and
resource lowering, independent Rust intent API, Fedora compiler-role resolution,
generic contribution and explicit source cutover. jq/SQLite/library and the C
pilot pass real Fedora build/rebuild checks. The installed catalog aliases, PATH,
SQL/jq/library and persistent service workflow pass on exact unsigned native image
`b44e6c3b55a3bdb82248b94a5c2131def3d75d0b1858946928bc6f53a79575b2`.
Actual deadline/memory refusals preserve winners and reap the child. Source
compatibility and fresh full-RPM parity pass on both native targets in
[run37511811578](https://github.com/Reidond/kedra/actions/runs/37511811578).
Both source architectures pass final-code Check37517872017/37517864795 at
`6f1269df4210622b87e706a24a8779fa02bd6f67`. The same
container run passes desktop11/12 and ARM12/13; each remaining failure is the
existing unqualified Noctalia5.2.1 home review, with no cleanup failure.
The legacy home fixture now derives its private compatibility requests through
the public source CLI; local preparation passes, corrected hosted rerun pending. See worklog WL-20261006-PKGDSL-06/07/08. Existing
release/installer/native gates below remain independent; no production publication.

## Repair verification and remaining qualification (2026-10-06)

[PR40](https://github.com/Reidond/kedra/pull/40) is merged at
`f38f1065d0e42a9c5867675c2b38d538b790cee4`. Both architectures pass the
post-merge workspace Check37429382725. The actual
[release37429383144](https://github.com/Reidond/kedra/actions/runs/37429383144)
passes both image builds, the ARM installed catalog case and image identity
inspection. This verifies the original Containerfile-selection repair in its
real release workflow.

The run nevertheless completes failure: refreshed candidates contain Noctalia
5.2.1, while home review qualifies exact5.0.1/5.1.0/5.2.0 runtimes. Desktop
passes11/12 and ARM12/13 cases; each fails only `native::home_review_cycle` when
public home initialization refuses the unqualified version. Both reports are
uninterrupted and have no recorded cleanup failures. Both signing and stable
publication jobs are skipped, so neither new candidate is published as stable.

The immutable tested candidates are desktop-builds digest
`d76b0ab0baa242719e5acf9f14962909e11fc1798f4aa58fa5030b4b2feb3598` and
qemu-arm64-builds digest
`f3f61f5ae89c4308ec30cc9501e4a92995476bc5c30d86d68d018aac8c53469b`.
Worklog WL-20261006-02 records exact artifacts/report hashes. Additional5.2.1
compatibility qualification is proposed to the owner; the version gate remains
intact. Earlier installer/native fault gaps remain independent.

## Scheduled release failure (2026-10-06)

PR28/29/31/32 and PR33–37 are merged into main9b25c788.
[Scheduled run37394465020](https://github.com/Reidond/kedra/actions/runs/37394465020)
passes the desktop build, exact-candidate validation, signing and stable
publication. Its publication receipt records verified digest
`sha256:7505f70fb0b7998d516a256112b0bdbf6502f3b39f65e6298f25a06b9f5aab34`.

ARM build112047024567 fails before candidate validation/signing because the
catalog builder context contains `Containerfile`, while the Docker invocation
omits `--file` and looks for `Dockerfile`. The initial post-merge release
37359496666 has the same failure. [Fix PR40](https://github.com/Reidond/kedra/pull/40)
explicitly selects the prepared catalog Containerfile. A real Docker29.4.0 scratch-context probe
reproduces the failure, then builds successfully with the corrected selection,
matching payload readback and successful owned cleanup. YAML/Bash/whitespace
checks pass. The subsequent main run above verifies actual ARM build recovery
and records the new runtime-qualification blocker separately; this original
probe by itself did not establish Fedora package build or release success.

The separate home-artifact [plan PR39](https://github.com/Reidond/kedra/pull/39)
targets dev and remains unimplemented. Prior installer/native qualification gaps
below are unchanged; successful desktop publication is not an installed result.

## Pre-merge delivery checkpoint (2026-10-05)

All five dependent drafts remain open: [D1 PR33](https://github.com/Reidond/kedra/pull/33),
[D2 PR34](https://github.com/Reidond/kedra/pull/34), [D3 PR35](https://github.com/Reidond/kedra/pull/35),
[D4 PR36](https://github.com/Reidond/kedra/pull/36), and [D5 PR37](https://github.com/Reidond/kedra/pull/37).
D1, D3 and D4 are locally qualified in their recorded scopes. The independent
signed-cache consumer now also passes explicit rebuild qualification. Full D2
and D5 acceptance remains incomplete; nothing has been merged or published to production.

Published plan/documentation head77300b9 passes both architectures in
[push Check37285542956](https://github.com/Reidond/kedra/actions/runs/37285542956).
The QMP-fixed direct ARM runs have now failed:
D2 37234495769, D3 37234495358, D4 37234495136 and D5 37234496224.
All paired x86 jobs pass. All four actual insecure-boot refusal/QMP shutdown
phases pass in368–411 seconds, then fresh installation exceeds its7200-second
deadline. Final bounded diagnostics show firmware/GRUB/kernel/systemd startup
but no installation-complete marker; they do not locate the later guest cause.
Cleanup passes.

Owning D2 deb442c passes both architecture source checks and the x86 signed
update/rollback job in [run37283817872](https://github.com/Reidond/kedra/actions/runs/37283817872).
ARM installation fails at 7201.558s. Its observer starts, the payload verifier
actually exits successfully, and Kickstart validates the selected disks and writes
the storage include. These markers do not establish storage-plan application.
No post-install stage or final completion is observed. Unit metadata becomes
unavailable around 3046–3059s without enough resource evidence to establish why.
ARM refusal/shutdown and structured cleanup pass; independent x86 cleanup is
not established. Worklog WL-20261005-13 records exact artifact hashes and the
bounded task/resource diagnostic follow-up. No root cause or successful ARM
installation is claimed. The aligned upper stack remains local and preserved.

Owning D2 `7c59fd8` passes both architecture source checks and x86 A/B/rollback
in [run37305788202](https://github.com/Reidond/kedra/actions/runs/37305788202),
but ARM installation fails at7201.685s. Its157 increasing guest heartbeats
continue through7186.790s with no sequence gaps;239 host/owned-QEMU samples
are retained. No sampled guest OOM or low available memory is observed. ARM
refusal/shutdown and structured cleanup pass. The pinned Anaconda DBus modules
use different log formats/routes from the main process, which that revision's
task reader handled incorrectly. Missing task markers cannot locate the stall.
Owning `c01ee2f` corrects the classifier; source push Check37330502152 and PR
Check37330513592 pass both architectures. Its
[run37330502213](https://github.com/Reidond/kedra/actions/runs/37330502213)
attempt1 fails before guest execution. Attempt2 passes x86 A/B/rollback but ARM
installation fails at7201.573s. The corrected observations establish entry into
bootc deployment after physical-root cleanup returns, before argument retrieval,
mount setup and child creation. No bootc completion/post-install marker appears.
Guest157 increasing health frames and239 host samples do not establish the
installer's subsequent operation; no guest OOM is recorded. ARM refusal/QMP
shutdown and structured cleanup pass. The cause remains unresolved; no further
speculative run or acceptance relaxation is proposed. Full hosted ARM
installation/update/rollback and native fault qualification remain incomplete.
The aligned stack is published as drafts. Exact D2 ad75aa6, D3 948d3aa,
D4 7171e4e and D5 5e94e8b pass both architecture workspace jobs in
[Check37355982483](https://github.com/Reidond/kedra/actions/runs/37355982483),
[Check37355982477](https://github.com/Reidond/kedra/actions/runs/37355982477),
[Check37355982505](https://github.com/Reidond/kedra/actions/runs/37355982505) and
[Check37355984106](https://github.com/Reidond/kedra/actions/runs/37355984106).
Eighteen redundant runtime runs from the cascade are cancelled, not qualified.
The home-artifact plan remains unimplemented. Nothing is merged, installed on
the workstation or published to production. See WL-20261005-17/19 for exact
runtime receipts, source review and final publication evidence.

The Docker backend and twelve-field inspection-default correction pass the complete
hosted QEMU workflow on D2 source332f450/run37217459554 and D5 source8c35755/run37217459773:
candidate composition/transfer, disk construction, UEFI Secure Boot, kernel lockdown,
enforcing SELinux and exact guest image markers; QEMU exits0 in288s/370s. These are
unsigned disposable boot results. The direct ARM update workflow now passes backend selection and signed media
construction. D2/D3 diagnostics observe actual insecure-boot refusal before
a controller timeout. The shared QMP helper previously closed before a queued quit
command was necessarily dispatched; the owning correction waits for a reply or
server EOF and then real owned-process exit0 within min(15 seconds, original phase
budget). One manual QEMU11 process check and all four hosted refusal/shutdown
phases pass; complete hosted installation/update qualification remains incomplete.
Old D5run37220454933 passed refusal but timed out during fresh installation, a
separate unknown guest boundary. x86 jobs pass. Protected main-only signing is unrun.

The local D2 compatible fixture6146d96 preserves producer1dc. The complete public
HVF sequence passes insecure-boot refusal, fresh installation, A/B/rollback and
finish, including security, twelve native artifacts and home/var persistence.
Every new launcher/native guard and the continuation supervisor returns0. The
original Docker execution reports exit0; the missing old host-parent wait remains
unavailable. Cleanup and protected custody are independently verified. Failed
monitor/fixture attempts remain in the record. Aggregate SHA
92ad69e969437f64ec68e28c196ac9a61e8d1b37cb4aefac9310f981abc9f8fe.

TC05 full sanctioned container coverage passes13/13 on exact candidatee0fd with
frozen57 harness50dca and overlaynone. Its initial12/13 attempt lacked Git metadata
at the compiled source path; adding genuine exact-commit metadata leaves tracked
bytes/modes unchanged, and the complete rerun passes with no cleanup failure.
TC03 genuine release-composer contribution now passes on frozen580/material2271c5c2:
actual foundation/catalog construction, complete public composer, exact-image
jq42/SQLite `TC03|42`, selected separate-library initialization, installed paths,
nine expected public-composer refusals and final store verification. Candidatea28a
receipt SHA c29d2d548a65c7969f26d827cc6309e23ad5a5f55a14ea71b2c577b34a4a3292.
The original32GiB resource-stop remains failed; its separately admitted retry
finishes in2161.39s within60minutes, with peak44.35GB below48GiB trigger and no
reclaim. All runtime containers are absent; controller/observer are stopped,
and retained artifacts/source/binaries are unchanged. Setup/heartbeat/fixture
failures remain recorded. Settlement SHA
aded3a8cbf8eb3447e9a6c15cc2c3bad0d1dfea2bfaf4384cda1a47c2f261a19.
TC05's full suite result stays bound to its earlier exact candidatee0fd.
Separate generic composition/verification/static replay also passes `jq_tool`
and `sqlite-tool` execution with unchanged production-author material. This uses
the separately checked optimized27df CLI and does not expand the catalog
author's fixed alias set. Its result/settlement are recorded in D2 verification.

D5 results use frozen product source2b52355:

- Nine metadata refusals, exact valid-from admission, natural expiry during copying,
  successful admission wholly within the final valid second, incomplete-closure
  refusal and corrupt-existing-content refusal pass. A genuine valid same-recipe/different-result winner also refuses conflicting
  substitution while preserving the receiver and profile; Linux readbacks pass.
- Fresh external consumers compiled against the exact immutable source. Actual signed
  transfer, producer-unavailable execution, reuse, explicit independent rebuild of
  support/report, unchanged public receipts and execution with total43 pass. All28
  workflow commands return0; seven recorded owned container names are independently
  absent. Evidence summary SHA c24d25ecbcffcb8b00ccf02cdc17cc5fb1c322068770fa87ea42f197f8ae0abd.
- Eight genuine-lease schema/member/mode/inode/link refusal cases preserve payload
  and sentinel; restoring authentic state permits public recovery and exact removal.
  Actual wrong-owner/EACCES and filesystem replacement (device and inode both
  changed) also refuse and preserve state. Actual ACL-induced failure after
  snapshot-directory removal leaves the genuine lease intact; restoring ACLs
  permits orphan recovery and an empty repeat. Killed-process cleanup at that
  boundary is not claimed.
- Source-import ENOSPC and actual root-publication ENOSPC pass recovery/retry and
  prior-root preservation. The root-publication case leaves an empty root temporary;
  it was discarded only with the disposable tmpfs, not by product recovery. A genuine source-import journal write also fails with ENOSPC and recovers/retries
  successfully. Actual snapshot-copy ENOSPC and identical same-filesystem retry
  also pass on3GiB tmpfs with no OOM. Native transaction/binding ENOSPC remains
  unqualified.
- One native pre-ID publication window is captured and its abandoned snapshot was
  publicly recovered. Native derive retry is unrun, so no C6 window is fully qualified.
  Pending journal/image state is preserved. Earlier automatic approval review blocked
  that runtime work with a generic possible-cybersecurity-risk reason; it was not bypassed.

Current source/Git ownership is with the primary Codex agent; the Astra runtime worker
owns local Docker/VM operations. Local runtime work is settled; the next work is
to diagnose the hosted ARM installer after early boot. The local VM, genuine contribution, supplementary
generic aliases and listed ordinary cache/recovery checks are complete in their
recorded scopes. Retained signed OS/recovery baselines and historical failures remain
separate. Dated sections below retain their earlier source scopes; worklog entries
33–38 and subsequent exact run evidence supersede older preparation wording.

## Nix recreation investigation (2026-10-01)

The owner requested research into a sharable Rust Nix-style build system and
declarative OS configuration, targeting Kedra. Kedranix is reference only and a
future consumer after rewriting. Sources, citations and the completed research
proposal are retained in [the research workspace](../../../../.specs/nix-recreation/README.md).
Status: research complete; engine, static composition and replay are published
as dependent drafts. Closed native generation and installed container workflows
are locally implemented and tested; fresh-daemon/fault qualification is active.
The [engine guide](ENGINE.md) describes package planning/builds, immutable store,
runtime closure transfer, profiles/develop and collection/recovery. The
[system adapter](SYSTEM.md) exports typed config/runtime contexts over a retained
Fedora44 ARM foundation. The [native stage](NATIVE.md) generates settings caches,
unit links, initial account defaults and generic QEMU initramfs content.
[Package declarations/delivery](PACKAGES.md) explains retained Fedora RPM lists
and ordinary engine store artifacts. Nix frontend/nixpkgs and production backend
integration remain subsequent phases. Existing Fedora bootc release and home
contracts remain in force.

The [proposal](../../../../.specs/nix-recreation/proposal.md) recommends an
independent core, first aarch64/one-output build-and-transfer workflow, then
typed Rust/native-config OS composition into the existing image pipeline.
[Implementation evidence](../../../../.specs/nix-engine/verification.md) records ten
engine cases observed across a nine-pass/fixture-failure full run and repaired
context-only pass. The real C executable/shared library, independent rebuild,
offline receiver store, rollback, artifact/image refusal, interruption and private
Docker context-switch behavior pass with a macOS controller/native ARM Docker29.4.
Native Linux ARM controller compile and seven filesystem/planning/recovery cases
also pass offline. Workspace format/Clippy/E2E/release build and legacy release
interop/material gates pass. The engine is published as draft
[PR28](https://github.com/Reidond/kedra/pull/28) at `7329ef1`; both architecture
workspace checks pass in [run36857909727](https://github.com/Reidond/kedra/actions/runs/36857909727).
All observed engine checks pass, including both architecture workspace/container,
ARM boot, desktop, native, native-home and signed-VM workflows. The dependent
`codex/nix-system-composition` is draft [PR29](https://github.com/Reidond/kedra/pull/29),
implementation `bace56e`, with [concrete requirements](../../../../.specs/nix-system-composition/requirements.md).
Both PRs are in native GitHub stack30 submitted through gh-stack; observed checks pass.
Its final native Fedora composition workflow passes 2/2, including compiled Rust
authoring, actual package/RPM observations, deterministic contexts, typed
references, foundation passthrough and refusal/preservation cases. Workspace
format/Clippy/E2E/release build and legacy release CLI checks pass; exact coverage
is in [composition evidence](../../../../.specs/nix-system-composition/verification.md).
The historical candidate cache still exports incomplete data and is refused.
Continuation restores reviewed signed production9d6eb030 under the installed
signature policy, verifies all78 layers/config/root, and loads a complete native
archive. [Replay evidence](../../../../.specs/nix-context-replay/verification.md)
records public context verification, compiled ELF/runtime-library execution after
producer removal,1MiB config round trip and semantic/cache refusals (2/2 pass).
Generated-unit systemd container case passes1/1; actual pre-build journal
interruption/retry passes. Current source3350732 now composes20 files/1435 RPM rows
and builds a complete static image. The adapter asserts assembly's existing0755
session-wrapper mode while preserving raw source provenance. New branch
`codex/nix-context-replay` is draft [PR31](https://github.com/Reidond/kedra/pull/31),
implementation09473e1, above PR29 in gh-stack30. Both architecture workspace
checks pass on exact head1c85ca4 in
[run36869495760](https://github.com/Reidond/kedra/actions/runs/36869495760).

Continuation `codex/nix-native-artifacts` is the fourth local layer above exact
PR31 head. Closed GLib/systemd/initial-skeleton/QEMU-initramfs derivations and
fixed offline harness recipe bind actual artifacts, unchanged RPM material and
exact parent filesystem layers. Final public native CLI passes1/1 in570.08s;
installed harness passes1/1 in8.56s (report1790873248-91553). Fresh user defaults,
explicit dconf preferences, unit start/mask/default, initial account/existing-home
preservation and initramfs content are executed evidence. The first module-name
and missing dbus-run-session fixture failures are preserved with their corrections
in [native qualification](../../../../.specs/nix-native-artifacts/verification.md).
Cold TLS attempts1/2 hit import timeout/disk budget and cleaned owned resources;
serialized attempt3 passes194.55s on a new empty Docker29.8.1 daemon with exact
ELF/library/config stdout and private binding. Final local workspace/compiler/
legacy release CLI gates pass. Cache repeat and four actual static publication
SIGKILL/retry windows pass: pre-ID rebuild, then exact-ID resume after journal,
binding and final tag publication. A workspace build replaced the shared release
path with a different dependency-feature variant; per-fault hashes were absent
and remain a disclosed provenance limit. Corrective pinned final-executable cold
replay140.43s / cache40.08s pass on another new empty daemon with identical
before/after hashes. All four attempts' exact owned resources/credentials are
removed; outer daemon/default are preserved. Final native cached readback64.14s
passes with that same pinned executable, exact image/identity and actual material.
Implementation3a966f0 is [draft PR32](https://github.com/Reidond/kedra/pull/32),
fourth in native gh-stack30 above exact PR31 head; remote/local ancestry match.
Both architecture workspace checks pass on exact PR32 head3ba7d1b in
[workspace36900727082](https://github.com/Reidond/kedra/actions/runs/36900727082).
Native-cache publication faults,
boot/SELinux/install/update and signed integration remain unqualified.

The owner has authorized five further dependent drafts with Astra subagents:
[derived boot, release integration, catalog, reuse and cache/recovery](NIX-DELIVERY.md).
The first layer is draft [PR33](https://github.com/Reidond/kedra/pull/33),
`codex/nix-derived-boot`, published through gh-stack above PR32; exact initial
published head8a8796c matches local/remote ancestry. Final head712927e has passing
workspace checks on both architectures in
[run36941595156](https://github.com/Reidond/kedra/actions/runs/36941595156).
All six image/runtime workflows also pass on implementation head `8a8796c`:
[container](https://github.com/Reidond/kedra/actions/runs/36941403723),
[desktop](https://github.com/Reidond/kedra/actions/runs/36941403745),
[QEMU](https://github.com/Reidond/kedra/actions/runs/36941403765),
[home](https://github.com/Reidond/kedra/actions/runs/36941403758),
[direct GHCR](https://github.com/Reidond/kedra/actions/runs/36941403715) and
[signed updates](https://github.com/Reidond/kedra/actions/runs/36941403740).
Its exact image source6c38ab05 composes successfully and generates twelve native
artifacts/kernel7.2.7. The disposable signed disk recipee19113d preserves target
production trust and uses a separate generated-key buildroot. Bounded signature
probes and actual signed BIB/install/disk pass806.088s after retained context/
tool-dependency failures. Cold30.119s and warm22.113s pass exact host-bound
deployment/kernel/initrd/native/trust checks, Secure Boot/lockdown, enforcing
SELinux and real Metal desktop/settings/input. AVC notices/denials are retained.
Corrected public native diagnostics passes2.032s on script1f32337; no guest root
authority is added. Earlier systemd Alias-only enable/disable regressions pass.
Compiler/ordinary CLI/legacy release gates pass. D2 source is adopted on
`codex/nix-release-composition`; producer `1dc8d2e` passes fresh foundation,
composition/native generation, complete retention and verified Podman transfer.
Earlier fixture `f1db145` passes independently pinned retained admission and
refusals, with actual ordinary-user public-file reads. It then exposes recorded
fixture umask and layer-representation failures. Corrected fixture `d4d4c77`
passes retained admission on controller `696341b8`, generated-authority signed
media construction and disabled-Secure-Boot installer refusal. Fresh Anaconda
installation under Linux TCG fails at its unchanged 7,200-second deadline; the
disk remains incomplete. Native-HVF transport/cancellation/cleanup is subsequently
qualified. Host source `81528db` installs a new encrypted disk in 291.553 seconds
from the original ISO with separately bound external Kickstart, and cold checks
match all twelve native artifacts and three initial home seeds. A boot passes
security/native-material checks, but its fixture process exits 120 after saving
an enrolled response; no complete A pass or CLI return code is recorded. Narrow
read-only diagnosis and full-hash cleanup pass. Original keeper cleanup now records
real parent exit 143, adopted keeper exit 1 and acknowledged ABORT; retained replay
custody remains explicit. Identical host commits are integrated, and fixture-output,
public graphical-default and embedded-marker corrections are integrated at
`cad6e7a`. That source passes retained admission, twelve regenerated variants and
unsigned/wrong-key public-installer refusals. Default-media construction is stopped
by the capacity guard; a subsequent stage guard fails closed before productive
resume, and the original media deadline expires while held. Controlled cleanup
records real parent exit 130, cleanup_failed=false, no remaining bound consumers,
private mounts or inner containers, and verified restricted replay custody. No final
ISO, new installation or complete update/rollback passes. See
[release verification](../../../../.specs/nix-release-composition/verification.md).
D3's latest native `.sh`/`.conf`/`.service` template revision passes isolated
pinned 1.98.1 formatting, workspace/all-target check and Clippy with all twenty
frozen source hashes unchanged. D3–D5 remain inactive in the owning checkout;
their actual package/reuse/recovery runtime gates remain not-run. D1's dedicated
base/overlay disks and two obsolete PR31/D1 foundation-archive clone aliases are
subsequently retired after exact ownership/reference/hash checks. Historical
qualification, metadata and reports remain; that D1 instance is no longer runnable
from retained disks. Current D2 native/foundation/replay and default/recovery
resources remain preserved. Retirement raises host free space to about 83.5 GiB,
while immediate controller samples remain about 66.4 GiB and fail admission.
Three later independent settled samples, without further deletion/settings change,
report 84,208,209,920 controller bytes and at least 89,623,035,904 host bytes,
passing the unchanged 78 GiB both-filesystem gate. The earlier failed samples
remain recorded. A reviewed fresh exact-cad6 replay then passes actual unsigned
and wrong-key public-installer refusals. Run `ad57f07f5459d594` enters A's normal
public media construction with immediate Linux/host capacity above 78 GiB and
the healthy original guard, then exits 1 after 4.339 seconds. Actual parent wait
is 1; original cleanup and independent remote absence pass. The 9,000-byte private
constructor log was removed after generic failure classification, so the exact
cause remains unknown. A controlled diagnostic invocation requires bounded private
log custody outside the unchanged cleanup root before another run. The earlier
pre-registry permission failure remains separate evidence. No final ISO or new VM
success is claimed; producer, fixture and owning-source identities remain distinct.

That controlled diagnostic subsequently runs as `1710f8ceb4110185`: U/W refuse,
A and its actual parent exit 1, cleanup/independent absence pass, and the complete
9,000-byte constructor log remains in restricted local-only custody. Safe parsing
and a separate anonymous raw inspection establish that the selected old Fedora
base `9ac02a78…e443b` currently returns `manifest unknown` upstream. Its retained
local raw manifest still hashes to the same digest; no registry-retention cause is
inferred. The next smallest supported route under review is the existing explicit
`--base-image` parameter with a pinned retrievable official Fedora 44 ARM base,
separately recorded from signed payload/native producer 1dc. No mirror substitution,
pull-policy relaxation or native-payload relabeling is performed.

The official ARM base `ad037f87…` is subsequently acquired and verified in a
separately guarded phase: two normal exact pulls and an actual FROM-only
pull-always/no-cache probe all exit 0. A final read-only lookup syntax failure is
preserved and corrected without repeating those operations. Linked completion
and all 65 local layer identities/backing entries pass review. Only the already
budgeted 2,056,122,368 base bytes may now be credited, yielding warm admission
81,695,739,904 bytes on both filesystems with fresh cache revalidation at every
call. Other reserves and the 35/30 GiB guard boundaries remain unchanged. No new
constructor/ISO/VM pass is claimed while actual space remains below this gate.
Exact attempts, failures and provenance distinctions are retained in
[derived-boot verification](../../../../.specs/nix-derived-boot/verification.md).

Power-loss/ENOSPC windows, the complete adverse matrices, native x86_64,
further frontends/caches and new OS composition/boot qualification remain
not-run. No image publication or deployment was performed. Research
archive provenance and its dated proposal review remain preserved.

The owner's current policy is **automatically signed GHCR images only**. At 00:00 UTC, changed inputs must pass public validation, isolated OCI signing and strict verification before stable publication. No human approval or manual signing action is required. Unchanged inputs publish nothing. Local ISO construction remains on demand and never uploads.

Read-only observation on2026-10-02: existing main
[release36946414360](https://github.com/Reidond/kedra/actions/runs/36946414360)
passes all eight jobs on `b224d5711e857f7dbcaabf7ed42870916525800c`.
Its verified ARM publication receipt advances the prior9d6eb030 image to
`sha256:7a8f6324305c1d17e9a76df7a21fed5fc11b1d59bbb7ad883e905fff2a230c9f`.
This is the existing main implementation, not the unmerged Nix delivery pipeline.
The session neither triggered that publication nor installed the new image;
earlier installer/native hardware observations remain tied to their exact retained
images and recovery media.

## Active completion and identity replacement (2026-09-30)

The owner explicitly requires finishing the missing qualification and retiring
existing Kedra UTM VMs plus the signed `utm` identity. Current source now defines
`qemu-arm64` with separate `kedra-qemu-arm64` / `kedra-qemu-arm64-builds` images
and a fresh dedicated authority `80551368…1a515`. Its main-only GitHub signing
environment produced the first strict signed publication. Release
[36617035503](https://github.com/Reidond/kedra/actions/runs/36617035503) completed
all six jobs and published stable digest
`sha256:7795329a030d2fc2697d6b88a666ca73f8ff7938f84b245863f90ec16ffab877`.
The old `kedra-utm` and `kedra-utm-builds` packages now return API 404, and the old
signing environment is deleted; environment readback lists only desktop and
qemu-arm64 authorities. The exact local old UTM signing backup was also removed
without reading key contents; the historical `~/VMs/Kedra.utm` bundle is absent.
The three exact old UTM signing/account/disk Keychain entries were identified by
scoped metadata and removed, with item-not-found readback and no secret reads.

UTM CLI and native UI report an empty VM list. Six stopped Kedra prototype
instances using the old target were removed through the owned lab CLI; no
registered owner VM was found or removed. Source now rejects the retired `utm`
target and contains only `qemu-arm64` for this ARM identity. The clean replacement
candidate booted with ANGLE Metal, Secure Boot and enforcing SELinux. Its ignored
runtime, disks and raw reports were subsequently deleted with the Rust `target/`
tree by the owner, so they are historical observations recorded in
WL-20260928-08 rather than currently inspectable artifacts. The 218 MiB runtime
and base disk are rebuilt under `~/Library/Caches/kedra/qemu`; retained
per-checkout VM state lives under `~/.local/share/kedra/lab/<checkout-key>`, outside
Cargo output. Five restored starts reached actual Metal with distinct boot IDs;
full CLI median is 19.239 s/max 21.253 s and readiness median is 16.437 s.

## Current development scope (2026-09-28)

The owner requested a custom QEMU replacement for local UTM tooling, with fast
GPU-rendered manual desktop testing on this Apple Silicon Mac, alongside an
extended Testcontainers edit/sync/screenshot workflow. The
[requirements](../../../../.specs/qemu-desktop-iteration/requirements.md) are
approved by the owner on 2026-09-28. The
[design](../../../../.specs/qemu-desktop-iteration/design.md) is also approved;
the [12 tasks](../../../../.specs/qemu-desktop-iteration/tasks.md),
[28 cases](../../../../.specs/qemu-desktop-iteration/test-cases.md) and
[verification plan](../../../../.specs/qemu-desktop-iteration/test-plan.md) are
approved for implementation. The local replacement is implemented: `kedra-lab vm`
prepares a locked private QEMU/HVF/Cocoa runtime and disposable native disks, keeps
named desktops, applies validated home changes, captures guest pixels and controls
input/lifecycle. Actual niri and Wayland EGL rendering report
`virgl (ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, ...))`; the same desktop
was inspected in the native window. Secure Boot is enabled, SELinux is Enforcing,
the generated keyring is unlocked and no failed system units were observed.

The UTM launcher, cocoa-way/waypipe build/bridge and unused live-view code are
removed. The renamed ARM boot workflow `test-qemu-arm64.yml` retains its security
assertions. The signed source identity, repositories and authority are now
`qemu-arm64`; old external UTM package/environment resources are removed after
verified replacement publication. Signed-media preparation now lives in
`installer/macos`. Public replay produced
`/Users/andriishafar/Kedra/iso-qemu-7795329a/kedra-qemu-arm64-44-7795329a030d2fc2.iso`
(3,129,743,360 bytes, SHA-256
`6351c4b9a81b81e654a9a967e2b79a0e5c0eb6d0564cd5e36a09c705c2b5cfff`).
The first build's stale mount failure is preserved; `build-local.py` now mounts
`str(HERE)`, the actual `usr/src/kedra/installer` directory, and replay passes. The old UTM ISO is
historical only and the current verifier correctly refuses its retired identity.
The owner completed both credential handoffs and fresh encrypted installation
completed on the new 96 GiB disk. The 64 MiB sentinel and original ISO retain their
full hashes; stopped detach passes. ISO-free boot reaches signed Fedora/GRUB and
LUKS, but the missing initramfs GPU/input drivers leave Cocoa inactive. Corrected signed media and the separately observed healthy installed boot now pass.
TC21's private APFS media-refusal matrix passes: changed ISO byte/checksum,
SHA256SUMS/manifest mismatch, wrong desktop target, changed image-digest identity
and existing output all refuse with exit 1; valid verification exits 0, corrupt
installer preflight creates no instance/QEMU, and original media metadata/content
remain unchanged. Evidence is
`~/.local/state/kedra/evidence/2026-09-30/media-refusal-matrix.json`; private copies
were removed.

Native sources, compiler archives, ANGLE dependencies and patches are pinned in
`tests/container/qemu/inputs.json` and its companion locks. Besides the EGL/Cocoa
compatibility patches, the runtime includes upstream stable-11.0 HVF WFI fix
`3b98370b55de7fff540092c1a6760726a6816625`: the unpatched idle VM consumed several
host cores. The corrected runtime measured 2.9% host CPU during a quiet 10 s sample.
Authorized prerequisites were installed; no system QEMU or UTM libraries are used.
Rust development/test debuginfo is now disabled by default, with
`CARGO_PROFILE_DEV_DEBUG=2` retaining an explicit full-debug path. One isolated
before/after comparison measured clean `kedra-lab` compilation at 42.98 s versus
39.85 s, touched-source rebuild at 1.39 s versus 1.15 s, and build-tree size at
1,411,224 KiB versus 801,284 KiB. These are single comparisons, not a general
percentage claim.

PR [#23](https://github.com/Reidond/kedra/pull/23) merged to main as
`bafd1884a569d4890e768c72e335514d824bbf1f`. Main workspace, container on both
architectures, desktop, signed-home, direct-GHCR, signed-update and agent workflows
pass. ARM run [36617035132](https://github.com/Reidond/kedra/actions/runs/36617035132)
attempt 1 retains its 90-minute PID 1 freeze before the observer starts. Exact same
SHA attempt 2 passes, completing 2026-09-29 21:21:02 UTC with the boot step in
5m06s (21:15:52–21:20:58). Main CI is green; later PR24/PR25 follow-up state is
recorded separately below.
Follow-up [PR #24](https://github.com/Reidond/kedra/pull/24) merged as `5dc7b99`;
its final `ad8f385` workflow set is green. The first subsequent release
[36692694223](https://github.com/Reidond/kedra/actions/runs/36692694223) built both
targets, then both validate jobs failed solely because the raw-SHA Git bundle was
empty. Signing/publication were skipped and stable stayed at `7795329a…ab877`,
demonstrating the new guard fails closed.

Current Fedora 44 now resolves Noctalia 5.2.0. The narrow two-file qualification
preserves persisted `APP_VERSION=5.0.1`, accepts exact 5.2 output and refuses
unknown versions. Native validation/export still exposes only three approved typed
fields from a 29-section live export; private weather/plugins stay excluded. Home
case `1790750757-17043` passes in 14.31 s and full ARM worktree run
`1790751520-28479` passes 13/13 with two workers in 109.92 s/no cleanup failure.
The final five visible 5.2 sync+shot samples are 5.136, 4.319, 4.606, 4.453 and
4.335 s (median 4.453/max 5.136); five warm starts have median 18.690/max 23.580 s,
captures median 0.937/max 1.055 s, and five presentation rates span
96.57–101.01/s (median 99.055, every p95 16.667 ms with `sce_`). The guest/source
were restored; final Settings shows Metal, Secure Boot, enforcing SELinux,
Noctalia 5.2 and no failed units. Evidence and screenshot
`final-native-qualification-summary.json` and
`final-ready-settings-1790763286328579000.PNG` are outside Cargo output.

PR #25 fixes the exact-candidate source bundle through a private 0700 named ref
with sanitized Git environment. The previously failed candidate `91e…` had no
overlay; hostile-environment home cycle `1790761204-42859` passes in 50.82 s and
the complete report binds exact `5dc` HEAD while leaving the checkout unchanged.
Old/new commit and payload SHA values match. Materialized snapshot and six review
risks pass, including literal/private-key/hard-link/FIFO/global-ignore/concurrent
lock and SIGINT-130 cleanup. Latest head `193bb0e` changes four final-scope files;
workspace/container runs `36701445476`, `36701440939` and `36701441228` pass.
PR #25 merged as `f3d69dbaba0a50fc167efeb2e0e5d3a6caf6b2c8`. Root CLI E2E passes 2/2 on macOS (Linux-only targets provide zero local
coverage); release build passes in 25.89 s, and fresh-directory release interop and
material checks pass after existing directories correctly refused overwrite.
Independent review found the signer checked remote main while the publisher used
only its checkout; the current fix rechecks remote main immediately before publish.
Release [36702944904](https://github.com/Reidond/kedra/actions/runs/36702944904)
passes all eight jobs: both exact candidates pass the full harness before isolated
signing and strict stable publication. QEMU stable is `sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71`.
Corrected ISO export `cb8578a4…49504` passes the independent public media verifier
and full checksum after its host wrapper was interrupted. Corrected encrypted
graphical boot is now qualified by the separate corrected fresh-install/ISO-free
unlock/login and installed-system checks, not by export alone.

Five stable-tag worktree-overlay suites pass 13/13 with wall times 58.959, 66.322,
64.764, 65.563 and 72.787 s (median 65.563/max 72.787), image tag
`6f95044a24e38c69`, reports `1790708814`, `1790708873`, `1790708939`,
`1790709025` and `1790709090`, with the receipt outside Cargo output. The earlier
no-overlay stale-provenance refusal remains recorded. Independent concurrent
Bitwarden (`1790709593-88410`) and desktop (`1790709593-88411`) executions both
pass, clean up independently and leave the default retained lab healthy.

The initial restored desktop's five full-CLI warm starts were 19.239, 19.348,
18.461, 21.253 and 18.533 s
(median 19.239/max 21.253), with median readiness 16.437 s and five distinct boot
IDs reporting Metal. Capture wall times are 0.869, 0.874, 0.875, 0.883 and 0.893 s
(median 0.875/max 0.893). Final state is an unlocked, healthy 2560×1600 scale-2
desktop. Unlocked scale-1/1.5/2 Noctalia Settings screenshots were independently
viewed with valid geometry; resize to 1920 worked, and reopening QEMU restored
actual 2560. Audio passed twice with owner confirmation; physical typing,
Command+Enter, scrolling and window movement pass.

Five 30 s Virtual-1 presentation observations after 5 s warmups report
81.967, 91.126, 89.334, 81.634 and 90.500 presentations/s (median 89.334,
minimum 81.634); every p95 interval is 16.667 ms. Visible EGL/SHM clients ran in
native Cocoa. This qualifies guest Virtual-1 only, not a physical monitor or other
hosts.

Runtime unchanged replay passes. A real interrupted private-bundle preparation
exits 143 without selecting a runtime/receipt; incomplete private pins exit 1 for
missing ANGLE without selection; the current runtime hash matches its original
state. Public CLI hard-link, TPM-tree link, log-link and predictable pending-file
refusals preserve foreign data and the running default. A nonresponsive public-CLI
stop refuses after 10.583 s without forcing; explicit force completes in 10.789 s,
with controller 10 s behavior, disks and foreign/default state unchanged. The
literal grace case now passes: after 31.336 s it reports accepted poweroff without
implicitly forcing; a 45-second ordinary guest unit is verified to ignore TERM.
Explicit force completes in 1.487 s, preserving disk/VARS/foreign sentinel, then
public remove passes. The default was stopped before/after with stale PIDs, so this
case makes no running-default preservation claim; the separate 10-second variant
provides that evidence.

The corrected signed ISO was freshly installed on only the new 96 GiB disk. Visible
ISO-free LUKS unlock and greetd login now pass. Installed doctor, Secure Boot/
lockdown, enforcing SELinux, Metal, LUKS2 and read-only TPM/PCR7 dry-run pass;
bootc confirms exact 9d6 signed image with no staged/rollback deployment. Original
ISO and full unselected 64 MiB sentinel hashes remain equal after final stop. The
final graceful stop timed out and explicit public force was required. The corrected
installed VM is retained stopped; the obsolete diagnostic VM is absent. Native
and container defaults are restored running. Evidence: `tc22-fixed-summary.json`
outside Cargo output. Earlier missing-driver failure is historical and preserved.
For the pushed follow-up, formatting, ruff and workspace
all-target Clippy pass (30.32 s); empty/whitespace `down` selectors exit 1 while
the default stays healthy, and exact stopped-owned removal passes using the rebuilt
release binary. Fresh container Settings capture
`target/kedra-lab/shots/final-container-desktop-1790748508-70906.png` was viewed
unlocked and healthy with explicit software rendering. See
WL-20260928-08 and the [delivery review](../../../../.specs/qemu-desktop-iteration/review.md)
for exact evidence and remaining qualification.
Earlier production and qualification records below retain their original dates and
source revisions.

## Current summary (2026-09-25)

- **Production.** `ghcr.io/reidond/kedra-desktop:stable` resolves to `sha256:3fb355b4151ca5741fb5cfb53689eaade6ea262dabdfb7f74fd124f9ecaae85f`, published by push release run [36132498983](https://github.com/Reidond/kedra/actions/runs/36132498983) (run #26) from main `664ffc1088cef1a3374f665b61bc261cfcb0c673` (PR #15, bootc 1.16.13, Fedora base 44.20260925.0). Anonymous registry readback on 2026-09-25 confirms the digest and identity (run 26, attempt 1). Fourteen signed stable publications exist in total; see the table below.
- **Desktop source.** PR #14 (Adwaita desktop, adw-gtk3, Noctalia 5.1 compatibility, right-side clock) merged on 2026-09-14 as `17105c5`. On that exact commit, workspace [34820938028](https://github.com/Reidond/kedra/actions/runs/34820938028), desktop [34820938007](https://github.com/Reidond/kedra/actions/runs/34820938007), signed home [34820938085](https://github.com/Reidond/kedra/actions/runs/34820938085), direct GHCR [34820938156](https://github.com/Reidond/kedra/actions/runs/34820938156) and signed updates [34820938031](https://github.com/Reidond/kedra/actions/runs/34820938031) pass. Push release [34820938097](https://github.com/Reidond/kedra/actions/runs/34820938097) published it.
- **Daily images are not VM-tested at their exact digests.** Publication does not wait for the VM workflows; each nightly image differs from the tested source build only in refreshed Fedora inputs.
- **bootc compatibility.** Fedora 44 moved `bootc-1.16.13-1.fc44` to stable on 2026-09-25. The shared contract still qualifies 1.16.10, so builds that resolve 1.16.13 fail closed before signing until the contract is deliberately updated and requalified. PR #15 bumped the contract to 1.16.13; it passed every workflow at `89edf80` and again on main at `664ffc1` (workspace 36132498938, RPM refresh 36132499073, agents 36132499274, signed updates 36132499148, direct GHCR 36132498767, signed home 36132499008, desktop 36132498749). A fresh Anaconda installation with 1.16.13 is not-run.
- **In progress (WL-20260925-02, `codex/secure-boot-utm`):** product-wide UEFI Secure Boot enforcement (installer refusal, required `secure_boot` doctor check, every VM workflow on Secure Boot firmware with a snakeoil-key negative case) and the separately signed aarch64 `utm` target for UTM 5.0.6 on Apple Silicon. The `kedra-utm-signing` environment, secrets and public variables exist (fingerprint `76ca7a65915adb1907acbe0885af83c5c569dd2964b87decbfb67059dee366c6`); `ghcr.io/reidond/kedra-utm` and `kedra-utm-builds` exist, public and linked to the repository, holding only an unsigned `bootstrap` placeholder. No utm image is built, signed or published yet, and no Secure Boot VM run has happened in Actions.
- **utm on the owner's Mac (2026-09-26, manual):** the first signed `kedra-utm` image (`sha256:b9a5c32e…`, run 28) was built into local installer media, installed encrypted into a UTM 5.0.6 VM and booted with UEFI Secure Boot enforced (NVRAM Fedora entry, lockdown integrity), SELinux enforcing, `sysroot doctor` all OK, VirGL OpenGL ES 3.0 and Vulkan through `Virtio-GPU Venus (Apple M2 Pro)` API 1.4.334, no failed units. UTM Shared Network gave the guest no IPv4 on that Mac; Emulated VLAN works and is now the tooling default. Enrollment, signed forward update, clipboard/resize and aarch64 Bitwarden/Codex are not yet qualified there (WL-20260926-01).
- **Container tests and lab (2026-09-27, PR #21 merged as `490e13a`):** `usr/src/kedra/tests/container` implements the owner's Testcontainers harness specification. Scenarios and native tests boot the image under test with systemd as PID 1 and run the real Kedra session nested in headless sway: image contents, the session, Noctalia, portals and keyring, doctor, Bitwarden, Codex, R03/R04 home review and recovery, and GTK 3/libadwaita choosers. On the owner's Mac (utm, working tree over stable `sha256:45fe5f72…`) all 13 tests pass locally. A full local build of the working tree built in 8 min. `kedra-lab` runs the desktop from any stage and takes 2560×1600 screenshots; `kedra-lab up --live` shows it in cocoa-way built from pinned sources. On main at `490e13a`, test-container.yml passes on both targets ([36323569209](https://github.com/Reidond/kedra/actions/runs/36323569209)), with every VM workflow, check, agents and RPM refresh. test-desktop.yml and test-utm-image.yml keep only boot-level checks. Push release [36323569281](https://github.com/Reidond/kedra/actions/runs/36323569281) (run 34) published both targets: desktop `sha256:b6b017fab000f30338e6be84eb4b400da9f0c97397799e9c74ea65a41e4ab614`, utm `sha256:f3230ed446ccc500ac5db6fd2e57c99d26bd31e4395ca3bc22b1e6494dbd2129`.
- **Not qualified:** Secure Boot on physical hardware, physical hardware, production-image forward update on an installed system, deterministic registry race/interruption, native OCI-platform mismatch, key rotation, VM suspend/resume, scaling/high contrast/accessibility, authenticated agent and Bitwarden use. The local ISO was built and installed only from `acafed57…` (2026-09-13).

## Production publications

| Run | # | Date (UTC) | Source | Published stable digest |
|---|---|---|---|---|
| [34746729066](https://github.com/Reidond/kedra/actions/runs/34746729066) | 10 | 09-13 dispatch | `5dea673` | `sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14` |
| [34747330145](https://github.com/Reidond/kedra/actions/runs/34747330145) | 11 | 09-13 dispatch | `5dea673` | none (no-change) |
| [34752129104](https://github.com/Reidond/kedra/actions/runs/34752129104) | 12 | 09-13 push | `9b5005a` | none (no-change) |
| [34792981175](https://github.com/Reidond/kedra/actions/runs/34792981175) | 13 | 09-14 | `ca3ce33` | `sha256:5e2dd1bb589574dac2175e52f643be0b879f2e2ae0a1c4de3a61b409a35f12a9` |
| [34820938097](https://github.com/Reidond/kedra/actions/runs/34820938097) | 14 | 09-14 push | `17105c5` | `sha256:528b0faa074dce510b9a8662693347fbe3547e3a44aa52cb87849867aee31867` |
| [34913413931](https://github.com/Reidond/kedra/actions/runs/34913413931) | 15 | 09-15 | `17105c5` | none: failed closed on a transient HTTP 500 downloading cosign; fetches now retry |
| [35040133877](https://github.com/Reidond/kedra/actions/runs/35040133877) | 16 | 09-16 | `17105c5` | `sha256:c6d1535259d2495b135eb10b892a279550ef52015ee201f4cc0fd6c356e63113` |
| [35166692228](https://github.com/Reidond/kedra/actions/runs/35166692228) | 17 | 09-17 | `17105c5` | `sha256:1dc46b761e246e1c1b91568fdc93b20615f59b14c30bf598e62b86a13cd03927` |
| [35291343086](https://github.com/Reidond/kedra/actions/runs/35291343086) | 18 | 09-18 | `17105c5` | `sha256:f5f99c74b62d0f95a8e584d5ed4597ed936d16def47657c3e11ff9da2fac7201` |
| [35409458319](https://github.com/Reidond/kedra/actions/runs/35409458319) | 19 | 09-19 | `17105c5` | `sha256:9c1ebf0140cf630a9c3c294e8618f0017643f7c8e62daf946877183b99844868` |
| [35479024601](https://github.com/Reidond/kedra/actions/runs/35479024601) | 20 | 09-20 | `17105c5` | `sha256:b3e6c74686ad0deba7b211c66c8057575ce99cfe3984cd3daee7c30a12375f27` |
| [35547977428](https://github.com/Reidond/kedra/actions/runs/35547977428) | 21 | 09-21 | `17105c5` | `sha256:36885fd9c2d1eba6a1bf52ff93e797ae451d06669aaa78559590526a62f718eb` |
| [35672122067](https://github.com/Reidond/kedra/actions/runs/35672122067) | 22 | 09-22 | `17105c5` | `sha256:77374239346a677ec5fc6df30a98570a98ef174369d4fe0942e53ede1501d6b0` |
| [35802122457](https://github.com/Reidond/kedra/actions/runs/35802122457) | 23 | 09-23 | `17105c5` | `sha256:7c64834abd21dc92d1a3a5ac6b8326c5d2a0970446f6f066bc33091b761563b8` |
| [35938559348](https://github.com/Reidond/kedra/actions/runs/35938559348) | 24 | 09-24 | `17105c5` | `sha256:64a118939ba59d43930cc50936c205eada3d0d35f48eb273be17f721c1fe44c8` |
| [36077781275](https://github.com/Reidond/kedra/actions/runs/36077781275) | 25 | 09-25 | `17105c5` | `sha256:fe61a11d37b97ca04b58d87cd98fc5e95bb33af69e1c84d0d7fdda26041d7675` |
| [36132498983](https://github.com/Reidond/kedra/actions/runs/36132498983) | 26 | 09-25 push | `664ffc1` | `sha256:3fb355b4151ca5741fb5cfb53689eaade6ea262dabdfb7f74fd124f9ecaae85f` |
| [36323569281](https://github.com/Reidond/kedra/actions/runs/36323569281) | 34 | 09-27 push | `490e13a` | desktop `sha256:b6b017fab000f30338e6be84eb4b400da9f0c97397799e9c74ea65a41e4ab614`; utm `sha256:f3230ed446ccc500ac5db6fd2e57c99d26bd31e4395ca3bc22b1e6494dbd2129` (runs 27–33 are not listed here) |

Each nightly rebuild is driven by a new `quay.io/fedora/fedora-bootc:44` base digest and, on most days, changed Fedora packages. Every published receipt is `verified` and chains `previous_digest` to the row above it. Immutable `run-<id>-1` tags and signatures are retained; there is no garbage collection.

## Agents, credentials and repository skills

- **Codex** 0.153.4 (x86_64) is bundled under `/usr/libexec/sysroot/agents/codex` from the pinned official archive with Sigstore verification. [test-agents 34695674783](https://github.com/Reidond/kedra/actions/runs/34695674783) and the desktop VM (`KEDRA_R05_IMAGE_RUNTIME_PASS`) pass for runtime selection, profiles and `--version`/`--help`. Authenticated model use, MCP, skills and hooks are not qualified.
- **Claude** is not bundled: `public_preinstallation_approved` remains false pending the owner's Commercial Terms decision. `sysroot claude --runtime user` is the only path.
- **Repository skill discovery (R11):** on 2026-09-07 Codex CLI 0.153.4 marketplace and fresh-profile probes found no Kedra skills at the root or crate cwd. On 2026-09-27 the owner chose project registration (AGENTS.md): checked-in `.codex/config.toml` and `.claude/settings.json` enable `kedra@kedra-local` and `rust-skills@kedra-local`. On the owner's Mac, Codex CLI 0.156.1 lists both as installed and enabled, and its cache matches the working tree. Claude Code 2.1.282 there lists both as enabled at project scope from the repository root (disabled from a subdirectory) and offers their 30 skills in session; the checked-in marketplace path is the relative `./`, whose resolution in a fresh clone is unverified. A model session invoking them is not-run.
- **Bitwarden Desktop** 2026.8.0 is bundled at `/usr/lib/bitwarden`. The desktop VM covers native sandbox startup while logged out (`KEDRA_R06_LOGGED_OUT_PASS`) and `SSH_AUTH_SOCK` propagation. Vault login/unlock, key serving, signing approval and Git over SSH are not-run.

## History

The sections below are the chronological qualification record, newest first. Each keeps its original source scope.

### First automatic production publication (2026-09-13)

The automatic production changed-image and no-change paths are now verified. Run [34746729066](https://github.com/Reidond/kedra/actions/runs/34746729066) at main `5dea673f6ebd3a86c44797517889e5a80ba5e78a` completed build, isolated sign-image and publish-stable successfully without a reviewer gate. Both `ghcr.io/reidond/kedra-desktop:stable` and immutable run tag `run-34746729066-1` resolve to `sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14`.

Publication artifact `10313929124` independently matches ZIP SHA-256 `122832f43653688aa139bcb9cabf15c656f4c8ad1497c158ad64c54fdebfea6c`. Its receipt is `verified`, with `previous_digest: null`, and binds the exact main source and published digest. The verified image identity is protocol 2, desktop/Fedora44/x86_64, rank `(1,10,1)`. Remote Docker readback independently confirmed both tags. `KEDRA_RELEASES_ENABLED=true`; environment 21492153153 retains only its main branch restriction, with administrator bypass disabled and no required reviewers.

Repeat run [34747330145](https://github.com/Reidond/kedra/actions/runs/34747330145) at the same exact main source succeeded after verified stable-image comparison and package preflight. Changed-image construction, signing and stable publication were all skipped; the run produced zero artifacts. Independent Docker readback confirms stable remains at the same signed digest. This demonstrates an actual production no-change run, not only a fixture result.

### Right-side bar clock

The layout follow-up from `308b03f467a50df91aca20930857122ed471785b` places the
clock in the right-side status group immediately before Control Center and
leaves the center empty. Cached native Noctalia 5.0.1 validation passes without
warnings; full effective export confirms the exact order and no default clock
reinserted. Local 5.1 validation is not-run because that binary is unavailable.
Exact source `b4c942d88f6be1fdfcf21c83267f7afb2fd66e1c` now passes desktop
[34786061737](https://github.com/Reidond/kedra/actions/runs/34786061737), signed
home [34786061738](https://github.com/Reidond/kedra/actions/runs/34786061738) and
workspace [34786061733](https://github.com/Reidond/kedra/actions/runs/34786061733) /
[34786063461](https://github.com/Reidond/kedra/actions/runs/34786063461).
Independent review of artifact `10327410805`'s `vm/desktop.png` and
`vm/settings.png` confirms the clock after battery and immediately before
Control Center, an empty center and no collision at 1280×768. The requested
layout is qualified in the disposable VM. Physical displays/other scaling remain
separate; no user-machine apply or production publication occurred. Earlier
`4d030519` GUI evidence retains its centered-clock scope. See WL-20260914-02.
Home artifact `10327097877` records passing stage B, accepted B/rollback staging
and retained-A home rollback markers at 22:48–22:49 UTC.

### adw-gtk3 follow-up

**Latest exact-source result:** `4d0305194341b702f3e39fb6abba6a1f6b3f29a0`
passes desktop [34784038994](https://github.com/Reidond/kedra/actions/runs/34784038994),
workspace [34784039095](https://github.com/Reidond/kedra/actions/runs/34784039095) /
[34784041029](https://github.com/Reidond/kedra/actions/runs/34784041029), direct
GHCR [34784039027](https://github.com/Reidond/kedra/actions/runs/34784039027) and
signed-update regression [34784039089](https://github.com/Reidond/kedra/actions/runs/34784039089).
Desktop artifact `10325794069` contains 35 PNGs: GTK 3 Wayland/X11 report
adw-gtk3, Adwaita Sans 11/icons and successful exact-content file selection;
libadwaita 1.9.3 retains Adwaita-empty/native light StyleManager behavior; Qt 5/6
KDE/Breeze and personal-font workflows also pass through KEDRA_R07_SESSION_PASS.
Candidate adw-gtk3-theme 6.4-3.fc44/CSS path are verified. Independent screenshot
review confirms rounded GTK 3 controls and preserved native libadwaita.
Signed home [34784039080](https://github.com/Reidond/kedra/actions/runs/34784039080)
also passes, with STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME at
21:57–21:58 UTC. Source and disposable-VM qualification are complete; PR #14
remains open, with no production publication or installation. Physical displays,
high contrast and dynamic GTK 3 Xwayland dark-mode propagation remain separate.
Persisted State.app_version=5.0.1 is preserved at source level; migration of a
record created by the old 5.0.1 CLI into the 5.1 runtime is not-run as an E2E.
The signed A/B/A pass qualifies the current 5.1 managed workflow.
The earlier failures below retain their original source scope.

At `f9d46a79462bb433d3120071ad44e6e2dbe511dc`, desktop
[34782406383](https://github.com/Reidond/kedra/actions/runs/34782406383) builds
the candidate with adw-gtk3-theme 6.4-3.fc44, but refreshed Fedora packages supply
Noctalia 5.1.0-1.fc44. The VM correctly refuses unqualified Noctalia home review
shortly after login, before toolkit cases. This is a compatibility safety guard,
not an adw-gtk3 failure. Native 5.1 export/IPC/override comparison is in progress;
exact-version compatibility support is now implemented from native evidence,
without blanket acceptance or a package pin. Home-transition
[34782406358](https://github.com/Reidond/kedra/actions/runs/34782406358) also fails
on stage-B boot: artifact `10325981426`, stage-b/serial.log line 67, records the
same unqualified-version refusal in `sysroot home init`. Workspace push
[34782406364](https://github.com/Reidond/kedra/actions/runs/34782406364) and PR
[34782409405](https://github.com/Reidond/kedra/actions/runs/34782409405) pass.
Prior full Noctalia 5.0.1 GUI evidence does not qualify the new runtime.
See WL-20260914-01 for the continuation across local midnight.

The source now accepts runtime versions exactly 5.0.1/5.1.0, mapping their
measured CLI version strings to the existing safe projection. Persisted
APP_VERSION/state identity remains 5.0.1, preserving old records; unknown
versions still refuse. The native 5.1 probe passes config/export, IPC dark/light,
settings-path and stopped-writer/restart checks with clean RPM verification.
Local Rust 1.98.1 formatting, all-target Clippy, release build and whitespace
pass. The orchestrator committed/pushed exact source
`4d0305194341b702f3e39fb6abba6a1f6b3f29a0`; workspace
[34784041029](https://github.com/Reidond/kedra/actions/runs/34784041029) /
[34784039095](https://github.com/Reidond/kedra/actions/runs/34784039095), desktop
[34784038994](https://github.com/Reidond/kedra/actions/runs/34784038994), signed home
[34784039080](https://github.com/Reidond/kedra/actions/runs/34784039080), direct GHCR
[34784039027](https://github.com/Reidond/kedra/actions/runs/34784039027) and legacy
signed updates [34784039089](https://github.com/Reidond/kedra/actions/runs/34784039089)
are in progress. No outcome, new GUI qualification or publication is inferred.

The follow-up from `3dd56e9254a5b531d71b4ff7e177cb3c3160f42f` implements official
Fedora 44 `adw-gtk3-theme` 6.4-3.fc44 with GNOME and GTK 3 fallback defaults.
Disposable native GTK 3.24.52 applications on X11 and nested-niri Wayland render
with adw-gtk3/Adwaita Sans 11. New Wayland clients honor a personal adw-gtk3-dark
GSettings selection and reset. Strict schema compilation passes with inherited
deprecated-path warnings. The runtime probe records ADW_GTK3_RUNTIME_PASS and
its disposable container was removed.

Libadwaita 1.9.3/PyGObject 3.56.3 initializes Adwaita-empty with dark=false,
high_contrast=false and color_scheme=0 in a real Xvfb application. Plain GTK
4.22.5 can use the package's GTK 4 CSS through the shared theme setting; all
RPM-owned assets remain intact. Qt/Breeze is unchanged. The VM fixture now
checks candidate/fixture RPM/CSS consistency, exact GTK 3 theme on both backends
and native Adw StyleManager behavior; the final Actions VM run passes above.
See [desktop guidance](DESKTOP.md#gtk-and-qt-applications) for package/upstream
sources and explicit dark-variant selection. WL-20260913-10 and WL-20260914-01
are completed; earlier full Adwaita evidence remains scoped to its original theme.

### Noctalia Greeter evaluation

Evaluation at `7f76857f0e903b61891f7bf1584bece98c5c73cc` recommends Noctalia
Greeter 1.5.0 as a separately qualified follow-up. [PR #14](https://github.com/Reidond/kedra/pull/14)
remains open with its existing greetd/tuigreet implementation and green checks;
no Greeter implementation, merge or production publication occurred. The
[desktop evaluation](DESKTOP.md#noctalia-greeter-evaluation) records sources and
the proposed pinned Actions build using official Fedora dependencies, static
administrator-owned Adwaita styling and preserved PAM/keyring/niri integration.
Greeter login, session selection, TTY recovery, scaling and SELinux checks are
not-run. Runtime-directory handling and portable compiler flags require review
before adoption. This is a completed evaluation, not a blocked implementation.

### Adwaita desktop source configuration

The `codex/adwaita-desktop` changes from `ca3ce333ac33fa17e81f6c92610e4cdca802018e`
implement [GNOME HIG-informed desktop defaults](DESKTOP.md): a dark Adwaita-like
Noctalia shell around native light/default GTK applications, a full-width top bar,
Adwaita fonts/icons/cursor, rounded niri windows and familiar overview, launcher,
lock and screenshot shortcuts. Existing user preferences remain authoritative;
the Noctalia home projection still contains only its three supported safe fields.

On 2026-09-13, native validation against the cached signed production image
`sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14`
passed in isolation: niri 26.04 and Noctalia 5.0.1 accepted the new configuration
without warnings, and Noctalia's full effective export was inspected. The lock
shortcut uses the verified v5.0.1 command `noctalia msg session lock`. GLib 2.88.3
strict schema compilation passed with schemas 50.1; inherited deprecated-path
warnings remained. All nine GSettings defaults read back correctly, a temporary
user `prefer-dark` override/reset worked, and fontconfig resolved Adwaita Sans/Mono.
The five explicitly listed Adwaita/schema packages were already present in that
cached image. Manual palette calculations checked light and dark primary roles
at contrast ratios above 4.5:1; this is not a full accessibility audit.

Shell syntax, Git whitespace, Rust 1.98.1 formatting, Clippy with warnings denied
and release build passed. The Windows E2E command succeeded but executed zero
cases, so it provides no new Linux behavior coverage. At that initial milestone,
new-image and graphical checks were not-run; the later exact-source native/VM
results follow below. Physical qualification remains separate. The earlier
published image and ISO results remain evidence for their original source only.
See WL-20260913-07 in the
[worklog](../../../../worklog.md) for this source task's checks and next step.

### Native GTK and Qt integration follow-up

**Current exact-source result:** `71cb8c9158871f82cb38f1fa59df2e7260dd51cf`
passes workspace [34769161713](https://github.com/Reidond/kedra/actions/runs/34769161713)
and desktop [34769161693](https://github.com/Reidond/kedra/actions/runs/34769161693).
Artifact `10321492608` contains 35 PNGs and passing wallpaper-source,
no-video-bridge, managed Noctalia fallback-warning absence at startup/after
restart, all six native GTK 3 Wayland/X11, libadwaita and Qt 5/6 KDE-dialog/user
font workflows with exact selected-file content, KEDRA_TOOLKITS_PASS and
KEDRA_R07_SESSION_PASS markers. Independent desktop.png/settings.png review
confirms blue Adwaita accents and charcoal panels without yellow/navy fallback
or the black bridge tile.

Exact-head signed home-transition
[34769161731](https://github.com/Reidond/kedra/actions/runs/34769161731) also
passes, including STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME markers
at 17:02–17:03 UTC. Independent GPT-6 Astra read-only review found no actionable
defects. Source and disposable-VM visual/workflow qualification are complete;
physical hardware, scaling/high contrast and full accessibility remain separate.
No production publication or installation is claimed. Next: review/merge a PR
if the owner authorizes, then separately qualify the published image and physical
target. The milestones below preserve earlier failures and their corrections.

The follow-up from `4d1d0d2888a43c3ea2cdf281ab9be44ef263a582` adds KDE platform
integration and native Breeze styles for Qt 5/6, KDE Qt Quick Controls styles,
Breeze icons and system `kdeglobals` defaults. The systemd user environment
selects the KDE platform theme while retaining explicit user choices.
GTK/libadwaita Adwaita defaults and niri's desktop/portal identity remain intact.
See [desktop integration](DESKTOP.md#gtk-and-qt-applications).

Disposable native Xvfb application probes pass for Qt 5.15.18 and Qt 6.11.2:
both resolve Breeze, Adwaita Sans 11, Breeze icons and the document-open icon.
The measured KDE packages are Plasma integration/Breeze 6.7.5, KF6 styles/icons
6.30.0 and the KF5 desktop style 5.116.1. Fedora systemd 259.8 environment
generation defaults to `kde` and preserves an explicit `qt6ct` value. GTK 3
also resolved Adwaita/Adwaita Sans 11 in a real Xvfb client without the GNOME
daemon. Both Qt versions honored a personal `kdeglobals` override to Fusion and
Adwaita Mono 12. GTK 3 Xwayland uses the static settings fallback and requires application restart after
changes. An isolated GNOME XSettings 50.1 experiment published initial settings
but failed dynamic font propagation outside GNOME; that daemon is not shipped
or enabled by this follow-up.

The desktop VM workflow now prepares real GTK 3 Wayland/Xwayland, libadwaita,
Qt 5, Qt 6 and Qt 6 personal-preference application cases. QMP interaction opens
native file choosers, selects generated text files and checks their returned
content, with screenshots at each stage. The package inventory is captured before
adding test-only bindings. Syntax and whitespace checks pass according to the
implementation worker; new GUI and image qualification remain pending. No
exact-base Actions run existed when this follow-up began.

Workspace run [34758096731](https://github.com/Reidond/kedra/actions/runs/34758096731)
passed at `610da61`. Home-transition run
[34758096745](https://github.com/Reidond/kedra/actions/runs/34758096745) failed
because its fixture still expected niri `gaps 12` after the baseline changed to
8. The correction in `prepare.py` and `home.py` is locally ready and passes
native niri validation. Its signed A/B/A rerun
[34758449303](https://github.com/Reidond/kedra/actions/runs/34758449303) at
`076814d0a68933d0a384b7db02ba4598ffa4fe88` passed the full workflow, including STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME graphical VM markers. Desktop run
[34758096727](https://github.com/Reidond/kedra/actions/runs/34758096727) was
cancelled when the test-only Qt chooser fix at `24c61b7` was pushed. At
`24c61b7a5cfb2310b97d3c3e8439e335dd8b843e`, workspace
[34758255401](https://github.com/Reidond/kedra/actions/runs/34758255401) passed;
desktop [34758255393](https://github.com/Reidond/kedra/actions/runs/34758255393)
failed after candidate build/validation, disposable-disk creation and login,
before the toolkit cases. Artifact `10317828820` shows that the old fixture set
the already-selected light mode, so home staging correctly refused an unchanged
value. A dynamic mode choice is being implemented in `check.sh` and `recovery.py`.
The failure screenshot shows the rendered shell/bar, a cartoon wallpaper and a
large black focused window of unknown identity. The current source adds
the native-validated Noctalia 5.0.1 wallpaper default `color:#222226`, preserving
personal overrides, and adds wallpaper readback plus niri window inventory to
the VM evidence. The [desktop design](DESKTOP.md) links the tagged wallpaper
implementation and example. Later startup inventory identifies the black client
as `xwaylandvideobridge`; niri-specific autostart exclusion is now implemented,
with GUI qualification pending.
See WL-20260913-08 for the
continuing evidence.

At current source `711bdf223120efab675c7fd2dff73618c7573cf1`, workspace
[34759390627](https://github.com/Reidond/kedra/actions/runs/34759390627) passes
with matching head/status/conclusion independently checked. Desktop
[34759390621](https://github.com/Reidond/kedra/actions/runs/34759390621) failed
after passing candidate/disk creation, wallpaper readback and Noctalia
projection/recovery markers: `niri-review.py:118` still expected the historical
`gaps 12` baseline. Artifact `10318453176` records that failure and identifies
the focused 628×716 black tile as `xwaylandvideobridge` (Wayland to X Recording
bridge). The fixture is being corrected. `build/assemble.sh` now adds
`NotShowIn=niri;` to the packaged bridge autostart entry, retaining its package
and manual launcher. New VM checks require no bridge process/window at startup
and after the session workflow; execution of this correction is pending.
Native Wayland portal sharing remains configured; legacy X11 bridge capture is
opt-in and separately unqualified. See [desktop sharing](DESKTOP.md#screen-sharing-and-the-x11-bridge).
the toolkit cases have not run. Home-transition
[34759390618](https://github.com/Reidond/kedra/actions/runs/34759390618) passed
at `711bdf223120efab675c7fd2dff73618c7573cf1`.
The earlier signed home-transition pass does not qualify this exact source or
the new graphical workflows.

Current source `e0e1031a92b189ae30e419ac1211196930445f8c` includes the bridge
autostart exclusion and desktop fixture corrections. Workspace
[34760607440](https://github.com/Reidond/kedra/actions/runs/34760607440) passes.
Desktop [34760607452](https://github.com/Reidond/kedra/actions/runs/34760607452)
failed in the new gtk3-wayland fixture because `toolkit-app.py` imported Gdk 4
before Gtk 3, causing a GI namespace conflict; a fixture correction is underway.
Artifact `10319262013` confirms wallpaper-get, empty startup window inventory,
KEDRA_NIRI_NO_VIDEOBRIDGE_PASS, Noctalia projection/recovery, niri line
review/recovery and portal/keyring/doctor passes. The independently inspected
failure screenshot shows an uncluttered charcoal desktop/top bar without the
black bridge tile. Toolkit GUI behavior remains unqualified. Home-transition
[34760607435](https://github.com/Reidond/kedra/actions/runs/34760607435) passed
at `e0e1031a92b189ae30e419ac1211196930445f8c`.

Latest source `a165312` changes only the GTK GUI fixture and documentation/worklog;
runtime image/home source is unchanged from that passing home-transition run.
Workspace [34761609811](https://github.com/Reidond/kedra/actions/runs/34761609811)
passes. Desktop [34761609814](https://github.com/Reidond/kedra/actions/runs/34761609814)
failed waiting 45 seconds for the first GTK 3 Wayland chooser's selected marker.
Artifact `10319930862` shows the ready app and native Adwaita chooser; GTK 3
theme/icon/backend checks passed. The fixture lexically compares `/var/home`
from `Path.home()` against the `/home` symlink path typed through QMP, a possible
callback failure. The source now uses `samefile(sample)` and exact content
verification, explicitly reports callback failures and captures a post-submit
screenshot plus toolkit journal/result/window inventory on timeout. Disposable
Fedora alias, syntax and whitespace checks pass; the corrected VM rerun is
pending and the exact historical timeout trigger is not yet proven. Its final failure
screenshot follows trap poweroff and does not establish a compositor failure.
The six-case toolkit suite remains incomplete. Passing home-transition at
runtime-equivalent `e0e1031` and workspace CI retain their recorded scopes.

The subsequent desktop run
[34762761532](https://github.com/Reidond/kedra/actions/runs/34762761532) at
`e8f3272` corrects that diagnosis: artifact `10319314507` shows the path correctly
entered, the Open button enabled after one second, stage still dialog and no
callback error. The path-alias comparison was a latent fixture bug, not this
observed timeout. GTK 3's [chooser source](https://raw.githubusercontent.com/GNOME/gtk/gtk-3-24/gtk/gtkfilechooserwidget.c)
debounces location changes for 150ms; QMP had sent Return after 80ms. The fixture
now types the path, waits one second, captures it, then sends one Return and
captures again. That corrected GUI rerun remains pending, with no full toolkit
pass claimed.

Signed home-transition
[34762761530](https://github.com/Reidond/kedra/actions/runs/34762761530) passed
at `e8f32722eff2419c802660c67d77798000978db0`. Latest source
`6cb1b87d794707c6130338ad5bfe4041d50bcc1f` changes only the GUI keyboard timing
and documentation/worklog. Its workspace
[34763954122](https://github.com/Reidond/kedra/actions/runs/34763954122) and desktop
[34763954151](https://github.com/Reidond/kedra/actions/runs/34763954151) are in
progress; home-transition
[34763954121](https://github.com/Reidond/kedra/actions/runs/34763954121) is pending.
The prior signed workflow success does not establish latest-source GUI success.

Desktop [34763954151](https://github.com/Reidond/kedra/actions/runs/34763954151)
at `6cb1b87` passes both GTK 3 Wayland and Xwayland ready/dialog/selected workflows
with exact selected-file content; artifact `10319744781` records the results.
Libadwaita reaches its ready/dialog stages through GTK 4 FileChooserNative and
the Nautilus portal, then times out. The confirmed screenshot shows the first
Return navigated to the directory and selected the 28-byte sample with Open
enabled, but had not confirmed opening it. The source now adds a libadwaita-only
second Return after checking for completion/failure, with an additional
open-confirmed screenshot and unchanged strict file/content assertions. This
follows the [Nautilus 50 chooser](https://raw.githubusercontent.com/GNOME/nautilus/50.0/src/resources/ui/nautilus-file-chooser.blp).
The corrected rerun and remaining cases are pending; no full six-case pass is
claimed. Home-transition 34763954121 remains in progress.

Desktop [34765138011](https://github.com/Reidond/kedra/actions/runs/34765138011)
at `12c17e1`, artifact `10320836651`, passes GTK 3 Wayland/Xwayland and
libadwaita ready/dialog/selected workflows with exact file content. Qt 5 passes
native Wayland, Breeze, Adwaita Sans and Breeze-icon readbacks and renders its
KDEPlatformFileDialog/KFileWidget. Its chooser then times out: Ctrl+L/full
path/Return navigates to `/home/kedra-test/` but does not select the sample.
The fixture now uses KDE's Name editor with Alt+N, Ctrl+A, full
path and delayed Return. Local syntax/whitespace pass; the VM rerun is pending.
Qt 5 selection and Qt 6 cases remain incomplete;
the desktop workflow has no overall pass. Home-transition
[34765138099](https://github.com/Reidond/kedra/actions/runs/34765138099) is in progress.

Desktop [34766395233](https://github.com/Reidond/kedra/actions/runs/34766395233)
at `a99553f`, artifact `10320693521`, passes all six native GUI
ready/dialog/selected cases through QMP keyboard interaction and exact file
content: GTK 3 Wayland/Xwayland Adwaita, libadwaita through Nautilus, Qt 5/6
native Wayland Breeze with visible KDEPlatformFileDialog/KFileWidget, and
the Qt 6 Adwaita Mono 12 user override. The overall workflow nevertheless
**fails** afterward: fixture cleanup uses `rmdir` on a generated XDG_CONFIG_HOME
that contains normal Qt-written files. The local cleanup fix removes only that
rmdir; it still deletes the explicit kdeglobals override and leaves normal Qt
files on the disposable snapshot. All six GUI/override assertions remain,
Bash syntax/whitespace pass and the complete desktop rerun is pending. Home-transition
[34766395330](https://github.com/Reidond/kedra/actions/runs/34766395330) passed
at `a99553fcd4d5b27cc60352cbe9adc497e86aa587`.

Latest `2653cde` changes only desktop cleanup and docs/worklog, preserving runtime
image source; no new home workflow was triggered. Workspace
[34767484936](https://github.com/Reidond/kedra/actions/runs/34767484936) passes.
Desktop [34767484937](https://github.com/Reidond/kedra/actions/runs/34767484937)
**passes** at `2653cde3b7c9b43d4e2c10a6fabcca2ee6719378`. Artifact `10320989840`
contains 35 PNGs and passing wallpaper-source, no-video-bridge, all six
ready/dialog/selected toolkit cases with exact file content, KEDRA_TOOLKITS_PASS
and KEDRA_R07_SESSION_PASS markers. Measured versions are GTK 3.24.52,
GTK 4.22.5/libadwaita 1.9.3, Qt 5.15.18, Qt 6.11.2 and KDE platform/Breeze 6.7.5.

Independent visual review of desktop.png/settings.png nevertheless finds yellow
Noctalia selected accents and dark navy panels despite the configured Adwaita
blue/charcoal palette. Its cause is now identified in Noctalia 5.0.1's
[theme service](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/theme/theme_service.cpp):
both light/dark palette variants require terminal-color objects. The prior
Adwaita file omitted them, causing runtime fallback while the UI still displayed
Custom/Adwaita. Complete terminal colors are now implemented. A disposable native
Noctalia 5.0.1 session under nested niri renders charcoal panels/blue accents
without a fallback warning; native config validation and whitespace pass.
The VM workflow now has a read-only managed-journal fallback guard. The later
71cb8c9 Actions visual rerun passes as recorded above; the preceding 2653cde
functional GUI pass retains its historical visual limitation.

Local Rust 1.98.1 formatting, all-target Clippy with warnings denied, release
build, WSL Bash syntax and Git whitespace pass. The Windows workspace E2E
command passes with zero cases, providing no new Linux behavioral coverage.

### Removed legacy downloads

At the owner’s explicit direction, GitHub Release IDs 385117864 (r1), 385328126 (r2) and 385128562 (legacy channel), including all 31 uploaded assets, were deleted. The remote Releases list is empty. Their source Git tags remain; no source tag was removed. Historical signed-image/installation results remain in Git history and worklog, not as current download availability.

The owner confirmed on 2026-09-13 that nobody installed r1/r2. No deployed-system migration is required; earlier migration blockers assumed installations that do not exist. Fresh installations enroll directly in the signed GHCR workflow. Historical disposable-VM results remain valid only within their recorded scope.

### Verified implementation and remaining qualification

At exact f52c9e77e17eb029a6b25c5a23807ef655484795, workspace runs 34714337528 and 34714339961, direct GHCR 34714337561, desktop 34714337517, legacy signed update 34714337526 and home transition 34714337529 pass. Direct-GHCR artifact 10304408023 independently matched ZIP SHA-256 46ba4dda3f3a3ab5b099274486d3121f3a5386c8bb49fb31659bf0ca6f6f8a2d.

The direct-GHCR VM test covers actual v2 enrollment/check/stage and signed A/B/A boots, signature/scope refusals, replay/equivocation, offline failure, pending preservation, idempotence, identity-health retained rollback, persistent data, hold and resume. It uses a disposable local TLS registry and generated keys; it does not publish to production GHCR.

Production 34715490862 failed because the public build could not read an environment-scoped fingerprint variable. The subsequent scope fix retains source-key validation, independent signer-environment comparison and signer-output binding in the publisher. Production run 34717057402 at 7e923d4907e3b5caa28260983a0e7ef884a3dd87 passed its public build and was cancelled before signing for the final policy change. Queued schedule 34728220456 was also cancelled. Neither cancellation is successful signing or stable publication.

The shared bootc compatibility contract, exact image identity/material checks and local CLI/OpenSSL interoperability pass their recorded checks. Automatic production signing/publication, the no-change repeat and full local ISO construction with diskless smoke all pass. Deterministic registry races/interruption and actual OCI-platform mismatch remain unqualified. Physical/Secure Boot, an installed-system forward update using production images and key rotation are separate boundaries. Publication does not mean a workstation was staged, rebooted or installed.

The local builder completed successfully in Ubuntu WSL using Podman 4.9.3 and Skopeo 1.13.3. It produced `/var/tmp/kedra-local-iso.iM7J7gze/installer/kedra-desktop-44-acafed578d8431d2.iso`: 2,858,758,144 bytes, SHA-256 `b923a289347a878678dd49222c8d433f609eb0830a7e35f309e0ecd9d776660c`. `sha256sum -c SHA256SUMS` passed. `installer.json` binds the signed production digest and source above and records `diskless_smoke_passed: true`; the smoke check passed offline signed-payload verification and Anaconda startup without disks attached.

The fresh encrypted offline installation passed in a private Xvfb session with only the verified ISO and two generated 64 GiB QCOW2 disks attached. Serial-mapped `vda` (`KEDRA-INSTALL-ONLY`) was selected; `vdb` (`KEDRA-KEEP-DATA`) remained unselected. A generated administrative account was configured; root remained locked. The Anaconda Complete screenshot was independently reviewed. Installer ACPI shutdown exited QEMU successfully, and `qemu-img compare` confirmed the unselected sentinel disk was unchanged.

ISO-free second boot reached LUKS unlock and the niri/Noctalia desktop. The generated account and sudo worked. Exported native checks confirmed the expected signed digest/source/trust, LUKS2 on `vda3`, enforcing SELinux, read-only `/sysroot`, writable home with successful write/read/remove, required `sysroot doctor` checks passing, and no partitions or mounts on `vdb`. A separate awake-session Noctalia GUI Shut Down check passed from an ISO-free boot: the visible Shut Down tile was selected, QEMU promptly exited with code 0 without an injected ACPI event or terminal poweroff, and the sentinel comparison passed. An authenticated `sudo -n systemctl poweroff --no-block` recovery check also exited cleanly with the sentinel unchanged.

An earlier QMP `system_powerdown` sent a short ACPI power key: the guest journal records `Power key pressed short` followed by S3 suspend/resume, not a shutdown attempt. The display stayed inactive after virtual-GPU resume; that VM was deliberately stopped with QMP `quit`, a forced host stop, and subsequently recovered to the installed desktop. VM suspend/resume remains unqualified. This does not establish a Noctalia or polkit shutdown defect.

The complete ISO is also available at `C:\Users\reido\Downloads\kedra-desktop-44-acafed578d8431d2.iso`; its SHA-256 was independently verified against the value above. Only the ISO was copied to Downloads. Generated password/passphrase files were removed, and no QEMU system process remains running.

The Ubuntu WSL build initially reported a Podman cleanup warning because `netavark` was missing. Installing that helper restored cleanup; the leftover inspection container was removed. The local ISO and smoke passes above are unchanged. Global sudoers hashes matched before and after the isolated build.

The earlier signed candidate 34697167136 at b4e9f78 passed its exact fresh encrypted offline installation and ten qualification checks. That historical installation result does not qualify installation of the newly published GHCR-only image or restore deleted Release assets.

[INSTALL](INSTALL.md) describes local media construction; [UPDATES](UPDATES.md) describes direct updates and recovery. The [worklog](../../../../worklog.md) retains exact historical evidence and the next operational step.

### Follow-up qualification correction (2026-09-30)

Fresh Fedora container CI at implementation `4e0f4fc` exposed Noctalia 5.2.0,
which the exact-version home guard correctly refused. Only that runtime/output
string is now added; persisted projection identity stays 5.0.1 and unknown
versions still refuse. Native validation/export, private-field exclusion and
the real home-review cycle pass; full cached ARM `1790751520-28479` passes 13/13
in 109.92 s with two workers and complete cleanup. New-source CI is still required.

The encrypted boot defect is separate: the signed image has virtual GPU/input
modules in rootfs but omits them from initramfs. A real dracut experiment confirms
that target force_drivers for virtio_gpu/virtio_input includes both and
virtio_dma_buf. Target configuration and explicit qemu-only image-build initramfs
regeneration are implemented and undergoing actual candidate validation. Neither
that build nor the earlier successful installation is a healthy-boot pass.

### Final corrected encrypted installation (2026-09-30)

The owner entered credentials directly in the guest; none were captured. Corrected
image 9d6/ISO cb8578 passes fresh encrypted installation, visible ISO-free LUKS unlock
and greetd login. The installed system has all doctor checks passing, active Metal
rendering, enforcing SELinux, Secure Boot/lockdown integrity, LUKS2/Btrfs and no
failed services. Read-only TPM dry-run finds TPM2/PCR7/LUKS UUID correctly; no
enrollment was performed. Bootc reports signature verification and the exact signed
image, no staged or rollback state. ISO/sentinel full hashes remain unchanged.
The final graceful stop required explicit force; this limitation remains recorded.
The native development desktop is restored and visibly healthy; its expected
unsigned-development release-trust notice is separate from the installed signed
VM's full trust pass. The original broken installation is absent.
