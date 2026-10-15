# Managed home files

Live home files are ordinary writable files. The image names the few it manages
in one data file, [`usr/share/kedra/home-files.toml`](../../../share/kedra/home-files.toml),
and `sysroot home` keeps them in step with the image's baseline without losing
your edits. `sysroot` itself knows no application: each entry carries its own
check and reload commands.

```toml
schema_version = 1

[[file]]
path = ".config/niri/config.kdl"
check = ["/usr/bin/niri", "validate", "--config", "{file}"]

[[file]]
path = ".config/noctalia/config.toml"
check = ["/usr/bin/noctalia", "config", "validate", "{file}"]
reload = ["/usr/bin/systemctl", "--user", "try-restart", "kedra-noctalia.service"]
```

- `path` is relative to the home directory and to the image baseline in
  `/usr/share/sysroot/home/default`, which comes from root `etc/skel/` and the
  target overlay `usr/src/kedra/image/targets/<target>/etc/skel/`.
- `check` validates a candidate before it replaces the live file. `{file}` is the
  candidate's path; the candidate sits in the live file's directory under a name
  no application loads (`.sysroot-check-<token>`), so relative includes resolve.
- `reload` runs once after the files that name it changed. niri needs none: it
  reloads its configuration itself when the file changes.
- Both are argv with an absolute program, never shell text. Absent means none.

To manage another file, add its baseline under `etc/skel/` and an entry here.
Changing the data file needs an image update; it is not a home file.

## Commands

Run them as the ordinary desktop user, never with sudo:

```sh
sysroot home status                 # every managed file: live vs baseline, with a diff
sysroot home status .config/niri/config.kdl
sysroot home update                 # merge baseline changes, check, reload
sysroot home capture .config/niri/config.kdl --repo ~/src/kedra
```

Each accepts `--json`. `--state DIR` selects another state directory and
`--root DIR` reads the data file and baselines from an unpacked image root
instead of `/` (tests and the lab use it).

### status

For each managed file `status` reports whether the live file exists, whether it
matches the installed baseline, whether the last update recorded the installed
baseline (`applied`: `current`, `outdated` or `none`) and whether a conflict is
waiting. When the live file differs it prints the unified diff from the
baseline to the live file, made by `git diff --no-index` on private copies.

### update

`update` takes three versions of each file: B, the baseline the last update
merged (recorded per user); L, the live file; and N, the installed baseline.

| Situation | Result |
|---|---|
| N equals B | `up_to_date`: nothing changes, local edits stay |
| L equals B | `updated`: L becomes N |
| L equals N | `recorded`: L is checked and reloaded, then N is recorded |
| both changed | `git merge-file` merges them: `merged` |
| both changed the same lines | `conflict`: see below |
| L missing and no record | `created` from N |

A merged or new file passes its `check` before it replaces L, through an atomic
replacement that keeps the file's mode, group and SELinux label and refuses
symbolic links, hard links, devices and group- or world-writable files. If the
check fails, L stays as it was (`check_failed`). Reloads run after all writes;
N is recorded as applied only after its write, check and reload succeeded. If a
reload fails (`reload_failed`), L already holds the new content but the record
stays outdated, and the next `update` checks and reloads L again before
recording N: whenever the record is missing or outdated and L already holds
the result, the application is made to load it. `update` exits nonzero when any
file needs attention.

On a conflict, L and the running application stay as they were and the merge
with conflict markers (`<<<<<<< live`, `||||||| last applied baseline`,
`>>>>>>> new baseline`) goes beside it as `<file>.kedra-new`. No application
loads that name. Resolve it in L, then delete `<file>.kedra-new`; the next
`update` reports `resolved`, checks and reloads L as it is and records N. To
keep your version, delete `<file>.kedra-new` without changing L. While the
conflict file exists, `update` reports `unresolved` for that file. An existing
`.kedra-new` that no update wrote is never overwritten.

New accounts record their baseline at creation: the image copies the baseline
into `/etc/skel/.local/state/kedra/home/applied/`. Without a record (another
`XDG_STATE_HOME`, or an account from before this change) B is unknown: a live
file equal to N is checked, reloaded and recorded, a missing one is created, and a different one gets
N beside it as `<file>.kedra-new` to merge by hand.

### capture

`capture PATH --repo CHECKOUT` copies one live file into the checkout's source
path for it: the target overlay's `etc/skel/` copy when the checkout has one for
the installed target, otherwise the shared `etc/skel/`. It writes the working
tree only; it never stages or commits. Pick the lines to keep with
`git add -p -- <source path>` and discard the rest with `git restore`. The
checkout must declare the installed target
(`usr/src/kedra/image/targets/<target>/target.toml`).

## Privacy and safety

- Only declared files are ever read. Credentials stay out unless a data-file
  entry names them, which the source review of that file would show.
- `status` diffs and `capture` copies show your live content to you. Review a
  diff before staging it: anything you stage can become a public Git object.
- Writes go through `sysroot_linux::replace_file`: prepared privately beside the
  file, exchanged atomically and checked against the content that was read.
  Conflict markers never enter a live file.
- State lives in `$XDG_STATE_HOME/kedra/home` (default
  `~/.local/state/kedra/home`), mode 0700: `applied/` and `pending/` hold image
  baselines, which are public; `lock` serializes updates. It holds no live
  content, review database or journal. The previous `~/.local/state/sysroot/home`
  review store is ignored and can be deleted.
- An update interrupted between preparation and cleanup can leave a private
  `.sysroot-activation-<token>` directory beside a file; it holds the previous
  content. Inspect and delete it.

Noctalia also keeps GUI overrides in `~/.local/state/noctalia/settings.toml`.
They win over `config.toml` and are not a managed file.

## Related commands

`sysroot doctor` runs each declared check on the live file and reports it as
`home_file:<path>`. `sysroot update status --home` adds a read-only summary
(`up_to_date`, `update_available` or `conflict`) without creating state.
After an OS update and reboot, run `sysroot home update`; staging an image
never changes a home file.

In the container lab, `kedra-lab sync` stages the working tree's home baselines
as a private image root and runs `sysroot home update --root` in the test
account, so the lab exercises the same merge, check and reload.

## Verification

- `cargo test -p sysroot --test e2e_home`: the public CLI over a generated image
  root, home and Kedra checkout, with real Git: record, fast-forward, merge,
  failing check, shared reload, a failed reload retried by the next update,
  conflict, resolution, the no-record conflict,
  link refusal and capture into the shared and overlay paths without staging.
- `native::home_files_update` in the container harness: in the real nested
  session, the account starts recorded, niri loads a merged file by itself,
  Noctalia restarts with its merged file, niri's own validator refuses an invalid
  baseline and a conflict leaves the live file and niri untouched until resolved.

Recorded results are in [STATUS](STATUS.md).
