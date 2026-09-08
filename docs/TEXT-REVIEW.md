# Review niri configuration by line

This interface is prepared for qualification. It supports the ordinary
`.config/niri/config.kdl` file on an installed Linux Kedra desktop. It does not
apply or discard live text changes yet. Other files and niri includes remain
unmanaged. Use your editor for live changes; niri's normal validation/reload
behavior applies.

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
Binary/non-UTF-8 files, CRLF, missing final newline, unsafe links/ownership and
files over 128 KiB or 8192 lines are refused. Preserve an unreadable store for
recovery; do not reset it. See [ADR 0019](adr/0019-ordinary-text-review.md).
