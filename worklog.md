# Kedra worklog

Shared continuation record for Codex, Claude and other authorized contributors.
Follow the maintenance contract in `AGENTS.md`. Keep the status snapshot current;
append completed work entries without silently rewriting history. This file is
public: no credentials, transcripts, private home data or sensitive raw logs.

## Current project status

Last updated: 2026-09-07.

| Area | Verified status / next boundary |
|---|---|
| Phase | Research-ready repository bootstrap; not an installable operating system. |
| Implemented | Three-package flat Rust workspace, help/version/bootstrap status, refusal of unimplemented operations, standard Cargo CI and documentation. |
| Repository knowledge | One Codex/Claude plugin with 12 Kedra and 18 selected upstream skills in plugins/kedra/skills. Ordinary files; no submodule, symlinks or duplicate discovery copies. No personal installation. |
| Latest verified check | Local Rust 1.98.1: fmt, Clippy, 7 tests + 1 doctest and release build passed; both plugin manifests and Claude skill validation passed. See [local report](docs/research/R11-rust-workspace/plugins.md). Remote update-plan run 6 on bfb1902 is recorded as passed; merged-revision CI is not-run. |
| R11 | Bootstrap subset passed. Overall packet remains blocked on installed interactive plugin/editor discovery and distribution review. |
| R01-R10 | Not run, as recorded in `docs/research/status.json`. No OS/ISO/signature/home/auth/hardware success is implied. |
| Current task | Completed local plugin packaging, xtask removal and removal of the unused skills lockfile. Upstream provenance lives in the plugin NOTICE.md. Concurrent update/refresh planning from b7b2962 is preserved. Plugin installation and merged-revision CI remain not-run. |
| Machine effects | None. No workstation, home directory, vault, personal agent profile or installation was changed. |

