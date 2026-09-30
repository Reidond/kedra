# Delivery review — 2026-09-30

PR #23 merged to main as `bafd1884a569d4890e768c72e335514d824bbf1f`.
Release 36617035503 passes all six jobs and publishes strict qemu-arm64 stable
`sha256:7795329a030d2fc2697d6b88a666ca73f8ff7938f84b245863f90ec16ffab877`.
Main ARM run 36617035132 attempt 1 retains its 90-minute TCG boot failure; exact
same-SHA attempt 2 passes. Fresh installation awaits owner unlock. The native GPU and Testcontainers
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
instances were removed. UTM CLI and UI showed no registered owner VM. Old UTM
packages return API 404 and the old signing environment is absent after verified
replacement publication.

## Dependent surfaces reviewed

| Surface | Callers and verification |
|---|---|
| Native dispatcher/controller | Rust clap → uv standard-library Python → owned QEMU/QMP/SSH; lifecycle, independent instances, failed-start cleanup and captures executed |
| Runtime locks/receipt | Preparation and read-only checker bind sources, recipe, compiler downloads, ANGLE closure, firmware and bundled binaries; missing/changed artifacts refuse |
| Native disk import | Rust resolves engine/image IDs; private builder checks engine and canonical Docker/Podman rootfs/config/platform before installation; resulting disk boots |
| Source/cache/sync | Existing source resolver is reused; compiler copy is locked; archive/cache publication atomic; both transports exercise guest validator/transaction |
| Capture metadata | Native actual niri output and installed/synced source; container actual scale and explicit software status; screenshots independently viewed |
| Cancellation/deadlines | Runner, scenario engine, Docker waits, cleanup and report writes reviewed; real active cancellation and negative deadline cases executed |
| Media and ARM boot/identity rename | Fresh qemu-arm64 authority/repositories and published stable; new signed ISO replay passes; old UTM ISO is historical and refused by current identity; exact same-SHA main ARM retry passes after retained attempt-1 failure |
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
  owned writable sentinel and refuses detach if its full hash changed. Public CLI
  hard-link, TPM-tree link, log-link and predictable pending-file checks pass against
  a real private runtime clone while preserving foreign data and the running VM.

## Observed checks and performance

Formatting, workspace all-target Clippy with warnings denied, ruff, release build,
shell syntax and Git whitespace checks pass. Workspace CLI E2E ran two macOS cases;
Linux-only targets were empty. Release/OpenSSL interop and material CLI checks pass.
At `1de0d8a`, workspace runs 36478416455/36478409648, ARM boot 36478409639,
both-architecture container run 36478409572, desktop 36478409495 and signed-home
36478409677 pass.
Five stable-tag current worktree-overlay suites pass 13/13 with wall median
65.563 seconds/max 72.787, tag `6f95044a24e38c69`, and reports `1790708814`,
`1790708873`, `1790708939`, `1790709025` and `1790709090`. The receipt lives outside
Cargo output. Earlier forward/reverse/two-worker and stale-provenance refusal
results retain their actual scopes.

Active eventually cancellation returned 130 in 0.536 seconds, retained its report
and removed execution containers while the retained lab stayed healthy. Actual
2-second eventually and 500 ms command negative probes failed as expected. Sync
validation, conflicts, GUI overrides, deletion, interrupted-write recovery and
newer guest edits were exercised; all generated source edits were restored.

All six native GTK3 Wayland/Xwayland, libadwaita, Qt5 and Qt6 chooser workflows
passed and their screenshots were viewed. Wrong client/host/token authorities,
state/runtime/symlink/foreign-PID refusals, private-engine loss, startup
interruption and a forced second signal were exercised successfully. Independent
concurrent Bitwarden and desktop executions both pass, clean up independently and
leave the retained default healthy. Current
optimized filtered desktop-session samples are 6.680, 7.714, 6.578, 7.181 and
6.256 seconds (median 6.680/max 7.714); complete command wall median is 7.038 s,
so the 15 s gate passes. Five complete public cached `up` samples are 10.125,
4.087, 4.438, 4.016 and 4.091 s (median 4.091/max 10.125), passing the 20 s gate.
They use clean cached base `6288…` without a working-tree overlay and exclude
one-time source/overlay preparation; stdout was observed without a separate raw
timing file. Post-change full two-worker run
`1790706590-5077` passed 12/13; home review failed because its image source revision
`48ebd03f…` was absent from this checkout. Cleanup did not fail. This is a recorded
suite failure for the no-overlay stale-provenance scope.
Corrected worktree-overlay run `1790706944-44808` passes 13/13 with two workers in
60.07 s. A local release build completed in 1m25s and both release CLI workflows
pass. The no-overlay refusal stays recorded for its actual stale-provenance scope.

