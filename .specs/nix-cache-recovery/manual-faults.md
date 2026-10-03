# D5 manual qualification commands

Status: prepared procedure only; every fault below is not-run. Execute only after
parent adoption, source freeze, compiler gates and reservation of the bounded
qualification resources. Keep the retained default VM/container untouched.

## Public workflow entrypoints

Pin the final product and Cargo E2E executables as private single-link mode 0500
copies outside Cargo output. Record each executable's SHA-256 before and after
every invocation, including retry. Keep its exact compiled source/feature/toolchain
provenance. The names below denote those independently recorded copies and inputs;
none is a live workstation path or production key.

```sh
"$D5_SYSROOT" store substitute \
  --store "$D5_STORE" --bundle "$D5_BUNDLE" \
  --receipt "$D5_RECEIPT" --signature "$D5_SIGNATURE" \
  --public-key "$D5_PUBLIC_KEY" --cache-policy "$D5_POLICY" \
  --scope "$D5_SCOPE" --plan "$D5_GRAPH" --root "$D5_ROOT_NODE"

"$D5_SYSROOT" system recover --workdir "$D5_ARTIFACTS/native"
"$D5_SYSROOT" system recover --workdir "$D5_ARTIFACTS/composition"
"$D5_SYSROOT" store recover --store "$D5_STORE"
```

These new adapters exist only in the ignored shadow until adopted. The independent
consumer derives the expected recipe from `resolved.plan.specs[resolved.recipe.root]`
and binds `$D5_SCOPE` to its own catalog namespace/package before opening a store.

Generated fixture signing uses the same interoperable encodings as the E2E source:

```sh
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$D5_PRIVATE_KEY"
chmod 600 "$D5_PRIVATE_KEY"
openssl pkey -in "$D5_PRIVATE_KEY" -pubout -out "$D5_PUBLIC_KEY"
openssl dgst -sha256 -sign "$D5_PRIVATE_KEY" -out "$D5_SIGNATURE_DER" "$D5_RECEIPT"
openssl base64 -A -in "$D5_SIGNATURE_DER" -out "$D5_SIGNATURE"
```

Only newly generated disposable keys may be used. Public-key fingerprint is
SHA-256 of `openssl pkey -pubin -in KEY -outform DER` output; policy selects that
value independently. No OS production authority or unlocked credential store.

## Prepared actual-process E2E

After adoption, the existing Cargo E2E binaries contain these opt-in cases:

```sh
KEDRA_ENGINE_E2E_BINARY="$D5_SYSROOT" "$D5_ENGINE_E2E" \
  --ignored --exact cache_recovery::signed_closure_refuses_untrusted_changes_then_runs_without_producer \
  --test-threads 1

KEDRA_ENGINE_E2E_BINARY="$D5_SYSROOT" \
KEDRA_CONTEXT_FAULT_SOURCE="$D5_CONTEXT" \
KEDRA_CONTEXT_FAULT_IDENTITY="$D5_COMPOSITION_ID" "$D5_CONTEXT_E2E" \
  --ignored --exact killed_context_copy_recovers_without_removing_a_stopped_reader \
  --test-threads 1
```

The first builds, signs, rejects wrong authorities/metadata, transfers a real
ELF/library closure, removes producer state, executes, reuses and independently
rebuilds. The second observes two actual large-copy processes, verifies SIGSTOP,
kills one, recovers it while preserving the stopped survivor, resumes the survivor
and requires successful verification. It also checks a legacy foreign sentinel
and preserves the caller's original 0755 work-directory mode. Missing the copy
window fails rather than counting as coverage.

## Bounded filesystem ENOSPC

Provision a new disposable Linux controller with a dedicated empty 16..256 MiB
tmpfs mount, owned by its nonroot controller UID. No host Docker socket is needed
for the prepared source-only import case. Also provision an explicit writable
`TMPDIR` on a different device, for example a separate owned 64 MiB `/proof` mount.
The fixture source, producer, exported bundle and foreign sentinel belong there,
not on `/scratch`. The test checks device separation and creates/removes a tiny
writability probe before creating fixture payloads. A read-only controller root
must not silently fall back to an unwritable `/tmp`. Budget both mounts and the
controller processes so the bounded filesystem fills before a memory limit fires.
Pass only fixture binaries/generated
inputs; do not mount the repository, home or production credentials. The test
independently requires an exact tmpfs mountpoint and checks the size ceiling before
writing its filler. It never mounts, reformats or fills the workstation filesystem.

