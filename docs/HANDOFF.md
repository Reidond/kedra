# Continue Kedra from this bootstrap

## Objective

Build a personal, agent-ready Fedora 44 bootc system managed by a small Rust
`sysroot` CLI. The owner describes a change, an official coding agent edits the
repo, GitHub Actions builds/tests/signs an image and ISO, and the machine deploys
the exact approved digest while preserving writable-home changes. This handoff
is intended to replace having to reconstruct the original planning conversation.

## What exists now

A flat-source Cargo workspace, read-only bootstrap CLI, non-operational helper,
repository checks, per-target design inputs, R01-R11 specifications, the session
contract, twelve Kedra skills, and a pinned upstream Rust-skills submodule with
repository-local discovery links. The check workflow has no signing secrets,
package publication, workstation access, or OS installation step.

Only help/version/status are CLI capabilities. `sysroot home`, `update`,
`deploy`, `rollback`, `setup`, `doctor`, `context`, `codex`, and `claude` refuse
operation. Source placeholders are deliberate boundaries, not hidden TODO
implementations to trust. `host.toml` and package lists are design inputs; no
production assembler consumes them yet. Read the actual latest CI result.

[The update/refresh plan](UPDATES.md) now details periodic Fedora input refresh,
no-change/freshness checkpoints, release approval and notify-only client defaults.
[ADR 0002](adr/0002-updates-and-fedora-refresh.md) and
[the supplemental test cases](research/update-refresh/EXPERIMENTS.md) scope its
implementation. No cron, client timer, signing environment or deployment was
enabled by writing that plan. Both first-party/upstream skills remain checkout-only,
never an OS/global skill library. Maintain root worklog.md as AGENTS.md requires.

## First work session

1. Read AGENTS.md, worklog.md, the context and Rust-workspace skills, PLAN.md,
   RESEARCH.md, and latest reports/status. Inspect Git state and source revision.
2. Initialize the pinned submodule when missing; do not update it to a moving
   branch. Use existing personal Codex/Claude, not an unimplemented launcher.
3. Run `cargo xtask check`. Resolve any bootstrap failure without relaxing checks.
4. Complete R11's missing real-agent skill-discovery and editor-layout tests.
   Static link validation does not prove discovery, optional tool availability,
   model compliance, or profile isolation. Keep those cases not-run until tested.
5. Start one independent research track: R01/R02 signed-image and installer proof;
   R03 synthetic home line-selection/local-only semantics; or R05 official-agent
   packaging/coexistence. Do not build the whole product in one speculative PR.

There are no production release keys, configured protected signing environments,
qualified disk layouts, or validated desktop/XPS hardware. Do not invent them.
Register missing external prerequisites as blockers and continue independent
safe experiments rather than installing over the current workstation.

## Suggested initial Codex assignment

> Read AGENTS.md, docs/HANDOFF.md, PLAN.md, RESEARCH.md and the relevant repository
> skills. Inspect current checks and research evidence. Complete the remaining
> R11 bootstrap/discovery validation, then implement a disposable R03 prototype
> using synthetic homes and Git. Prove line staging, unchanged staged content
> during later app writes, local-only hunks excluded from export, and conflict
> handling. Do not enroll a real home or implement production deployment.
> Record reproducible evidence, exact versions, statuses, and an ADR; update the
> relevant skills and worklog.md with what the experiment actually established.

Alternatively assign the refresh track:

> Read docs/UPDATES.md, ADR 0002, kedra-github-actions and kedra-release-signing.
> Implement only slice 1: a disposable Actions RPM-refresh experiment using fixture
> repositories and the exact Fedora tools. Prove base/source-fixed dependency
> updates, cache invalidation, genuine no-change and required-repository failures.
> No production keys, promotion, permanent schedule, host DNF or real-home changes.
> Record evidence under the owning R02/R07/R08 packets and update skills/worklog.

Connect that experiment to R01/R02's signed image/installer flow only when the
corresponding trust and VM resources/gates are ready.

## Non-negotiable acceptance story

Install A in a VM. Establish recovery access, Bitwarden SSH, separate GitHub API
and model login. Install a personal coding-agent version. Keep ordinary commands
personal and management commands bundled by default. Make three edits in one
managed file: publish one, leave one visible/uncommitted, keep one local-only.
Include a GUI-written Noctalia override. Ask an agent to add a program, publish
only approved content, follow the exact CI run, and stage B. After authorized
reboot prove the running digest, feature, and preserved local edits. Test invalid
signatures, wrong target, conflicts, concurrent writers, interrupted activation,
and rollback without deleting newer personal data. Repeat target updates
independently. Recover offline without the agent or Bitwarden GUI.

## Knowledge maintenance

The short skills are entry points; full plans, evidence, ADRs and source links
are supporting context. Update both when behavior changes. Avoid duplicated
conflicting specifications: preserve source provenance and link to canonical
records. Carry forward user requirements even when generic upstream guidance
suggests different defaults. A researched implementation change needs an ADR;
reversing a settled user requirement needs the user's decision.
