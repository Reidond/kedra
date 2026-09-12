# State model and adversarial examples

Supported Noctalia/niri workflows use independent review, publication and recovery
state. See docs/HOME-REVIEW.md, docs/TEXT-REVIEW.md and docs/STATUS.md for actual scope.

## Separate state dimensions

B: baseline accepted by the current home application group.
L: latest captured/live configuration.
N: next image baseline, with source/host/version provenance.
S: selected publishable snapshot/index state.
I: local-only hunk decisions and explicit app-owned fields.
P: content published to source but not yet deployed.
J: activation/recovery journal and checkpoints.

A merge(B,L,N) does not by itself preserve S/I/P or prove safe application. Do not
pretend these are all branches that can be merged without a policy. Preserve
activation and backward-compatible state. Avoid
a prematurely fixed database/schema based only on these letters.

## Minimal fixture

B: font=12, border=2, animation=200.
L: font=14, border=2, animation=150.
N: font=12, border=3, animation=250.
Expected content: retain font=14, take border=3, flag animation conflict. Keep the
whole app group live on its previous configuration until a safe coherent apply.
Do not put conflict markers in a compositor config or advance B prematurely.

Now select only font=14 for publication while animation=150 stays visible, then
change font to 16 from an app. S stays the reviewed 14; the new difference is
unstaged. A public source commit must contain exactly the selected change, not L.
If font=14 was instead ignored as a hunk, changing it to 16 becomes visible again.
If font is explicitly app-owned, local changes remain suppressed and never export.

## Export cases

Source HEAD may advance between review and export. Rebase/apply the approved
patch against known source provenance; stop on conflicts. Do not force-push the
review repo or copy the entire captured file. A desktop host file exports to its
host path, not accidentally shared configuration. A value already published but
awaiting the image is tracked as P, not continuously offered as a new change.

Local-only ignored settings are local to a machine. Shared behavior must be
intentionally committed; local review/ignore state is not cross-machine sync.

## Activation hazards

Recheck a file after capture AND coordinate writers. A hash match before write
alone cannot stop an app from saving stale in-memory state afterward. Use a
researched stop/restart contract where necessary. A file deletion from N must not
delete locally modified L silently. A rename, file/directory swap, mode change,
symlink/hardlink, newline/encoding difference, or full disk has explicit behavior.
Open/validate paths relative to trusted roots without following unsafe traversal;
resolve race resistance in R10 rather than claim canonicalize alone makes it safe.

Home review and activation run with user privileges. Candidate validators must
not execute untrusted checkout code as root. Store schema versions: rollback may
boot an older sysroot that cannot read new journal/state. Fail recoverably instead
of silently treating unknown versions as an empty configuration.

## Privacy

A local Git object can retain a deleted secret. Exclude credential paths before
capture; do not rely on later .gitignore, display masking or removal from the
latest commit. Synthetic secret fixtures are allowed; real secrets never enter
fixtures/logs, even when reports will later be redacted.
