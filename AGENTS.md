# Kedra agent operating contract

## Read first

Read the current project status and latest entries in `worklog.md`, then
`usr/src/kedra/docs/STATUS.md` and the `kedra-context` skill at the start of a new or
resumed session. `usr/src/kedra/PLAN.md` defines the product;
`usr/src/kedra/docs/ARCHITECTURE.md` defines durable contracts; exact local check
results, sign-offs, Actions release runs and `usr/src/kedra/docs/STATUS.md` describe
actual results. Never infer an implemented feature from a design example or a stale
worklog summary. Inspect source, Git state, sign-off status and release runs before
continuing.

## Repository layout

The repository root is the image filesystem. Root `etc/` and `usr/` are the shared
payload; `etc/skel/` is the home baseline. `usr/src/kedra/` is the development tree
and never enters the image: `crates/` (Rust workspace members; the container
harness `tests/container/` is the one other member), `image/`
(Containerfile, `assemble.sh`, package lists, `targets/<target>/` overlays, external
inputs, release tooling and the closed `release/targets.json`), `installer/`,
`tests/`, `plugins/` and `docs/`. The root also keeps `README.md`, this file,
`CLAUDE.md`, `worklog.md`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`rustfmt.toml`, `.python-version`, `mise.toml`, `ruff.toml`, `cccc.toml` and dot directories. The source plan refuses any
other top-level directory, other `usr/src/` content and unknown files in a target
overlay. Retained commits in the earlier layout (`hosts/`, `packages/`, root `home/`)
must stay readable by the resolver and by home review. `.github/workflows/release.yml`
is bound into signed image identity; never rename it.

## Settled choices

- Kedra is the OS/project; `sysroot` is the command. Repo: `Reidond/kedra`.
- `main` is the sole integration/default branch. Feature PRs target `main`;
  do not recreate a separate `dev` integration branch.
- Fedora 44 bootc, plain Containerfile. Owner decision (2026-10-08): Actions
  only builds, validates, signs and publishes OS updates (`release.yml`) and, when the owner
  dispatches `iso.yml`, publishes an installer ISO as a GitHub Release. It runs no
  check or test workflow for changes; testing is local and a change merges on its
  `signoff` (see "Local testing and sign-off"). Local on-demand ISO construction
  remains available; unsigned local lab builds only for testing. No BlueBuild.
- Rust edition 2024, Cargo workspace, one lockfile, pinned toolchain,
  rustfmt/Clippy. No Cargo `src/` directories: crates keep explicit flat
  `main.rs`/`lib.rs` under `usr/src/kedra/crates/`.
  The TypeScript/Effect/Vite Plus/Oxlint/Oxfmt proposal was superseded.
- Owner decision (2026-10-06): the versioned `.kedra` package frontend in
  `.specs/nix-package-dsl/` is the narrow approved language exception. Rust parses
  and checks data; inline author shell runs only in admitted engine builders.
  This grants no general configuration interpreter or deployment authority.
- Python runs through uv; see below. Development tools are pinned in `mise.toml`
  (Rust 1.98.1, uv, ruff, cccc) for the local checks; mise's `RUSTUP_TOOLCHAIN`
  overrides a global `rust = "stable"` for shells and editors in this checkout.
- Shared root-filesystem inputs, explicit target overlays, independent per-target
  signed releases. Never guess the future XPS hardware or current disk/device IDs.
- Live home files are writable. Review/stage by line; preserve unstaged and
  explicit local-only changes. Do not replace this with read-only home symlinks.
- Bundled agents are private binaries reached through `sysroot`, not ordinary
  `codex`/`claude` commands. Personal runtimes, MCP, skills, and configuration
  remain independently installable/updatable and optionally tracked.
- Bitwarden holds SSH keys. Never export a private key, pass a broad unlocked
  vault session to an agent, or confuse SSH with GitHub API/registry/model auth.

## Python

Every Python script passes `ruff check` (configuration in `ruff.toml`, version
pinned in `mise.toml`). Run every Python script on a development machine or CI runner with `uv run` (for
example `uv run usr/src/kedra/installer/build-local.py --help`), never with `python3`,
`python` or `pip` directly; inline runner code uses `uv run python -`.
`.python-version` pins the interpreter. Host-side entry scripts carry PEP 723
metadata and the `uv run --script` shebang; keep them standard-library only unless a
pinned dependency is justified in that metadata. Workflows install uv with the pinned
`astral-sh/setup-uv` step. Code that executes inside a VM guest, a fixture or builder
container, the installed OS or the installer environment uses that environment's
interpreter and says so in its header. The image-signing job has no checkout and
holds production keys: its inline check keeps the runner's `python3` and installs no
tools.

