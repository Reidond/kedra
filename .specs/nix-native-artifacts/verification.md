# Native artifact qualification — 2026-10-01

Scope: `codex/nix-native-artifacts`, immediate parent
`1c85ca42088e100ce84feba053d11c6b88d588f8` (draft PR31, gh-stack30).
Controller: Apple Silicon macOS, pinned Rust1.98.1; native ARM Docker29.4.0.
Only public CLI/manual processes and the existing sanctioned container harness
were used. Scoped implementation/review/qualification workers used Astra.

## Native CLI and installed workflows

| Case | Outcome | Actual evidence |
|---|---|---|
| NA-T1, closed plan/identity | pass | Compiled Rust authoring, `system compose`, producer removal, `derive-plan` and `derive`; unknown command, unsafe/conflicting units, wrong identity and unsupported managed skeleton deletion refuse. Final explicit CLI test1/1 in570.08s. |
| NA-T2, defaults/preferences | pass | Fresh account sees compiled `derived-default`; a separate real gsettings client sees persisted dconf `explicit-preference` through the systemd user bus. |
| NA-T3, units/default | pass | Actual enabled/start/result/status/journal checks, masked-start refusal and multi-user default target. |
| NA-T4, initial skeleton | pass | Fresh and subsequent accounts receive selected defaults; existing modified home and explicit preference survive. Missing inherited managed niri baseline refuses before generation. |
| NA-T5, initramfs content | pass | Image-local kernel7.2.7-200.fc44.aarch64; actual generated archive, kernel hash, complete listing and generic crypto/storage plus virtio_dma_buf/gpu/input modules checked independently. No kernel boot claim. |
| Native final image/cache | pass | Independent final readback verifies actual artifact bytes/modes/links, unchanged seven-column RPM material and exact parent RootFS layer prefix; installed preparation reuses the bound derived image. |
| Read-only Astra review | pass | Final model/harness/Docker/CLI review found no remaining concrete blocking defect after root/workdir, hardlink, systemd ancestry and parent layer fixes. This is review evidence, separate from execution. |
| NA-T6, cold daemon | pass | Final pinned workspace executable: new empty mutual-TLS daemon cold140.43s / cache40.08s, exact stdout and same binding; before/after hashes identical. Earlier attempts and artifact correction remain below. |
| NA-T7, later interruption | pass | Four actual static publication windows captured with SIGSTOP/SIGKILL; public retry rebuilds the pre-ID state and resumes exact ID for post-ID, post-binding and post-final-tag states. |
| Final standard gates | pass | Formatting/whitespace, workspace Clippy, ordinary CLI E2E, release build, uv/OpenSSL release interoperability/material and extracted guest-driver Ruff all pass. Ignored native tests were executed separately above. |

The final retained CLI fixture is
`target/nix-native-artifacts/native-e2e-final/`. It includes the verified context,
normalized native plan, separately retained expected identities and actual receipt:

- Foundation: `sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71`.
- Composition: `f90f1d973532b6a7de2073368f388ca647feb92ee4ec005ed16968f4f6c69c17`.
- Derivation: `ed0f65d2e500e09b908d24df8d9e677b041f3bb2f3b28b3864cb538eac567d72`.
- Parent image: `sha256:67c8d12bd48d73b3aa4078db6515e9fe3dbe42d8f4c92e91b69ee44c08a5d154`.
- Derived image: `sha256:7aa35dbf24f4a1e8de0e39ef3fedb468ca42adf453a85e0ea9ab173956ba6aeb`.
- Outer daemon: `3745e40d-81b2-48e8-96d2-a2c4446cf0e2`.
- Initramfs:87,366,923 bytes, SHA256 `95ad934258c7812dafcebcc75b657f2ed76101c8f0e7cd05a88e173b004fccc0`.
- Compiled schemas:77,156 bytes, SHA256 `eb64f0a433c28da8b141bddebfbc902db8faf990bed1a0f89d939f411513f989`.

Commands actually executed:

```sh
KEDRA_ENGINE_E2E_BINARY="$PWD/target/release/sysroot" \
KEDRA_NATIVE_LAB_BINARY="$PWD/target/release/kedra-lab" \
KEDRA_NATIVE_E2E_RETAIN_DIR="$PWD/target/nix-native-artifacts/native-e2e-final" \
cargo test -p sysroot --test e2e_native --locked -- --include-ignored --test-threads 1

KEDRA_LAB_IMAGE="composition:$PWD/target/nix-native-artifacts/native-e2e-final/context" \
KEDRA_LAB_COMPOSITION_IDENTITY=f90f1d973532b6a7de2073368f388ca647feb92ee4ec005ed16968f4f6c69c17 \
KEDRA_LAB_NATIVE_PLAN="$PWD/target/nix-native-artifacts/native-e2e-final/native-plan.json" \
KEDRA_LAB_DERIVATION_IDENTITY=ed0f65d2e500e09b908d24df8d9e677b041f3bb2f3b28b3864cb538eac567d72 \
KEDRA_LAB_TARGET=qemu-arm64 KEDRA_LAB_OVERLAY=none \
KEDRA_LAB_ARTIFACTS="$PWD/target/kedra-lab" \
cargo test -p kedra-container-tests --test container --release --locked -- native_artifacts --test-threads 1
```

Installed result:1/1 in8.56s; execution1790873248-91553,
`target/kedra-lab/runs/1790873248-91553/report.json` (`interrupted=false`, no
cleanup failure, no retained container). Preparation64.3s precedes the case time.
Per-case artifacts retain actual receipt/unit status/journal/default/home/module
observations, rather than source assertions.

Final standard commands pass:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot \
  --workdir target/nix-native-artifacts/release-interop
uv run usr/src/kedra/tests/cli/release-material.py \
  --workdir target/nix-native-artifacts/release-material
