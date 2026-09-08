# ADR 0022: Accept only an observed installed niri baseline

Date: 2026-09-08. Status: implemented for qualification.

ADR 0020 previews source reconciliation without installed-image authority. ADR
0021 introduces journaled native discard and explicit managed-file activation.
They now share a prepared reconciliation result, while keeping distinct plan IDs
and authority checks.

`activate-plan --repo PATH` reads the root-owned installed source manifest and
hash-checked niri baseline. It plans that exact commit using the complete public
source history needed to account for accepted/publication anchors. Planned
target, path, source revision and file content must equal the installed baseline.
The caller cannot substitute a different source commit or use a source-preview ID.

`apply --repo PATH --plan ID --activate-managed-file` recomputes the plan and
validates the native candidate. The existing single-file controller persists the
reservation/checkpoint, reloads the managed file and waits for new native load
status. Only validated completion stores the proposed B/reference/S/I/P state.
The journal may contain that proposed state, which comprises public source bytes
and explicitly retained decisions. Complete live/desired native text remains in
memory or the private atomic-I/O candidate/checkpoint, not in SQLite or Git objects.

Resume checks the installed baseline still matches before publication and after
reload. Abort and keep-current preserve the previous accepted state. A no-file-
change plan still requires native validation/reload and a final live/state check
before baseline acceptance. Existing discard journals omit the optional proposed
state and retain their original behavior. Older readers fail closed on unfamiliar
pending state/journal fields.

The native E2E derivative includes a Git bundle of the actual public source and
its history. It fetches that bundle into a generated user checkout with the real
project origin, then exercises stale-plan refusal and acceptance of the actual
currently installed baseline while preserving a later native edit. The bundle
is confined to the research image. It does not install repository skills into a
global profile, ordinary image or personal home baseline.

Qualification is pending. This current-baseline scenario is not evidence of an
A-to-B OS transition. Actual changed-image baseline activation, rollback and
interruption across that change remain R04 work. OS staging, multi-file groups,
include transactions and competing external IPC actions remain separate.

Sources: ADRs 0019-0021 and their pinned Git/niri evidence; root-installed source
provenance as implemented by the existing source planner and installed reader.