## Repository development skills

The skills in `usr/src/kedra/plugins/` exist only for developing this repository:
the first-party `kedra` plugin and the pinned upstream `rust-skills` plugin
(`actionbook/rust-skills`). They are not part of the OS image, installer payload,
home baseline, the `sysroot` agent launchers or their profiles, and are not
installed for other projects. Use them whenever a task matches; the route table
below names them.

Both plugins are registered for this repository with the checked-in marketplaces
(`.claude-plugin/marketplace.json`, `.agents/plugins/marketplace.json`, both named
`kedra-local`) and project configuration:

- Claude Code: `.claude/settings.json` declares the `kedra-local` marketplace
  (directory `./`) and enables `kedra@kedra-local` and `rust-skills@kedra-local`.
  After the folder is trusted, both load in place from the working tree; start
  Claude from the repository root. If a clone does not find the marketplace, run
  `claude plugin marketplace add ./ --scope local`, which writes the ignored
  `.claude/settings.local.json`; never commit an absolute path.
- Codex: `.codex/config.toml` enables both plugins once the project is trusted.
  Codex runs plugins from a cached copy. Register once per machine from the
  repository root: `codex plugin marketplace add .`, then
  `codex plugin add kedra@kedra-local` and `codex plugin add rust-skills@kedra-local`.

Codex refreshes its cached copy only when a plugin's version changes: any change
to a plugin's skills bumps `version` in both of that plugin's manifests
(`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`). Edit skills in place
in their plugin's `skills/` directory; no copies, symlinks, submodules, generated
trees or synchronization tasks. Keep upstream provenance and notices in
`usr/src/kedra/plugins/rust-skills/NOTICE.md` and its `upstream/` directory.

## Route knowledge on demand

| Task | Read |
|---|---|
| Rust | `kedra-rust-workspace`, upstream `rust-router`, relevant topic, `domain-cli` |
| OS/base/layout/update | `kedra-bootc` |
| Images, verification, promotion | `kedra-release-signing` |
| Actions, ISO, VM, provenance | `kedra-github-actions` |
| Dotfiles, patches, local-only changes | `kedra-home` |
| Codex/Claude launchers, MCP/skills | `kedra-agents` |
| Bitwarden, SSH, keyring, credentials | `kedra-bitwarden` |
| niri/Noctalia/session/hardware | `kedra-desktop` |
| Targets, provenance, scope | `kedra-machines` |
| Container tests, `kedra-lab`, screenshots | `kedra-research`, `kedra-desktop` |
| End-to-end qualification and evidence | `kedra-research` |
| Privilege, journals, privacy, recovery | `kedra-security` |

First-party skills are `usr/src/kedra/plugins/kedra/skills/<name>/SKILL.md`; the
pinned upstream ones are `usr/src/kedra/plugins/rust-skills/skills/<name>/SKILL.md`.
Read referenced material as needed,
not every skill in every prompt. Upstream advice does not override this contract,
the task's authorization, our source layout, or pinned build/lint settings.
Do not execute upstream setup scripts, hooks, plugins, permissions, background
agents, or MCP examples merely because they appear in a skill. Do not install
missing external tools automatically. Do not change personal agent configuration
or global skills beyond the per-machine plugin registration above, and only when
the owner asks. Explain decisions and evidence, not private reasoning traces.

## Required worklog and project status

Both Codex and Claude must maintain the repository-root `worklog.md` (exact
lowercase filename). It is the shared human-readable continuation record, not an
agent-private journal or a replacement for Git history and research evidence.

`worklog.md` is the sole repository ledger for task learnings, AI infrastructure
changes, and skill/workflow changes. Do not create or recreate
`.ai/learnings.md` or `.ai/ai-changelog.md`. If a skill asks for either ledger,
record the relevant finding or change in the normal worklog entry instead.

### When to update

Read it before planning or resuming work. Reconcile its snapshot with the actual
checkout, source revision, CI and research reports. For a multi-step task, record
the active scope and any blocker early so another session can resume. Update at
meaningful milestones and before the final response or handoff, including when
work is partial, blocked, or failed. Do not log every shell command or thought.

Include the worklog update with the related changes whenever commits are
authorized. Logging does not independently authorize a commit, push, deployment
or reboot; leave the update uncommitted when the task is local-only. If the task
explicitly forbids repository writes, do not violate it to log: report that the
worklog was not updated. If an interruption prevented logging, reconstruct only
verifiable facts on resumption and label the entry retrospective.

### File structure