Evidence: [R11 report](docs/research/R11-rust-workspace/REPORT.md),
[research index](docs/research/status.json),
[bootstrap run 2](https://github.com/Reidond/kedra/actions/runs/34138882781),
[baseline run 3](https://github.com/Reidond/kedra/actions/runs/34139103852),
[documentation run 4](https://github.com/Reidond/kedra/actions/runs/34140106798),
and [update-plan run 6](https://github.com/Reidond/kedra/actions/runs/34145002985).
The snapshot summarizes those records; source, exact CI runs and case-level
reports remain authoritative for what was tested.

### Next concrete actions

Read docs/HANDOFF.md and docs/UPDATES.md. The update track starts with a disposable
CI RPM-refresh proof (slice 1), covering unchanged-base dependency updates, stale
cache, no-change and required-repo failure. Connect to signed-image/installer gates
only after those proofs. R03 home and remaining R11 discovery work can proceed
independently. Do not enable production cron or enroll a real machine/home yet.

## Work entries

### WL-20260907-01 — 2026-09-07 — Bootstrap baseline (retrospective)

- Agent / state: ChatGPT repository handoff; completed bootstrap subset only.
- Scope / base: `main` through `e807dfe219ba1d57ebb751bd40d46897af31b6d7`.
  This entry was reconstructed from existing source, the R11 report and observed
  Actions results; it is not a claim that the prior session maintained a log.
- Completed: Four flat-source Cargo packages, bootstrap CLI/helper refusal,
  `xtask`, pinned toolchain, repository-only skill links/submodule, plans,
  research packets and continuation instructions. See the source history and
  `docs/research/R11-rust-workspace/REPORT.md` for the detailed scope.
- Checks / evidence: `pass` — bootstrap run 2 records formatting, Clippy,
  metadata, eight unit/integration tests plus one doctest, release compilation,
  layout and static skill wiring. `pass` — baseline run 3 at `e807dfe...` was
  rechecked through GitHub for this update. The earlier formatting failure and
  correction are preserved in the R11 report, not erased from history.
- Remaining / blockers: Actual Codex/Claude discovery and editor behavior were
  not tested. R01-R10 remain not-run; no image, ISO, production signing or live
  system management was produced.
- Next: Use the handoff and selected research packet, not the bootstrap status
  command, as the guide to implementing the next capability.

### WL-20260907-02 — 2026-09-07 — Repository skill scope and worklog contract

- Agent / state: ChatGPT using the GitHub connector; completed documentation
  edits, with the resulting CI not yet observed at entry creation.
- Scope / base: `main`, inspected base
  `e807dfe219ba1d57ebb751bd40d46897af31b6d7`. Documentation only; no host target
  package, runtime, workflow or deployment behavior changed.
- Completed: Updated `AGENTS.md` to prohibit global/system-wide distribution or
  registration of the repository's skills and to require both agents to maintain
  this exact lowercase `worklog.md`. Updated `skills/README.md` to reflect the
  same scope. Initialized the status snapshot and retrospective baseline entry.
  `CLAUDE.md` already imports `AGENTS.md`, so it remains unchanged.
- Checks / evidence: `pass` — inspected current main, agent instructions, skill
  catalog, Claude import, research status and baseline CI through GitHub.
  `not-run` — new-revision CI at entry creation; record an observed result in a
  follow-up entry rather than borrowing the prior commit's pass.
- Remaining / blockers: These are repository instructions, not new executable
  enforcement or a claim of real agent compliance. Existing research gaps remain.
- Next: Observe the exact resulting Actions run, record its outcome and update
  the status snapshot before handing off to the next coding session.

### WL-20260907-03 — 2026-09-07 — Verify the documentation change

- Agent / state: ChatGPT using the GitHub connector; completed verification and
  handoff update. Follow-up to WL-20260907-02; its original pending observation
  remains preserved above.
- Scope / base: `main` at `1e094eca8f482b0203dcbf041c14e5883c8f4123`.
- Completed: Confirmed the published change modifies only `AGENTS.md`,
  `skills/README.md` and `worklog.md`. Read back the instructions and worklog;
  observed the exact resulting Actions job succeed. Refreshed this status
  snapshot without changing the research index or claiming new implementation.
- Checks / evidence: `pass` — GitHub comparison against `e807dfe...` confirms
  the three-document scope. `pass` — [run 4](https://github.com/Reidond/kedra/actions/runs/34140106798),
  job `101799936466`, completed all workspace/skills/format/lint/test/release
  checks and clean-source verification successfully on `1e094eca...`.
- Remaining / blockers: CI checks repository consistency, not an agent's future
  adherence to prose. Actual agent discovery/editor behavior and R01-R10 remain
  untested. This record does not imply any OS or home deployment.
- Next: Resume the selected research packet using `docs/HANDOFF.md`; read and
  maintain this worklog in the next Codex or Claude session.

### WL-20260907-04 — 2026-09-07 — Plan updates and scheduled Fedora refresh

- Agent / state: ChatGPT using GitHub and primary web documentation; completed
  planning, with resulting CI not yet observed at entry creation.
- Scope / base: `main` at `f3b44f1fd63af5805167c30e139bb5bf3f9aa3f4`.
  Documentation/repository-skill changes only; existing research gate statuses stay.
- Completed: Added docs/UPDATES.md, ADR 0002 and an update-refresh experiment
  supplement. Added detailed skill references and updated the bootc, CI and signing
  skills plus HANDOFF. Proposed 12-hour complete Fedora refresh, full RPM closure,
  no-change/freshness separation, same-digest tests/ISO and notify-only clients.
  Planned explicit staging/reboot control, pending-image preservation, rollback
  holds, repository/cache failure handling and signed per-target checkpoints.
- Checks / evidence: `pass` — inspected repository instructions/worklog/source and
  checked current bootc/DNF5/Podman/GitHub primary docs (U01-U11 in docs/UPDATES.md).
  This is documentation review, not experiment success. `not-run` — actual refresh,
  no-change proof, signer/installer/channel/client behavior and new-revision CI
  at entry creation. Fedora web docs challenged retrieval; no new exact-content
  claim relies on those failed fetches.
- Remaining / blockers: R01/R02/R04/R07/R08/R09/R10 own implementation gates.
  No schedule, signing keys, protected environment, host package upgrade, client
  unit or deployment was created. Exact metadata publication and cache/equivalence
  behavior require tests; policy timings are proposed defaults, not upstream facts.
- Next: Verify this revision's repository check and readback. Then assign slice 1
  to Codex with disposable CI fixture RPM repositories; preserve all no-loss,
  signature and repository-only skill boundaries.

### WL-20260907-05 — 2026-09-07 — Verify update-plan publication

- Agent / state: ChatGPT using GitHub; completed documentation verification.
- Scope / base: `main` at `bfb190226c5461292e06dc470c42f86f998b07af`;
  follow-up to WL-20260907-04 without rewriting its pending observation.
- Completed: Read back the published plan and worklog, confirmed the commit changes
  ten documentation/skill files only, and observed the exact bootstrap run succeed.
  Refreshed this status snapshot; no implementation/research status was advanced.
- Checks / evidence: `pass` — GitHub compare against f3b44f1 confirms no runtime,
  package, Cargo, active workflow or timer changes. `pass` —
  [run 6](https://github.com/Reidond/kedra/actions/runs/34145002985), attempt 1,
  completed successfully for bfb1902. This is repository validation, not evidence
  that Fedora refresh, client update, signing, home merge or an ISO works.
- Remaining / blockers: The new experiment supplement remains entirely not-run.
  The final worklog-only follow-up has no claimed future CI result.
- Next: Assign the disposable slice-1 refresh experiment using docs/HANDOFF.md;
  keep production scheduling disabled until its prerequisite gates pass.

### WL-20260907-CODEX-04 — 2026-09-07 — Local Rust toolchain mismatch

- Agent / state: Codex; partial (diagnosis and recovery documentation completed).
- Scope / base: `main` at `f3b44f1`; R11 local bootstrap troubleshooting.
- Completed: Confirmed Cargo requires Rust 1.98 and repository/CI pin 1.98.1.
  Active toolchain is stable, selected by RUSTUP_TOOLCHAIN, reporting rustc
  1.97.0 (2d8144b78 2026-07-07), Cargo 1.97.0 (c980f4866 2026-06-30).
  Added explicit installation/selection guidance to README.md and the Rust
  workspace skill. Kept version requirements and personal configuration intact.
- Checks / evidence: `pass` — clean initial Git state, manifests, workflow,
  rustup active-toolchain/list and compiler version inspection.
  `blocked` — `rustup run 1.98.1 cargo xtask check` reports the Windows MSVC
  toolchain is not installed; no validation steps ran. `blocked` — latest CI
  retrieval with `gh run list --limit 3` denied by network restrictions; prior
  recorded CI evidence above was not freshly verified.
- Remaining / blockers: Local full check requires installing the pinned toolchain;
  AGENTS.md prohibits automatic missing-tool installation. No commit or push.
- Next: Install Rust 1.98.1 with rustfmt/Clippy explicitly, run
  `cargo +1.98.1 xtask check`, and record its actual outcome.

### WL-20260907-CODEX-05 — 2026-09-07 — Shared plugin and removal of xtask

- Agent / state: Codex; completed local changes.
- Scope / base: main at f3b44f1. Owner requested removing submodules/copying
  relevant skills, then explicitly removed the custom Cargo check runner and
  requested Codex/Claude plugins without symlinks. Supersedes the uncommitted
  copy-synchronization approach and interrupted symlink-diagnostic patch.
- Completed: Moved 12 Kedra and 18 selected upstream skills plus support files
  into plugins/kedra/skills. Added native Codex/Claude manifests and repository
  marketplace catalogs. Removed all discovery links/copies, the submodule and
  local registration, xtask package and Cargo alias. Regenerated Cargo.lock;
  CI runs standard Cargo commands. Updated AGENTS, README, handoff, plan,
  research packet/status, skills, provenance/notices and ADR 0002.
- Checks / evidence: pass — Rust 1.98.1 fmt, Clippy with warnings denied,
  7 unit/integration tests, 1 doctest and release compilation on Windows MSVC.
  pass — Codex plugin-creator validator, Claude 2.1.263 manifest/marketplace/skill
  validation. Codex CLI 0.153.4 does not list the unregistered repo plugin in its
  marketplace query; no personal registration or installation was performed.
  pass — git diff --check. Detailed evidence: docs/research/R11-rust-workspace/plugins.md.
  not-run — new-revision CI and installed interactive plugin use.
- Remaining / blockers: Standard cargo without an explicit pin can still inherit
  RUSTUP_TOOLCHAIN=stable; README documents removing the process override or
  selecting +1.98.1. Upstream declares MIT but lacks LICENSE text; recorded in
  third-party notices. R11 remains blocked on separate discovery/editor/review
  gates. No commit or push. Submodule gitlink removal is staged; other edits
  remain uncommitted. Original upstream and intermediate copies are preserved
  under ignored target/ backups.
- Next: Review the local changes and load the shared plugin in each agent.
  Use standard Cargo commands for Rust; do not restore the removed task runner.

### WL-20260907-06 — 2026-09-07 — Remove unused skills lockfile

- Agent / state: Codex; completed.
- Scope / base: main at f3b44f1, existing uncommitted plugin migration; user
  requested deleting skills.lock.toml.
- Completed: Deleted the unused lockfile, preserved the upstream source/revision
  and all 18 selected skill names in the plugin NOTICE.md, and updated current
  documentation and skill references. Superseded ADR 0001 remains historical.
- Checks / evidence: pass — reference review and git diff --check.
  not-run — Rust tests; no Rust code, manifests or build behavior changed.
- Remaining / blockers: Existing plugin installation and research gaps unchanged.
- Next: Review the uncommitted plugin changes and load the plugin when desired.

### WL-20260907-07 — 2026-09-07 — Prepare authorized commit and push

- Agent / state: Codex; completed publication preparation.
- Scope / base: main at f3b44f1; user explicitly authorized committing and pushing
  the plugin migration, xtask removal and unused skills-lockfile removal.
- Completed: Reviewed the pending tracked and untracked scope; retained the shared
  plugin, provenance notices, updated policies/research and standard Cargo CI.
  Ignored target backups and temporary authoring scripts are excluded.
- Checks / evidence: pass — earlier Rust formatting, Clippy, 7 tests plus
  1 doctest, release build and both plugin validators remain applicable; only
  documentation changed afterward. pass — git diff --check.
- Remaining / blockers: Commit/push and resulting Actions observation follow
  this preparation record; no result is claimed in advance. Plugin installation
  and separate R11 gates remain not-run.
- Next: Commit the reviewed changes, push main and inspect the resulting CI.

### WL-20260907-08 — 2026-09-07 — Preserve concurrent update planning

- Agent / state: Codex; completed merge preparation for the authorized push.
- Scope / base: local plugin commit a872522 and origin/main b7b2962; merge is
  required because two update-planning commits arrived during local work.
- Completed: Preserved docs/UPDATES.md, the update-refresh experiment, ADR 0002
  and all remote skill edits. Moved two new skill reference files into the shared
  plugin. Reconciled HANDOFF and both agents' worklog entries. Disambiguated
  colliding local entry IDs 04/05 with CODEX in their headings; bodies remain
  historical. Renumbered the plugin ADR to 0003; earlier mentions of plugin
  ADR 0002 refer to that pre-merge name. No reset, rebase or force push.
- Checks / evidence: pass — reviewed remote diff and resolved both content
  conflicts while retaining the plugin layout and standard Cargo CI.
- Remaining / blockers: Merge commit/push and CI observation follow this record.
  Update mechanisms remain planned; plugin installation and R11 gaps unchanged.
- Next: Validate the merged plugin, commit the merge and push main.
