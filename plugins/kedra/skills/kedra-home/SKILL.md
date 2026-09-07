---
name: kedra-home
description: Implement or research writable dotfile capture, Git partial staging, line-level local-only changes, safe export, three-way image-baseline reconciliation, application writers and rollback.
---

# Writable home is a core feature

Applications and users edit ordinary files. The repository supplies a baseline,
not every byte of live state. Do not replace this with immutable symlinks,
OverlayFS, blanket rsync, an automatic commit daemon, or Git rooted at all HOME.
Read references/state-model.md before choosing data structures.

## Changes have independent dispositions

A user can publish selected lines, keep others visibly uncommitted, mark particular
hunks local-only, let the application own selected structured settings, or safely
discard selected changes. These decisions may coexist in one file. Git's index
supports selected staging; later app writes must not modify staged content.
Git does not persistently ignore selected edits to tracked files: skip-worktree,
assume-unchanged and .gitignore are not the missing policy engine.

Keep a private review repository distinct from the OS source checkout. Only
approved selected content crosses into source; local snapshots/commits must not
become pushed ancestry. Capture adopted safe paths only; unknown files are review
candidates, never auto-staged. Exclude credentials before reading into snapshots,
including agent auth/transcripts, Bitwarden/keyring data and private SSH keys.
Mixed configuration/secret files need a tested safe projection or no management.

## Merge and apply

Track prior accepted baseline B, live L and candidate baseline N per application
group; compute a three-way candidate away from live files. Keep staged/local-only
state separately across that merge. Git merge-tree can prepare candidates without
modifying the live worktree, but it does not decide our policy or race behavior.

Never write merge conflict markers into a running app's config. Do not advance a
group's accepted baseline after a failed/partial apply. Preflight before OS staging;
activate only at the researched new-software/session boundary. Stop writers where
needed, recheck hashes, checkpoint and journal multi-file changes, apply, validate
and reload/restart. Atomic rename of one file is not an atomic multi-file update.
Watchers are hints; rescan on capture/apply because events can race or overflow.

## Export and local policy

Content-anchor ignored hunks to their baseline, not line numbers. Ambiguity and
new local values return to review. An app-owned key is a different persistent
rule; enforce it in export AND reconciliation, not only diff display. Preserve
host/shared source provenance. Track published-but-not-deployed edits to avoid
repeated export. Noctalia GUI state can override curated TOML; see the desktop
skill for narrow effective-setting projection without broad state capture.

Tests must cover overlap, app writes after staging/preflight, deletes/renames,
non-UTF paths policy, modes/labels, symlink escapes, full disk, interruption,
upstream adopting the same value, rollback after new edits and source-branch drift.
Gates R03/R04 precede real-home adoption. Sources: docs/SOURCES.md git-stage,
git-merge, git-faq, overlayfs, inotify, bootc-fs, noctalia. This system is not yet
implemented; never use a simplistic merge prototype on a real home.
