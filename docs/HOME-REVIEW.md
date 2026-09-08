# Noctalia home review

The current Linux commands review three effective Noctalia 5.0.1 settings:
`theme.mode`, `shell.button_borders` and `shell.input_borders`. They store only
these projected values in a private SQLite directory. Other exported settings,
credentials and raw application exports do not enter that store.

This implementation is undergoing native R03 qualification. There is no promoted
owner image yet. It does not implement generic file/line review, source export,
discard or live activation; those remain required before complete home management.
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
`selection` emits only the pinned publishable field changes. It does not write
the source checkout, create a public commit or claim deployment. Effective capture
uses Noctalia's native full-export command with bounded output and a timeout; only
the supported typed fields survive parsing. Raw parser failures are not printed.

Changed review records use compare-and-swap revisions and retain prior records.
Concurrent changes cause a conflict rather than overwriting an unseen decision.
Unknown schemas, damaged state, unsafe ownership and links fail; they never mean
an empty baseline. The existing store should be preserved for recovery. This
private state is not a security boundary against another unrestricted process
running as the same user, and it cannot authorize OS deployment.
