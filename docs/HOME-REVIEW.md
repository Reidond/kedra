# Noctalia home review

For the separate niri line-selection workflow, see [ordinary text review](TEXT-REVIEW.md).

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
sysroot home init
sysroot home status
sysroot home stage theme.mode
sysroot home selection
```

The default store is `$XDG_STATE_HOME/sysroot/home`, or
`$HOME/.local/state/sysroot/home` when XDG_STATE_HOME is unset. The state directory
must already exist and exclude writes by other users; init creates the private
0700 sysroot parent. Use `--state /absolute/private/path` to choose a different
store or continue an existing one. Keep using that same option for every command
on a custom store. Profile symlinks and unsafe ownership are refused at setup.
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
sysroot home export \
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
sysroot home record-source \
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
sysroot home plan
sysroot home apply --plan PLAN_ID
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
the current settings, restarts and validates Noctalia, and clears the pending
operation without advancing the baseline. If startup fails or changes the
effective settings, recovery remains pending for review. Keep-current leaves any
private checkpoint for inspection. After validated cleanup the old
checkpoint may be gone, so use resume or keep-current. Never delete/reset the
review store to work around an error. See [R04 evidence](research/R04-activation/REPORT.md).

## Check the caller's accepted baseline after an OS change

The opt-in command below adds a `caller_home` summary after the installed update
helper succeeds. Run it as the ordinary user whose home is being checked; root
assessment is refused. The unchanged `sysroot update status` command reports only
deployment state.

```sh
sysroot update status --home
sysroot update status --home --home-state /absolute/private/review-store
```

`caller_home` applies only to the invoking UID and selected store. It does not
assess other users, source checkouts, live configuration or application health.
Noctalia and niri are reported separately:

| Status | Meaning and next step |
|---|---|
| `not_adopted` | The store is genuinely absent, or niri has no adoption record. Nothing is initialized. Adopt separately only after the required review. |
| `accepted_baseline_matches_installed` | The validated accepted baseline equals the installed public baseline, including provenance and supported content. Live edits may still exist. |
| `reconciliation_required` | The accepted baseline differs from the installed one. Review `sysroot home plan` for Noctalia or `sysroot home file activate-plan --repo /path/to/kedra` for niri, then explicitly apply an accepted plan. |
| `recovery_required` | A validated pending reservation and journal require explicit recovery. Inspect `sysroot home recover` or `sysroot home file recover` before choosing an action. |
| `unavailable` | State, profile or installed provenance is unsafe, busy, unreadable, corrupt or incompatible. Preserve the store, close competing operations and inspect the relevant home command's diagnostic. Never reset state to obtain a clean status. |

The overall status gives unavailable state precedence, then pending recovery,
then reconciliation. Read each group's status when one group is unadopted or
unavailable. Accepted and installed revisions are included only after validation;
selected values, local-only values, live contents and raw home journals are omitted.
With a custom store, insert the same `--state /absolute/private/review-store` after
`sysroot home` in the suggested next command. Suggestions never run automatically.
Application-profile overrides are refused, including a nondefault `XDG_STATE_HOME`
that would also redirect Noctalia's native state. Use `--home-state` to select a
different review store while keeping the qualified application profile.

This assessment performs no logical review-state or live-file change. Opening an
existing SQLite store can touch its sidecars and the coordination lock, so the
storage bytes are not promised to stay identical. It does not capture settings,
prepare a plan, validate or reload an application, initialize missing state, or
repair a journal. A helper failure returns an error without a combined successful
status. Its existing deployment fields and privilege checks remain independent of
`caller_home`; a match is not an OS health or home activation claim.
Recovery status validates the stored journal and reservation relationship without
opening native checkpoints. The explicit recovery command checks whether those
checkpoints and the current native file still permit a chosen recovery action.

Linux [workspace 34287224455](https://github.com/Reidond/kedra/actions/runs/34287224455)
and signed [R04 A/B/A 34287224388](https://github.com/Reidond/kedra/actions/runs/34287224388)
pass at `8288cc2` (2026-09-09, Europe/Kiev). The R04 fixture exercises the actual
public CLI for absence, preserved independent home
decisions, B mismatch/explicit acceptance, rollback mismatch, real process-kill
recovery and unavailable stores/profiles. It checks both groups independently:
after niri accepts B, Noctalia's unaccepted B keeps the overall assessment at
`reconciliation_required`; after explicit niri rollback handling, both match A.
Its generated guest has a root-owned,
`visudo`-checked grant limited to the installed helper with no arguments; that
unattended fixture is not evidence of interactive password authentication and
the grant is never part of the production or shared image.