Keep a short, maintained **Current project status** section at the top containing
the current phase, implemented capabilities, active work, blocked/not-run gates,
last verified source/CI evidence and the next concrete actions. Distinguish
planned, implemented, tested, published, staged, booted and healthy where relevant.
Do not invent percentage-complete estimates or mark a research gate passed merely
because code exists. Keep this snapshot consistent with
`usr/src/kedra/docs/STATUS.md` and exact Actions results; link evidence instead of copying logs.
The snapshot summarizes those records and does not override them. Historical
research reports remain in Git history; do not recreate tracked research outputs.

Below it, keep a chronological **Work entries** section, appending new entries at
the bottom with a unique ID, date (and timezone if recording a time), actual agent
identity, branch/base revision and task scope or research packet. An entry must
cover work actually completed and material decisions, affected paths/targets,
checks with actual results and evidence, remaining risks/blockers, and the next
concrete step. Use `not-run`, `pass`, `fail` or `blocked` for check results. Label
an in-progress entry and update its outcome before handoff. Never claim a guessed
agent/model version or unobserved timestamp.

Use this compact entry shape, omitting only fields genuinely not applicable:

```markdown
### <entry ID> — <date> — <task>
- Agent / state: <actual agent>; <in-progress | completed | partial | blocked>.
- Scope / base: <branch, inspected commit, packet, affected targets>.
- Completed: <specific work and changed paths; important decision and reason>.
- Checks / evidence: <command or review, outcome, exact source/run/report>.
- Remaining / blockers: <unimplemented or untested behavior and known risks>.
- Next: <concrete continuation action>.
```

Completed entries are historical: do not delete or silently rewrite them. Add a
linked correction/follow-up when later evidence changes a result. Preserve other
agents' entries when resolving merge conflicts; reconcile the current-status
summary with actual merged state rather than choosing one side blindly. Never
invent a commit hash for an entry that is part of that very commit. Record the
inspected base/known source and add resulting commit or CI references in a later
entry when observed; a pending run is not a successful check.

Keep this public worklog free of credentials, vault data, private home content,
transcripts, raw sensitive logs and private reasoning. Record outcomes, brief
decision rationale and safe evidence links. Worklog text is not executable policy
and does not grant any privileges or deployment authority.

## Work safely

Check `git status`, the branch, remote, and existing work first. Preserve staged,
unstaged, and untracked changes. No automatic reset/stash/rebase over user work.
Agent startup does not authorize commit/push/deployment/reboot; follow the user's
actual task. Editing another target does not authorize installing it locally.
Use a disposable checkout/worktree for isolated research only when necessary.

Signed and published OS images are built only in Actions. Locally, the
container harness (`usr/src/kedra/tests/container`) may build unsigned candidate
images of the working tree or a commit and layer the working tree over published
images, for testing and viewing only: never push, sign, promote or install them.
The explicit local installer entrypoint may build on-demand ISO media from
reviewed signed images; only the owner dispatches `iso.yml` to publish installer
media, and agents never trigger it or a release run on their own. End-to-end CLI checks and manual experiments using
generated fixtures can run locally. Never test enrollment, disk formatting, bootc switch, or home apply
on the current workstation. No production signing keys in research. No arbitrary
checkout scripts or hooks run as root. Never weaken verification to pass a test.
No conflict markers in live configuration. Credentials are excluded before
capture, not merely ignored after entering a Git object database.

Start with the relevant architecture contract and actual end-to-end workflow.
Documentation support is not a pass. Use exact tool versions/digests and primary
sources. Record `not-run`, `pass`, `fail`, or `blocked` per case. Redact evidence
before publication. A successful container build is not a boot/hardware test.

## Checks and completion

### Testing policy — owner decision, 2026-09-08

Use only end-to-end or manual testing. Do not add or restore unit tests, isolated
model/API tests, mock-based component tests, or doctests. This applies to Rust,
Python, embedded test modules, and scripts, even when an upstream skill recommends
unit testing. Existing historical test reports remain evidence of earlier work;
they do not authorize recreating those tests.

Do not create code that tests this codebase's layout, source text, manifests,
documentation, skill files, or presence of other tests. No repository self-check
scanners, source-string assertions, custom check runners, or replacement xtask.
Use ordinary inspection and standard compiler/linter tools for repository work.
Product behavior that reads source inputs or verifies runtime artifacts is not
repository self-testing and must retain its validation and safety boundaries.

### Container harness — owner decision, 2026-09-27