```

Ordinary CLI selection:11 pass/6 intentionally ignored; the engine filesystem
recovery case took147.94s with debug binaries. The new native case and the parent
context runtime workflow were executed explicitly rather than counting ignored
listings as coverage. Both legacy release CLI fixtures use real OpenSSL3.6.4
signatures/material for desktop and qemu-arm64. Extracted fixed image-side Python
driver passes `uv run ruff check` on stdin under the repository configuration;
the driver executes only with the image's interpreter. Ordinary container
`--list` readback still has14 original cases and provisions nothing; native
derivation cases are selected only by their complete explicit source/identity pair.

## Earlier failures and corrections

- First CLI run:0/1 in622.31s. Native build completed, but module validation
  incorrectly required underscore spelling where the kernel filename used a
  hyphen (`virtio-gpu.ko.xz`). Corrected equivalence while preserving exact kernel
  containment and accepted module suffixes. Its retained small metadata is in
  `target/nix-native-artifacts/native-e2e/`. Removed only that owned failed
  fixture's redundant6.4GB foundation copy after identity/owner/link verification;
  `cleanup-record.json` records this, and the context is intentionally incomplete.
- Signed foundation has exact dangling `/root -> var/roothome`; dracut attempted
  to follow it. For only that exact case, the native driver creates an empty0700
  target around generation and removes only its own still-empty directory.
  Existing data is never removed. The whole filesystem audit still enforces
  unchanged unrelated paths.
- Dracut's inherited syslog configuration warns that build-container `/dev/log`
  is unavailable. The diagnostic is preserved; successful generation and final
  content readback establish the scoped result.
- First installed case:0/1 in7.34s,
  `target/kedra-lab/runs/1790872997-89807/report.json`. Fedora lacked the fixture's
  assumed dbus-run-session. Corrected the fixture to use the existing systemd user
  bus/dbus-broker; no RPM was added and product derivation identity did not change.

## Cold independent daemon

Cached native Docker DinD image:
`sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0`,
actual inner Docker29.8.1. Each attempt has fresh nonce-labelled data/certificate
volumes and an internal network, bounded resources and empty initial images.
The generated mutual-TLS credentials remain private. A short-lived loopback relay
passes encrypted bytes to the isolated daemon; CA/SAN/client authentication and
no-client-certificate refusal are observed. There is no host credential/socket/path
mount into the consumer and no global Docker context/configuration change.

- Attempt1: new ID `8d99198b-dde1-4f61-b981-f994af9cc1bf`, empty inventory.
  Complete retained archive upload hit the old120s API request timeout (failure,
  no output image). Only the image-import call now uses600s; ordinary request
  timeouts remain120s. Scoped relay lifetime/connection bounds were corrected.
- Attempt2: new ID `bc42687a-a81a-4e15-adaa-95af2af6015b`, empty inventory.
  Halted at147.13s when the conservative24GiB extra-host-disk bound fired during
  concurrent native verification. Minimum free48,008,642,560 bytes, above30GiB
  floor. No successful replay is claimed. Exact owned cleanup removed the test
  daemon/network/data/certificates; unrelated resources were preserved.
- Attempt3: serialized after native qualification, baseline67,275,829,248 bytes
  free; new empty ID `7134dee4-cbb0-4cc8-81e4-1042389f7cc7`. Pass194.55s, exit0,
  exact expected stdout. Result image
  `sha256:6d1e830d9677a855115e41fa0629e353ce102b12379dc71597935b80e9bd2375`;
  durable owner-private single-link binding has exact context/foundation/daemon
  identity. Pending journal/tag are absent. Minimum free47,427,575,808 bytes,
  maximum observed additional usage about18.49GiB, below24GiB budget and above
 30GiB free floor. Cache repeat passes72.18s with the exact same image, binding
  and stdout; minimum free43,568,111,616 bytes. All four later interruption
  captures/retries pass within the same disk bounds. Final cleanup found the
  executable-provenance issue below; corrective final-artifact replay is active.

Private raw observations/cleanup records are under
`target/nix-native-artifacts/cold/attempt{1,2,3}/`. The retained producer-deleted
context is the parent layer's
`target/nix-context-replay/consumer-e2e-release/context`, expected identity
`173705f43be7338df42e2d1dc696f7243ff969a24e43f3b333977598b591b785`.
Expected real ELF/shared-library/config output:
`context-fixture:replay:runtime-v1:context-config-v1`.

## Actual publication interruption

The first later window passes with a real stopped process, image-tag event and
SIGKILL, without product hooks or reconstructed journal state:

- Capture43.35s: nonce pending tag resolves to an owned built image;
  `journal.image=null`, independent binding/final tag absent. Actual child exit-9.
- Retry40.11s: the public CLI removes the exact unbound nonce tag, rebuilds through
  the verified static recipe, records the image, publishes a private single-link
  binding/final tag and removes the pending journal/tag. Actual stdout matches.
  Image events observe untag/delete/create/tag; cache reuse may return the same
  content ID, so a different image ID is not required as evidence of rebuilding.
- Source/expected context/producer absence stay unchanged. Only the exact owned
  binding/final tag from the completed prior case were reset before creating the
  new real transaction. `fault1.json`, `fault1-events.jsonl`,
  `fault1-recovery.json` and `fault1-recovery-events.jsonl` retain observations.

SIGKILL prevents destructor cleanup of the private6.4GB verified snapshot.
The check captured its exact directory/file device/inode/owner/mode/link/size
metadata before killing; after child exit it removed only those four recorded
ordinary files and their private directory to retain the disk budget. Durable
journal and pending tag were preserved byte-for-byte until the public retry.
This temporary-input cleanup is explicit and separate from cache recovery;
automatic collection of force-kill leftovers is not implemented.

The post-ID/pre-binding window also passes: native kqueue observes the actual
journal inode replacement, then the consumer is stopped and killed. Capture38.80s
retains a non-null recorded image ID, exact owned nonce tag, absent binding and
absent final tag. Public retry40.13s reuses exact image
`sha256:ab9fee7a580b15031d6995427d6d57b0e1127dbc5419a5b9e5c29abec5830ad8`,
returns expected stdout and retires the journal/tag. No build output is observed.
`fault2.json` and `fault2-recovery.json` retain captured/recovered state.

Binding-before-final-tag capture43.83s passes: actual kqueue directory event stops
the consumer with a0600 single-link binding, matching recorded image/nonce tag
and absent final tag; actual SIGKILL follows. Public retry50.11s passes with exact
recorded image `sha256:4a8d5ffe8186291bef8abc9415f4d6ecd3dbd9e069d66c3bd75ceeca3e036cd6`,
expected stdout, no rebuild and retired journal/pending tag. Evidence:
`fault3.json` / `fault3-recovery.json`.
Final-tag-before-retirement also passes: image event capture38.83s stops and
kills the actual consumer with matching journal/image/binding/final tag still
present. Public retry40.10s reuses exact image
`sha256:f6d57119165ca35ebd2715c79150df3199a5786795b78451904c4245cdc587b4`,
keeps binding bytes identical, returns expected stdout and retires pending state
without a rebuild. Evidence: `fault4.json` / `fault4-recovery.json` and image events.

| Captured state | Capture / retry seconds | Actual recovery |
|---|---|---|
| Temporary image, journal ID null, no binding/final tag |43.35 /40.11| Removes unbound nonce tag and rebuilds verified recipe. |
| Journal ID recorded, no binding/final tag |38.80 /40.13| Resumes exact recorded image without building. |
| Binding present, final tag absent |43.83 /50.11| Preserves binding bytes, publishes tag and retires pending state. |
| Final tag present, journal not retired |38.83 /40.10| Preserves binding/image, retires pending state without building. |

Power-loss/ENOSPC micro-windows are not-run, rather than inferred from adjacent
successes. These actual fault cases qualify the static consumer's publication;
new native-cache interruption is a separate unexecuted surface.

## Executable provenance correction

The initial cold invocation recorded release executable SHA256
`dfbea2728df61ad80b1b78195725709fbef2d42b35c4ebfad7259f56d3ab64f5`.
During that run, the parent's final `cargo build --workspace --release --locked`
restored another cached dependency-feature variant to the same release path:
`352e75710a6d55d06523e77e713bfbe879468209e8d1d19081199db52cd87df3`.
Both retained dependency executables exist. Cargo fingerprints share rustc,
package features/profile but differ in dependency fingerprints. Product files
precede both builds and remained frozen; this is not identical-artifact proof.

The four interruption captures/retries are actual source-level evidence with an
explicit limit: their invocation path was recorded, but individual executable
hashes were not captured. No hash is assigned retrospectively to those runs.
The final strict same-executable audit correctly refused a same-artifact claim.
Cleanup ownership is independent of that executable and remains exact nonce/ID
scoped. Attempt3 cleanup passes: exact owned container/network/data/certificates
removed, generated client credentials removed, outer daemon ID unchanged and
retained default still running (`attempt3/cleanup.json`). Its maximum observed
additional usage22.604GiB remains below24GiB; minimum free40.052GiB stays above30.

Corrective attempt4 pins the final workspace executable to a private0500
single-link copy, with SHA256 checked per invocation. Fresh empty independent
daemon `b3b453ce-6a3e-4a55-9393-83c4ad791c75` passes cold replay140.43s and cache
repeat40.08s with that final executable, exact expected stdout and identical
before/after executable hashes. Exact result image:
`sha256:e36de0756c88fd475bbd3097522c2293a3178b7271ca4ed1c13d84751a641386`.
All four attempts' exact owned daemons/networks/data/certificates/relays and
generated client credentials are removed; outer ID is unchanged and retained
default is running. Final attempt's maximum additional disk23.228GiB/minimum
free39.412GiB stay within24GiB/30GiB bounds. Consolidated readback is
`target/nix-native-artifacts/cold/summary.json`, with exact per-attempt cleanup.
Native cached readback with that same pinned final artifact passes64.14s: exact
derived image7aa35dbf…, parent67c8d12b…, native identityed0f65d2… and unchanged
outer daemon. It independently validates native plan/material/parent layers and
actual generated artifacts on the cached image. JSON/stderr evidence:
`target/nix-native-artifacts/final-native-readback.{json,stderr}`. Before/after
pinned executable SHA352e7571… is identical; no Cargo build replaced its path.

## Remaining boundaries

Inherited verified dracut configuration is supported; changing that input requires
a separate versioned native boundary. No broadening of static schema1 overlays.
Initial skeleton generation does not reconcile existing home or accept ambiguous
managed baseline deletion. Native identity names selected inputs and implementation;
actual output hashes are retained separately, without claiming independently
reproduced initramfs bytes. Kernel boot, Secure Boot/SELinux, install/update,
production signing integration, x86 native generation and power-loss/ENOSPC
matrices remain unqualified by this layer. No signing, deployment, reboot or live
home apply was performed.
