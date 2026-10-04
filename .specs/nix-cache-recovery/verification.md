# D5 owning-source qualification

Current outcomes are in the dated completion appendices below and
`usr/src/kedra/docs/STATUS.md`. Earlier preparation and pending checkpoints are
historical; full native fault acceptance remains incomplete.

Adopted 2026-10-03 on `codex/nix-cache-recovery` above D4 draft PR36
`d591d2e0865cbaa68662ae7f50a6c398dab183dc`. The seventeen-path source delta
comes from reconciled manifest
`ed7333434da66f798a569568bc9e8bc0f4baa9cb9a8f9a2f591fec400f71c14f`.
All base/prepared hashes matched before adoption. The first current fmt check
fails on two layout-only chunks in the newer ENOSPC TMPDIR fixture; standard
Cargo formatting corrects them. No behavior or admission guard changes.

## Actual gates

- Pass: current Rust 1.98.1 workspace fmt and all-target Clippy with warnings denied.
- Pass: full ordinary CLI E2E, 13 passed and 11 ignored on macOS. Linux-only
  ENOSPC is not compiled/executed by that host result. Workspace release build
  passes in 49 seconds. Final mode-0500 host binaries are pinned in
  `target/nix-delivery/publication/d5-binaries`; sysroot SHA256 is
  `68e0e9b486ca78adacd602fec729bb68cf7285442c9b6b35b2e7547f67eadd9a`.
- Pass: repository Ruff and whitespace checks.
- Pass: real public `store substitute` rejects FIFO receipt (0.432 s, exit78),
  symlink policy (0.024 s, exit1) and hardlinked key (0.025 s, exit78), with each
  destination absent and every generated input preserved. Each process has a
  five-second outer deadline. Pinned current debug CLI hash before/after is
  `e96fbd9c0b4b7f86120227ad0ae9ef79cbd3070260abea68d3c713290b54a1cf`.
  Receipt: `target/nix-delivery/publication/d5-input-refusals/results.json`.
  This qualifies bounded metadata refusal, not valid signed cache import.
- Source review: focused independent Astra review of the immutable manifested
  packet reports no verified blocker in cache scope/key/revision/expiry/expected
  recipe, private proof before journal admission, lease lifetime or import.next
  recovery. Actual CLI/independent-consumer and synchronous build/archive callers
  were inspected. This is source evidence, not runtime qualification.

## Remaining required execution

Signed native substitution and independent-consumer cache policy/transfer, actual
stopped-reader/SIGKILL lease recovery, four native publication fault windows and
real bounded Linux ENOSPC remain not-run. The source-only preparation and compiler
checks cannot satisfy these cases. Source-input FIFO refusal above is narrower
than full archive import or native snapshot recovery.

D2 fresh media/install/A/B/A and D3/D4 native applications/lifecycle also remain
separate pending gates. Protected production signing is not exercised. See
[test plan](test-plan.md) and [manual procedure](manual-faults.md).

## Impact and review boundary

Cache policy/proof APIs are consumed by public CLI substitution and the generated
independent consumer. Bundle import rechecks authenticated exact root/hash/bytes
and expiry after staging and before the first journal/admission. Existing unsigned
explicit-hash import remains separate. Snapshot guards are consumed by verified
composition, static replay and native derivation; public `system recover` exposes
recognized abandoned cleanup. Callers finish only after synchronous Docker/context
consumers return. This inspected dependency set is a source-review floor; actual
crash/stop/ENOSPC execution remains required. Generic engine dependencies add the
existing pinned p256/base64 crates without a sysroot-core or OS authority coupling.

Additional public import refusal checks pass with the same pinned debug CLI:
FIFO bundle (0.068 s), symlink bundle (0.074 s) and directory bundle (0.083 s)
all refuse, preserve a previously admitted/verified source root and leave no
import journal. `publication/d5-input-refusals/bundle-results.json` records the
actual exits/diagnostics. The first manual add-source invocation refused an absent
store; explicit public `store init` corrected the invocation before these checks.
This is a fixture invocation correction, not a product change.

Final legacy checks pass: independent OpenSSL release interoperability and both
target release-material workflows run against the D5 source/pinned release CLI.
Receipts are `target/d5-publication-release-{interop,material}`. Two updated
external cache consumers also compile and directly reject FIFO metadata and
foreign cache scope before store access; results and executable hashes are in
`publication/d5-consumers/results.json`. The first manual scratch Cargo manifest
omitted rustix already present in the tracked generator; correcting that scratch
manifest yields both builds without changing product source. D4 binaries retain
their original hashes; these new binaries have distinct `-cache` names.

