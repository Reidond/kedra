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
git-merge, git-faq, overlayfs, inotify, bootc-fs, noctalia. Production home management
is not implemented; never use a synthetic merge prototype on a real home.

## Established synthetic R03 findings

Follow-up 2026-09-08: the pure Noctalia 5.0.1 model projects three safe fields
from native full-export TOML; unknown/private fields and error excerpts are not
persisted. It separates pinned S, exact I, app-owned keys and publication chains,
so an intermediate N can retire only a known published prefix while preserving
newer live values. Ten local tests pass; native VM projection follows in R07.
Canonical JSON reconstruction is not filesystem crash durability. Real activation
and source writes remain gated. See ADR 0007 and R03-home-review/noctalia.md.

On 2026-09-07, Git 2.55.0.windows.1 and Rust 1.98.1 passed the std-only
`cargo run -p sysroot-core --example r03_home --locked` experiment and 12 tests.
One selected line from a larger hunk stays fixed in a pinned Git tree after later
app writes. Export B-to-S through a temporary source index with
`git apply --cached --3way`; source conflicts leave the real source index/worktree
and live fixture unchanged. Audit all source objects, not just the visible diff:
private snapshot commits and local-only bytes must not cross into source history.

The prototype binds exact before/after line decisions to B, rejects ambiguity,
relocation, stale values and selection/policy overlap, and checks policy at export.
This is conservative single-line replacement evidence, not persistent hunk-ignore
or Noctalia/app-owned-field support. Published S/source commit is separate from B;
repeat export is a no-op and publication never implies deployment. R03 remains
blocked for real adoption; R04 activation is not-run. See
[the report](../../../../docs/research/R03-home-review/REPORT.md) and
[ADR 0004](../../../../docs/adr/0004-synthetic-home-review.md) for exact cases,
failure history, unsupported inputs and the next Noctalia/disposition experiment.

Published source e492258 also passed the same 12 cases with Git 2.55.0 and
Rust/Cargo 1.98.1 on Linux in Actions run 34164331109, observed 2026-09-08.
This adds platform evidence without qualifying the remaining R03/R04 gates.