```sh
TMPDIR=/proof KEDRA_CACHE_ENOSPC_ROOT=/scratch \
KEDRA_ENGINE_E2E_BINARY=/input/bin/sysroot /input/bin/e2e-engine \
  --ignored --exact cache_recovery::full_bounded_store_recovers_and_retries_real_import \
  --test-threads 1
```

This case fills only the selected tmpfs, observes actual ENOSPC through public
import, removes only its filler, recovers and successfully retries while checking
the preexisting root and external sentinel. It qualifies copy/admission failure;
it does not claim a journal-publication or daemon-layer-storage window.

For journal/native metadata windows, provision an independently bounded filesystem
large enough for the real input copies. Native preparation can retain two complete
verified foundations at once (historical foundation is about 6.4 GB each); do not
use a multi-gigabyte memory-backed filesystem on the workstation. Parent must
record capacity, host free-space floor and total resource budget before starting.

After stopping the actual selected controller at an observed window, fill only
that bounded filesystem, then continue it:

```sh
dd if=/dev/zero of="$D5_BOUNDED/filler" bs=1048576
kill -CONT "$D5_PID"
```

The filler command must actually return ENOSPC. Preserve its exact path/metadata,
then remove only that filler before public recovery/retry. A failure somewhere
else is not evidence for the intended window.

| Observed boundary before filling | Required observation and public retry |
|---|---|
| Native initial transaction absent, prepared snapshots present | Initial `.next` write fails with ENOSPC; no malformed authoritative journal. Retry creates a valid transaction. |
| Native pending image exists, journal image null | Image-ID journal replacement fails; old complete record survives; retry removes only the nonce tag and rebuilds. |
| Native recorded image ID, binding absent | Binding `.next` write fails; retry verifies exact parent/material and binds the recorded ID. |
| Complete import staging, before journal publication | Actual `import.next` failure or no-replace publication interruption; `store recover` retires only the exact temporary/link state and retry admits a complete closure. |
| Valid import journal/admitted objects, before root-state publication | ENOSPC preserves prior root selection; recovery retains valid admitted objects, then retry completes. |

Use filesystem/image events or existing private experiment observations to stop
the actual process and read actual state. Do not manufacture a journal, synthesize
an I/O error or add a product fault hook. A missed window is not-run. Partial
records unknown to the protocol remain refused, never silently deleted to pass.

## Four native SIGKILL windows

Resolve a fresh native plan with the adopted v2 private-context implementation;
do not reuse the previous v1 derivation identity.

```sh
KEDRA_LAB_ARTIFACTS="$D5_ARTIFACTS" "$D5_LAB" derive-plan \
  --target qemu-arm64 --overlay none --image "composition:$D5_CONTEXT" \
  --composition-identity "$D5_COMPOSITION_ID" --native-plan "$D5_NATIVE_PLAN"

KEDRA_LAB_ARTIFACTS="$D5_ARTIFACTS" "$D5_LAB" derive \
  --target qemu-arm64 --overlay none --image "composition:$D5_CONTEXT" \
  --composition-identity "$D5_COMPOSITION_ID" --native-plan "$D5_NATIVE_PLAN" \
  --derivation-identity "$D5_DERIVATION_ID"
```

Capture the actual process using OS filesystem/image events, SIGSTOP it, verify
stopped state and read journal/binding/tag/ID before `kill -KILL "$D5_PID"`.
Repeat the same public derive command after exact snapshot recovery.

| Captured state | Required retry |
|---|---|
| Owned temporary image; journal ID null | Remove the exact unbound nonce tag and rebuild. |
| Recorded image ID; binding absent | Revalidate and reuse exact image ID. |
| Binding exists; final tag absent | Preserve binding bytes, publish final tag, retire pending state. |
| Final tag exists; journal still present | Preserve binding/image and retire only pending state. |

Record actual exit signal, source and executable hash for each capture/retry,
daemon/parent/nonce/image identity, metadata links/modes, stdout, journal transitions,
resource bounds and exact cleanup. Static replay's earlier four cases do not count
as native evidence. Neither this procedure nor tmpfs metadata failures establish
power-loss durability, Docker layer-storage exhaustion or production eligibility.

## Small metadata filesystem with disk-backed snapshots (not-run)