Remote D2 native failure is independently diagnosed as Quay CDN blob download
EOF while pulling the pinned amd64 builder in run37107489350/job111158768746,
before fixture keys/registry/VM. The manifest resolved; no source defect or
missing digest is established. Its concurrent ARM job remains in progress.


## Native signed workflows — 2026-10-03

**Pass:** existing `cache_recovery::signed_closure_refuses_untrusted_changes_then_runs_without_producer`
and `reuse::independent_consumers_authenticate_cache_before_store_admission`
each pass1/1, zero failed/ignored, in52.09s and67.71s. Both actual parent waits0;
guards54.309s/69.300s. Exact source is `2b52355764d0e82d26010af437a79c9f4a7ca4da`,
CLI SHA `63c2f608452c8eafe7031f5eacc3ee51c761fd831bf3dcc29b6d6a78aa9d229b`,
E2E SHA `8b8a33cbc446b2e9c88d2a8a75d222ee5a23d1d2970051b00eb521fa87d72d7d`.

Actual coverage includes FIFO and authentication refusals before admission,
signed import, producer-absent execution/reuse and forced rebuild in the signed
case; independent namespace/package/key/recipe ownership, producer-absent result43
and unrelated-project preservation. The independent case does not rebuild.
All543 frozen source files and binaries remain unchanged; temporary roots are
empty and owned host process groups/containers absent. Peak charged allocations
are10,312,855,552 and10,390,278,144 bytes, with the original capacity floor intact.
Aggregate `target/nix-delivery/d5-signed-cache/result.json` SHA256
`cad381235f4734043ba192007b0b2b60cbbd52ea337f6e61566d06c2cc8045e1`
binds guards and stdout/stderr. Both cases use the retained native ARM daemon;
no cold-daemon equivalence is claimed. Context lease interruption, native fault
windows and Linux ENOSPC remain separate unpassed gates.


## Interrupted context lease recovery — 2026-10-03

**Pass:** existing public `killed_context_copy_recovers_without_removing_a_stopped_reader`
passes1/1 in6.46s, actual parent wait0 (guard8.115s), on immutable source
`2b52355764d0e82d26010af437a79c9f4a7ca4da`, CLI63c2f608 and context E2E SHA
`41f350b6db221879533224f77794cdb2d4301887b4077408491b98c081a83541`.
Composition is91eff75d. Two actual SIGSTOP readers have copied92,340,224 and
116,260,864 bytes; the abandoned child wait records signal9. Public recovery
removes only abandoned state, preserves the stopped live reader and foreign
sentinel, and the survivor resumes successfully. Owned temporary root is empty,
process group absent, binaries unchanged; peak charged allocation12,245,504,000
bytes with the original capacity floor intact. No daemon mutation or VM check.
Aggregate `target/nix-delivery/d5-context-lease/result.json` SHA256
`0ae779bb60ada1f23e88c67d7019daba59f01dd8465b2ab92d0b4f4e56476c60`.
Native publication fault windows and Linux ENOSPC remain separate unpassed gates.


## Bounded Linux ENOSPC recovery — 2026-10-03

**Pass:** existing `cache_recovery::full_bounded_store_recovers_and_retries_real_import`
passes1/1, zero failed/ignored, in1.07s (guard3.390s). Exact source
`2b52355764d0e82d26010af437a79c9f4a7ca4da` is freshly compiled for native Linux
ARM64 with pinned Rust1.98.1, offline/locked Cargo and readonly source/vendor.
CLI SHA `265b0e92bb18b58038691265136298780760e97df75720f3f65369b39975276f`;
E2E SHA `6d733da3540af5be388105317c08c77be88913f350f51cfa5747d6353c71bfec`.

Actual64MiB tmpfs exhaustion refuses public source import, preserving an already
admitted root and foreign sentinel. Writable proof/TMPDIR resides on a different
device. Removing owned filler, public recovery and successful import retry pass.
Both compile/case host-attach and container waits are0; exact owned containers
are removed without volumes, process groups absent and pinned binaries unchanged.
Case peak charged allocation10,457,255,936 bytes respects the existing floor.
Aggregate `target/nix-delivery/d5-enospc/result.json` SHA256
`a9e1482de2e19d94bd0e4a90bd7c848de514a77c47e1d3d5b3264f560de9651f`
binds the source/toolchain/ELF handoff, phase guards, assertions and cleanup.

