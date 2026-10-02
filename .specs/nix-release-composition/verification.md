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
Actual cleanup/mount/sentinel refusal is not-run.

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

Before the attempt, four exact old qualification archives were retired after
full hash, receipt, owner, single-link and inactive-consumer checks. Their
manifests and evidence were retained, together with the complete canonical
foundation and D1 context/VM/media. Actual available space rose from45GiB to69GiB;
the temporary owner-write change on sealed parent directories was restored.
Receipts are in `target/nix-delivery/d2-runtime/retired-archives*.json`.

Not-run: fresh normal ARM foundation/composition/native transfer; normal
generated-authority encrypted Anaconda install; ISO-free boot; public forward
update/rollback with persistent home/data and refusals; disabled Secure Boot
installer refusal; actual cleanup/resource evidence and new source CI.
Protected-main production signing/publication/no-change repetition remain
separate from disposable fixture qualification.

D1 [PR33](https://github.com/Reidond/kedra/pull/33) both architecture workspace
checks pass on exact head712927e in
[run36941595156](https://github.com/Reidond/kedra/actions/runs/36941595156).
Earlier whole-payload workflows on8a8796c are still running; they are not
borrowed as D2/new-source results.