Use this topology for the later metadata ENOSPC cases instead of filling a large
filesystem. It is a proposed disposable-controller fixture, not installed state.
Keep the controller alive throughout child interruption, recovery and retry: its
tmpfs metadata would disappear if the controller were stopped or recreated.

| Controller path | Backing / purpose |
|---|---|
| `/work/artifacts/native` | New owned 32 MiB tmpfs; key locks, native journals, bindings and `.next` files remain siblings here. |
| `/work/artifacts/native/.sysroot-snapshots-v1` | Bind mount of one preinitialized owned disk registry; bulky snapshots and their leases stay on disk. |
| `/work/artifacts/composition` | Separate owned disk cache; preserve its actual binding and composition identity. |
| `/input/context`, `/input/bin` | Selected generated context and pinned Linux binaries, read-only. |

Initialize the disk registry using a public invocation before the nested mount.
For a known valid context, an independently selected wrong identity causes `system
verify` to create its registry and copy the bounded composition manifest, then
refuse before copying the foundation. Record the actual expected identity refusal,
then run public `system recover` and require all three result lists empty. Inspect
the resulting marker/lock ownership, modes and hashes. Do not hand-author either
file. Mount only this empty legitimate registry; no existing active lease may be
transported between host and controller device/inode namespaces.

The later controller creation can use these mount arguments with its existing
reviewed image, exact owner labels, selected nonroot UID/GID and private daemon
connection. These are arguments to a new disposable controller, never host mount
commands or additions to a retained environment:

```sh
--tmpfs "/work/artifacts/native:rw,nosuid,nodev,noexec,size=32m,mode=0700,uid=$D5_UID,gid=$D5_GID"
--mount "type=bind,src=$D5_DISK_REGISTRY,dst=/work/artifacts/native/.sysroot-snapshots-v1"
--mount "type=bind,src=$D5_DISK_COMPOSITION,dst=/work/artifacts/composition"
```

Before running a product command, verify the actual resulting mounts rather than
assuming runtime mount ordering. Record mountpoint, filesystem type/size/source,
device/inode, owner and modes with `findmnt --mountpoint` and `stat`. Native metadata
must be exactly the bounded tmpfs; its registry child must be the selected disk
mount, not tmpfs or a symlink. Require 0700 directories and single-link owned 0600
marker/lock files. Run recovery inside this same controller namespace and require
no refusals. The guard checks each named directory against its opened descriptor;
it does not require the registry's device to equal the metadata parent's device.

At an observed write boundary, a second ordinary-user process creates one unique
0600 filler in `/work/artifacts/native`, records its device/inode/link identity,
then writes until the kernel reports ENOSPC. Recheck the exact mount type and
32 MiB ceiling immediately before filling. Continue the held product process with
unchanged arguments and require its actual ENOSPC. Remove only the recorded filler
after checking its identity again; publicly recover/retry. Snapshots remain on
disk, so neither filling nor cleanup requires another multi-gigabyte allocation.
Keep any failure diagnostics on a separate bounded evidence disk, not the full
metadata mount. Never delete journals to make retry succeed.

For `roots/state.json` ENOSPC, retain the whole private Store on one disk mount
and mount only its `roots/` child on a small tmpfs. Seed it from a byte-identical
root state produced by public store commands while the store is idle/locked; verify
the seed hash and private metadata. Keep `transactions`, `objects` and `images`
together: separating those into independent bind mounts can make admission rename
fail with EXDEV, which is not the intended fault. Stop before the actual root-state
temporary write, fill only roots tmpfs, continue, and verify the old selected root
before freeing the filler and retrying. Record any surviving owned root temporary;
do not claim universal temporary cleanup from snapshot recovery.

Before controller retirement, complete public pending-state recovery and snapshot
cleanup, export the bounded receipt/evidence, preserve foreign sentinels, and
verify exact resource ownership. Kill/stop only recorded child/controller IDs.
These tmpfs tests qualify live-filesystem process failures, not power-loss durability.

## Observer readiness and held publication boundaries (not-run)

No GDB availability or ptrace capability has been observed for the selected D5
controller. Probe them only after the delivery lead assigns the runtime and grants
its readiness check: `command -v gdb`, its exact version, and an own-child syscall
catch on `/usr/bin/true`. If unavailable, propose a reviewed dependency in that
disposable fixture; do not install a host debugger or change host ptrace/security
settings. Keep the existing Docker-event/filesystem observer as the fallback.

