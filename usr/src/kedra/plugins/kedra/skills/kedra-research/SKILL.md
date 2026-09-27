---
name: kedra-research
description: Qualify Kedra features through real end-to-end workflows, container scenarios and disposable VM checks.
---

# Qualification

Read usr/src/kedra/docs/ARCHITECTURE.md and usr/src/kedra/docs/STATUS.md, then choose the smallest real workflow covering the changed feature. Existing tests live under usr/src/kedra/tests/ and .github/workflows/test-*.yml.

Use real CLI/process/Git, RPM transactions, native applications, container scenarios and disposable VMs. Include wrong authority/target, stale inputs, interruption and recovery where relevant. No unit/model/mock/doctests, source assertions or repository scanners; the container harness is the one sanctioned runner (AGENTS.md, owner decision 2026-09-27).

Choose the layer by what the behavior needs:
- **Host CLI.** Cargo `e2e_*` and the Python release checks, for source, release and CLI logic.
- **Containers.** usr/src/kedra/tests/container, run through `cargo test -p kedra-container-tests --test container`, for installed-system behavior without a kernel boot: files, units, the real session, portals, keyring, home review, apps and screenshots. Add YAML scenarios for linear checks, native tests (native.rs) for control flow, and guest probes (lab/probes) for in-guest logic.
- **VMs.** For Secure Boot, SELinux enforcement, VT/greetd password login and PAM, bootc switch/update/rollback, the installer, and Xwayland/Qt keyboard-driven dialogs.

Container adaptations and limits are listed in the harness README. Examples: no SELinux labels, a shared kernel (per-UID limits apply across containers), and bootc images hard-linked to their ostree objects. A passing container scenario never qualifies boot, firmware or hardware.

Image stages for local runs are `stable`, `run-*`, `sha256:*`, `builds:*`, `ref:*`, or `build[:rev]`, with an optional working-tree overlay. Local builds are unsigned and never pushed or installed. Published OS images are built and signed only in Actions; local on-demand ISO builds consume reviewed signed images. Local CLI E2E uses generated fixtures; never install over the workstation, enroll real home, use vault content or production signing keys as fixtures.

Record source/run/attempt, exact artifacts, versions and expected/actual result. Separate pass/fail/not-run/blocked, build/boot/install/healthy and physical hardware. Link Actions evidence from usr/src/kedra/docs/STATUS.md and worklog.md; do not commit raw logs, screenshots or research reports. Historical evidence is retained in Git history.

Update the relevant operational skill when a durable failure or version boundary is learned. Documentation is not test evidence; an untested integration stays unqualified.
