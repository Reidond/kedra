# ADR 0020: Source ancestry in ordinary-text reconciliation

2026-09-08. Prepared for actual CLI qualification. Source-only preview; native
validation, live apply and accepted-baseline advancement are not implemented by
this change. R04 remains open for those behaviors.

`home file plan --repo PATH [--commit FULL_COMMIT]` resolves committed source
through the actual Git/source planner. It requires the correct project origin,
target/path/mode and complete accepted/publication history. Missing or shallow
history refuses. Git exit 1 means a publication is not an ancestor; other Git
errors are failures, not guesses about deployment.

Publication ancestry determines the known prefix reflected in candidate N. The
last reflected publication is the live merge base, or accepted B if none is
reflected. This preserves later live edits when an intermediate image arrives.
Remaining publication references are rebased over N separately from live data;
older source previews retain future publications instead of resetting them.

Only currently matching exact-local decisions can override incoming defaults.
Known disjoint ranges are reanchored over the candidate. Partial/boundary overlap
is refused. A source conflict with a pinned selection also refuses; the selected
value is not silently rebased into authority to overwrite an upstream change.
There is no whole-file `--ours` or blind overwrite fallback.

Git performs the actual merges. Complete live bytes are supplied through sealed
anonymous Linux memfd files opened via the parent process's live descriptors;
no complete live file is written into scratch storage or Git objects. Diff input
continues to use stdin. Public references and explicitly selected/local decisions
can be used in the private temporary index. Source export still admits only the
approved selected result to public source objects.

The result includes old/candidate source revisions, observed/desired hashes,
current/proposed diffs against the old publication reference, retained decisions,
and reflected/pending publication counts. The plan hash binds the exact old
state, live content and proposed result. This is advisory source preflight, not a
native apply receipt or release-verification assertion. The CLI does not persist
the proposed state. Qualification extends the actual CLI/Git/file workflow with
source commits and later edits; no isolated model tests or source scanners.