The linked readiness-sideeffect receipt explicitly records rustfmt/Clippy
component acquisition caused by the existing disposable-controller rustup shim.
The compiler itself predates that invocation; subsequent compilation uses direct
offline executables, with no workstation installation. This case qualifies
source-copy/admission failure only, not metadata publication, Docker-layer
exhaustion or power-loss durability. Four native interruption windows remain
pending; debugger readiness alone does not qualify them.


## Native debugger readiness — 2026-10-03

**Blocked before all four native publication windows.** On source2b and unchanged
lab53b006f6, the first Mac LLDB invocation refuses an unsupported setting
(debugger exit1, owned inferior signal9). The corrected invocation times out20s
attaching the exact stopped inferior; debugger and inferior actual waits are
signal9. Scoped logs bind the orphan debugserver to that inferior; exact SIGTERM
and absence after6.7ms settle it. Its parent wait is unavailable, not inferred
from absence. No permission denial, approval dialog or successful attach is
established; hardware-breakpoint/detach readiness was not reached and no host
permissions changed. Safe aggregate `target/nix-delivery/d5-native-faults/result.json`
SHA `92c69bad5b2e4bf22c9ef1c3cde9c044fafb2c33d9912b11d75d3cee0f516969`.

A target-only isolated GDB image/command proposal is prepared, not executed.
It uses the observed immutable Debian13.7 controller image, official arm64
GDB16.3-1 artifact identity, ordinary UID502/private PID/default seccomp with
explicit ptrace and daemon-socket authority, unchanged budget and a new exact2b
Linux lab build. Missing-tool installation requires the operator's explicit
choice under AGENTS.md; it does not justify weakening native identity or tests.


## Final dependency readiness and pending decision — 2026-10-03

Authenticated isolated APT metadata and simulation pass in9.00s, host/container
waits0:8 new packages,0 upgrades/removals,7,164,876 archive bytes and26,850,304
declared installed bytes. No DEBs downloaded or installed; dpkg status is unchanged
and the exact query container is removed. Canonical `package-plan.json` SHA
`91dcd553c600dac79361a7d246d57ba4db07116bdb6bc1b8be090f18a66c0296`
binds versions/architectures/URIs/hashes and authenticated metadata. Earlier
metadata sandbox-capability failures remain separate.

The initial install-plan review rejected `--no-upgrade` as insufficient to bind
dependency changes. Revised target-only Containerfile SHA
`a7e7d59fba9e0b38146f340f1860f152e5ad80b6e7db7f1a759d8464dc5a1b0b`
and proposal SHA `dee32ffa1f17209feb4d67baed6373fc3bb27192918a0c9cb9956b58aa94d2bf`
receive limited review CLEAR. V2 binds baseline dpkg status and all eight local
DEB hashes, uses empty repository/index/archive inputs, network none,
`--no-download --no-remove`, and exact same-state simulated Inst/Conf comparison
before installation. Artifacts are under `target/nix-delivery/d5-gdb-proposal/`;
the rejected proposal is retained. This is source review, not permission or execution.

All runtime jobs are settled. No owner decision has been received for isolated
GDB installation; AGENTS.md119–120 prohibits installing missing tools automatically.
Four native publication windows remain not-run. D2 is independently short of its
unchanged81,695,739,904-byte warm gate by9,299,288,064 host /13,870,231,552 Linux
bytes at the last observation, before fresh authority reserve. Its resource choice,
fresh authority and exact retained/new producer eligibility remain unresolved;
no VM or complete five-outcome qualification is implied.

## Approved debugger image and full acceptance checkpoint — 2026-10-03

The owner approved the exact eight-package disposable image. Acquisition passes
all hashes/sizes (7,164,876 bytes); successful offline v5 build takes7.0302s with
actual wait0. Image `sha256:44bbc303e5b199430e51614e1831524ed5bec76c95ec853a22e7648a00da66de`
contains exactly8 new packages,0 upgrades/removals. Base696/alias and all16 parent
RootFS layers remain unchanged; added logical bytes18,187,035. GDB16.3 ELF SHA
`c1d45465045b4b97ee5fe0c2c1aacb4ef976082335ede3c59c12c7196c891fe0`.
Readback container create/remove waits0; no host/controller installation.
Receipt `target/nix-delivery/d5-gdb-execution/image-proof.json` SHA
`ffb9f5be74527aff157c4990babbb641b8e4f3e526d4d4f1d1fca1b57505112e` binds actual
recipe03b0cf58/acquisitionf748f4ae, unchanged baseline, exact same-state simulated
operations and offline installation. The prior bare-ID/digest FROM lookup failures
and APT3.0.3 pre-dpkg pathname failure remain failed attempts. Named local FROM
and eight explicitly verified cache archives resolve those preparation failures.
Exact2b Linux lab compilation/readiness and four native windows remain not-run.