The rebuilt 218 MiB external runtime/disk passes unchanged replay. A real interrupted
private preparation exits 143, incomplete pins exit 1 for missing ANGLE, neither
selects a runtime/receipt, and the current runtime hash matches its original state.
Five full-CLI warm starts have median 19.239 seconds/max 21.253, readiness median
16.437, five distinct boot IDs and Metal each. Captures have median 0.875/max 0.893.
Native audio passed twice with owner confirmation; physical typing, Command+Enter,
scrolling and window movement pass. Unlocked scale-1/1.5/2 screenshots were viewed
with valid geometry; resize to 1920 passes and reopening QEMU restores actual
2560×1600 scale 2.

Five visible native-Cocoa EGL/SHM presentation observations after 5-second warmups
run 30 seconds each at 81.967, 91.126, 89.334, 81.634 and 90.500 presentations/s
(median 89.334/minimum 81.634), with p95 interval 16.667 ms in every run. This
qualifies guest Virtual-1 only, not physical monitors or other hosts.

The Rust dev/test profile disables debuginfo by default and keeps
`CARGO_PROFILE_DEV_DEBUG=2` as the explicit full-debug path. A single isolated
comparison measured clean build 42.98→39.85 s, touched-source rebuild 1.39→1.15 s
and build tree 1,411,224→801,284 KiB; it does not support a broad percentage claim.

For the uncommitted follow-up, formatting, ruff and workspace all-target Clippy
pass (30.32 s). Empty/whitespace `down` selectors exit 1 while the default stays
healthy; exact stopped-owned removal passes using the rebuilt release binary.
Fresh container Settings capture
`target/kedra-lab/shots/final-container-desktop-1790748508-70906.png` was viewed
unlocked and healthy with explicit software rendering.

Container run 36613093449 failed before provisioning because Quay deleted tooling
index `1ccd18224d9b302fe62d3cefd0a9e0724c365ea8ac3bfb017c569540fe76433a`
within a day. Current `63abc19` pins Docker Official
Fedora 44 index `43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80`
and ARM child `e402cca673711fee025f9ce21c6c08b1bd25ee26b3c482119c8675fbaaedbc85`; manifests,
config/layers and native Fedora 44 execution were checked. Production bootc inputs
and signatures are unchanged. PR #23 then merged as `bafd1884`; main workspace,
both container architectures, desktop, home, direct-GHCR, signed-update and agent
workflows pass. Release 36617035503 passes all six jobs and publishes strict stable
`sha256:7795329a030d2fc2697d6b88a666ca73f8ff7938f84b245863f90ec16ffab877`.
Main ARM run 36617035132 attempt 1 retains its 90-minute PID 1 freeze before the
observer starts. Exact same-SHA attempt 2 succeeds, completing at 21:21:02 UTC with
the boot step in 5m06s. Main CI is green.

## Scope and unresolved qualification

No unit/mock/doctest/source-scanner tests or replacement runner were added. Python
uses uv on the host; guest/build scripts declare their own interpreter scope.
Safe flat-workspace Rust and production privilege boundaries remain intact. No
registered owner VM data, real accounts, workstation raw devices or global agent
configuration changed. A fresh qemu-arm64 signing authority/environment and public
repositories were configured. Replacement OS publication passes; no production
installation occurred.

The new signed qemu-arm64 ISO replay passes with exact size/hash after mounting
`str(HERE)`, the actual installer directory. The old UTM ISO is historical and correctly refused by the
current identity. The private APFS media-refusal matrix covers changed bytes/checksum,
manifest/SHA mismatch, wrong target, changed digest identity and existing output;
all refuse without starting QEMU or changing original media. Fresh encrypted
installation/ISO-free boot awaits owner unlock.
TC09's literal 30-second grace variant, new current fixture boot/hot-sync and
publication of local post-merge fixes also remain open.
Automatic receipts retain `gpu_qualified: false`; the complete manual Mac graphics,
input, audio, scaling and presentation matrix is recorded separately.

## Documentation and handoff

README/INSTALL/STATUS, container/native/media instructions, specs and worklog are
being reconciled with the final source and evidence. Durable build/lifecycle and
renderer findings are in the repository skills. Historical worklog and production
records remain intact.

The owner deleted the earlier Rust `target/` runtime/disks/reports. Their outcomes
remain in WL-20260928-07/08. Runtime/images are restored under
`~/Library/Caches/kedra/qemu`, with retained per-checkout state under
`~/.local/share/kedra/lab/<checkout-key>`. New native artifacts cover Metal, five
warm starts/captures, audio, input, scaling, presentation and refusal checks.
No claim that every planned case passed accompanies this review.
