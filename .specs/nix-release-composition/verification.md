# D2 verification

Source is adopted on `codex/nix-release-composition` above D1
`712927eb0180610fd1ea3e29cccd5fdc6ac16f2b`. The twenty-path preparation was
hash/mode reconciled before adoption; later-layer modules remain inactive.
Explicit local fixture support is active implementation. No runtime production
signing/main override is introduced.

Pass: adopted Python Ruff/Bash syntax/diff checks; standard public
release-material workflows for both targets, including no-change/rank/content/
scope/architecture refusals; release-interop public trust, history and sixteen
OpenSSL signature/artifact/installer workflows. Exact local fixtures:
`target/nix-delivery/d2-release-{material,interop}`.

Pass on the adopted D2 checkout above712927e: pinned Rust1.98.1 workspace
formatting, Clippy with warnings denied, ordinary CLI E2E (11 passed, six
Docker-dependent cases explicitly ignored) and workspace release build. No Rust
source was edited during those gates. Ignored cases supply no runtime coverage.

Source review caught root-owned failed installer scratch cleanup; correction
captures/rechecks its owned root, refuses all kernel mountpoints beneath it,
uses fixed narrow privileged removal and preserves failure/reporting semantics.
The manual same-device mounted-descendant refusal is subsequently qualified below;
whole-fixture cleanup outcomes remain tied to their individual attempts.

## Controller inputs

Read-only Docker inspection verifies cached native ARM Ubuntu24.04 image
`008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`,
Rust1.98.1 image `a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546`
and Docker image `3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0`.
Actual Docker client29.8.1 binary hash is
`55bfa076d51381e0bc28b12d0a6338b74ec5e9e0ca537df04988ea21ef48e42c`.

