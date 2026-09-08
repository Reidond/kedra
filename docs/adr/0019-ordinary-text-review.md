# ADR 0019: Explicit ordinary-text review through Git

2026-09-08. Actual Linux CLI/Git qualification passes 34233086757 at 86bb96b;
native niri qualification is pending. Initial adapter: `.config/niri/config.kdl` only. No live file
activation or new-baseline acceptance is provided by this adapter yet.

Niri configuration remains an ordinary writable file. After `home init`, the
owner explicitly adopts it with `home file init --reviewed-safe`. This confirms
reviewing its custom commands for credentials; mixed secret/configuration files
remain unsuitable. Other home files, niri includes and agent profiles are not
automatically adopted. Metadata/path guards precede the bounded UTF-8/LF read.

The installed hash-checked source manifest supplies accepted B and target/override
provenance. Private SQLite holds that public baseline, a separate publication
reference, exact selected changes, exact local-only decisions and source receipts.
It does not retain whole live snapshots. Git's `diff --no-index reference -`
receives live bytes through stdin, so they do not enter a Git object database or
a scratch file. Git 2.55.0's
[diff-no-index implementation](https://github.com/git/git/blob/v2.55.0/diff-no-index.c)
defines stdin handling and exit codes 0/1; manual Git 2.55.0.windows.1 comparison
also confirms this path. Actual Kedra end-to-end qualification is separate.

Zero-context Git hunks define content-bound decisions. Equal-size adjacent line
replacements split into individual choices; insertion/deletion or unequal-size
replacement blocks remain indivisible hunks. IDs include the exact reference,
range and before/after bytes. Ambiguous overlap is refused, not relocated by
searching for similar text. Selected and local-only decisions cannot overlap.
Later file changes do not alter S. An exact I value that changes returns to review;
the owner can clear the old decision explicitly. There is no skip-worktree,
assume-unchanged or whole-file ignore substitution.

Git applies only S through a private temporary index. Export merges that approved
result with the committed source using `git merge-file`, refusing conflicts
before emitting a patch. Only current public source plus the selected result may
be added to source objects. The real index/worktree and live file are preserved.
Source HEAD is rechecked before writing a new patch; existing output is refused.
No conflict markers reach live configuration or a source patch.

An exact source receipt verifies ancestry, target/override and inclusion of S.
It advances the publication reference, reanchors disjoint I ranges over that
exact selected change and clears S. Accepted image B remains unchanged, and a
newer live value remains unselected. This is a local source receipt, not proof of
push, build, promotion or deployment. Generic image-baseline reconciliation,
file deletion/rename, cross-file groups and safe native apply remain separate work.

Qualification uses actual CLI/file/Git/SQLite workflows and native niri edits in
a disposable desktop VM. No model/unit/mock tests or repository scanners are used.
