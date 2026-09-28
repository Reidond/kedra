# Local delivery review — 2026-09-28

Base `main` at `a3a39a4a8a1d7777f02ab68d80ad4f39abf40c87`. The changes,
including new files, remain uncommitted. A clone of HEAD does not contain them.
The working native GPU and Testcontainers workflows are delivered; the complete
28-case qualification matrix is still partial, as recorded in [test-plan](test-plan.md).

## Delivered behavior

`kedra-lab vm` prepares a private locked QEMU/HVF/Cocoa/VirGL/ANGLE runtime,
reusable native fixture disks and isolated retained instances. It exposes start,
status, exec, sync, screenshot, input, logs, stop/remove and signed installer media
commands. Both native niri and a Wayland EGL client reported ANGLE Metal on Apple
M2 Pro, with visible Cocoa presentation, Secure Boot enabled and enforcing SELinux.
Prepared native commands work without a Docker engine.

The existing Testcontainers harness now has prepared native source archiving,
validated incremental home transactions/recovery, truthful capture receipts,
partial reports, bounded cancellation/deadlines and optional two-worker execution.
It remains the sole container runner. Software container rendering is explicit.
The UTM launcher, cocoa-way/waypipe builder and TCP bridge are removed. ARM boot
workflow/scenarios/observer are renamed while retaining their boot assertions.
Signed `utm` identity, release authority and existing owner data are unchanged.

## Dependent surfaces reviewed

| Surface | Callers and verification |
|---|---|
| Native dispatcher/controller | Rust clap → uv standard-library Python → owned QEMU/QMP/SSH; lifecycle, independent instances, failed-start cleanup and captures executed |
| Runtime locks/receipt | Preparation and read-only checker bind sources, recipe, compiler downloads, ANGLE closure, firmware and bundled binaries; missing/changed artifacts refuse |
| Native disk import | Rust resolves engine/image IDs; private builder checks engine and canonical Docker/Podman rootfs/config/platform before installation; resulting disk boots |
| Source/cache/sync | Existing source resolver is reused; compiler copy is locked; archive/cache publication atomic; both transports exercise guest validator/transaction |
| Capture metadata | Native actual niri output and installed/synced source; container actual scale and explicit software status; screenshots independently viewed |
| Cancellation/deadlines | Runner, scenario engine, Docker waits, cleanup and report writes reviewed; real active cancellation and negative deadline cases executed |
| Media and ARM boot rename | Existing fixed-key media authority retained; signed ISO verifies/boots; boot-level assertion changes inspected; new-source CI remains not-run |
| Removed live transport | Command callers, session host mode, packages and mounts removed together; final complete container suite passes |

External strict report consumers are UNKNOWN; the repository's current report
writer and artifact handling were inspected. No new public production CLI format
or signing contract was changed.

## Findings fixed during actual workflows

- BuildKit rejected bare local image IDs; owned tags now resolve exact IDs.
- Shared Cargo outputs could race copying; the volume lock covers build and copy.
- Reattached captures reported scale zero; actual compositor state supplies it.
- Native disk import IDs differed between Docker/Podman despite identical content;
  canonical platform/rootfs/config now binds the import before tagging.
- Iterating live cgroup membership skipped processes; the builder snapshots it
  before migration. Compressed QCOW2 export wasted over 14 minutes; raw followed
  by sparse uncompressed conversion removed that cost.
- Native ACPI shutdown opened the shell's power menu; the disposable fixture has
  exactly one sudo poweroff command, and genuine SSH disconnects require VM exit.
- Pinned QEMU 11 spun idle HVF vCPUs. The exact reviewed stable-11.0 WFI fix
  `3b98370b55de7fff540092c1a6760726a6816625` is included with attribution/checksum.
  Quiet post-fix host CPU measured 2.9% over 10.007 seconds.
- Agent-shell background policy throttled the retained VM; normal process policy
  is restored only for its owned QEMU/TPM children. No global policy changes.
- Portal startup raced doctor; fixture readiness now starts and pings the portal.
- A 6-second eventual observation could pass a 2-second deadline. The remaining
  budget now bounds execution; late success is rejected. Fractional limits remain
  fractional, and cancellation no longer retries until the eventual deadline.
- Failed native startup could leave a TPM child; cleanup now reaps owned children,
  preserves disk/logs and leaves an existing running VM alone. Real failed-start
  experiment passed in 2.738 seconds with zero owned children remaining.
- Four SSH round trips per native capture added latency; metadata is collected
  in one request without dropping source/display evidence.

## Observed checks and performance

Formatting, workspace all-target Clippy with warnings denied, ruff, release build,
shell syntax and Git whitespace checks pass. Workspace CLI E2E ran two macOS cases;
Linux-only targets were empty. Release/OpenSSL interop and material CLI checks pass.
Five final full ARM suites passed 13/13 each, median 85.61 seconds (complete command
91.03 seconds); earlier forward/reverse and two-worker runs also passed.
The final native edit/capture loop median is 4.676 seconds, capture 1.043 seconds,
and retained startup 24.443 seconds. Every sample and maximum is in test-plan.md.
All these timings exclude one-time runtime/image construction.

Active eventually cancellation returned 130 in 0.536 seconds, retained its report
and removed execution containers while the retained lab stayed healthy. Actual
2-second eventually and 500 ms command negative probes failed as expected. Sync
validation, conflicts, GUI overrides, deletion, interrupted-write recovery and
newer guest edits were exercised; all generated source edits were restored.

## Scope and unresolved qualification

No unit/mock/doctest/source-scanner tests or replacement runner were added. Python
uses uv on the host; guest/build scripts declare their own interpreter scope.
Safe flat-workspace Rust and production privilege boundaries remain intact. No
owner VM data, real accounts, workstation raw devices, signing keys, release
workflow or global agent configuration changed. No commit/push/CI dispatch or
production install occurred.

The existing signed ISO reached Anaconda and reported its Secure Boot/signature
markers; this is not a new ISO build or completed encrypted installation. Full
GTK/Qt/Xwayland/audio/scaling/physical-key qualification, frame pacing, the private
engine-loss and remaining cancellation/corrupt-state matrix, repeated cached
container startup/filter budgets and x86/current-source CI remain open.
The native fixture uses a cached unsigned base plus working-tree payload;
container runs use a cached published ARM base plus working-tree overlay. Neither
is a newly published complete OS. Automatic receipts retain `gpu_qualified: false`
because successful startup/capture does not grant the whole manual GPU matrix.

## Documentation and handoff

Updated README/INSTALL/STATUS, the container/native/media instructions, specs and
worklog. Durable build/lifecycle/renderer findings were added to existing Kedra
skills; both first-party manifests are 0.3.3. The AI changelog records that local
knowledge update; no duplicate memory buffer or invented behavioral metrics.
Historical worklog/production results remain intact.

Evidence is ignored under `target/kedra-lab/`; specific files and run IDs are in
test-plan.md and WL-20260928-07. The default native desktop and retained container
are available for manual and agent iteration. A user-visible screenshot completes
the handoff. No claim that every planned case passed accompanies this delivery.
