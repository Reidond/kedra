---
name: kedra-home
description: Implement or research writable dotfile capture, Git partial staging, line-level local-only changes, safe export, three-way image-baseline reconciliation, application writers and rollback.
---

# Writable home is a core feature

Niri 26.04 discard/reload finding, 2026-09-08 (ADR 0021): relative includes use
the main file's parent. Validate candidates with the same directory context.
IPC LoadConfigFile can switch the runtime path; argv/environment cannot prove the
active file. Require explicit managed-file activation. The action acknowledgement
enqueues asynchronous work, and the initial ConfigLoaded event describes the prior
load. Subscribe first, consume initial status, request reload, then require a new
event. Failures retain recovery state. Sources: pinned niri-config/src/lib.rs,
niri-ipc/src/lib.rs, src/input/mod.rs and src/utils/watcher.rs linked in ADR 0021.
This impacts R03/R04/R10; implementation is pending native qualification.

Native process-kill recovery, 2026-09-08: R07 34230166262 at 3267e03 kills the
installed CLI during native publication, then passes abort/resume/keep-current.
Exact abort refuses a later real Noctalia edit; keep-current preserves it and now
starts/validates the writer before clearing pending state. Selection and file
metadata remain intact. Power-loss/full-disk and other interruption phases remain
open. Default XDG private state initialization also passes. See R04/ADR 0017.

Ordinary niri text review passes CLI 34233086757 and native 34233086974 at 86bb96b
(ADR 0019): selected/local lines, later writes, source receipts, insert/delete and
native config validation. Source-ancestry reconciliation preview is prepared in
ADR 0020; it does not provide live text activation or advance accepted B.
Never hash the full live text into Git objects: Git 2.55.0 diff-no-index.c supports
`git diff --no-index reference -` with stdin in memory and exits 0/1. Only explicitly
selected content may cross into source; init requires reviewing custom commands
for secrets. This bounded adapter does not yet activate new text baselines.

Current evidence, 2026-09-08: actual R07 desktop run 34223972271 at a5cd96e
passes native stale-plan refusal and selected-field discard with writer restart,
selection retention and owner/group/mode/SELinux preservation. Source export and
receipts also have actual CLI evidence. Generic file/line integration and broader
interruption/baseline-transition cases remain open. Earlier synthetic commands
below are historical evidence: their harnesses were removed under the owner's
E2E/manual-only decision and must not be recreated or run. See R03/R04 reports
and ADR 0017 for exact scope and fail-closed recovery behavior.

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
git-merge, git-faq, overlayfs, inotify, bootc-fs, noctalia. Full home management and
activation remain unavailable; never use a synthetic merge prototype on a real home.
The Linux `sysroot home --state PATH` commands now connect the three-field Noctalia
model to private review storage. See docs/HOME-REVIEW.md for implemented scope and
the R03 report for native qualification. Selected-field patch export and exact
source receipts now use temporary Git indexes and toml_edit 0.25.13 in the user
CLI (ADR 0014); native export qualification is pending. Live activation remains
unavailable. This interface was added on 2026-09-08 against Noctalia 5.0.1; unknown
versions, damaged state or unsafe file metadata fail without reinitialization.
Run 34188179270 at 3d108e9 passes the six Linux bridge cases; native desktop run
34188179252 passes GUI changes with durable staging and local dispositions.
Export must retain accepted/previous source ancestry, host/shared provenance and
the pinned S value. Refuse a conflicting source value, never copy L wholesale or
reset the real index. Receipt of a local commit is not a push, promotion or deploy.

## Established synthetic R03 findings

Follow-up 2026-09-08, R04/ADR 0017: Noctalia 5.0.1 native service stop/read-back/
restart passes 34212238491 at 3ce1c1a. Linux workspace 34217852366 at 4a7d02e
passes narrow plan/patch/file replacement and durable recovery tests. The new
home plan/apply/discard/recover commands are implemented for native qualification;
the new VM run stopped on an unavailable Fedora base before exercising discard.
Only native settings.toml receives changed safe fields. Full native file bytes
stay outside review/source storage; private atomic-I/O checkpoints are removed
after validated success or retained on conflict. Pending state blocks mutations
and older readers. Generic files and full fault/rollback cases remain open.
This updates the older unavailable-activation descriptions above; do not infer
full home qualification from the successful synthetic cases.

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
