# Review niri configuration by line

The actual Linux CLI workflow and native niri desktop checks pass at `86bb96b`
(runs 34233086757 and 34233086974).
This interface supports the ordinary
`.config/niri/config.kdl` file on an installed Linux Kedra desktop. It does not
manage other files or checkpoint niri includes. Native discard and installed
baseline activation are implemented for qualification below. Ordinary editor
changes remain writable; niri's normal validation/reload behavior applies.

First review the file for secrets, including values passed to custom commands.
Do not adopt a file containing credentials. Then, as the desktop user:

```sh
sysroot home init                         # once, if home review is not initialized
sysroot home file init --reviewed-safe   # once, after inspecting this file
sysroot home file status
```

The commands default to the private home-review state directory described in
[home review](HOME-REVIEW.md). Use the same explicit `--state PATH` on each
command if you already use a custom store. Initialization never replaces an
existing record. The baseline comes from the installed image's verified source
manifest. Live text is compared in memory; complete live snapshots are not saved
in review state or Git objects.

Status returns JSON. Each `changes` row includes before/after text and a `change.id`.
Copy the ID of the exact change you reviewed:

```sh
sysroot home file stage CHANGE_ID
sysroot home file keep-local OTHER_CHANGE_ID
sysroot home file selection
```

Adjacent equal-size line replacements can be selected individually. Insertions,
deletions and unequal-size replacements remain whole hunks. Changed or missing
IDs are refused. Later edits preserve the selected value; a changed local-only
value becomes visible again. Use `unstage CHANGE_ID` or `clear-local CHANGE_ID`
to remove the corresponding decision. Selected and local-only ranges cannot overlap.

Export the selection to the correct shared/host source path:

```sh
sysroot home file export --repo "$HOME/src/kedra" --output "$HOME/niri-selected.patch"
git -C "$HOME/src/kedra" apply --check "$HOME/niri-selected.patch"
# Review the patch and current checkout before applying and committing it.
git -C "$HOME/src/kedra" apply --3way "$HOME/niri-selected.patch"
```

Export preserves live files and the checkout/index. Git may refuse a conflict or
source-provenance change; resolve and review it explicitly. The command does not
commit or push. After making a source commit that contains the exact selection:

```sh
sysroot home file record-source --repo "$HOME/src/kedra" --commit FULL_COMMIT_ID
sysroot home file status
```

Recording clears that selection while preserving later live changes and local
decisions. It does not advance the accepted image baseline or imply deployment.

Before a source change is built or staged, preview its home reconciliation:

```sh
sysroot home file plan --repo "$HOME/src/kedra"
# Or preview an exact retained source version:
sysroot home file plan --repo "$HOME/src/kedra" --commit FULL_COMMIT_ID
```

This preview passes actual CLI workflow 34235455512 at 3c948aa. It reads committed source and
complete Git history. It reports the proposed file hash/changes and the selections
and local decisions that would remain. It keeps source publication separate from
accepted image state, including intermediate and older source versions. Missing
history, ambiguous ranges and genuine conflicts refuse. An exact local-only value
can override an incoming default; a later changed value returns to normal review.

A successful preview does not verify an installed image, run niri validation,
apply files or advance the accepted baseline. Its plan ID identifies the proposed
result; this source-preview ID cannot authorize native activation. See [ADR 0020](adr/0020-text-reconciliation-preview.md).

After booting a new image, plan acceptance of that installed baseline:

```sh
sysroot home file activate-plan --repo "$HOME/src/kedra"
sysroot home file apply --repo "$HOME/src/kedra" --plan PLAN_ID --activate-managed-file
```

This path requires the installed source commit and complete history in the
checkout. It checks that the planned target, source path and contents exactly
match the root-owned installed manifest and baseline file, then validates the
native candidate. The result shows the retained selection/local decisions and
pending publications. Apply recomputes that plan, reloads the managed file and
accepts the new baseline only after validation succeeds. Abort and keep-current
retain the previous accepted baseline; resume checks the installed baseline again.
This implementation is pending native qualification, including a real A-to-B
image transition. It does not stage an OS or turn advisory source preview into
installed-image authority. See [ADR 0022](adr/0022-installed-text-baseline.md).

Discard of one current change passes native VM 34237287511 at 5e238c7:

```sh
sysroot home file discard-plan CHANGE_ID
sysroot home file discard CHANGE_ID --plan PLAN_ID --activate-managed-file
sysroot home file recover
```

The plan restores an overlapping pinned selection when its range matches exactly;
otherwise it restores the public reference. Other edits and all selection/local
decisions stay intact. A stale plan or ambiguous selection boundary refuses.
The flag explicitly selects this managed file as the running niri session's
configuration, including when a previous runtime action selected another file.
Niri remains running. The command validates the candidate with the native parent
directory for relative includes, replaces the file with a private checkpoint,
requests a reload and waits for a new configuration-load event. Source and the
accepted image baseline do not change.

If interrupted, inspect `recover`, then choose `recover resume`, `recover abort`
or `recover keep-current`, each with `--activate-managed-file`. Resume validates
the planned file; abort only restores the exact checkpoint pair; keep-current
validates and retains a later edit. Pending recovery blocks other state mutations.
Keep-current preserves the private checkpoint for manual review. Failed native
validation or reload leaves recovery pending. The journal contains hashes and
file identities, never the complete live text. Successful validation removes its
private temporary file; interruption can leave `.sysroot-check-*` files for manual
review in the niri directory. Included files and concurrent external IPC actions
are outside this single-file transaction. Actual process-kill recovery passes
abort/resume/keep-current in the same native run; other interruption phases,
power loss and full disk remain unqualified.

Binary/non-UTF-8 files, CRLF, missing final newline, unsafe links/ownership and
files over 128 KiB or 8192 lines are refused. Preserve an unreadable store for
recovery; do not reset it. See [ADR 0019](adr/0019-ordinary-text-review.md).
