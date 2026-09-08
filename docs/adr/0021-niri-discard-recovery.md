# ADR 0021: Exact niri discard with explicit file activation

Date: 2026-09-08. Status: implemented; native qualification pending.

The adopted niri file needs ordinary discard without losing pinned publication
choices or stopping the compositor. A change ID alone is insufficient because
other live edits or review decisions may change between review and application.

`discard-plan CHANGE_ID` derives a plan from the current review state and complete
live/desired hashes. It replaces only that exact observed range. A matching
pinned selection supplies the replacement; otherwise the public reference does.
Ambiguous overlapping selections refuse. Selection, exact-local policy,
publication references and accepted image baseline remain independent and intact.
No whole-live Git object or review snapshot is created.

`discard` requires the exact plan and `--activate-managed-file`. Niri 26.04 permits
runtime switching to another configuration path without exposing the current path
in IPC. Inferring it from process arguments or environment would be insufficient.
The explicit flag authorizes loading the managed file into the running session.
It does not restart niri or advance the image baseline.

Native validation uses a private checked sibling file through the open directory
descriptor. Niri resolves relative includes against the path's parent, so a
candidate in a separate scratch directory would not validate the intended inputs.
The sibling is removed after validation only if its identity is unchanged. A
process termination or write failure can leave it for manual review.

Existing checked replacement preserves ownership, group, mode and SELinux label.
A state reservation and hash/identity-only journal commit before publication.
Resume recognizes publication before a journal update, validates the file and
reloads. The IPC connection checks peer UID and the installed executable. A
separate event stream consumes the initial prior-load status before requesting
reload, then waits for a new ConfigLoaded event. Niri's action acknowledgement
only means the asynchronous watcher request was enqueued.

Successful reload and unchanged live hash precede durable Validated, checkpoint
cleanup and clearing the reservation. Abort restores only the exact saved pair;
later edits cause refusal. Keep-current validates/reloads a later file and retains
the checkpoint. Pending state blocks other mutations and is unreadable to older
versions that deny the newly present field. Completed ordinary state omits it.

This is a single-file transaction. Includes are validated by niri but are not
checkpointed or locked. Competing external editors/IPC actions are not isolated;
the operation does not promise an atomic multi-file application group. Native
file bytes exist only in memory or private atomic-I/O files, never the journal.
No source-only reconciliation plan authorizes image baseline acceptance.

Evidence: R07 actual CLI/desktop scenarios are prepared for stale-plan refusal,
pinned-line restoration, unrelated edits, metadata and relative includes.
Linux 34236565772 at 7d14ada initially failed the rustix PID method name; corrected
in the follow-up. Successful native discard/recovery is not yet claimed.

Primary sources (niri v26.04, inspected 2026-09-08):

- [Config parsing and include paths](https://github.com/niri-wm/niri/blob/v26.04/niri-config/src/lib.rs)
- [IPC actions and ConfigLoaded](https://github.com/niri-wm/niri/blob/v26.04/niri-ipc/src/lib.rs)
- [Action handling](https://github.com/niri-wm/niri/blob/v26.04/src/input/mod.rs)
- [Asynchronous file watcher](https://github.com/niri-wm/niri/blob/v26.04/src/utils/watcher.rs)

Impacted gates: R03 independent review decisions; R04 native activation/recovery;
R10 private durable state and checked file replacement.