GDB can hold an actual syscall at entry or return. Its hardware breakpoints can
stop before an instruction without modifying it, where supported. `finish` runs
the selected function until it returns. These tool capabilities are documented,
but do not establish readiness or a captured Kedra window. Sources:
[syscall catchpoints](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Set-Catchpoints.html),
[hardware breakpoints](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Set-Breaks.html),
[finish](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Continuing-and-Stepping.html).

Before starting, choose an actual owner that can reap the product process and
record its terminating signal. Prefer the existing observer's own Child/Popen
handle, with GDB attaching to that exact live inferior; qualify attach permission
separately because an own-child debugger smoke does not prove sibling attach is
allowed. If `gdb -nx -q --args` launches the inferior instead, arrange and verify
an inferior waiter within GDB, which is that inferior's parent, before detaching.
The outer shell can wait for GDB,
not automatically for GDB's detached child. GDB exit status and successful `kill`
command delivery are never substitutes for an observed inferior wait status.

Record the inferior PID, process-start identity, pinned executable hash and Linux
PID namespace, and send all signals in that same disposable controller namespace.
The owning waiter must report `WIFSIGNALED` with `WTERMSIG == SIGKILL` (or its
equivalent negative signal return code) for the exact inferior. Keep that owner
alive until the inferior is reaped. If there is no qualified owning waiter, stop
at readiness; no crash-recovery pass may be claimed from process disappearance.

Run the pinned product with its ordinary `derive` arguments. Use all-stop mode
and no startup shell. Do not change syscall
arguments, return values, registers or product memory, and do not force function
returns. First observe the actual filesystem calls; select path-specific libc
wrapper breakpoints when an exact userspace boundary is needed before converting
the debugger stop into SIGSTOP. Dynamic `rename`/`unlink` symbols and hardware
breakpoint support must be checked in the actual runtime, not assumed from source.

```text
set pagination off
set non-stop off
set startup-with-shell off
set breakpoint pending on
catch syscall openat fsync renameat renameat2 linkat unlinkat
run
```

Record the target architecture and actual ABI before interpreting arguments. On
native Linux AArch64, inspect syscall path arguments at `x1`/`x3` as applicable;
libc `rename(old,new)` uses `x0`/`x1`, and `unlink(path)` uses `x0`. Use `x/s` to
inspect selected paths. GDB's internal `$_streq` can restrict a breakpoint to a
known exact path when its Python support is available; it does not call the
inferior's `strcmp`. [Convenience functions](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Convenience-Funs.html)
document that distinction. Read `/proc/PID/fd/FD` to identify an observed fsync FD.

| Required native state | Held observation point in the actual implementation |
|---|---|
| Pending image, authoritative journal ID null | After successful fsync of a non-null `transaction.next`, before its rename onto `transaction.json`; alternatively a path-specific libc rename entry breakpoint. |
| Recorded journal ID, no binding | After the real rename returns, with actual journal readback showing the ID; hold before binding publication. |
| Binding, no final tag | After the real unlink of `binding.next` returns, leaving a single-link binding; confirm the final tag is absent. |
| Final tag, authoritative journal still present | Before the real unlink of `transaction.json`; a libc unlink entry breakpoint avoids racing an event consumer against that syscall. |

At each held point, independently read the real journal/binding/tag/image and
save its exact state. To require the literal SIGSTOP gate, set signal handling to
pass SIGSTOP, queue it, then detach. GDB documents that queueing does not itself
resume execution and detach does resume it; therefore verify the post-detach
kernel `State: T` and re-read the state before killing. If anything crossed the
requested boundary, discard the sample. Do not label a ptrace-only `t` state as
delivered SIGSTOP. After killing, require the owning waiter's observed SIGKILL
status before recovery. Rehearse both the detach-to-SIGSTOP transition and actual
inferior reaping on generated disposable operations; preserve actual wait results.
[Queueing signals](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Signaling.html)
and [detach](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Attach.html)
describe those operations.

```text
handle SIGSTOP stop print pass
queue-signal SIGSTOP
detach
```

For ENOSPC, the debugger may remain attached at the selected `.next` open/write
boundary while the separate filler process exhausts only the verified tmpfs.
Then continue the real operation and observe the kernel's actual error. No error
code/return-value injection is permitted. Rehearse the stop-to-SIGSTOP transition
on generated disposable file operations before expensive native captures, so
the final qualification depends on verified tool behavior rather than timing.