Pinned [uv0.12.19 release](https://github.com/astral-sh/uv/releases/tag/0.12.19)
provides the ARM GNU archive/checksum. Direct release URL returns403; official
GitHub release asset587158851 is downloaded through authenticated API and matches
SHA256 `0804e9b164c64b6914182d5920c08551958a095986f10a3731056df701126436`.
Only bounded regular ARM ELF uv/uvx files are extracted under private
`target/nix-delivery/d2-runtime-inputs`; no workstation tool installation.
Its receipt retains exact origin/hash/size. New controller execution/ABI remains
not-run; library version comparisons confer no pass.

## Remaining gates

First local invocation on deee813 fails during the real Podman package resolver:
its Netavark nftables backend cannot find `nft` in the dedicated controller.
The reviewed base was pulled and pinned agent/Bitwarden inputs were prepared;
foundation composition and installation had not started. Add the required
`nftables` package to the disposable controller and rerun from a fresh owned
fixture context. This is a retained failure, not a native-composition pass.
Terminal cleanup passes: original exit1 is retained, private scratch is removed,
no registry was created and no Podman container remains. The mount guard reports
no mounted descendants; deliberate mounted-sentinel refusal remains not-run.
Pinned executable hashes are unchanged. Safe attempt evidence is in
`target/nix-delivery/d2-runtime/attempt1-evidence`.

The nftables correction in86afd605 allows actual Fedora resolver startup.
Attempt2 was deliberately stopped before composition or fixture secrets after
review identified an unowned-client timeout path: killing a Podman client alone
can leave its container alive. The exact observed resolver was explicitly removed,
then the candidate received its pending termination. Original exit143, private
scratch removal, empty Podman container inventory, unchanged binaries and the
retained default are recorded in `attempt2-evidence` under the same runtime root.
The correction records a uniquely labelled resolver and its inspected ID, removes
it with bounded commands, checks exact absence, and propagates cleanup failure.
Managed child process groups receive a graceful cancellation period before forced
termination. Actual cancellation/retry qualification of that correction is pending.

The corrected public candidate on1dc8d2e5 passes actual cancellation: an observed
running resolver7711612a is followed by SIGTERM to the real candidate process,
which exits1 with the signal15 diagnostic. Its cleanup reports removal and the
independent `podman container exists` returns1. A separate live sentinel and the
retained default remain running; all three pinned executable hashes are unchanged.
The generated sentinel is subsequently retired with bounded force removal; its
five-second graceful-stop escalation is retained in the log. This proves observed
signal cancellation, not elapsed2400-second deadline coverage. Exact evidence is
`target/nix-delivery/d2-runtime/termination-gate-*`. Fresh full attempt4 is active
against source1dc8d2e5, controller recipe86afd605 and the unchanged deee813 Rust
binary source; no composition/install/update result has been recorded yet.

Attempt4 now passes actual package resolution, exact owned-resolver removal and
fresh foundation construction (assembly260s, export122s). Public store retention
validates the complete2,591,289,344-byte archive. Independent source and RPM
readback match the deterministic273,022-byte resolved-input material. The Docker
foundation ID is `sha256:12261f43bd1c11d9ebb32b3daaa6040bda9372454523dcf18c524c5259c9cc2e`;
its separate config digest is `sha256:bb246035b4a51cc3e99e496ac0f3d0f089e6c79befec11af2a6056f1894ecf23`.
Composition and public composition verification pass; native derivation is active.
The host receipt is `target/nix-delivery/d2-runtime/attempt4-foundation.json`.
These results do not yet qualify transfer, signing, installation or update.

Native generation, independent actual source/RPM/native-receipt readback, complete
native retention and Docker-to-Podman transfer subsequently pass. Composition
identity is `b411990772cf34748be7cf2513b4bc66f9d4cceef2fb5dac5c622b0788a7a01a`;
native identity is `0f109bbba6fe28ed5d86fcf33b7d977c47c2f65f8316da852add0460c518ce8a`.
The native Docker image and destination manifest both name
`sha256:e0fd3f5ac864e1cacc66df955c894e46984fdae33e11146cd1814015c8c926f4`;
the separate config/Podman ID is
`sha256:d36316539b09061fa0c39bf11e5e3b4f1d285c9ec376f16394e73fbd594976b7`.
Source archive manifest `bbb428e0…` and destination manifest are recorded as
distinct domains in `attempt4-candidate.json` and `attempt4-native.json`.

Attempt4 then refuses the fixture registry's pinned AMD64 child image at its
architecture guard. This happens before keys, fixture context or installation;
terminal cleanup passes with the resolver removed and no registry created.
The guarded run's minimum observed host free space is40,798,572,544 bytes, above
the32GiB stop threshold. Two further exact old context archives were retired with
full hash/manifest/metadata/inactive checks and parent modes restored; observed
net free-space increase was6,379,663,360 bytes during active derivation. Their
manifests, the canonical old store and D1 context remain intact.

Review also confirms that fixture `umask077` plus plain COPY would make public
source/trust metadata0600, breaking actual ordinary-user doctor/home readers.
Explicit0644 public-file COPY modes and the correct ARM registry pin are pending
fixture corrections. A bounded resume path must independently reverify the
qualified candidate while recording its source separately from new fixture
recipe revisions; production/composer source guards must remain intact.

Those fixture corrections are implemented and reviewed: explicit0644 public COPY
modes preserve ordinary-user access while host private inputs stay private.
Registry `sha256:3ffcae348822784850e836f23449ff1d0503933524cef57ddcbdbd263eca0c52`
was selected from the official3.1.2 index and actually executed as native ARM64
registry3.1.2. The local retained-candidate path checks the independently selected
receipt SHA, committed ancestor/closed fixture-only changes, original production
recipes and executable pins, actual manifest/config/RootFS and source/RPM/native
material. Copied metadata refuses aliases before and after non-dereferencing
ownership adjustment. Review corrected raw-versus-canonical RPM hashing to match
the producer's exact contract. Static checks and final source review pass;
actual full retained admission/refusals/install/update remain pending.

Fixture `f1db145` subsequently passes actual wrong-receipt-SHA and wrong-source
refusals before observer/key generation, with private cleanup intact. Correct
retained admission passes complete image/material/recipe/binary checks and observer
removal. Built A allows UID502 to read all nine public files at0644 and parse its
generated public key. All twelve fixture variants are prepared, including the
deliberately unsigned U and wrong-key W. Both normal public installer refusal
cases pass. The valid A installer invocation then fails before a media image is
completed; its private raw log is removed by terminal cleanup, so no cause was
inferred from that first failure. Attempt6 cleanup removes private inputs, registry
and observer, with no Podman containers left.

A bounded diagnostic restores only the exact retained signed registry data with a
new disposable TLS certificate and the retained public signing authority. It does
not recover a signing key and cannot qualify updates for A's older embedded CA.
Redaction-before-capture reproduction proves valid A signature copying completes,
then the ordinary installer cannot read root-created `payload.digest`: inherited
fixture umask077 creates it0600. A fixture-only subshell umask022 around the public
installer restores its normal environment; enclosing private directories remain
0700 and credential files0600. Diagnostic media construction is pending.

The controller also required an operational lookup correction for its existing
`mkfs.ext4`/`mke2fs` (e2fsprogs1.47.2-3+b12): a root-owned alias to the existing
verified executable made it reachable under the clean PATH. No tool was installed;
the controller entrypoint PATH must be corrected in source after the frozen run.
An independently verified APFS clone preserves the D1 foundation path/bytes/mode
while sharing canonical archive blocks and reclaiming6.33GB. Exact generated
incremental/compiler caches were retired after ownership/open-file/process checks;
pinned executables and reusable dependency/release caches remain intact.

With the installer's normal umask restored, the diagnostic passes strict A
signature copying, digest readback, installed source/trust/policy validation and
builder interface/platform checks. It builds the installer environment through
dracut, then BIB correctly refuses a layer-representation change at a fixed
destination digest. The original local A manifest is
`5e1de6effe5e1016cb2cd4316ef9b934a21a61bdb677d5cb8fd639fa6071d456` with uncompressed
OCI tar layers; the fixture's Podman push converted it to registry manifest
`6ac6f5a6…` with gzip layers while retaining config
`7a3555a457a0f6024d007a0e7353f32bedefdadbf7d36fa55a7a4454985fcfb1`.
This is a producer representation failure, not a consumer-verification waiver.
The ARM fixture now uses the existing production/desktop Skopeo
`--preserve-digests` approach and requires source/published manifest equality.
Only public-metadata producer and installer invocations use a scoped umask022;
private creation and log redirection retain the enclosing077. Fixed error
classifications preserve failure status without exporting raw private output.
Both shell files, all four inline Python blocks and final source review pass.
The corrected complete fixture still requires actual execution.

Fresh attempt8 on fixture `d4d4c77` passes retained admission again. Controller
`696341b8…` resolves the existing mkfs tool through its corrected PATH without an
alias. Actual A source/registry manifest equality passes at
`433fa2fbad3d84d78f4b2a7f58e126d15cac05f4b67cf91d92c64867dc8d3e8a`, with uncompressed
layers preserved and config `f14f47d682c8e82429e688a67fd325efeceb08fcfe4655d688d94f3b56129d0c`.
All twelve variants complete and unsigned/wrong-key installer refusals pass again.
The valid media build reaches BIB staging; no complete ISO or installation has
been reported.

The storage guard pauses the exact current controller at33,135,529,984 free bytes.
Queued writes continue to31,950,131,200 bytes (29.756 GiB), briefly below the30GiB
floor. This is a recorded failure of the original guard margin. Removing an exact
stopped obsolete controller restores the floor, but the current controller stays
paused pending further verified cache/duplicate-archive reclaim and a larger,
faster guard. The pause is not an ENOSPC recovery test and does not satisfy D5.
Any public subprocess deadline crossed during the pause remains a real timeout.
Pinned executables, active shared libraries, source/evidence and owner defaults
remain protected; unrelated filesystems or broad cache pruning are excluded.

The exact obsolete `kedra-utm-podman-storage` volume is retired after a read-only
content audit and a fresh identity/reference check. Its 18 images are named
Fedora/Kedra installer build inputs or 14 unnamed layer prefixes of those images;
it holds no containers, stored volumes, secret payloads or unknown named images.
Historical installer source identifies it as payload/builder cache and keeps
ISO/scratch outputs in separate volumes. The retirement receipt records immediate
free space of 41,134,522,368 to 44,901,539,840 bytes; asynchronous reclaim continues
afterward. The remaining media-stage estimate is at least 16 GiB, so admission
requires at least 52 GiB settled free space with the revised 35 GiB / 250 ms guard
active. The earlier 48 GiB threshold would not cover that estimate above the
guard trigger. Exact audit, metadata and
retirement receipts are retained under `target/nix-delivery/d2-runtime/legacy-utm-*`.

The separate `kedra-macos-media-storage` cache is also retired after fresh
identity/no-reference checks and a read-only audit: 50 known images with matching
config hashes, all 42 unnamed entries verified as build-layer prefixes, and no
containers, volumes, secret payloads or media/VM outputs. The QEMU Podman cache
contains D1 fixture/buildroot references and is preserved. Separate output media,
VM state and retained canonical/D1/native archives remain intact.

Settled free-space samples reach 66.15, 66.24 and 66.24 GiB. The existing media
command is 3,246 seconds old with 3,954 seconds remaining on its unchanged
7,200-second deadline. A new guard is active before unpause: 52 GiB admission,
35 GiB trigger and 250 ms sampling. The exact controller resumes and the default
development container remains unchanged. Resumption supplies no ISO/install pass;
the original floor violation remains a failure. Receipts are
`macos-media-cache-{content-audit,retirement}.json`, `settled-resume-space.json`
and `disk-guard-attempt8-resumed.ready.json` under the same evidence root.

The resumed public installer completes both preliminary and labeled media passes
and exits successfully. Independently read-back `attempt8-evidence/installer-media.json`
binds A manifest `433fa2fb…` to `kedra-qemu-arm64-44-433fa2fbad3d84d7.iso`,
3,260,559,360 bytes, SHA-256
`beccfe3ab8bc8694750e8dfd817b77cf263fb159c7c55b91e790117e15ca8757`.
It records source `1dc8d2e5`, public fixture authority `3c447d74…`, no upload and
no installation/smoke claim. The independent authority/retained-candidate records
keep fixture revision `d4d4c77` separate. Public installer success is also observed
by the subsequent Secure-Boot-disabled QEMU phase passing the explicit exit guard.
The exact generated Anaconda environment is retired after builder exit and a
zero-consumer check; QEMU/default/media resources remain separate. Disabled-Secure-
Boot refusal is active; fresh encrypted installation and A/B/A remain pending.

The disabled-Secure-Boot installer refuses in 236.853 seconds, with ISO, firmware
trust and unselected sentinel unchanged; QMP quits only after the actual verifier
refusal. Receipt: `attempt8-evidence/secureboot-disabled.json`. Fresh encrypted
installation then starts. The nine exact-A public files have independently
matching hashes, including native receipt `aa76f708…`. Host OpenSSL independently
verifies the exported ECDSA signature payload against the generated public key.
Its identity is the exact producer reference `ghcr.io/reidond/kedra-qemu-arm64:A`
and manifest `433fa2fb…`; an earlier manual untagged-reference assertion failure
is retained in the signature-verification receipt and required no consumer change.

The fresh Anaconda installation under Linux ARM TCG fails at its unchanged
7,200-second phase deadline. The actual exception is `ARM fixture exceeded its
phase deadline`; neither the completion marker nor an install-result receipt
exists. The passive pidfd observer captures kernel wait statuses: QEMU 0 during
host termination cleanup, boot controller 256 (exit 1), and uv 256 (exit 1).
QEMU's cleanup exit does not establish installation success. QEMU/boot are absent;
the exact outer shell remains deliberately stopped at the pre-cleanup boundary.
The existing target is incomplete and must never be admitted as installed A.

Observed progress includes substantial target writes and later active paging;
the paging sample records 12,846 major faults and about 63 MB swap-in over 30
seconds. Its share of the delay is unproven. Controller/QEMU cgroups have no
restrictive memory limit or OOM events, so no resource setting was changed.
Bounded reversible VT inspection reveals no interactive error and returns to
the original console. No guest or helper deadline is extended.

An inactive HVF fallback preparation is under review. A fresh native attempt must
start from a new blank target using the exact already-verified ISO and original
generated trust/negative registry states. Original and new tool/context revisions
must remain distinct. The observed same-controller Bash pending-SIGTERM/SIGCONT
probe reaches existing EXIT cleanup, preserves its foreign sentinel and skips
the next command, with real parent wait status -15. The trap itself saw prior
status 0; cleanup status is therefore never used as the fixture/HVF outcome.
Actual handoff, native installation and updater outcomes remain not-run.

The first Mac transport-readiness probe refuses the draft loopback route:
ordinary UID 502 cannot bind `127.0.0.1:443` (`EACCES`, errno 13). The socket is
closed; no sudo, host DNS/CA change, helper execution or VM launch follows.
An unprivileged per-connection QEMU forwarding revision is being prepared and
must preserve the guest's exact registry/control endpoints and raw TLS identity.
The original stopped boundary and generated trust remain retained; there is no
new signing or media rebuild.

Independent manual cleanup qualification passes for the unmodified source-`1dc`
and fixture-`d4` cleanup functions. A real same-device descendant bind mount causes
exit 1 and preserves both private scratch and a foreign sentinel. After exact
unmount, cleanup removes only owned scratch with exit 0; the foreign sentinel
remains byte-identical. These are actual ordinary-UID/kernel/filesystem operations,
not a full fixture rerun. Exact whole-source/function hashes and outcomes are in
`attempt8-evidence/cleanup-descendant-{manual,current-manual}.json`.

TC-04 passes the unchanged production constructor plus public material comparison
CLI. Baseline, repeated invocation and final original readback all reproduce
`185d14f21e05da183b8b9f837cd74566d7e6495fb12a3c9a18f79caa7f9b290b` byte-for-byte.
Actual recipe, recorded immutable archive-pin and disposable executable-byte
mutations each produce changed canonical material in the expected section.
Later invocation with generated caller-side output metadata remains unchanged;
that metadata is explicitly outside constructor arguments. No production
skip/publication, compatible package rebuild or native build is claimed. Exact
decisions/hashes are in `attempt8-evidence/material-constructor-manual.json`.
The original five-artifact input remains intact; TC-03 supplementation is separate.

The bounded TC-02 subset passes 13 public composer/assembly refusals: wrong
target, committed source, input/tool/recipe/source-plan material, foundation
receipt hash/source/target/material, malformed immutable image, preexisting
output and unsupported assembly mode. Each intended reason is observed before
build/import/final-result publication, with retained inputs/tools and foreign
sentinels unchanged; the explicit preexisting-output case preserves its output.
Evidence: `attempt8-evidence/composer-early-refusals.json`.

The public material CLI accepts exact signed-A raw inputs before and after five
refusal cases: changed source bytes, resolved material, RPM-content hash,
config identity label and OCI architecture. Protected inputs remain intact.
An initial manual setup reserialized signed source JSON and correctly failed
its byte-hash binding; that failure is preserved separately, followed by the
unchanged raw-byte success. No product source changed to pass the comparison.
Evidence: `public-material-refusals.json` and
`public-material-manual-setup-failure.json` under the same directory.

The remaining native binding refusal passes in 2.279 seconds through public
retained-candidate admission. A new independently hashed candidate record changes
only the expected native-receipt hash; one stopped owned observer reads actual
unchanged receipt `aa76f708…`, is removed and independently absent, then admission
refuses with `Retained installed material changed: native-receipt.json`.
Original inputs/images/registry/boundary remain intact, with no build/import/
signing/key operation or final result. This qualifies changed expected binding,
not mutated installed-image bytes. Receipt:
`attempt8-evidence/native-receipt-binding-refusal.json`.

The reviewed four-file native-HVF bridge is adopted after the observed TCG and
privileged-port failures. It retains the original fixture resources with a bounded
handoff, admits independently observed old-code boundaries, and separately binds
original context/source revisions to a normally validated new tools context and
closed committed transition. Required reflink exports exclude the incomplete disk
when selecting fresh installation. Exact pidfd finish invokes original EXIT
cleanup without treating its prior status as the outcome.

Transport uses the retained QEMU fork `29d25d77…` and libslirp 4.9.5 semantics:
the virtual host moves to `10.0.2.254`, while per-connection forwarding preserves
guest `10.0.2.2:443/18080` through an independently hash-checked fixed system nc
connector and owned private Unix sockets. It binds no host TCP port and changes
no guest trust, signature check, verifier or helper deadline. Source review fixes
path containment, a Python name-shadowing error and rename-aware transition
enumeration before adoption. Pinned Ruff/Python/Bash syntax checks pass on the
prepared bytes; actual transport/adoption/finish/HVF qualification remains not-run.

Before the attempt, four exact old qualification archives were retired after
full hash, receipt, owner, single-link and inactive-consumer checks. Their
manifests and evidence were retained, together with the complete canonical
foundation and D1 context/VM/media. Actual available space rose from45GiB to69GiB;
the temporary owner-write change on sealed parent directories was restored.
Receipts are in `target/nix-delivery/d2-runtime/retired-archives*.json`.

Current remaining: owning-source default-media rebuild/fresh installation and
complete public forward update/rollback with persistent home/data and refusals;
exact final-fixture container suite, contribution supplementation and new source
CI. Original-ISO/external-Kickstart installation and partial A observations are
qualified only to the bounds recorded in the continuation below. Disabled-Secure-Boot installer refusal passes
as recorded above; the complete trust decision table remains to be reconciled.
Fresh ARM foundation/composition/native transfer, earlier failed-attempt cleanup,
retained-candidate admission and the observed storage incident are recorded above.
Protected-main production signing/publication/no-change repetition remain
separate from disposable fixture qualification.

D1 [PR33](https://github.com/Reidond/kedra/pull/33) both architecture workspace
checks pass on exact head712927e in
[run36941595156](https://github.com/Reidond/kedra/actions/runs/36941595156).
All six image/runtime workflows on D1 implementation head `8a8796c` also pass:
container36941403723, desktop36941403745, QEMU36941403765, home36941403758,
direct-GHCR36941403715 and signed-updates36941403740. They are D1 results and do
not qualify D2.

## Native host continuation and failed A observation

The isolated host history through `81528db582feee393991c141a84b6a193bf67947`
is now integrated by exact fast-forward after original keeper cleanup. Original
product source `1dc8d2e5`, fixture producer `d4d4c77`, held controller `fc77f0e`
and individual host revisions remain separate; none is relabeled as another.

Transport gates pass eight concurrent manifest requests, all 25 expected
manifests, unsigned/missing refusal, cancellation and 32+1 admission with zero
layer downloads. A naturally observed Darwin EPERM during child cleanup is
accepted only after actual bounded child exit; the corrected transport retires
all owned children/sockets/markers. Original failures remain retained.

Original-ISO host recovery exposes two separate installer defects. A chrooted
serial-marker write creates a regular target file, so that installation remains
failed despite observed account/receipt creation. A reviewed external Kickstart
uses the existing signed firmware/GRUB/kernel and unchanged ISO/initrd, adds a
selection token and emits completion outside chroot only through a verified
character device. Its first corrected installation passes in 400.653 seconds,
but a cold readback and subsequent A failure show default.target changed from
graphical to multi-user during installation. All other eleven native artifacts
match; exact A provides greetd's graphical-login capability, so missing-provider
causation is not established.

Host 815 additionally requests `xconfig --startxonboot`. Its new blank installation
passes in 291.553 seconds after a 34.479-second disabled-Secure-Boot refusal.
External input manifest is `bc70c0394449e8483a5b3cc0193db997602237a9fde8f7bf4c843ec50b163c50`;
original transfer `2f3ef18faebb39880b72fdd641cb72ae3c9841e2ededc48392e874145cbdd0e5`
and original ISO/trust remain unchanged. Cold read-only observation matches all
twelve native artifacts and all three initial seed hashes, modes and UID/GID;
the regular target serial path is absent. Full before/after disk hashes and
reverse mount/mapper/observer/NBD cleanup pass. Its retained pre-boot disk hash is
`940e98eb72ab59d2b456454876527aeccdbf6a45324492e980b4fbee91875d34`.

First A on that disk passes LUKS unlock, Secure Boot/lockdown, enforcing SELinux,
native artifacts and fresh-install-record checks. All 90 registry HTTP200 blob
responses match their manifest sizes, totaling 6,425,949,184 bytes. The fixture
unit nevertheless exits normally with status 120/result exit-code about 38 seconds
after start. No complete A result or failure marker reaches serial; its failed
unit is independently visible at the login screen. Controlled graceful poweroff
produces actual parent exit 1 and healthy guard exit, preserving the failed state.

Restricted read-only diagnosis finds generated enrollment stdout (3073 bytes,
SHA-256 `68175e11b8a454263cd20a8d5ee71189808cd903a8e09d364b30487e00bc25dd`)
reporting enrolled:true and exact A for booted/high-water digest. Its stderr has
the signature-verification and manifest-completion messages; fixed EIO/broken-pipe
classifiers are empty. CLI exit code and parsed enroll-a.json were not persisted.
The harness saves subprocess output, then prints before publishing that JSON;
its failure/finally paths also print before poweroff. Python documents exit 120
for cleanup/standard-stream flushing errors ([Python documentation](https://docs.python.org/3/library/sys.html#sys.exit)).
This supports a fixture-output failure, without proving an errno or a product
failure. The enrolled failed disk cannot be reused as fresh A. All selected
readonly cleanup and final original/clone/pre-boot hashes pass; no raw home,
stdout/stderr or journal content is exported.

Resource evidence distinguishes the earlier guard-floor breach, old ps-based
guard timeouts and later qualified native guards. Source 815 installation uses
an exact process/owner/inode guard sampled every 250 ms: minimum host free
71,424,757,760 bytes, maximum allocation 10,136,387,584 bytes under a
12,204,314,624-byte cap, maximum sample gap 0.282 seconds, healthy through exit.
A uses the original 67 GiB admission on both filesystems and a +24 GiB target
allocation cap. A delayed free-space recovery occurred without another cleanup
or settings change; no cause is inferred. Only the three enumerated superseded
b4 disks were removed after new cold success and full hash/ownership/no-consumer
checks, with measured settled host gain about 10.09 GB and all other files kept.

Original keeper finish now passes exact pidfd TERM/CONT ownership, actual original
parent wait 143, adopted keeper wait 1 and public abort-launcher wait 0 with ABORT
acknowledgment. Original EXIT cleanup reports cleanup_failed=false, original
private-root removal and registry/observer/process absence; its prior status 0
is not a qualification outcome. Bounded replay custody separately retains public
registry/media/native/store material plus private generated TLS/disk-unlock
copies; no image-signing private key is copied. Controller-local fixture CA,
registry configuration and hosts entry were separately inventoried. The subsequent
bounded reset removes exactly two inode/hash-bound public files and the unique
hosts suffix, retaining the same hosts inode and exact 175-byte prefix hash.
Native/foundation images and the running default remain unchanged; no workstation
configuration, cache, image or volume is modified.

Safe continuation evidence is under
`target/nix-delivery/d2-runtime/attempt8-evidence/`, especially
`hvf-graphical-install-source-guard.json`, `hvf-graphical-cold-observation.json`,
`hvf-graphical-A-diagnosis-enrollment-classification.json`,
`hvf-graphical-A-diagnosis-host-final-integrity.json`,
`hvf-actual-parent-waits.json` and `hvf-original-cleanup-readback.json`.

Next owning-source qualification must execute the corrected default-media
constructor, selected embedded Kickstart, cold native/home checks, complete A/B/A
and final exact-image container suite. External-input recovery is not evidence
that newly rebuilt default media passed. Reviewed source corrections keep the
product helper, signature policy, relay and deadlines unchanged; actual new-source
runtime results will be recorded separately.

The integrated correction uses a fixture-only journald `TTYPath=/dev/ttyAMA0`
drop-in, without changing the product's serial-then-graphical console arguments
or enabling global forwarding. The discarded proposal appended those arguments
in reverse order, but bootc 1.16.13 deduplicates without reordering; review caught
that before any runtime. A fixed journal identifier and strictly anchored
installed-phase parser account for the documented console prefix. Media
completion remains an exact raw line.

The new explicit HVF branch prepares normal firmware/blank target/sentinel state
without launching QEMU, writes a private hash-bound preparation receipt and
requires its independent hash for fresh install handoff. It does not manufacture
a TCG timeout or accept a warmed target. Full input/source/context hashes are
rechecked around reflink export; legacy timeout/boundary adoption and ordinary
TCG execution remain separate. `macos-transfer.py` is pinned through the committed
fixture revision and transfer helper hash, while run-arm64/boot-arm64 and the
marker emitter are material recipe inputs. Independent combined source review,
Ruff and syntax/diff checks pass; these are not runtime preparation/route passes.

Pre-run admission inspection of `668d3f8` identifies a closed-list compatibility
gap before any observer/key operation: the original resume gate predates the
three host helpers, changed public-media Containerfile and four development-only
skill/plugin metadata files. The correction names only those exact paths. Each
new runtime/media file still must equal its selected committed bytes; all original
production recipe, executable, source/image/RootFS/native/RPM comparisons remain
unchanged. Live retained material lists twenty-four production recipe paths and
does not include the installer media Containerfile. Source inspection/Ruff/syntax
passes are not retained-admission runtime success; the next frozen invocation
must establish that independently.

## Owning-source media attempt 9 and controlled cleanup

Fixture `cad6e7a811433924d596cd00dd2b6e93ed225db8` actually passes the
closed retained-candidate admission against original producer `1dc8d2e5` and
candidate receipt `20826ea359490373b7d737dfdd3d9edf9437673242d5bcf2d3ed74b67e06e4f9`.
The native manifest/config/77 RootFS IDs, original production recipes and three
pinned executables remain unchanged. Twelve fresh variants are generated. A's
local and registry manifest bytes match at
`7c3c9210157a22e6655f733495d8ad5b7c3840c6470dccdd4ad605aae1f8dcb4`,
config `e69de7a0b0bf1154344502bb7bfeebdced4a26c74b3f80335cf0d72c2b210a1a`;
unsigned U and wrong-key W public-installer refusals pass. Public media completes
its preliminary pipeline and enters the labelled Skopeo stage, but no final ISO
or successful public media receipt is produced.

The 35 GiB/250 ms guard pauses only the owned controller. Settled free space is
35,438,272,512 bytes, so the 30 GiB hard floor is preserved on this attempt.
Hash/identity/no-consumer checked retirement of the obsolete failed 815 disk pair
and preliminary unpublished ISO retains the pristine cold disk/firmware and all
failure evidence. Seventy-seven identical immutable native-base blobs are shared
by reflink only after reviewed source/destination hash and no-writer checks. Full
old-backup and new-registry tree hashes remain unchanged; measured settled gains
are recorded rather than inferred from logical sizes. Variant/config/authority
files are not changed.

A proposed stage checkpoint is never productively activated: the live guard
fails closed before the unpause preflight, leaving the controller frozen. The
historical failure records only AssertionError, so its exact predicate is unknown.
A separately bounded 33.78-second read-only reproduction fails the conservative
one-second freshness predicate, with an age interval of approximately
0.615–1.069 seconds. This proves that reproduction's freshness uncertainty, not
a product defect or a unique clock/transport cause. Diagnostic observers exit
and are removed. No bound, source, validation or deadline is relaxed. The original
7,200-second public-media deadline expires while the controller is held.

Before cleanup, explicit UID502/mode0700 replay custody retains 42 selected
recipe/context/case/generated-TLS/disk-unlock files (17,141,348 bytes), plus the
477-file registry tree (6,426,488,299 bytes) with unchanged SHA-256
`100ea5d84c6cc0629fc6a9938656fd38a398de7fa27150944c55aff3e85ed540`.
No image-signing private key is copied. Exact installer PID/start/argv/UID and
caught/unblocked SIGINT are reverified through pidfd; pending SIGINT is observed
while frozen, then cleanup-only unpause enters the unchanged subprocess/finally
and original EXIT trap. Actual original fixture parent exit is 130; the separate
receipt observer exits 1 at its unchanged deadline. `cleanup_failed=false`,
private input removal and registry/observer removal are actual cleanup results.
Final readback finds no bound source consumers, private root/descendant mounts,
inner Podman containers, NBD or scoped mappers, and rechecks all custody hashes.
The controller is idle/running/unpaused; source freeze is released. Observed final
free space is 56,944,463,872 controller bytes and 60,863,606,784 host bytes.

Safe receipts are in `target/nix-delivery/d2-runtime/attempt9-evidence/`:
`admission.json`, `retained-candidate-verification.json`, `fixture-variants.json`,
`installer-refusal-U.json`, `installer-refusal-W.json`,
`capacity-stop-settled.json`, `registry-reflink-complete.json`,
`checkpoint-failure-trace.jsonl`, `replay-custody-before-cleanup.json`,
`cleanup-pending-sigint.json`, `cleanup-unpause.json`, `cleanup.json`,
`cleanup-final-readback.json` and `actual-parent-waits-and-final-state.json`.
These qualify failed-attempt containment and cleanup, not media/install success.
Fresh default-media replay/admission, HVF preparation/install, cold twelve-artifact/
three-seed checks, complete A/B/A, exact-A full harness and TC-03 remain pending.

Post-cleanup readback finds one separately identifiable anonymous osbuild cache
volume from the failed builder: `cde4d20248ff351ed9fb7c8ee8e850f2234e4c631f793262189b0ff38f2479e9`,
device 41/inode 401639655, created 2026-10-02T09:45:39.074267699Z. It has zero
container/config/descriptor/mount references, no unreadable processes, empty
completed objects and no source-cache files; only incomplete stage/tmp remains.
Following explicit bounded authorization and immediate revalidation, exact Podman
volume removal returns 0 and independent existence/path checks confirm absence.
Three settled samples show 70,902,149,120 controller bytes free, an observed gain
of 14,024,830,976 bytes. Host free space is 75,752,673,280 bytes, an observed gain
of 14,966,546,432 bytes. No prune or other volume removal occurs. The difference
between logical stage allocation and measured filesystem gains is preserved.
Receipts: `orphan-builder-cache-readonly.json`, `orphan-builder-cache-retirement.json`
and `orphan-cache-retirement-host-after.json` in the same attempt evidence folder.
This is capacity recovery, not fresh media admission or execution.

The next whole-media plan requires 78 GiB free on both filesystems: the unchanged
35 GiB pause threshold plus a conservative 43 GiB gross build envelope, without
credit for anticipated reflinks or deletion timing. Exact read-only investigation
finds only 1,785,671,680 uniquely allocated bytes in the thirteen-record obsolete
Anaconda image lineage; no image is removed. Four separately authorized completed
test artifacts are retired instead: the dedicated D1 base/overlay disks and both
APFS-clone aliases of the old `9d6eb030` PR31/D1 foundation archive. Current D2
uses the independently verified `1dc8d2e5`/`12261f43`/`e0fd3f5a`/`185d14f2`
candidate tuple in separate custody. No current image, default/recovery input,
815 pristine cold disk or replay custody is retired.

Full hashes/identities and zero selected references precede removal. After the
first alias is unlinked, the survivor has the same clone ID, clone count one and
6,411,501,568 private bytes. Initial readiness misses its sealed 0555 parent,
so the next unlink fails EACCES without another mutation. A bounded amendment
allows only that held UID502 directory (device 16777230/inode 126358839) to change
0555→0755 for the exact unlink, restoring 0555 in finally. A further pre-mutation
comparison incorrectly compares Python tuples to JSON lists; a reviewed standard
JSON round-trip fixes representation while preserving every field/value check.
Both failed executions remain evidence. Corrected continuation exits 0; all four
paths are absent, the same parent is 0555, all 58 adjacent entries and current
custody/default/recovery snapshots remain unchanged. Metadata/reports/signatures
are retained, but the old archives and D1 runnable disks are explicitly retired.

Three actual settled readings report 89,620,234,240 host bytes and 71,323,308,032
controller bytes. Thus retirement succeeds while whole-media admission fails:
Mac free-space recovery does not establish controller logical free capacity.
No productive retry starts. Next inspection concerns the actual deficient Linux
filesystem, not more Mac output deletion. Evidence: `d1-four-path-retirement-proposal.json`
(SHA `94ca9c1a608fcc55ae776a4a78df0e5e203dcbb696b803756de277f517aa9537`),
`d1-retirement-permission-stop-readback.json`, and
`d1-four-path-retirement-continuation2.jsonl` / `-final.json` in attempt9 evidence.
The fresh standalone fixture checkout is actual clean detached `cad6e7a`; its new
lifecycle orchestration remains preparation pending independent review and actual
resource admission. Producer, fixture and final owning-source identities remain
separate; no historical failure is relabeled.

Later bounded filesystem readback changes the capacity result without another
deletion or settings change. `/work` and `/var/lib/containers/storage` are separate
named volumes on the same Btrfs `/dev/vdb1`, device 41; `/repo` is read-only
virtiofs/mac. Three settled samples report 84,208,209,920 bytes on both Linux
paths and host minimum 89,623,035,904 bytes, exceeding 83,751,862,272 bytes (78 GiB)
on both filesystems. Independent requests are bracketed solely by host monotonic
timestamps, lasting 0.153–0.164 seconds and spaced three seconds apart. The
earlier immediate failed admission is preserved, and no precise accounting cause
is inferred. `capacity-filesystem-relationship.json` and
`d1-retirement-settled-capacity-readback.json` record this later pass. No additional
retirement or staged-clock alternative is needed on this evidence. Fresh lifecycle
review and immediate 78 GiB/healthy-original-guard revalidation remain necessary
before productive public-media replay; no execution is inferred from admission.

## Fresh retained-signed-A replay — public-installer failure

The new target-only lifecycle uses a real clean detached cad6 checkout and fresh
runner/evidence/private roots. It preserves the original signed variants, cases,
TLS and generated recipe/unlock bytes; no signing key is copied or created.
Unchanged fixture.py creates the context and resume-arm64.py readmits the original
1dc native producer. The public installer and fresh-HVF helpers remain unchanged.
Independent review covers original cleanup, single-use U/W/A admissions, actual
host/remote ownership, independent child/mount absence before settlement, and
identity-verified pause if guard health fails. No stage-clock checkpoint is used.

First actual fresh owner exits 2 before registry/media: ordinary cmp cannot read
the retained root-owned 0600 controller CA. Candidate observation and cleanup
pass, with actual Docker-client parent wait 2, private root removed, registry not
created, retained observer removed and remote absence independently checked twice.
The two fixed CA/config comparisons are changed only to privileged read-only cmp;
no permissions, policy or public code changes. Earlier failure remains recorded.

Reviewed wrapper manifest
`3da297dbe29f8a4bee72b092b64706a67a7581132f56e7c6bcaefd03455e7511`
and host supervisor
`538208106df7a47387310b7b5fd0d9f0da355259e9eb0176598cbcc10733770b`
then launch fresh run `ad57f07f5459d594`. Actual original host client PID is 11242;
remote Bash PID/start are 123432/24575103 and boot identity is
`efc8aa1d-8345-48e4-b03f-13a4dae388eb`. Context SHA is
`0fa74cd4aa332ab2181230a3d3b6cc2b35a7f4ce9535892998269d1b8d26cbaa`.
Unsigned U and wrong-key W each exit 1 with the exact expected signature refusal.
A receives its one-use admission at Unix 1790946570.763 after host-monotonic
capacity requests, independently advancing guard sequences and immediate local
recheck: Linux 84,166,709,248 bytes, host 89,557,524,480 bytes. Registry ID is
`6dcf5ac271d6863e090d01df5206216c7399f3185a30437e8ea806a338777b92`.
These records prove actual public-installer invocation, not successful construction.
No ISO, install, cold, A/B/A or full-suite pass is implied. Host events are preserved in
`target/nix-delivery/d2-runtime/signed-replay-ad57f07f5459d594/supervisor.jsonl`;
original deadlines, 35 GiB guard/30 GiB floor and source/input pins remain intact.

Actual A invocation exits 1 after 4.339 seconds. The original Docker-exec parent
11242 waits 1; original cleanup reports cleanup_failed=false, registry and retained
observer removed, private root absent. Two independent remote-absence proofs have
empty live/unreadable lists and no Podman containers remain. No replay result or
media/HVF pass exists. The fixed classifier records `public_installer_failed`
and 9,000 private log bytes, but original cleanup removes that sole constructor
log. Retained Podman events return no records for the exact interval, and the
controller has no sudo journal/auth log. These facts do not identify the cause.

The next narrowly reviewed change is diagnostic retention only: bounded original
constructor log bytes in a new owner-private 0700 directory/0600 files outside
the original cleanup root, ignored and local-only. Public evidence receives only
size/hash/role metadata; no raw logs or production/signing keys. Original cleanup,
constructor, policy and deadlines remain unchanged. A fresh controlled diagnostic
invocation still requires source/custody/guard and both-filesystem 78 GiB checks;
the removed historical log is not reconstructed and the failure is not relabeled.

The reviewed diagnostic capture is executed in fresh run `1710f8ceb4110185` with
wrapper manifest `a1ae12ac7b1ae9bc2e349ca18e2ba521a94be6dd7d6b1600e3d49969ec69d9f2`
and supervisor `543c801ab77012336bfce5009da91045e90ad7dd30b1d70cf207a6c7b2318315`.
U/W refuse; A and actual parent 33748 exit 1. Original cleanup and two independent
remote-absence proofs pass with no remaining Podman containers. Capture succeeds:
complete 9,000 bytes, SHA `e678553a944550a36d0dfc0fdd31c312bddee0f21fede9ce016ee348fbceff7c`,
0600 file in a separate 0700 local-only custody directory. Raw bytes are not public
evidence or committed output.

Safe parsing identifies this new run's error at Containerfile STEP1/FROM: the
unchanged `--pull=always` path requests the signed payload's selected base
`quay.io/fedora/fedora-bootc@sha256:9ac02a78e406f2a31659ce2d58d46e2c6548c073ddfcc6efede1a9f0a01e443b`,
and Podman reports `manifest unknown` while reading it. A separate anonymous
upstream `skopeo inspect --raw` exits 2 with that fixed error. The retained local
raw manifest is 19,946 bytes and hashes exactly to 9ac; its 10,780-byte config
hashes to `6953ae88dbf79fa6c7440bb438a6a7e2280a5b6c07c770df9d1050ddd576baf7`
with linux/arm64 and 65 rootfs diff IDs. Its 65 gzip layer descriptors total
977,770,374 bytes. Original base-resolution evidence records the official :44
index digest `62e0fe047be7b9c00abab3911fc84f8a22b2b2e097a6ca76119ebe395a6da3bf`;
the unchanged resolver validates the remote selected raw digest/platform before
returning it. No historical HTTP body is reconstructed and no particular registry
GC/retention cause is inferred. The earlier ad57 failure remains unknown.

The public constructor already accepts explicit `--base-image`, selecting only
the Anaconda environment while keeping signed PAYLOAD_IMAGE/admission/native
checks intact and recording the actual installer base in installer.json. A
currently retrievable official Fedora 44 ARM base and complete combined allocation
budget are being reviewed for this existing option. No input substitution, mirror,
pull-policy change or relabeling of the old native tuple occurs. Safe diagnosis:
`target/nix-delivery/d2-runtime/signed-replay-1710f8ceb4110185/base-manifest-readonly-diagnosis.json`.

## Exact-base acquisition and conditional warm admission

Official :44 index `5ab8ee5189bb0f3870279723f5343474df002d8d6f46564702b3ad3ba7382d98`
selects linux/arm64 manifest `ad037f87320981be7e3c13b293a809b3613a5b59205d5a80a35fe79620606c12`,
config `4611a0c6c85b15375669b3f80e48469eda6efe73d1ac32bcefd14e2ca117a75f`.
Raw hashes/sizes and Fedora 44.20261002.0/bootc/kernel7.2.8 labels pass. Twenty-one
changed blobs pass both compressed digest and decompressed diff-ID checks through
bounded streaming; forty-four descriptors match retained material. Full base TAR
is 2,056,122,368 bytes, changed TAR 781,060,608 and full compressed input 977,780,668.
No image is imported during that read-only qualification.

The cold 78 GiB constructor gate then prevents v5 `cb887b3d35bc4afe` before any
owner/context/registry. Two exact completed isolated compiler-output directories
are retired after ownership/reference checks, preserving sources, evidence and
all runtime binaries. Actual private allocation is 384,933,888 bytes, not the
larger nominal du total. Its immediate failed and later marginal capacity samples
remain recorded. Bounded controller/D1/attempt8 checks find no useful exclusive
image allocation and cause no further deletion. Actual two-run preparation costs
are approximately 23 MB; the earlier approximately 109 MB global-free delta is
not attributable setup proof and is explicitly withdrawn as a fixed requirement.

A separate 52 GiB acquisition phase uses the original guard with a conservative
8,311,250,876-byte envelope. In run `5368460882343cfb`, the first exact pull passes
37.076 seconds, repeated normal pull 2.206 seconds, and actual FROM-only build
with literal `--pull=always --no-cache` passes 2.432 seconds; all actual wrapper
waits are 0. No ONBUILD or RUN/COPY code executes. The stage helper subsequently
exits 1 on an invalid read-only Skopeo ID-reference spelling. Corrected
`containers-storage:@4611…` yields the exact config; a final read-only helper exits
0 and proves the full local cache. Successful pulls/build are not repeated and
the original failure is not rewritten.

Linked completion SHA `437a88ab479aad54ad55fac5be1366bf8109e115dbaf56aed328f23e02d8d0bd`
binds original command receipts, probe IID/RootFS, corrected config and final
verification in `target/nix-delivery/d2-runtime/base-stage-5368460882343cfb/`.
An earlier linkage field mistakenly named Path.exists values as PID absence;
that file is preserved and fresh absence confirms the corrected interpretation.
The final proof matches raw manifest/config, all 65 layer IDs/ancestry and 130
backing entries, with stable metadata. It explicitly does not rehash every
resident payload file. Repeated-pull controller net free decrease is zero and
the probe returns the same base image/RootFS. Observed allocated layer bytes
2,172,284,928 are nonexclusive and are not extra credit.

Independent review accepts only the originally budgeted 2,056,122,368 base bytes
as already present. Conditional warm admission is therefore 81,695,739,904 bytes
on both filesystems; all other Anaconda/new-stage/output/transient/headroom
allowances remain, as do the original 35 GiB trigger/30 GiB floor. Source-reviewed
packet `7e3836b17569a886cf74ad973b05fb091857b8a63950d267c568539c1a13c4ff`
requires fresh complete cache proof at prelaunch and each U/W/A call, with shared
host/controller validation. Missing/drifted proof refuses; the no-proof API keeps
cold admission. No constructor is admitted merely by source review: live capacity,
input, TLS and guard gates still apply. No new ISO, fresh installation or full
update flow passes at this checkpoint.

## Owning-source publication checkpoint — 2026-10-03

Inspected D2 head `0b8bd1bd5e17127ed9c77db4ed645f8fb16ecb0b` is
above exact D1 `712927e`, with no tracked source modifications. Refreshed pinned
Rust 1.98.1 formatting and all-target Clippy pass; ordinary public CLI E2E passes
11 cases with six resource-dependent cases ignored. The real interrupted
collection workflow passes, including SIGKILL/recovery and preserved live roots.
Repository-wide Ruff, changed shell syntax and whitespace checks pass.
Release-material passes both enabled targets and unknown-target, signature, rank,
no-change, content, scope and architecture refusals in
`target/d2-publication-release-material`. Workspace release build passes (77 seconds). Independent OpenSSL interoperability
passes all sixteen signatures, target trust, historical/fresh channel ordering,
tamper, artifact and installer workflows in `target/d2-publication-release-interop`.

Publication is a reviewable draft boundary, not acceptance of remaining runtime
gates. Fresh corrected media/install, complete A/B/A, exact-image full harness,
contribution execution and protected production signing/publication remain
unpassed. Retained TLS fails the unchanged 36,000-second validity admission.
Runtime audit `target/nix-delivery/d2-runtime/resume-20261003/audit.json` also
records insufficient warm capacity and stale Podman boot-runtime state after
controller restart. A fresh fixture and actual resource admission are required;
no source, trust, timeout or storage guard is relaxed to publish the draft.

## Hosted failure diagnosis and bounded public metadata — 2026-10-03

At PR34 head `582049b`, run37107489350 native job111158768746 fails on Quay
CDN blob unexpected EOF while pulling the pinned amd64 builder. The manifest
resolved, and no fixture keys/registry/VM started. Only that failed job is retried
without a source change. Native ARM job111158768679 separately fails in the
candidate's foundation subprocess. Artifact11268807271 omits the referenced
11-foundation.log and nested logs; its cause is unknown, with cleanup passing
before registry or installation. It is not classified as the same CDN issue.

The owning D2 fix adds optional `--diagnostic-output` to compose.py. On failure,
it emits a new bounded JSON record with operation, first failing command or last
observed step/tool/exit, allowlisted exception type/category, numeric errno,
source basename/line, bounded log hashes/sizes/completeness and four fixed observed
log markers. No messages, argv, private paths or raw logs are published. Candidate
foundation/composer calls select fixed metadata filenames in existing public
evidence and additionally retain outer invocation exit/log fingerprints. Existing
outputs refuse replacement; reporting failure cannot mask the original error.

Initial source review catches successful finally cleanup overwriting the failed
command's record; the revised run wrapper preserves its first propagating failure
before cleanup. Candidate defaults to the signed fixture evidence folder; the direct QEMU
caller explicitly selects its existing qemu-arm64-evidence folder. Reporting
checks the destination only on failure, so unavailable diagnostics cannot become
a new admission failure. Current Ruff/Bash/YAML/whitespace pass. Actual public host platform
refusal, existing-output preservation and unavailable-destination failure pass
(`publication/d2-diagnostic-cli/results.json`); native child-step coverage and
final review remain pending. These checks do not retroactively identify the lost
ARM failure or qualify media/install/update.

Final v3 source review is clear after first-failure preservation and both caller
evidence routes. The actual ordinary-user native ARM public composer check passes
in 0.307 seconds: a new non-Git repository produces real `source-head`/`git`
exit128; the public composer retains exit1 and emits only sanitized metadata.
Only the first command's two log files exist, the store is absent, and no Docker
command is reached. Source, resolved inputs and all three protected ELF hashes
match before/after. Actual diagnostic-copy allocation is 33,378,064 bytes.
Safe receipt `publication/d2-native-diagnostic/receipt.json` has SHA
`c3712c8e084a4090aa713ed3fa009a660ceb1040e5bfe4ea108f2b1d9ab2f193`;
public diagnostic SHA is
`60112a1cc89ab1891a8e0f53fda30cfa3305868b096a521ed3b5b8868ff048ba`.
Private raw logs are retained only in the owned controller directory. This
qualifies actual failing child metadata; cleanup-clobber preservation is source
reviewed, not a newly injected Docker-copy cleanup fault. Both target material
checks, Ruff, Bash/YAML syntax, help and public host refusal checks pass.


## Retained-volume and installer-base fixture options — 2026-10-03

The reviewed two-file patch adds explicit local-retained volume name/CreatedAt,
read-only backing identity and consumer checks before create, and exact stopped
container/mount/owner verification against the private before record prior to
start. `:nocopy` prevents initial registry payload copy-up. Default fresh-volume
behavior and existing EXIT cleanup are unchanged. An explicit immutable official
Fedora installer base reaches all U/W/A calls through their common installer
argument array; this changes only the installer environment, not signed payload.

Both limited Astra reviews clear patch SHA
`98bf72490249aee521d3f60451cd78508b7a8f10a8794586bcd822aa335ea09e`.
Ruff/Bash syntax/diff/help and seven real host CLI refusal invocations pass.
Actual ordinary UID502/Linux ARM64 Podman5.4.2 proof passes with compatible
fixture-only commit `f6209a68142b5c8c8130ada4de2ff6ab4b2202b9`, original producer1dc,
retained candidate20826ea3 and three unchanged ELF inputs. Public pre/post
inspection preserves exact backing identity; stopped `:nocopy` creation preserves
sentinel bytes/mode/inode. Wrong CreatedAt and unexpected stopped consumer each
refuse exit1. Host actual wait0; exact new container/volume are removed and
protected working/backup volume metadata unchanged. Aggregate
`target/nix-delivery/d2-volume-proof/evidence/result.json` SHA256
`88e20a725d1efb28ab8b096b150f36a11cdfb90d5a76ef272b2578049df5ebfd`.

The registry was never started, and no TLS/media/signing/VM operation occurred.
This proves payload preservation under serialized controller ownership; stopped
container/volume metadata allocation still occurs. Current469 production recipe
changes correctly prohibit old1dc replay; f6209 differs from cad6 only in the two
already allowlisted fixture files. No recipe gate is widened. Full replay still
requires fresh authority and unchanged capacity/deadline admission; actual
installer-base media execution and signed A/B/A remain unpassed.

## Local admission seam — 2026-10-04 (pending runtime qualification)

Fresh runtime storage observation now exceeds the previous warm gate: host
595,460,612,096 and Linux564,071,206,912 bytes (`d5-native-oct04/audit-plan-proof.json`,
SHA `e4ef3d6c489b74f0ebd8afe5d584e6ecb4be2fea22849a2e6a25751af2954738`).
Old containers were absent at resumption, cause unknown; retained images/volumes
were not changed by that audit. Fresh TLS/source/custody and media/VM outcomes
remain unqualified.

The small local fixture patch adds a fixed supervised pre-registry/per-U/W/A
nonce handshake, immediate local cold capacity/TLS checks, and bounded private
failed-installer custody. It deliberately takes zero base-cache credit and
requires83,751,862,272 bytes on both sides; the existing overall deadline,
35GiB trigger/30GiB floor/250ms monitoring remain external supervisor contracts.
No production recipe or context schema changes. Retained source1dc must still
pass every original recipe/binary check from a compatible fixture-only descendant.

Host checks: pass — Bash syntax, pinned Ruff check, public help, missing-context
admission refusal(exit2), missing-supervisor wrapper refusal(exit1), conflicting
operation refusal(exit2). These check the actual public argument boundary only;
Linux success/stale-nonce/EOF/capacity/TLS/capture/cleanup execution remains not-run.
Revised patchd4537cef received scoped Astra CLEAR after the initial review found
and the source owner repaired a directory-open identity race. The reviewer matched
fixture SHA8a596379 and wrapper41756481. Actual runtime qualification remains
required; this source review does not pass those cases.

## Hosted archive-retention diagnosis — 2026-10-04

Exact7932622 QEMU run37197340534 built the foundation, then public
`sysroot store add-image` refused78. Uploaded foundation-failure.json SHA
`1bd36569309eac81d284218b95a8feb13a8714aee5191bede86831f87d2cc113`
binds98-byte stderr SHA
`9a037aaab5f16844baabca5cb22420e7abdeffd127471d0dcd346b901f3248bc`,
which exactly matches the fixed public root-identity/multiple-roots error.
This is not a CDN failure. Root-count versus selected-root mismatch remains
undetermined; runner inventory Docker28.0.4 does not establish its live backend.

A reviewed preflight in the existing QEMU workflow now builds a tiny native
scratch image containing one literal public file, then executes real public
store init/add-image before full candidate construction. It records only
allowlisted live backend/version, selected ID, bounded archive root/manifest/
config digests, actual exit and bounded log hashes/fixed error marker. Archive
metadata reads never extract files or publish contents. Exact-image unpin and
private-store GC handle sealed directories; owned tag/work cleanup exits are
recorded separately while preserving the original failure. No engine/archive
validation, schema or producer recipe is changed.

Pass: YAML parse, Bash syntax, Ruff on the exact uv heredoc, public CLI help and
limited revised Astra review (workflowe10dc64c/patchb2d434a3). Hosted execution
is not-run at this source checkpoint; no backend or compatibility correction
is justified until the actual preflight distinguishes the failure.

The first hosted tiny preflight failed before metadata/add-image on D2
37199328971 and D5 37199328839. Both artifacts contain only cleanup receipt
SHA `f72edcac78a64b30a589a4fad28ddfb0b210db6c1f3785bfb713dbaaab682db1`,
original exit1; zero cleanup codes did not distinguish skipped operations.
No backend/root/config conclusion follows. Source review found the generated
Containerfile lacked an explicit Docker --file selection. Revised workflow
7b103ed8 adds that selection and bounded stage diagnostics before cleanup, with
attempted/skipped cleanup flags. YAML/Bash/Ruff and scoped revised review pass;
corrected hosted execution remains pending. The lost original build log is not
reconstructed or presented as observed Docker stderr.
