---
name: kedra-machines
description: Add or change Kedra host targets, shared/host overlays, home source provenance, enrollment, cross-target checks and independent updates without a fleet framework.
---

# One repository, separate target releases

The repository root is the image filesystem. Shared packages
(usr/src/kedra/image/packages.list) and root etc/ and usr/ are assembled first;
etc/skel/ is the home baseline and the usr/src/kedra/ development tree never
enters the image. The target's usr/src/kedra/image/targets/<target>/ inputs
(target.toml, packages.list and the etc/ and usr/ overlay) follow with explicit
same-path replacement. Report replaced paths. Use native app includes for content
composition, not an arbitrary deep merge. Start with two levels; add shared laptop
profiles only when actual repeated hardware policy warrants them. No
machine-specific long-lived branches or repos.

The first real target is desktop. xps reserves a future laptop and is disabled
until selected/qualified. Use a second synthetic VM target to prove architecture
without pretending it certifies any Dell model. Both targets may build from the
same source commit/base but produce different image digests and installers.

The enabled ARM target is `qemu-arm64`, a native Apple Silicon QEMU/HVF guest.
The owner explicitly retired the old `utm` identity on 2026-09-28. Its closed
entry is in `image/release/targets.json`, embedded by sysroot-core and read by
Python tooling. It has its own repository, public key and signing environment.
Keep output/scale overrides in disposable lab state unless a product default is
measured. The target preserves qemu-guest-agent's restricted RPC list; fixture
SSH grants do not enter production. SPICE integration and Venus diagnostics were
removed because the native backend uses Cocoa/VirGL/ANGLE Metal, not SPICE/Venus.
Bitwarden uses the reviewed official arm64 tarball because no arm64 RPM exists.

## Provenance and identity

Implemented 2026-09-08: `sysroot source plan --repo PATH --host TARGET [--json]`
reads one committed HEAD via raw Git tree/blobs, excludes index/worktree/untracked
edits and emits package intent plus source paths, replacements, modes and SHA-256.
Disabled/mismatched targets, links, package options, payload collisions,
unexpected top-level directories, usr/src/ content other than usr/src/kedra/ and
unexpected target-overlay files fail. Source planning does not establish
two-machine lifecycle. See usr/src/kedra/docs/ARCHITECTURE.md for ownership and
usr/src/kedra/docs/STATUS.md for actual implemented/qualified behavior.

The follow-up `source archive --host TARGET --output FILE` writes a deterministic
tar from those raw blobs plus source.json. It creates a new output only, keeps
Git modes, fixes archive ownership/time, and rejects known credential paths and
private-key markers. It never reads live
homes or claims to detect every secret; public input review remains necessary.

Record each home file's source path/revision/host/content hash/mode/app group.
Shared keybindings normally export to etc/skel/, monitor settings to
usr/src/kedra/image/targets/<target>/etc/skel/. Retained commits in the earlier
layout (hosts/, packages/, root home/) still resolve, selected per commit; a
commit mixing both layouts is refused. Provenance compares layer and
home-relative path, so a baseline recorded before the move stays valid.
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
Sources: usr/src/kedra/PLAN.md and usr/src/kedra/docs/ARCHITECTURE.md (user
decisions), plus bootc-switch and bootc-kargs in usr/src/kedra/docs/ARCHITECTURE.md.
Hardware qualification is R07, not a matrix build.
