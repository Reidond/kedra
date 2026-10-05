# Specification review

Status: specification self-review completed 2026-10-05; implementation not started.
Reviewer: Codex side conversation, single-agent review. No subagents were used.

## Scope and evidence

This branch contains only `.specs/nix-package-dsl/` and scoped status/worklog
documentation. Runtime/catalog/image/workflow code and package lists are unchanged.
Source comparison and the primary references in design.md support the current-state
description. Proposed APIs are illustrative and have not been compiled or executed.

The worktree is based on main while engine/catalog contracts were inspected at
the explicitly pinned delivery revision. Snapshot links distinguish that evidence
from files available on main. The PR can be reviewed independently; implementing
it must first reconcile with the eventual delivered source.

## Design review resolutions

- A single undifferentiated package node would hide Fedora solver/privilege
  requirements. The design retains separate provider kinds and explicit role
  resolution under one authoring API.
- Requiring exact builder image IDs during initial authoring creates a dependency
  cycle when the same DSL declares compiler RPMs. Intent uses symbolic roles;
  checked resolution precedes lowering into the existing engine graph.
- Sandboxing only the emitted program misses Cargo build scripts. Both compilation
  and evaluation use the declared isolated environment; `--frozen` alone is not
  described as isolation.
- An RPM version list cannot guarantee future replay from moving servers. The
  design retains actual complete material checks and requires retained artifacts
  or explicit re-resolution rather than promising a mirror it does not build.
- External-catalog parsing alone does not generalize release contribution. T5
  owns the fixed package inventory, templates, preflight and actual candidate
  path, with TC-07 requiring a third real package without central-name edits.
- Inferring format from file presence would let legacy lists mask DSL failures.
  New source formats require an explicit discriminator, mixed-authority refusal
  and legacy source compatibility before cutover.
- A protected-base rule without an owner could become an author-controlled
  escape hatch. Independent TargetPackagePolicy now owns required base names and
  is bound to resolution; T1 and TC-02 cover its introduction and refusal.

Security, minimal scope, failure handling, affected files, rollback, task ownership
and verification coverage were reviewed against the specification checklist. AI
service/web/API-specific checklist items do not apply. Project E2E/manual-only
policy overrides generic unit/mock recommendations. Every AC/NFR is represented
in test-plan.md; every TC has a task owner. No unresolved implementation-design
question is used to imply current implementation readiness.

## What remains for the owner

Review the proposed authoring ergonomics, migration sequence and initial scope.
Approving this documentation or merging this spec does not start implementation,
authorize a new release or waive current delivery qualification. Detailed method
names can evolve during implementation as long as these acceptance contracts hold.

## Checks for this PR

- Pass: current source/API and actual recipe/list/release consumer inspection.
- Pass: manual requirements/design/tasks/case reconciliation and preservation of
  the independent privilege and release boundaries.
- Pass: `git diff --check` and ordinary changed-path inspection. Exact remote PR
  readback for draft PR38 confirms exactly nine Markdown documents against main;
  the publication follow-up is recorded in worklog.md.
- Not-run: all future runtime cases, Cargo builds for the proposed API, DNF/image
  transactions, VM/installer flows and production signing. No such code is added.

Closing challenge found no further unresolved design defect after the seven
resolutions above. No implementation symbol changed, so runtime callers and
signing behavior are unchanged by this diff. Future API ergonomics, isolation
limits and actual solver/candidate behavior remain unverified until their named
implementation cases run. Automatic workspace CI checks the existing main code;
even a pass there would not qualify the proposed DSL.
