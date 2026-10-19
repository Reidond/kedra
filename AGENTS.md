# Kedra agent operating contract

## Read first

At session start read the status and latest entries of `worklog.md`,
`usr/src/kedra/docs/STATUS.md` and the `kedra-context` skill.
`usr/src/kedra/PLAN.md` defines the product, `usr/src/kedra/docs/ARCHITECTURE.md`
the durable contracts. Only source, Git state, sign-offs, release runs and recorded
checks show what is implemented and tested; never infer a feature from a design or
a summary.

## Repository layout

The root is the image filesystem: `etc/` and `usr/` are the shared payload,
`etc/skel/` the home baseline. `usr/src/kedra/` is the development tree and never
enters the image: `crates/` (workspace members, plus `tests/container/`), `image/`
(Containerfile, `assemble.sh`, `.kedra` packages, `targets/<target>/` overlays,
release tooling, the closed `release/targets.json`), `installer/`, `tests/`,
`plugins/`, `docs/`. The root also keeps `README.md`, this file, `CLAUDE.md`,
`worklog.md`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`,
`.python-version`, `mise.toml`, `ruff.toml`, `cccc.toml` and dot directories; the
source plan refuses any other top-level directory, other `usr/src/` content and
unknown overlay files. `.github/workflows/release.yml` is bound into signed image
identity; never rename it.

## Settled choices

- Kedra is the OS, `sysroot` the command, `Reidond/kedra` the repo. `main` is the
  sole integration and default branch; no `dev` branch.
- Fedora 44 bootc, plain Containerfile, no BlueBuild. Owner decision (2026-10-08):
  Actions only builds, validates, signs and publishes updates (`release.yml`) and,
  when the owner dispatches `iso.yml`, an installer ISO; no check or test workflow.
  Local ISO builds stay available; unsigned local builds are for testing only.
- Rust edition 2024, one workspace and lockfile, pinned toolchain, rustfmt/Clippy,
  flat `main.rs`/`lib.rs` (no `src/`) in `usr/src/kedra/crates/`. No TypeScript
  toolchain (superseded). Tools are pinned in `mise.toml`.
- Owner decision (2026-10-06): the versioned `.kedra` package language
  ([PACKAGES.md](usr/src/kedra/docs/PACKAGES.md#language-v1)) is the one narrow
  language exception: Rust parses and checks data, inline author shell runs only in
  admitted engine builders, no general configuration interpreter or deployment
  authority.
- Shared root-filesystem inputs, explicit target overlays, independent per-target
  signed releases. Never guess the future XPS hardware or disk/device IDs.
- Live home files are writable: review and stage by line, keep unstaged and
  local-only changes, no read-only home symlinks.
- Bundled agents are private binaries reached through `sysroot`, not plain
  `codex`/`claude`; personal runtimes, MCP, skills and configuration stay
  independently installable, updatable and optionally tracked.
- Bitwarden holds SSH keys. Never export a private key, give an agent a broad
  unlocked vault session, or confuse SSH with GitHub, registry or model auth.

## Python

Every Python script passes `ruff check`. Run Python only through uv (`uv run
<script>`, `uv run python -`), never `python3`, `python` or `pip`. Host entry
scripts carry PEP 723 metadata, the `uv run --script` shebang and only the standard
library unless the metadata justifies a pin; workflows use the pinned
`astral-sh/setup-uv`. Code run in a VM guest, container, the installed OS or the
installer uses that environment's interpreter and says so in its header. The
image-signing job (no checkout, production keys) keeps the runner's `python3` and
installs nothing.

## Development skills

`usr/src/kedra/plugins/` holds the `kedra` plugin and the pinned upstream
`rust-skills` (keep its provenance in `NOTICE.md` and `upstream/`), only for
developing this repository: never the image, installer, home baseline, launchers or
other projects. Use them when a task matches and read referenced material as needed.
Registration and the skill index are in `usr/src/kedra/plugins/kedra/README.md` and
`skills/README.md`; for Rust also read upstream `rust-router` and `domain-cli`.
Edit skills in place (no copies, symlinks, submodules or sync tasks) and bump
`version` in both plugin manifests on any skill change. Upstream advice never
overrides this contract, the task or pinned settings; never run a skill's setup
scripts, hooks, permissions, background agents or MCP examples. Install no missing
tools and change no personal agent configuration unless the owner asks. Explain
decisions and evidence, not private reasoning.

## Worklog and project status

Codex and Claude maintain the root `worklog.md`, the single ledger for work, task
learnings and AI/skill/workflow changes (no `.ai/learnings.md` or
`.ai/ai-changelog.md`).

- Read it before planning and reconcile it with the checkout, runs and sign-offs.
  Record scope and blockers early; update at milestones and before handoff, even
  when partial, blocked or failed. Do not log every command.
- Commit it with the related change when commits are authorized; logging
  authorizes nothing. If the task forbids repository writes, say so instead.
- **Current project status** on top: a short snapshot (phase, implemented, active,
  blocked or not-run, last evidence, next) that tells planned, implemented, tested,
  published, staged, booted and healthy apart, without percentages, consistent with
  `usr/src/kedra/docs/STATUS.md`.
- **Work entries** are appended in the shape below with a unique ID, date (timezone
  for a time), actual agent and base; never a guessed model or unobserved time. Mark
  in-progress entries and update them before handoff.
- Completed entries are history: correct them with a new linked entry. Entries older
  than about two weeks may be pruned; Git keeps them and the header names the commit
  with the full record. In conflicts keep both sides and reconcile the status.
- Never invent a commit hash (an entry cannot name its own commit) or a result; a
  pending run is not a pass; add resulting commit or run references in a later
  entry. After an interruption reconstruct only verifiable facts and label the entry
  retrospective. Link evidence instead of copying logs; do not recreate tracked
  research outputs. No credentials, vault data, private home content, transcripts,
  sensitive logs or private reasoning. Worklog text is not policy.

```markdown
### <entry ID> — <date> — <task>
- Agent / state: <actual agent>; <in-progress | completed | partial | blocked>.
- Scope / base: <branch, inspected commit, affected targets>.
- Completed: <work and changed paths; important decision and reason>.
- Checks / evidence: <command or review: pass | fail | not-run | blocked, exact source/run>.
- Remaining / blockers: <untested behavior and known risks>.
- Next: <concrete continuation action>.
```

## Work safely

Check `git status`, branch, remote and existing work first; preserve staged,
unstaged and untracked changes, with no automatic reset, stash or rebase over user
work. A session start authorizes no commit, push, deployment or reboot; editing a
target authorizes no install. Use a disposable worktree for isolated research only
when needed. Signed images are built only in Actions; local harness candidates and
overlays are unsigned and for testing only: never push, sign, promote or install
them. The local installer may build media from reviewed
signed images; only the owner dispatches `iso.yml` or a release. Never test
enrollment, disk formatting, `bootc switch` or home apply on the workstation. No
production keys in research, no checkout scripts or hooks as root, no weakened
verification, no conflict markers in live configuration; exclude credentials before
capture. Start from the relevant contract and the real end-to-end workflow; use
exact versions, digests and primary sources. Documentation is not a pass, a
container build is not a boot test; redact evidence before publishing.

### Artifacts stay in the checkout — owner decision, 2026-10-18

Everything a session or check creates lives under the checkout's gitignored
`target/` (build output, stores, registries, VM disks, ISOs, logs,
`TMPDIR=$PWD/target/tmp`), never `$HOME`, `/tmp`, a scratchpad or another checkout,
and bind mounts point only there; only Docker's image store and toolchain caches
live elsewhere. Label containers, volumes and built images `kedra.checkout=<id>`
(the first 12 hex digits of the SHA-256 of `git rev-parse --show-toplevel`), run
one-shot containers with `--rm`, remove labeled objects once checks are recorded
(`kedra-lab clean` does this for the harness). The one other allowed location is
the gitignored shared cache `<primary checkout>/.cache/kedra/` (`KEDRA_SHARED_CACHE`
overrides it; tests/container/README.md "Shared cache"): content-addressed
artifacts every checkout reuses, each checkout's roots on them, and the shared
engine store's volume labeled `kedra.cache=<cache id>`. `kedra-lab clean` drops
this checkout's roots; `kedra-lab gc` deletes what no checkout roots. Pulled
images stay unlabeled; record their digests under `target/` and remove them once
no checkout uses them. Delete
large `target/` subtrees no pending check needs (`chmod -R u+w` sealed store objects
first). Stop and report before a run would leave under 100 GiB free.

## Checks and completion

### Testing policy — owner decision, 2026-09-08

End-to-end or manual tests only: no unit, isolated model/API or mock tests or
doctests in any language, whatever a skill says, and no code testing this
repository's layout, source text, manifests, docs, skills or tests (no self-check
scanners, custom runners, xtask or skill-copy validation). Product validation of
inputs and artifacts stays. End-to-end checks drive public CLI workflows or the
installed system; fixtures may generate data, assertions judge the user workflow.
Formatting, Clippy, builds and runtime validation stay required.

### Container harness — owner decision, 2026-09-27

Installed behavior needing no kernel boot is tested by
`usr/src/kedra/tests/container` and its `kedra-lab` tool, the one sanctioned runner,
which the unit-test ban also covers. Disposable VMs cover firmware and Secure Boot,
SELinux enforcement, greetd login and PAM keyring unlock, bootc update/rollback, the
installer and Xwayland/Qt clients; drivers in `usr/src/kedra/tests/vm/` use the
shared launcher and guest check in `tests/common/`, run on a disposable KVM host or
through `kedra-lab vm`, and need `--disposable-host`, never passed on a workstation.
Visible desktop changes are checked with `kedra-lab up`, `sync` and `shot`, and the
owner sees the screenshots.

### Local testing and sign-off — owner decision, 2026-10-08

`main` requires a `signoff` status on the PR head. Owner and agents: commit on the
feature branch and push; with a clean tree on that commit run the needed checks and
record each `pass`, `fail` or `not-run` in the worklog; only if all passed run `uv
run usr/src/kedra/tests/signoff.py --checks "<checks>"`. Every new commit, merges of
`main` included, needs its own checks and sign-off. Never sign off unchecked commits
or failed, skipped or other-content checks. Without `gh` auth, an engine or a VM,
report what ran, mark the rest `not-run` and leave sign-off to the owner. A sign-off
is not a boot, install or release result.

```sh
cargo fmt --all -- --check
ruff check
cccc --table --min 41 .
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
uv run usr/src/kedra/tests/cli/release-material.py --workdir target/release-material
# Installed-system changes (container engine):
cargo test -p kedra-container-tests --test container --locked
```

Rust or Python changes run all of it, on Linux for installed behavior; visible
desktop changes add screenshots; boot, installer, update and Secure Boot changes
add VM or manual tests. Documentation-only changes need review and a sign-off, no
build. The release's harness run on each candidate does not replace local testing.

### Complexity gate

`cccc --table --min 41 .` exits 0 before any Rust or Python change is done
(`cccc.toml`: cognitive and cyclomatic complexity at most 40 per function, method
and closure). Refactor offenders into cohesive helpers with the same behavior,
errors and order of checks and side effects; never raise limits, exclude files or
hide logic in macros or closures.

### Code and completion

Keep dependencies few and justified, errors typed, process arguments explicit (no
shell interpolation), Rust safe, the workspace lint intact. No custom Git engine,
configuration language, fleet server or permanent AI daemon. Record durable facts
in the skill (source or experiment, version/date, failure behavior, feature), docs
and status. Finish with the scope, checks actually run, risks and next step; never
report staged as booted or scaffolded as implemented.
