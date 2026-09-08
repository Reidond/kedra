# Noctalia home review

The current Linux commands review three effective Noctalia 5.0.1 settings:
`theme.mode`, `shell.button_borders` and `shell.input_borders`. They store only
these projected values in a private SQLite directory. Other exported settings,
credentials and raw application exports do not enter that store.

The native R03 subset passes desktop run 34192732970 at a2c0e63. There is no promoted
owner image yet. Selected-field patch export and source receipts are now implemented
with the actual Linux CLI/SQLite/source round trip passing run 34192732940.
The narrow discard path passes the actual desktop/CLI workflow in 34223972271
at a5cd96e, including stale-plan refusal and native metadata preservation. Generic file/line
review remains required before complete home management.
The running workstation must not be used for its enrollment experiment.

On a qualifying installed Kedra desktop, run as the ordinary desktop user:

```sh
mkdir -p "$HOME/.local/state"
sysroot home --state "$HOME/.local/state/kedra-home-review" init
sysroot home --state "$HOME/.local/state/kedra-home-review" status
sysroot home --state "$HOME/.local/state/kedra-home-review" stage theme.mode
sysroot home --state "$HOME/.local/state/kedra-home-review" selection
```

The state parent must exist, belong to the user and exclude writes by other users.
`init` requires a new directory; it refuses to overwrite any existing or incomplete
store. The starting baseline comes from the installed image's hash-checked source
manifest, not the current checkout or a user-supplied claim. A machine/user binding
prevents silently reusing another installation's review record.

`stage` captures and pins the selected field value. A later GUI write changes
the live value shown by `status` without changing that selection. `unstage` removes
the selection without changing the application. `keep-local` suppresses exactly
the current changed value; a different value returns to review. `app-own` is a
persistent policy for that field. `clear-local` removes either local policy.
Selection and local ownership cannot overlap.

Each command prints JSON with baseline/live values, selection and local policy.
`selection` emits only the pinned publishable field changes and accepted source
provenance. It and `status --last-capture` work without invoking Noctalia. Effective capture
uses Noctalia's native full-export command with bounded output and a timeout; only
the supported typed fields survive parsing. Raw parser failures are not printed.

Changed review records use compare-and-swap revisions and retain prior records.
Concurrent changes cause a conflict rather than overwriting an unseen decision.
Unknown schemas, damaged state, unsafe ownership and links fail; they never mean
an empty baseline. The existing store should be preserved for recovery. This
private state is not a security boundary against another unrestricted process
running as the same user, and it cannot authorize OS deployment.

## Export selected settings to source

From the reviewed state, create a new patch file:

```sh
sysroot home --state "$HOME/.local/state/kedra-home-review" export \
  --repo "$HOME/src/kedra" --output "$HOME/noctalia-selected.patch"
git -C "$HOME/src/kedra" apply --check "$HOME/noctalia-selected.patch"
```

The export uses committed source, checks the target and shared/host provenance,
and preserves comments and unselected values. It refuses a different source value
on the selected field or missing accepted/previously-recorded source ancestry.
Patch generation leaves HEAD, the real index and working files unchanged. Git may
store a new blob containing public source plus the selected typed values; it never
receives the private review history or raw live export. The patch output is newly
created with private permissions and is never overwritten. A failed write is not
a usable patch.

Review the patch and existing source edits. Apply it through normal Git review;
`git apply --3way` can use the recorded blob identities and may require explicit
conflict resolution. Commit only the intended paths reported by the export, taking
care to preserve unrelated staged changes. After that commit exists, record it:

```sh
sysroot home --state "$HOME/.local/state/kedra-home-review" record-source \
  --repo "$HOME/src/kedra" --commit "$(git -C "$HOME/src/kedra" rev-parse HEAD)"
```

The receipt checks the exact retained commit, target/provenance, selected values,
and source ancestry before clearing selection. It leaves newer live values and
local policy in place, and reports the committed values as pending deployment.
A working-file or index change alone is insufficient. It does not create or push
a commit, prove registry publication, or advance the accepted image baseline.

When an export reports `already_in_source: true`, the selected values already match
the committed source and the patch is empty. Record the matching source commit
instead of creating a duplicate edit. A shallow or rewritten checkout that lacks
the recorded history needs an explicit fetch/merge or rebind review; the command
does not guess ancestry or change branches.

## Review activation and discard

These commands support the default Noctalia profile and the installed managed
service. Native discard is qualified; the broader interruption and image-baseline
transition cases remain under qualification before a complete owner home workflow.

```sh
sysroot home --state "$HOME/.local/state/kedra-home-review" plan
sysroot home --state "$HOME/.local/state/kedra-home-review" apply --plan PLAN_ID
```

Read the observed/desired values first, then supply that exact `plan_id`. Any
change in review state, effective settings or installed baseline makes the old
plan unusable. Local-only and app-owned decisions remain separate; a conflicting
baseline does not overwrite live settings or advance the accepted baseline.

To discard only the unselected difference in one field, use `plan --discard
theme.mode`, then `discard theme.mode --plan PLAN_ID`. The destination is that
field's pinned selection, latest recorded publication, or accepted baseline in
that order. Other fields and the selection remain intact.

An interrupted operation blocks other review mutations. Inspect `recover`, then
choose `recover resume`, `recover abort`, or `recover keep-current`. Resume
requires the recorded plan and installed baseline to still match. Abort restores
only exact recorded file versions and refuses later edits. Keep-current preserves
the live file, clears the pending operation without advancing the baseline, and
leaves any private checkpoint for inspection. After validated cleanup the old
checkpoint may be gone, so use resume or keep-current. Never delete/reset the
review store to work around an error. See [R04 evidence](research/R04-activation/REPORT.md).
