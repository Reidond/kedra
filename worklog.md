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
| Implemented | Flat-source Rust workspace, help/version/bootstrap status, refusal of unimplemented operations, repository checks and documentation. |
| Repository knowledge | 12 Kedra and 38 pinned upstream skills; links for both agents stay within the checkout. No global or system-wide skill installation. |
| Bootstrap evidence | `cargo xtask check` passed on source `98efe4d944a2bcfd27865edf6cbee0bd3ce4b94e`; the subsequent `e807dfe219ba1d57ebb751bd40d46897af31b6d7` check also passed. |
| R11 | Bootstrap subset passed. Overall packet remains blocked on actual agent/editor discovery, additional adversarial validation and distribution review. |
| R01-R10 | Not run, as recorded in `docs/research/status.json`. No OS/ISO/signature/home/auth/hardware success is implied. |
| Current task | Documentation-only clarification of repository skill scope and mandatory worklog maintenance. CI for this new documentation revision is not yet observed at entry creation. |
| Machine effects | None. No workstation, home directory, vault, personal agent profile or installation was changed. |

Evidence: [R11 report](docs/research/R11-rust-workspace/REPORT.md),
[research index](docs/research/status.json),
[bootstrap run 2](https://github.com/Reidond/kedra/actions/runs/34138882781),
and [baseline run 3](https://github.com/Reidond/kedra/actions/runs/34139103852).
The snapshot summarizes those records; source, exact CI runs and case-level
reports remain authoritative for what was tested.

### Next concrete actions

Verify the documentation change's CI, then continue from `docs/HANDOFF.md`.
Complete available R11 real-agent/editor and negative-validation cases without
claiming unavailable environments passed. R03 synthetic writable-home research
can proceed independently; R01/R02 use disposable CI-built image/installer tests.
Do not enroll a real home or deploy to the workstation during those experiments.

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
