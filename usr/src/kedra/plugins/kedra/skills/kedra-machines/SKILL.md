---
name: kedra-machines
description: Add or change Kedra host targets, shared/host overlays, home source provenance, enrollment, cross-target checks and independent updates without a fleet framework.
---

# One repository, separate target releases

Common packages and Linux-shaped etc/usr/home inputs are assembled first;
hosts/<target>/ inputs follow with explicit same-path replacement. Report replaced
paths. Use native app includes for content composition, not an arbitrary deep
merge. Start with two levels; add shared laptop profiles only when actual repeated
hardware policy warrants them. No machine-specific long-lived branches or repos.

The first real target is desktop. xps reserves a future laptop and is disabled
until selected/qualified. Use a second synthetic VM target to prove architecture
without pretending it certifies any Dell model. Both targets may build from the
same source commit/base but produce different image digests and installers.

2026-09-25 (spec for this change; source only until qualified): `utm` is the second
enabled target: aarch64, `ghcr.io/reidond/kedra-utm`, `hardware_status = "virtual"`.
It is an Apple Silicon Mac guest in UTM 5.0.6's QEMU backend (HVF, `virtio-gpu-gl-pci`,
UEFI + TPM Secure Boot), not a hardware claim. The closed table is
`crates/sysroot-core/targets.rs` and `build/release/material.py` `TARGETS`; any other
(target, architecture) pair is refused, and xps stays disabled. It has its own key,
signing environment, runner and rank history. Keep niri output/scale/input overrides
out of `hosts/utm` until measured in UTM. Facts found in disposable containers on
2026-09-25:
- Fedora 44 `qemu-guest-agent-10.2.2-1.fc44` blocks no RPCs (`QEMU_GA_ARGS` is
  commented out in `/etc/sysconfig/qemu-ga`). UTM always attaches its port, and
  `utmctl exec`/`file` use it (UTM v5.0.6 `Scripting/UTMScripting*Impl.swift`).
  `hosts/utm` therefore blocks the exec, file, password and SSH-key RPCs in a
  qemu-guest-agent drop-in.
- Bitwarden has no aarch64 RPM. `build/bitwarden/prepare.py` accepts only the reviewed
  flat layout of the arm64 tarball. Its desktop entry keeps upstream `%u`, and
  niri/Noctalia app-id matching on utm is unmeasured.

## Provenance and identity

Implemented 2026-09-08: `sysroot source plan --repo PATH --host TARGET [--json]`
reads one committed HEAD via raw Git tree/blobs, excludes index/worktree/untracked
edits and emits package intent plus source paths, replacements, modes and SHA-256.
Disabled/mismatched targets, links, package options and payload collisions fail.
Source planning does not establish two-machine lifecycle. See docs/ARCHITECTURE.md
for ownership and docs/STATUS.md for actual implemented/qualified behavior.

The follow-up `source archive --host TARGET --output FILE` writes a deterministic
tar from those raw blobs plus source.json. It creates a new output only, keeps
Git modes, fixes archive ownership/time, and rejects known credential paths and
private-key markers. It never reads live
homes or claims to detect every secret; public input review remains necessary.

Record each home file's source path/revision/host/content hash/mode/app group.
Shared keybindings normally export to home/, monitor settings to hosts/<host>/home/.
Do not infer ownership from the deployed filename alone. A host override can hide
a common file; show scope and all affected targets before publication. Ambiguous
scope requires an explicit reviewed choice, not silently rewriting shared defaults.

Keep root-owned local enrollment separate from the image's self-description.
Enrollment says what the machine may deploy; the image/manifest says what it is.
Compare target, repository and architecture before staging even if the key is
trusted for both. Renaming hostname does not change the update source. Changing
enrollment is an explicit migration, never an automatic hardware guess.

## Independent lifecycle

A shared commit may publish two candidates, but each installation chooses when
to stage/reboot. Running/staged digests, home baselines, local-only policy and
rollback history are independent. Offline status is last-known, not current.
Local/uncommitted/ignored dotfiles, OAuth, Bitwarden state, personal agent versions
and private MCP settings do not sync automatically. Only chosen source changes do.

An agent may execute on desktop while editing xps. Provide both facts; editing
another target does not permit deploying its image locally. No central fleet
server or workstation SSH key in Actions is needed. Future explicitly authorized
remote deployment should call the same local deterministic helper, not bypass it.

R09 fixture: two VMs, distinct host overrides/local edits; publish shared plus
host-only changes; update independently; reject cross-target images; keep one
machine offline; then reconcile and roll back separately without scope leakage.
Sources: PLAN.md and docs/ARCHITECTURE.md (user decisions), plus bootc-switch and
bootc-kargs in docs/ARCHITECTURE.md. Hardware qualification is R07, not a matrix build.
