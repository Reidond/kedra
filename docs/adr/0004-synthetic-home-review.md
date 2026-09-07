# ADR 0004: Immutable selection and isolated Git export for the R03 prototype

Date: 2026-09-07. Status: accepted for the synthetic experiment only.
Evidence: [R03 report](../research/R03-home-review/REPORT.md).
Preserves PLAN.md section 6 and ADRs 0001/0003. No production enrollment,
activation protocol or installed command is enabled by this decision.

Ordinary application writes must coexist with three dispositions in one file:
selected for source, visibly uncommitted, and explicitly local-only. A whole-file
copy exports all three. Merging B/L/N alone cannot retain the user's selection or
local-only decisions. Git's tracked-file index flags do not encode hunk policy.

Use a small, std-only Cargo example in sysroot-core, outside its library API.
Its CLI runs a fixed synthetic session and accepts no caller-supplied paths. All
fixtures are newly created under ignored target/r03 and retained for inspection.
There are still three packages, one lockfile and no first-party src/ directories.
`[[example]] test = true` includes its regression tests in ordinary Cargo tests;
there is no custom check runner. Production sysroot/helper refusals remain intact.

The experiment separates these states:

| State | Prototype representation |
|---|---|
| B | Immutable accepted text, private baseline commit and originating source commit |
| L | A single explicitly allowed ordinary file in a generated live-home directory; capture copies it to a separate review worktree |
| S | Git index snapshot pinned by tree OID, plus its reviewed text; subsequent capture never stages |
| I | Baseline-bound exact before/after full-line decisions in memory |
| N | Explicit next-baseline fixture file, merged into an off-live candidate by Git |
| P | Selected tree and synthetic source commit recorded separately; B is not advanced |
| J | Not implemented; activation and durable recovery belong to R04 |

Selection constructs only approved line replacements against B and uses Git
hash-object/update-index/write-tree to stage their bytes. This is a bounded text
projection, not a diff parser or custom Git implementation. Each decision requires
unique exact line anchors. The baseline is part of the decision; no line number
is stored as identity. A derived-position equality check conservatively rejects
relocation, insertion and deletion. Changed values, ambiguous anchors and overlap
return classification errors. This does not implement a general hunk UI or
persistent suppression across baseline changes.

Export computes a full-index Git patch from B to the pinned S tree, checks I again,
and uses `git apply --cached --3way` in a fresh temporary index initialized from
current source HEAD. The source path is explicitly supplied by the fixture's host
provenance. Dirty source is refused intact. Source HEAD must descend from the
recorded origin. Conflicts remain in temporary index stages, not live/source
files. Publication checks HEAD again and uses a compare-and-swap ref update.
The synthetic commit has only the source HEAD as parent. No private review fetch,
shared object directory or review ancestry crosses this boundary.

Only generated repositories are committed. Repeated or upstream-adopted selected
content produces no new commit. P records publication awaiting deployment while
the live file and accepted B stay unchanged. The publication helper's ref/index/
worktree operations are deliberately not a crash-safe production transaction.

Baseline preflight uses `git merge-file -p` with B/L/N fixture files; conflict
markers are retained only in the candidate artifact. There is no activation path.
Noctalia effective-field projection, application-owned keys, durable I/P, safe
discard, field migration, writer coordination and real-home path safety remain
unresolved. Exact-hunk and app-owned-field policies are different; only the former's
narrow refusal semantics have experimental evidence here.

Git invocations use explicit arguments, empty fixture config/templates/hooks,
fixed synthetic identity/dates and no inherited GIT_* overrides. They never invoke
a remote. Git for Windows 2.55.0.windows.1 rejected Rust's verbatim canonical path
in GIT_CONFIG_GLOBAL with exit 128; generated Git config/index arguments now use
ordinary slash paths. This conversion is not a general filesystem security proof.

Consequences: the requested four core behaviors are demonstrated with executable
fixtures and no dependencies. R03 remains blocked for real adoption because its
full matrix is incomplete; R04 still gates activation. Narrow refusals are measured
behavior, not claims of general insertion/rename/encoding support.

Primary references checked 2026-09-07: [Git update-index](https://git-scm.com/docs/git-update-index),
[Git apply](https://git-scm.com/docs/git-apply), and
[Git merge-file](https://git-scm.com/docs/git-merge-file).
These document mechanisms; the versioned report establishes observed behavior.