The earlier remaining-gate summaries were abbreviated, not acceptance waivers.
Required outstanding coverage from requirements/test-plan includes AC-C2/TC-C2
remaining authorization/content refusals, all validity boundaries and actual copy
crossing expiry; AC-C3 independent external forced rebuild and byte comparison;
AC-C5 legitimate-lease schema/member/owner/mode/inode/device/link refusals and
cleanup interrupted after directory removal before lease retirement; AC-C6 four
actual native publication windows; and AC-C7 snapshot, transaction.next,
image-ID journal, binding.next, import.next and root-state metadata ENOSPC with
public recovery/retry and preserved unrelated data. Existing64MiB source-copy
failure qualifies none of the missing metadata boundaries. All remaining outcomes
must be independently recorded; debugger installation is only a prerequisite.

## Linux debugger readiness checkpoint — 2026-10-04

Retrospective readback of the pre-interruption receipts confirms the exact2b
standard optimized Linux lab build passed in133.09s, actual host/container waits0,
using readonly source/vendor and direct offline Rust1.98.1. The pinned executable
SHA is `012112e9679dff00b6195a354a94bad8959875f404fecd423889499ba6f6ccc0`;
handoff SHA is `e490df9b82af3f6518d2291edbd91596c3e35305c5c65229b91d963fb2207ca3`.

GDB readiness passes on approved image44bbc303: the hardware main breakpoint
actually hit; detach left the same executable/start identity stopped and untraced;
the inferior's actual wait is-9 and debugger/outer/container waits are0. Exact
owned container removal and absence pass. Independent coordinator read/hash of
`target/nix-delivery/d5-gdb-execution/readiness-proof.json` confirms SHA
`74d5e8d47f3e78f3df109aad6e63a939d938ccf54cbd10fca60d09951bec26b7`.
This exercises inert public help only. Four native publication windows remain
not-run; the full AC-C2/C3/C5/C6/C7 list above is unchanged. D2 capacity,
fresh-authority and producer/source eligibility remain independent gates.

## Fresh runtime audit and planning — 2026-10-04

Coordinator read/hash confirms `d5-native-oct04/audit-plan-proof.json` SHA
`e4ef3d6c489b74f0ebd8afe5d584e6ecb4be2fea22849a2e6a25751af2954738`.
Prior containers were absent on the same daemon3745; the cause is unknown.
Retained images/volumes were untouched by that audit. Fresh host/Linux capacity
595,460,612,096 /564,071,206,912 bytes exceeds D2's storage prerequisite,
without qualifying authority/source/custody or media. Public derive-plan passes
with actual wait0 in5.69s and native identityd14a332d. At that aggregate's
checkpoint all four publication fault windows were not-run. Later attempts
require their own actual state/exit/retry evidence before changing coverage.

## First capture preserved; public retry blocked — 2026-10-04

Actual initial outcomes are preserved in capture-readiness-proof.json
(SHA `4008eb4b9652f0fdfe93c2189ef12a013b8e7c595a2fa17a8d68bda6824c6091`):
missed-main normal derive0/GDB1 produced natived14a/imagecd55; absent-unit
selection refused1/GDB1. Neither is a fault pass. Same-PID exec readiness then
passes hardware main, stopped/untraced detach and direct inferior-9/GDB0/outer0.

Third attempt captures the actual pre-ID boundary for native
`16d9843e05ab46f9a4b68149bcdb45926a0993bba683fd02db1dad41fd93f081`.
Pending imagee9dcd725/tag57348624 exists while live journal image is null
(journal SHA1beabbf4/next42b69426); binding and final tag are absent. Actual
inferior wait-9 and debugger/supervisor/tool waits0 are recorded. Read-only
settlement `target/nix-delivery/d5-native-oct04/settlement-proof.json` SHA
`e0930ff4ccbb0add279ca59c70f805da8c53f5072ac2416013c910023a80467a`
confirms no active writer/debugger, two idle owned controllers and preserved
pending snapshot/journal/image. Peak charged16,813,470,491 bytes; observed
maximum Linux sample gap0.272386s is recorded without relabeling it250ms.

