# State model and adversarial examples

See usr/src/kedra/docs/HOME.md for the workflow and usr/src/kedra/docs/STATUS.md
for actual results.

## State

B: last baseline merged into the live file, per user (`applied/<path>`).
L: the live file.
N: the installed image baseline.
P: a baseline whose merge conflicted (`pending/<path>`), beside `<file>.kedra-new`.

Selection for source is not stored: `capture` copies L into the checkout and Git's
index (`git add -p`) holds what the owner picked. Local-only lines simply stay
unstaged. There is no activation journal; replacement is atomic per file.

## Minimal fixture

B: font=12, border=2, animation=200.
L: font=14, border=2, animation=150.
N: font=12, border=3, animation=250.
Expected: font=14 and border=3 merge; animation conflicts. L stays untouched and
live, `<file>.kedra-new` holds the marked merge, P=N. After the owner fixes L and
deletes the conflict file, the next update checks and reloads L and records B=N.

## Cases to keep covered

- N equals B: nothing changes even when L differs (local edits persist).
- L equals B: fast-forward. L equals N: record only.
- No B: equal L is recorded, missing L is created, different L is a conflict with
  N as the conflict file. Never treat an unknown base as "adopt L".
- L deleted by the owner and N changed: conflict, the deletion is not undone.
- N absent: report it, leave L alone.
- Check failure: L unchanged, B unchanged, nonzero exit.
- Reload failure: L already written, B unchanged; the rerun merges to the same
  content and retries the reload.
- Links, hard links, devices, group/world-writable files and symlinked directories
  are refused; nothing is followed.
- An existing `<file>.kedra-new` not written by an update is never overwritten.

## Hazards

A merge result can be syntactically valid and still wrong; the declared check is
the application's own validator, not a semantic proof. Inspect visible changes
(kedra-lab shot). Reloads run as the user and must not need privilege. A writable
GUI override can still win over a merged file.

## Privacy

Only declared files are read. A local Git object can retain a deleted secret:
review a captured diff before staging; do not rely on later removal. Synthetic
secret fixtures are allowed; real secrets never enter fixtures or logs.
