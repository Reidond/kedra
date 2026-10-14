---
name: kedra-home
description: Maintain managed home files - the data-driven list, three-way update, capture into the checkout, local edits and privacy.
---

# Managed home files

Read usr/src/kedra/docs/HOME.md, usr/src/kedra/docs/ARCHITECTURE.md ("Writable
home") and references/state-model.md. Check usr/src/kedra/docs/STATUS.md for what
actually passed.

The image lists managed files in `usr/share/kedra/home-files.toml` (schema1):
`path` plus optional `check` and `reload` argv with an absolute program and a
`{file}` placeholder. `sysroot home` (crates/sysroot/home.rs, home/) knows no
application. Never add app-specific Rust (version allow-lists, IPC clients,
process-tree or service-unit checks, field projections); put behavior in the data
file or in the application's own CLI. `sysroot doctor` runs the same checks on
live files as `home_file:<path>`.

`update` merges with `git merge-file -p --diff3` over B (last applied baseline,
`$XDG_STATE_HOME/kedra/home/applied/`), L (live) and N (installed baseline). Check
the candidate beside the live file (`replace_file::Directory::validate_bytes`),
replace atomically (`prepare`/`publish`/`finish`, keeps mode/group/label, refuses
links), run each distinct reload once, then record N. A conflict writes
`<file>.kedra-new` and `pending/<path>`; deleting the conflict file resolves it.
Never write conflict markers into a live file, never overwrite a `.kedra-new` an
update did not write, never record N before check and reload succeed. New
accounts get B from `/etc/skel/.local/state/kedra/home/applied/` (assemble.sh,
lab overlay-apply.sh); without B a differing file becomes a conflict, not an
adoption.

`capture` writes one live file to the checkout's source path (target overlay copy
if the checkout has one, else `etc/skel/`) and never stages or commits; the owner
picks lines with `git add -p`. Only declared files are read. Diffs show the
owner's live content: review before staging, since staged text can become a
public Git object. No symlinked or whole-home capture, no SQLite review store.

Facts behind the shipped entries (2026-10-14, Fedora44 qemu-arm64 stable
c36c10c: niri-26.04-1, noctalia-5.2.1-1, git-2.56.0):
- `noctalia config validate [path]` validates one file when given a file; with no
  path it merges every `*.toml` in the config dir plus state overrides. The shell
  loads every `*.toml` there, so candidates and `.kedra-new` must not end in
  `.toml`. Unknown enum values are warnings (exit0), not failures.
- niri resolves relative includes against the config file's directory, so check
  candidates in the live directory. niri reloads its config itself; no reload argv.
- Noctalia GUI overrides in `~/.local/state/noctalia/settings.toml` win over
  `config.toml` and are not managed.
Source: `noctalia config validate --help` and `niri validate` runs in that image,
crates/sysroot/tests/e2e_home.rs, tests/container/home_files_tests.rs.

Test with the real CLI only: `cargo test -p sysroot --test e2e_home` (generated
image root via `--root`, real Git) and the container case
`native::home_files_update` (nested session: niri loads the merged file, Noctalia
restarts, check refusal, conflict and resolution). `kedra-lab sync` stages the
working tree as a `--root` image and runs the same update. Kill-at-publication
recovery is not qualified: an interrupted update can leave a private
`.sysroot-activation-<token>` directory holding the previous content.

History: until 2026-10-14 home management was a Noctalia three-field projection
with a version allow-list, a niri line-review store with IPC reload proof and a
qemu-arm64 niri artifact record (`home-artifacts.json`). The owner replaced it
with this data-driven model; no installed review state needed migration. See
worklog WL-20261014-HOME-FILES-01.