Automatic review stopped the runtime task twice, including a subsequent task
limited to ordinary public same-material recovery/retry. Its generic reason was
possible cybersecurity risk; no specific action was identified. No bypass, model
switch or further debugger attempt followed. This execution-review block is not
evidence of a product security defect. Captured windows1, fully qualified C6
cases0: recovery/retry and owned pending cleanup remain not-run. Windows2–4 and
remaining AC-C2/C3/C5/C7 are unchanged. D2 storage passes current measurements,
but its source6146d96 Linux protocol/capture and fresh-authority media remain
unqualified. Preserve the exact interrupted state for permitted continuation.

Hosted D2 backend correction inherited by D5: exact34e7d17 run37200381218
passes actual backend selection and the unchanged real public store retention
step, including its cleanup gate. Owning D2d18e7da run37200380542 also passes.
Full candidate validation and artifact upload remain running/pending; this
compiler/hosted retention milestone adds no native interruption/retry coverage.
See WL-20261004-28 for exact job/step readback.

## Ordinary admission and snapshot recovery reconciliation — 2026-10-04

Nine metadata refusals now pass with actual78 and public sentinel verify0:
wrong root/runtime/prefix/platform, duplicate/self references, before-from,
after-until and exact-until. Receipt dabfddf3 retains initial fixture errors
separately. Combined receipt `d5-admission-oct04/admission-combined.json` SHA
`484be00c51ccec5d66089bee2db70a4b77e11cca375bb2d780e49b763e43d6b2`
adds genuine positive admission0 in36.709s at exact valid-from (launch and first
real staged bytes in the same second), and natural expiry after actual growing
copy bytes followed by refusal78 in34.100s. No publication/remnants; genuine jq
object/closure and source-sentinel public checks0, pins unchanged. Peak10.402GB
is below derived10.509GB; actual maximum observation gap0.370264s is retained.
All processes settled. Exact until-minus-one success and Linux ELF execution
remain unrun; no full C2/C6 claim.

Historical permitted public snapshot recovery completed0 (stdout26866828,
guardda9d9fd7), removing only the abandoned snapshot-e1d330aa and leaving
active/refused lists empty. Pending native journal/.next/image remained intact.
Native derive retry still did not run, so the captured pre-ID window remains
unqualified. Earlier automatic-review blocks are not bypassed or relabeled.

## Batched provenance, lease refusals and inherited hosted pass — 2026-10-04

Old retained external-consumer hashes were not relabeled as exact2b. New
offline/locked/jobs2 builds use direct Rust1.98.1 and immutable543-file source,
with unchanged before/after source and fixture/lock pins, both actual waits0.
Receipt e86e32c8 pins fieldkit4cc15ddb/observatorye8408c9c, mode0500/single-link;
C3 runtime and independent forced rebuild remain unqualified.

Genuine-lease ordinary recovery receipt c769e815 records eight actual78
refusals (unknown member/schema/lease mode/member mode/directory mode/inode/
symlink/hardlink), preserved source/payload/sentinel and final authentic
restore/recover0 removing the exact snapshot/lease. Peak2.605GB below3GiB,
settled118,784 bytes. Owner/device/interrupted cleanup and C6 remain unrun.

Inherited release correction passes full hosted QEMU workflow on exact D5
8c35755/run37217459773 and owning D2332f450/run37217459554. Actual candidate/
transfer/lint/disk/SecureBoot/lockdown/SELinux/exact-observer checks pass with
QEMU0 in370s/288s. Report ci-oct04/QEMU-PASS.md SHA63aa519c binds final artifacts.
This is unsigned candidate/boot qualification, not local fresh installation,
signed A/B/A, production publication or native fault/retry coverage.

## Ordinary cache, lease and storage completion — 2026-10-05

Frozen product source2b now passes the genuine external signed transfer, execution
with producer unavailable, reuse and explicit independent support/report rebuild.
All28 public workflow commands return0, byte receipts remain unchanged and final
execution returns43; all seven recorded owned containers are absent. Evidence:
`d5-admission-oct04/c3-preparation/execution/summary.json` SHA
`c24d25ecbcffcb8b00ccf02cdc17cc5fb1c322068770fa87ea42f197f8ae0abd`.
Fresh consumer compiler provenance remains the separate exact2b receipt above.

