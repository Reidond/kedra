---
name: kedra-github-actions
description: Maintain Kedra signed-container CI, midnight package checks and native VM workflows.
---

# Actions

Read usr/src/kedra/docs/UPDATES.md, usr/src/kedra/docs/RELEASES.md and usr/src/kedra/docs/ARCHITECTURE.md. OS image builds run in Actions. ISO construction is explicit/local through usr/src/kedra/installer/build-local.py and never uploads. Do not create GitHub Releases, machine bundles or ISO/checksum assets.

The 00:00 UTC trigger reconciles the reviewed official Fedora 44 base and complete installed RPM closure. Do not let cached DNF layers claim freshness. Required repository, signature or solver failure is an error. Changed inputs produce a candidate; identical inputs do nothing, with no checkpoint renewal.

Build jobs have public trust. Automatic isolated signing executes no checkout/candidate/repository code while production keys exist. The main-only environment has no human approval gate. Sign and verify exact OCI digest/repository, then advance GHCR stable only after current-source and ordering checks. Pin Actions/tools and minimize credentials.

Per-target releases (implemented 2026-09-25; release 36617035503 passes both targets on 2026-09-29):
- release.yml keeps one non-cancelling `release-44` group. It calls reusable `release-target.yml` independently for desktop (`ubuntu-24.04`) and qemu-arm64 (`ubuntu-24.04-arm`).
- Each target uses its own `kedra-<target>-signing` environment, secrets, builds repository and artifacts.
- The callers use `secrets: inherit`, and only the signer job declares the environment. Observed 2026-09-25 (release run 36190624411 failed closed with empty signing secrets; probe run 36191986665): a called job that declares `environment:` sees that environment's secrets as empty unless the caller inherits secrets, despite the reusable-workflow docs. The repository has no repository-level secrets, so inheriting exposes nothing else.
- check.yml adds a native `rust-aarch64` leg next to the required `rust` leg.
- Hosted arm64 runners expose no `/dev/kvm`. test-qemu-arm64.yml therefore boots its disposable disk under TCG with AAVMF Secure Boot firmware and Microsoft-enrolled vars. Main run 36617035132 attempt 2 at `bafd1884` powers off with every security/bootc marker in 300 s.
- The four x86 VM workflows boot `OVMF_CODE_4M.secboot.fd` with a copied `OVMF_VARS_4M.ms.fd`, and require the guest's `KEDRA_SECUREBOOT_PASS`. test-signed-update adds a snakeoil-keys refusal case.
- SMM under KVM on hosted runners is unverified until those runs.

check.yml uses standard Cargo tools and actual CLI/OpenSSL workflows. test-container.yml (added 2026-09-27; main run 36617035048 passes both architectures on 2026-09-29) builds each target's candidate natively with the harness's full local build (`KEDRA_LAB_IMAGE=build`, docker/BuildKit, KEDRA_LOCAL_BUILDER inputs). It then runs every container scenario and uploads report.json, junit.xml and screenshots. VM workflows keep boot-level coverage:
- test-desktop: tuigreet login, PAM keyring, doctor gate, Xwayland/Qt choosers.
- test-qemu-arm64: the bootc contract and the TCG Secure Boot boot.
- Signed update, direct GHCR, home transition and RPM refresh.
Image-content and session checks moved from these workflows into container scenarios. No unit/model/mock/doctests or repository scanners.

Observed 2026-09-29, QEMU 8.2.2 / Fedora systemd 259.9 / kernel 7.2.7:
run 36617035132 attempt 1 froze PID 1 at guest 113.669 s, before the observer
started, then consumed the full 90-minute bound. The serial line
`systemd[1]: Freezing execution.` is a fatal boot outcome, not readiness.
The same SHA, firmware and package closure pass in attempt 2; retain both
artifacts rather than rewriting the first failure. `tests/vm/qemu-arm64/boot.sh`
now detects that exact fatal line every 2 s, preserves it and terminates the
owned timeout/QEMU process before failing. There is no automatic retry and all
Secure Boot, SELinux, bootc, unit and digest assertions remain required.

Local installer changes retain pinned image-builder, labeling, offline payload verification and deliberate disk choice. Media permissive SELinux never weakens installed enforcing SELinux/signature policy. Record actual local smoke/fresh-install results separately from image builds.

Update STATUS/worklog with exact observed outcomes. Never call staged booted, signed installed or syntax qualified.
