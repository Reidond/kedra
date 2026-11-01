# Managed home files

Live home files are ordinary writable files. A set's `home` declares the home
baseline, and its `managed` entries are the few files `sysroot home` keeps in step
with the image's baseline without losing your edits. `sysroot` itself knows no
application: each entry carries its own check and reload commands. The image
currently manages niri's `config.kdl`, Noctalia's `config.toml` and the Noctalia
Adwaita palette ([desktop.kedra](../../../../system/desktop.kedra)); an excerpt:

```text
home = {
    ".config/niri/config.kdl" = managed {
        content = file("./home/niri/config.kdl");
        check = ["/usr/bin/niri", "validate", "--config", "{file}"];
    };
    ".config/noctalia/config.toml" = managed {
        content = file("./home/noctalia/config.toml");
        check = ["/usr/bin/noctalia", "config", "validate", "{file}"];
        reload = ["/usr/bin/systemctl", "--user", "try-restart", "kedra-noctalia.service"];
    };
};
```

- The path is relative to the home directory and to the image baseline in
  `/usr/share/sysroot/home/default`. A plain `home` entry (inline text or a file)
  only seeds new accounts; a `managed` entry must be a `file("./…")` resource,
  which `capture` writes back to.
- `check` validates a candidate before it replaces the live file. `{file}` is the
  candidate's path; the candidate sits in the live file's directory under a name
  no application loads (`.sysroot-check-<token>`), so relative includes resolve.
  `sysroot derive` also runs each check on the image baseline, as `nobody` with a
  scratch home and no network, so an image with an invalid baseline is not built.
- `reload` runs once after the files that name it changed. niri needs none: it
  reloads its configuration itself when the file changes.
- Both are argv with an absolute program, never shell text. Absent means none.

The source plan generates the image's list,
`/usr/share/kedra/home-files.toml`, from the `managed` entries; changing it needs
an image update.

To manage another file, put it beside the module and declare it `managed` in a
set the target uses.

## Commands

Run them as the ordinary desktop user, never with sudo:

```sh
sysroot home status                 # every managed file: live vs baseline, with a diff
sysroot home status .config/niri/config.kdl
sysroot home update                 # merge baseline changes, check, reload
sysroot home capture .config/niri/config.kdl --repo ~/src/kedra
```

Each accepts `--json`. Before the subcommand, `sysroot home --state DIR`
selects another state directory and `sysroot home --root DIR` reads the data
file and baselines from an unpacked image root instead of `/` (tests and the lab
use it).

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
`XDG_STATE_HOME`, or an account created without it) B is unknown: a live file
equal to N is checked, reloaded and recorded, a missing one is created, and a
different one gets N beside it as `<file>.kedra-new` to merge by hand.

### capture

`capture PATH --repo CHECKOUT` copies one live file into the checkout's
`system/` file the installed image built it from, as the image's source manifest
records it (for example `system/home/niri/config.kdl`). It writes the working
tree only; it never stages or commits. Pick the lines to keep with
`git add -p -- <source path>` and discard the rest with `git restore`. The
checkout must hold `system/catalog.kedra`.

## Privacy and safety

- Only declared files are ever read. Credentials stay out unless a `managed`
  entry names them, which the source review of that module would show; the
  language refuses `.ssh`, `.codex`, `.claude`, `.cache` and credential files.
- `status` diffs and `capture` copies show your live content to you. Review a
  diff before staging it: anything you stage can become a public Git object.
- Writes go through `sysroot_linux::replace_file`: prepared privately beside the
  file, exchanged atomically and checked against the content that was read.
  Conflict markers never enter a live file.
- State lives in `$XDG_STATE_HOME/kedra/home` (default
  `~/.local/state/kedra/home`), mode 0700: `applied/` and `pending/` hold image
  baselines, which are public; `lock` serializes updates. It holds no live
  content.
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
as a private image root and runs `sysroot home --root DIR update` in the test
account, so the lab exercises the same merge, check and reload.

## Verification

`cargo test -p sysroot --test e2e_home` runs the public CLI over a generated
image root, home and checkout with real Git (record, fast-forward, merge, failing
check, shared and retried reloads, conflict and resolution, link refusal, capture
without staging). The container harness case `native::home_files_update` runs
the real nested session: niri loads a merged file by itself, Noctalia restarts
with its merged file, niri's validator refuses an invalid baseline and a conflict
leaves the live file untouched. Both pass locally, and the harness also runs on
every release candidate. Recovery after an update is killed midway is not
tested; see [STATUS](STATUS.md).