C2 adds successful admission wholly within the final valid second, incomplete
closure refusal and corrupt-existing-content refusal (small receipt SHA
`8a86d35ee446ad4de16fcd947b9b2fba3de195773aec9648f76be91228da1d82`).
Two independently built valid same-ID/same-derivation outputs then differ only
in a32-byte nonce. Signed A substitution into B specifically refuses
`bundle conflicts with stored output`, preserving full trees, roots, profile
and sentinel. Direct/profile runs return43; earlier Fieldkit43/Fedora-jq20
postchecks pass. All eleven owned containers are absent. Result SHA
`f90c24cf158c6a6ced29312bd1742cac87320834bde52bb613d2067250f678a1`.
The first fixture expected78 but observed external exit1; its failure is retained
before the corrected assertion and genuine repeated refusal/postchecks.

C5 adds actual lease/member owner503 mode0600 refusal through EACCES and actual
tmpfs directory device+inode replacement refusal. Authentic full payload hashes
and identities restore, then public recover0 removes the genuine snapshot/lease.
This does not isolate a uid-only or device-only branch. Source/sentinel/pins stay
unchanged; all ten containers and the exact new volume are removed. Result SHA
`58ee9c24526f28b3519f5be524e9027c61868e964bd39d4755979c32b3f3b84a`.

C7 root-publication ENOSPC occurs on a real full16MiB roots filesystem after a
genuine journal and complete object. Prior roots/sentinel survive; releasing
only the filler permits recover0/retry0/both public verifies0. Result SHA
`85f333696b17147348d4e1f1d877276bcc4bfa7ac2bbdef5faf7a7af25fbce2b`.
An empty root.pending remains and is discarded only with the disposable tmpfs;
product recovery cleanup of that temporary is not claimed.

One source-import journal attempt also reaches actual ENOSPC with genuine empty
import.next after validated staging, leaving prior root/object unchanged. Closing
only the filler permits public recover0 (removing the temporary), retry0 and
both verifies0; transactions end empty. Host/container waits0 and exact removal
are recorded. Result SHA
`e67c35e7d0ce0bfc14e94e5f8dbf7aa18af1d2c0c9bb4aeda2b17d4cc4a77487`.
Receipt root: `target/d5-import-journal-enospc-oct04/`.

Snapshot-copy, interrupted cleanup and native transaction/image-ID/binding
ENOSPC remain unqualified at this checkpoint. C6 still has zero completed cases;
automatic review blocked native retry, and pending native state is preserved.
None of the ordinary successes above bypasses or qualifies that boundary.

## Snapshot exhaustion and orphan lease recovery — 2026-10-05

One ordinary snapshot-copy attempt passes on an isolated3GiB tmpfs with a6GiB
memory ceiling and no container swap allowance. An anonymous filler reaches real
ENOSPC/free0; releasing exactly128MiB permits a genuine lease and206 observed
foundation-copy increases from65,536 to128,450,560 bytes. Public verify fails1
with errno28. Normal Drop already removes that partial snapshot; after closing
only the filler, public recover0 reports empty removed/active/refused lists and
the identical verification command succeeds0 on the same filesystem and size.
Final registry contains only lock/marker; source, CLI and sentinel are unchanged.
Actual host/container waits0, exact container removal and foreign-container
preservation pass. Memory peak5,928,349,696 bytes stays below6GiB, with no OOM
events. Result `target/d5-snapshot-copy-enospc-oct04/result.json` SHA
`841b82fb77f722084b70300deb1e4d537a8e63ce1e10da36b18628c6a316bc4c`.

The ordinary cleanup-failure case starts with an authentic public verification
snapshot/lease retained by a genuine unknown-member refusal78. After removing
only that fixture member, own-user ACLs allow lease read/write/locking and
snapshot removal but deny lease unlink. Public recovery actually removes the
directory, then refuses78 with the authentic lease bytes/inode/mode/owner intact.
Restoring only the fixture ACLs permits orphan recovery0 and a repeated empty
recovery0. Source/CLI/sentinel remain unchanged; ACLs are restored and the process
is absent. Peak2,607,251,456 bytes settles to57,344; parent wait0. Result
`target/nix-delivery/d5-acl-cleanup-oct05/result.json` SHA
`3684d879569b08dea1e47410973004acd3efdcbf9c674525b32882a397cc2fb2`.
This verifies cleanup failure after directory removal and orphan lease retirement;
it does not claim killed-process or power-loss behavior at that boundary.

Native transaction/image-ID/binding ENOSPC and C6 native publication retries
remain unqualified. No native interrupted state or blocked operation was used
by either ordinary workflow.
