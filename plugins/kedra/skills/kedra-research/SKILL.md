---
name: kedra-research
description: Qualify Kedra features through real end-to-end workflows and disposable VM checks.
---

# Qualification

Read docs/ARCHITECTURE.md and docs/STATUS.md, then choose the smallest real workflow covering the changed feature. Existing tests live under tests/ and .github/workflows/test-*.yml.

Use real CLI/process/Git, RPM transactions, native applications and disposable VMs. Include wrong authority/target, stale inputs, interruption and recovery where relevant. No unit/model/mock/doctests, source assertions, repository scanners or custom check runner.

OS and ISO builds run in Actions. Local CLI E2E uses generated fixtures; never install over the workstation, enroll real home, use vault content or production signing keys as fixtures.

Record source/run/attempt, exact artifacts, versions and expected/actual result. Separate pass/fail/not-run/blocked, build/boot/install/healthy and physical hardware. Link Actions evidence from docs/STATUS.md and worklog.md; do not commit raw logs, screenshots or research reports. Historical evidence is retained in Git history.

Update the relevant operational skill when a durable failure or version boundary is learned. Documentation is not test evidence; an untested integration stays unqualified.
