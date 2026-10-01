# Delivery stack

Owner requested a PR followed by continued work through `gh stack` on 2026-10-01.
Inspected trunk `main`: `b224d5711e857f7dbcaabf7ed42870916525800c`.

| Layer | Branch | Base | Deliverable | Review / state |
|---|---|---|---|---|
| L1 | `codex/nix-engine` | `main` | Verified independent private Rust engine, CLI, actual E2E, research citations and carried implementation documentation | draft [PR28](https://github.com/Reidond/kedra/pull/28), `7329ef1`; all observed CI checks pass |
| L2 | `codex/nix-system-composition` | `codex/nix-engine` | Typed static composition and Kedra adapter over existing native source/package/target inputs | draft [PR29](https://github.com/Reidond/kedra/pull/29), implementation `bace56e`; local gates pass, complete cached-Kedra archive/context replay remain unqualified |
| L3 | `codex/nix-context-replay` | `codex/nix-system-composition` | Strict snapshot consumer, signed complete foundation, static replay/unit and recoverable cache | draft [PR31](https://github.com/Reidond/kedra/pull/31), implementation09473e1; local replay/unit/current-source gates pass, CI pending |

Research/spec/worklog records are carried with their associated deliverable;
there is no journal-only prerequisite branch. Raw third-party documents and
fetched source captures remain local; public records preserve canonical citations,
hashes and focused notes. No deployment or merge follows from draft submission.

Use installed `github/gh-stack` v0.1.0 non-interactively: `init`/`add` with explicit
branch names, `submit --auto --remote origin`, and `view --json`. `--auto` creates
drafts. No cascade/rebase/force update is needed to publish these fresh branches.
Parent handles Git/index/stack operations while delegated writers are stopped.

Native GitHub stack30 contains both draft PRs; gh-stack submission and local
head/base readback match. Upper CI is pending publication; no merge/deployment.

Continuation updates stack30 to three drafts. PR28/29 observed workspace checks
pass. L3 qualifies signed9d6 foundation and actual replay; the older L2 unqualified
state remains historical. Later transforms/cold-daemon/fault windows are named in
the replay evidence. No lower branch was rewritten or force pushed.

Replan if OS composition requires replacing installed trust, custom configuration
language, destructive state migration, or a new independent concern. Inspect exact
PR head/base and branch ancestry before any later cascade; do not collect lower
engine fixes into the system-composition layer.
