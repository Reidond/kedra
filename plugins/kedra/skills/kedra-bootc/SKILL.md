---
name: kedra-bootc
description: Work on Fedora 44 bootc image derivation, filesystem ownership, image-managed configuration, exact-digest updates, boot states or rollback.
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

Status distinguishes available, verified, preflight, staged, awaiting reboot,
booted, home-reconciled and healthy. The agent need not survive reboot. Installed
post-boot checks finalize deterministic state. Keep preflight separate from actual
home activation under new software. Manual rollback changes the OS deployment,
not all persistent data or application migrations. Automatic health rollback is
a later tested feature. Recovery must work offline without an AI service.

Tests: install A/update B/rollback A, /etc local drift, persistent home retention,
interrupted staging, candidate-versus-running digest, and old journal readers.
Gates: R01/R02/R04/R07/R08/R10. Sources: docs/SOURCES.md bootc-fs, bootc-switch,
bootc-build, bootc-kargs, bootc-secrets. Read kedra-home for reconciliation.