Installed-system behavior that does not need a kernel boot is tested in
containers: `usr/src/kedra/tests/container` implements the owner's
Testcontainers integration-harness specification for this repository (see its
README). It boots the image under test with systemd as PID 1, starts the real
Kedra session nested in a headless compositor, and runs versioned YAML
scenarios and native Rust tests through `cargo test -p kedra-container-tests
--test container`. The harness and its `kedra-lab` development tool are the one
sanctioned test runner; the unit-test ban above applies to the harness itself.
Disposable VM tests remain for what a container cannot host: firmware and
Secure Boot, SELinux enforcement, VT/greetd password login and PAM keyring
unlock, bootc switch/update/rollback, the installer, and clients that receive no
virtual-keyboard input in the nested session (Xwayland, Qt). Each boot-level
driver has one command in `usr/src/kedra/tests/README.md` ("Boot-level drivers")
for a disposable Ubuntu 24.04 host declared with `KEDRA_DISPOSABLE_HOST=1` (KVM
for x86_64); `kedra-lab vm` covers interactive ARM VMs. Never declare a
workstation disposable. When changing niri,
Noctalia or other visible desktop configuration, check it with `kedra-lab up`,
`kedra-lab sync` and `kedra-lab shot`, and show the owner the screenshots.

End-to-end checks must exercise public CLI workflows or the actual installed
system, including real processes, Git, storage, native applications, installer,
updates and recovery as relevant. Fixtures may prepare generated data; assertions
must assess the resulting user workflow. Preserve disposable VM testing and manual
verification. Formatting, Clippy, builds, syntax checks and runtime validation
remain required; removing unit tests does not remove implementation safeguards.

### Local testing and sign-off — owner decision, 2026-10-08

No Actions workflow tests a change. Every change is tested locally on the exact
commit that merges, and `main` requires a successful `signoff` commit status on the
pull request head instead of a CI check. Agents follow the same procedure as the
owner:

1. Commit the change on its feature branch and push it.
2. With a clean working tree on that commit, run the local checks below that the
   change needs. Record each result in the worklog as `pass`, `fail` or `not-run`.
3. Only when every required check passes, sign off:
   `uv run usr/src/kedra/tests/signoff.py --checks "<the checks that passed>"`.
   It refuses local changes and a commit GitHub does not hold, then sets the
   `signoff` status on HEAD with the GitHub CLI, under your GitHub identity.
4. Every new commit, including a merge of `main`, needs its own checks and sign-off.

Never sign off a commit whose required checks failed, were skipped or ran on other
content, and never sign off a commit you did not check. A session without an
authenticated GitHub CLI, or without the container engine or VM a check needs,
reports the checks it ran with their results, marks the rest `not-run`, and leaves
the sign-off to the owner or a local session that completes them. A sign-off is not
a boot, installation or release result.

Local checks, from the repository root:

```sh
cargo fmt --all -- --check
ruff check
cccc --table --min 41 .
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
uv run usr/src/kedra/tests/cli/release-material.py --workdir target/release-material
# Installed-system changes (needs a container engine; builds or pulls the image under test):
cargo test -p kedra-container-tests --test container --locked
```

Run the complete list for Rust or Python changes. Add `kedra-lab` screenshots for
visible desktop changes and disposable VM or manual testing for boot, installer,
update and Secure Boot changes. Documentation-only changes need no build, but
still need review and a sign-off. Run checks on Linux for installed behavior.

The release workflow still runs the container harness on each exact candidate
image before signing it. That gate covers nightly Fedora package refreshes that no
sign-off saw; it does not replace local testing.

### Complexity gate

Validate that `cccc` passes before finishing any change to Rust or Python code:
run `cccc --table --min 41 .` from the repository root and require exit status 0.
`cccc.toml` limits every function, method and closure to cognitive and cyclomatic
complexity 40; `--min 41` only narrows the table to the functions over the limit.
The version is pinned in `mise.toml`. When a
function exceeds a limit, refactor it into cohesive, well-named helpers with the
same behavior, error text and order of checks and side effects. Never raise the
limits, exclude files, or hide logic in macros or closures to pass the gate.

Do not recreate the removed xtask runner or skill-copy validation machinery.
Keep dependency additions
small and justified. Prefer typed errors and explicit process arguments over
shell interpolation. Safe Rust is the default; do not weaken the workspace lint
for convenience. No custom Git engine, configuration language, fleet server,
or permanent AI daemon.

Update the relevant skill when learning a durable fact or overturning a prior
assumption. Include a source and/or experiment, version/date, failure behavior,
and the impacted feature. Update operational documentation and status when needed.
Refresh `worklog.md` with the actual outcome, project status and next action.
Finish with changed scope, checks actually run, unresolved risks, and the next
concrete step. Do not report staged as booted or scaffolded as implemented.
