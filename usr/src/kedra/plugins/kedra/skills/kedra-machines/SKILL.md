---
name: kedra-machines
description: Add or change Kedra targets, the system/ sets they compose, package selection, home source provenance, cross-target checks and independent updates without a fleet framework.
---

# One repository, separate target releases

system/ declares every target (image/package-inputs.json names system/catalog.kedra):
the `system` foundation, sets of packages, image files, home baseline and units
(base, login, desktop, ssh-agent, qemu), and one target block each in
catalog.kedra (architecture, image, hardware, os_release, hide_autostart, image
steps, `use` of sets). Small files are inline `text`; larger and managed home
files are `file("./…")` beside the module. Sets compose; a path declared twice is
refused with both origins, never replaced, so target-only files go in a set only
that target uses (qemu.kedra). The crates/ and usr/src/kedra/ development trees
never enter the image, and any other committed path, root etc/ and usr/
included, is refused. Use native app includes for content composition, not an
arbitrary deep merge. Start with two levels; add shared laptop profiles only when actual
repeated hardware policy warrants them. No machine-specific long-lived branches
or repos.

The first real target is desktop (x86_64). xps reserves a future laptop and is
disabled (`candidate_target = false`) until selected and qualified. Both enabled
targets may build from the same source commit but produce different image
digests and installers.

The enabled ARM target is `qemu-arm64`, a native Apple Silicon QEMU/HVF guest.
Release targets are the closed table usr/src/kedra/image/release/targets.json,
embedded by sysroot-core and read by Python tooling; each has its own repository,
public key and signing environment. Keep output/scale overrides in disposable lab
state unless a product default is measured. The target preserves
qemu-guest-agent's restricted RPC list; fixture SSH grants do not enter
production. The native backend uses Cocoa/VirGL/ANGLE Metal, so the target
carries no SPICE or Venus integration.
Codex and Bitwarden are the same `.kedra` packages on both targets, with
per-architecture archive pins (kedra-agents, kedra-bitwarden).

## Provenance and identity

`sysroot source plan --repo PATH --host TARGET [--json]` reads one committed HEAD
via raw Git tree/blobs, excludes index/worktree/untracked edits and emits package
intent, native image steps, source paths, replacements, modes and SHA-256
(crates/sysroot/source.rs, source_packages.rs). Disabled/mismatched targets,
links, payload collisions, unexpected top-level directories (including the
earlier `hosts/`, `packages/` and root `home/` layout, refused since 2026-10-14),
usr/src/ content other than usr/src/kedra/ and unexpected target-overlay files
fail. A `.kedra` source must carry the package-inputs.json descriptor, and a
source may not mix it with package lists; a commit with package lists in the
current layout still plans through the list reader. Source planning does not
establish two-machine lifecycle. See usr/src/kedra/docs/ARCHITECTURE.md for
ownership and usr/src/kedra/docs/STATUS.md for actual results.

`sysroot source archive --host TARGET --output FILE` writes a deterministic tar
from those raw blobs plus source.json. It creates a new output only, keeps Git
modes, fixes archive ownership/time, and rejects known credential paths and
private-key markers. It never reads live homes or claims to detect every secret;
public input review remains necessary.

Record each home file's source path/revision/target/content hash/mode; the
source manifest records the source file and declaring set (`origin`). Shared
keybindings go to the desktop set's home files, monitor settings to a set only
that target uses (`sysroot home capture` writes to the file the image recorded). Provenance compares layer and
home-relative path. Do not infer ownership from the deployed filename alone. A
target override can hide a common file; show scope and all affected targets
before publication. Ambiguous scope requires an explicit reviewed choice, not
silently rewriting shared defaults.

The image says what it is: its signed identity names target, repository,
architecture and channel, and the installed helper compares them with the
candidate before staging even if one key were trusted for both. There is no local
enrollment. Renaming the hostname does not change the update
source, and the helper never follows another target's repository
(docs/UPDATES.md); a target is never chosen by an automatic hardware guess.

## Independent lifecycle

A shared commit may publish two candidates, but each installation chooses when
to stage/reboot. Running/staged digests, home baselines, local edits and
rollback history are independent. Offline status is last-known, not current.
Local/uncommitted/ignored dotfiles, OAuth, Bitwarden state, personal agent versions
and private MCP settings do not sync automatically. Only chosen source changes do.

An agent may execute on desktop while editing xps. Provide both facts; editing
another target does not permit deploying its image locally. No central fleet
server or workstation SSH key in Actions is needed. Future explicitly authorized
remote deployment should call the same local deterministic helper, not bypass it.

Two-machine fixture (not-run): two VMs with distinct target overrides and local
edits; publish shared plus target-only changes; update independently; reject
cross-target images; keep one machine offline; then reconcile and roll back
separately without scope leakage. Hardware qualification is separate from a
matrix build. Sources: usr/src/kedra/PLAN.md and usr/src/kedra/docs/ARCHITECTURE.md
(owner decisions).

Package selection (owner decision 2026-10-06): source plan selects the one
committed package-inputs.json format, reads the admitted `.kedra` modules and
resources from raw Git and binds intent plus input hashes; a new format cannot
coexist with or fall back to package lists. Namespace, targets and required base
policy are independent (image/package-policy.json). Both targets realize selected
source packages natively on their own architecture; the former x86_64 refusal was
removed on 2026-10-16 after it failed the desktop release.
