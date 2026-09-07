---
name: kedra-bootc
description: Work on Fedora 44 bootc image derivation, filesystem ownership, exact-digest updates, notify-only client checks, pending deployments, boot states or rollback.
---

# Fedora bootc operating model

Use a plain Containerfile from Fedora 44 bootc, package lists and explicit file
assembly. OS/installer builds run in GitHub Actions. Do not adopt BlueBuild,
client-side RPM layering, or a live `dnf install` shortcut as the persistent Kedra
workflow. User-installed personal agents/apps are a separate ownership choice.

## Filesystem boundaries

Repository etc/ maps to image /etc; usr/ maps to /usr. Prefer image-owned defaults
under /usr where applications support them, including systemd/sysctl/tmpfiles
drop-ins. Do not author /usr/etc; it is generated/internal. Persistent /etc has
its own three-way retention behavior, including local metadata changes; a new
Git file need not override a locally modified /etc file. Surface drift rather
than promise continuous enforcement. Transient /etc is not an MVP shortcut:
first account for machine identity, credentials and networking persistence.

Persistent /var and backed home data are shared across deployments. Baking a new
home file into an image does not update an already-installed live home. Ship
resolved safe home baselines under /usr/share/sysroot/home, then use the separate
Git-backed writable-home mechanism. No blindly copying into /var/home or symlinking
all of .config. Prefer tmpfiles/StateDirectory for required runtime directories.
Repository knowledge skills, unlike explicitly adopted personal dotfiles, remain
checkout-only; do not ship this collection as system/global agent skills.

## Release/deployment procedure

Resolve and record base digest, RPM inventory and external artifacts. Never
claim exact rebuildability from a source commit against changing repositories.
Run bootc container lint in image validation, but do not treat it as a boot test.
All candidate references must be final registry digests, not local image IDs.

For a researched trusted update, the helper verifies release eligibility and
uses bootc switch with signature enforcement and the exact digest. A digest-pinned
installation needs a new switch for the next release; normal bootc upgrade does
not advance it. Do not insert operational switch commands into bootstrap code
before R01/R08/R10 pass. A normal mutable Fedora installation is not automatically
convertible through bootc switch; prove a fresh VM installer first.

Read [the update-client notes](references/update-client.md), docs/UPDATES.md and
ADR 0002. Proposed default: periodic signed-metadata check/notification only;
authorized sysroot update stages without an immediate reboot. Audit/mask inherited
bootc-fetch-apply-updates automation because it can reboot. Do not assume a timer
that only checks Kedra disables a second upstream updater. --download-only needs
version-specific pending-slot tests, not a generic safe-prefetch assumption.

Keep an existing manually staged image unless explicit replacement is requested.
Staging can affect the next ordinary reboot; report that even without --apply.
Rollback records a hold and never lowers channel trust high-water marks. Check
freshness separately from image age: a no-change Fedora check can be healthy,
while an old image with a stopped schedule or blocked candidate needs a warning.
A dirty/missing source checkout must not be reset or required by installed updates.

Status distinguishes available, verified, preflight, staged, awaiting reboot,
booted, home-reconciled and healthy. The agent need not survive reboot. Installed
post-boot checks finalize deterministic state. Keep preflight separate from actual
home activation under new software. Manual rollback changes the OS deployment,
not all persistent data or application migrations. Automatic health rollback is
a later tested feature. Recovery must work offline without an AI service.

Tests: install A/update B/rollback A, /etc local drift, persistent home retention,
interrupted staging, candidate-versus-running digest, and old journal readers.
Also run docs/research/update-refresh/EXPERIMENTS.md client cases. Gates:
R01/R02/R04/R07/R08/R09/R10. Sources: docs/SOURCES.md bootc-fs, bootc-switch,
bootc-build, bootc-kargs, bootc-secrets; docs/UPDATES.md U09-U11.
Read kedra-home for reconciliation and maintain worklog with actual evidence.
