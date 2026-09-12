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
