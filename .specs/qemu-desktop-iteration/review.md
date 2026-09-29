# Delivery review — 2026-09-29

Draft PR #23 is pushed at `1de0d8a` from `codex/qemu-desktop-complete`, based on
`main` at `a3a39a4a8a1d7777f02ab68d80ad4f39abf40c87`. Exact-source CI passes,
but the replacement is not merged or published. The native GPU and Testcontainers
workflows are implemented; the complete 28-case qualification matrix remains
partial, as recorded in [test-plan](test-plan.md).

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
Source now uses the `qemu-arm64` target, repository names and a fresh authority;
the retired `utm` target is rejected. Six stopped owned old-target prototype
instances were removed. UTM CLI and UI showed no registered owner VM. Old external
UTM packages/environment remain pending verified replacement publication.

## Dependent surfaces reviewed

| Surface | Callers and verification |
|---|---|
| Native dispatcher/controller | Rust clap → uv standard-library Python → owned QEMU/QMP/SSH; lifecycle, independent instances, failed-start cleanup and captures executed |
| Runtime locks/receipt | Preparation and read-only checker bind sources, recipe, compiler downloads, ANGLE closure, firmware and bundled binaries; missing/changed artifacts refuse |
| Native disk import | Rust resolves engine/image IDs; private builder checks engine and canonical Docker/Podman rootfs/config/platform before installation; resulting disk boots |
| Source/cache/sync | Existing source resolver is reused; compiler copy is locked; archive/cache publication atomic; both transports exercise guest validator/transaction |
| Capture metadata | Native actual niri output and installed/synced source; container actual scale and explicit software status; screenshots independently viewed |
| Cancellation/deadlines | Runner, scenario engine, Docker waits, cleanup and report writes reviewed; real active cancellation and negative deadline cases executed |
| Media and ARM boot/identity rename | Fresh qemu-arm64 authority and repositories; existing historical ISO remains old-identity evidence; renamed ARM assertions and exact-source CI pass |
| Removed live transport | Command callers, session host mode, packages and mounts removed together; final complete container suite passes |

External strict report consumers are UNKNOWN; the repository's current report
writer and artifact handling were inspected. No public report format changed; the
target-specific signing contract deliberately changed from `utm` to `qemu-arm64`.

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
- Container startup inherited no software-rendering choice into the user manager,
  causing Noctalia to probe Zink before falling back. A user-manager environment.d
  setting now applies `LIBGL_ALWAYS_SOFTWARE=1`, while image construction finishes
  ldconfig, hwdb and systemd-update-done work before runtime startup.
- VM state publication now uses exclusive, symlink-safe atomic writes and validates
  ownership, modes, link counts and nested TPM state. Installer mode can attach an
  owned writable sentinel and refuses detach if its full hash changed. These new
  controller paths still require public-CLI qualification after runtime restoration.

## Observed checks and performance

Formatting, workspace all-target Clippy with warnings denied, ruff, release build,
shell syntax and Git whitespace checks pass. Workspace CLI E2E ran two macOS cases;
Linux-only targets were empty. Release/OpenSSL interop and material CLI checks pass.
At `1de0d8a`, workspace runs 36478416455/36478409648, ARM boot 36478409639,
both-architecture container run 36478409572, desktop 36478409495 and signed-home
36478409677 pass.
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

All six native GTK3 Wayland/Xwayland, libadwaita, Qt5 and Qt6 chooser workflows
passed and their screenshots were viewed. Wrong client/host/token authorities,
state/runtime/symlink/foreign-PID refusals, private-engine loss, startup
interruption and a forced second signal were exercised successfully. Current
optimized filtered desktop-session samples are 6.680, 7.714, 6.578, 7.181 and
6.256 seconds (median 6.680/max 7.714); complete command wall median is 7.038 s,
so the 15 s gate passes. Five complete public cached `up` samples are 10.125,
4.087, 4.438, 4.016 and 4.091 s (median 4.091/max 10.125), passing the 20 s gate.
They use clean cached base `6288…` without a working-tree overlay and exclude
one-time source/overlay preparation; stdout was observed without a separate raw
timing file. Post-change full two-worker run
`1790706590-5077` passed 12/13; home review failed because its image source revision
`48ebd03f…` was absent from this checkout. Cleanup did not fail. This is a recorded
suite failure pending a corrected-source rerun.

The Rust dev/test profile disables debuginfo by default and keeps
`CARGO_PROFILE_DEV_DEBUG=2` as the explicit full-debug path. A single isolated
comparison measured clean build 42.98→39.85 s, touched-source rebuild 1.39→1.15 s
and build tree 1,411,224→801,284 KiB; it does not support a broad percentage claim.

## Scope and unresolved qualification

No unit/mock/doctest/source-scanner tests or replacement runner were added. Python
uses uv on the host; guest/build scripts declare their own interpreter scope.
Safe flat-workspace Rust and production privilege boundaries remain intact. No
registered owner VM data, real accounts, workstation raw devices or global agent
configuration changed. A fresh qemu-arm64 signing authority/environment and public
namespace-only repositories were configured; no replacement OS publication or
production install occurred.

The existing signed ISO reached Anaconda and reported its Secure Boot/signature
markers; this is not a new replacement ISO build or completed encrypted installation.
Audio, scaling/resize, physical-key qualification and unlocked-window frame pacing
remain open. Native timing evidence must be regenerated after runtime restoration.
First signed replacement
publication, old external identity removal and fresh encrypted installation remain
open.
The observed native fixture used an unsigned local `qemu-arm64` candidate;
recorded container runs used a cached published ARM base plus working-tree overlay.
Neither is a newly published complete OS. Automatic receipts retain `gpu_qualified: false`
because successful startup/capture does not grant the whole manual GPU matrix.

## Documentation and handoff

README/INSTALL/STATUS, container/native/media instructions, specs and worklog are
being reconciled with the final source and evidence. Durable build/lifecycle and
renderer findings are in the repository skills. Historical worklog and production
records remain intact.

The owner deleted the Rust `target/` tree, which also removed the ignored runtime,
VM disks, screenshots and raw reports. Their recorded outcomes remain in
WL-20260928-07/08, while exact-source CI artifacts remain in Actions. Runtime and
images are being restored under `~/Library/Caches/kedra/qemu`, with retained
per-checkout state under `~/.local/share/kedra/lab/<checkout-key>`, before final
manual qualification.
No claim that every planned case passed accompanies this review.
