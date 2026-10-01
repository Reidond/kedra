# Delivery stack

Owner requested a PR followed by continued work through `gh stack` on 2026-10-01.
Inspected trunk `main`: `b224d5711e857f7dbcaabf7ed42870916525800c`.

| Layer | Branch | Base | Deliverable | Review / state |
|---|---|---|---|---|
| L1 | `codex/nix-engine` | `main` | Verified independent private Rust engine, CLI, actual E2E, research citations and carried implementation documentation | runtime/transfer/recovery review; draft publication |
| L2 | `codex/nix-system-composition` | `codex/nix-engine` | Typed system composition and Kedra adapter over existing native source/package/target inputs | dependent runtime/tooling review; implementation follows L1 publication |

Research/spec/worklog records are carried with their associated deliverable;
there is no journal-only prerequisite branch. Raw third-party documents and
fetched source captures remain local; public records preserve canonical citations,
hashes and focused notes. No deployment or merge follows from draft submission.

Use installed `github/gh-stack` v0.1.0 non-interactively: `init`/`add` with explicit
branch names, `submit --auto --remote origin`, and `view --json`. `--auto` creates
drafts. No cascade/rebase/force update is needed to publish these fresh branches.
Parent handles Git/index/stack operations while delegated writers are stopped.

Replan if OS composition requires replacing installed trust, custom configuration
language, destructive state migration, or a new independent concern. Inspect exact
PR head/base and branch ancestry before any later cascade; do not collect lower
engine fixes into the system-composition layer.
