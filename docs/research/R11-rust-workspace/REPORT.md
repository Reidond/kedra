# R11: Rust workspace and static skill wiring

Date: 2026-09-07.
Bootstrap subset: **pass**. Overall R11: **blocked on remaining real-agent,
editor, adversarial-validator and distribution-review cases**.

Tested source: `98efe4d944a2bcfd27865edf6cbee0bd3ce4b94e`.
Actions run: https://github.com/Reidond/kedra/actions/runs/34138882781
Job: https://github.com/Reidond/kedra/actions/runs/34138882781/job/101796115965
Workflow: `.github/workflows/check.yml`, run 2, attempt 1, push to main.
The source commit and log, not this report's later commit, identify the tested code.

## Question and experiment

Can the agreed flat Cargo layout compile/test with a pinned Rust toolchain, and
can a fresh Actions checkout initialize the exact upstream Rust-skills gitlink
and resolve every repository-local skill link?

The experiment uses only a read-only bootstrap CLI, non-operational helper and
std-only validator. It is not a prototype for privileged deployment or home apply.
Actions checks out the source with recursive submodules, installs Rust 1.98.1,
records versions, runs `cargo xtask check`, and verifies no tracked source/lockfile
changes remain. No credentials beyond Actions' read-only checkout token are
configured or used by the check; no image or agent binary is downloaded/executed.

## Environment

See environment.json. Observed toolchain: rustc 1.98.1
(48a229cea 2026-09-01), Cargo 1.98.1 (797e8a9bc 2026-08-05), LLVM 22.1.8, Git 2.55.0.
Runner: Ubuntu 24.04.4, image ubuntu-24.04 version 20260831.293.1,
runner agent 2.337.0, x86_64-unknown-linux-gnu Rust host.

## Results

| Case | Expected | Observed | Status |
|---|---|---|---|
| Fresh recursive checkout | Exact upstream pin available | Gitlink and initialized HEAD match 5c40d3ad... | pass |
| Flat first-party paths | No src/; explicit targets and canonical inheritance | Validator and Cargo metadata accept four packages | pass |
| Skill wiring | Exact pin, no duplicate/unknown links, both agent roots | 50 canonical skills, matching links in both roots | pass |
| Formatting | Pinned rustfmt check exits 0 | No diff | pass |
| Lints | Clippy workspace/all-targets with -D warnings exits 0 | All four packages accepted | pass |
| Tests | Bootstrap CLI/core/helper/layout behavior works | 8 unit/integration tests and 1 doctest pass | pass |
| Negative nested-src fixture | A first-party module/src path is rejected | Test passes | pass |
| Release compilation | All four packages compile in release mode | cargo build exits 0 | pass |
| Source/lock consistency | Checks do not modify tracked files | git diff --exit-code exits 0 | pass |

The tests cover bootstrap JSON status, unsupported-argument handling, operational
command refusal, research-gate mapping, helper refusal, and nested src detection.
They do NOT prove any of the refused commands will be secure once implemented.

### Initial failure retained in history

Run 1 at source `0e8a6f1d93b69061f29c10c745e4286065b27746` failed because handwritten
source did not match rustfmt. Layout, skill wiring and metadata had already passed;
Clippy/tests/release build were not reached in that run. The next commit applied
the actual formatter output and removed an unnecessary string allocation. Run 2
then completed every check. No check was disabled or loosened to obtain success.
Initial evidence: https://github.com/Reidond/kedra/actions/runs/34138732851

## Limitations and remaining R11 cases

Real Codex and Claude were not launched. Their discovery of skills, skill-relative
reference loading, nested working directories, personal-profile conflicts,
configuration/state isolation, offline and missing-submodule UX, and absence of
personal writes remain **not-run**. Static link validation is not evidence of
agent behavior or model compliance. Rust-analyzer/editor validation is not-run.

The bootstrap validator enforces a narrow canonical manifest spelling, not a full
TOML dependency/target policy. Frontmatter checks are marker checks, not a complete
YAML/schema or supporting-reference audit. Add negative cases for changed pins,
escaping links, missing lint opt-in, additional unlisted crates/targets and indirect
helper dependencies before claiming complete repository policy enforcement.
Upstream source has not undergone a full security or redistribution/license review.

R01-R10 remain not-run. No OS image, ISO, signature enforcement, home merge,
credential bootstrap, physical hardware or deployment operation was tested.
The Ubuntu Rust release binaries are not Fedora-qualified OS artifacts; eventual
image builds need a compatible target ABI/toolchain environment.

## Reproduce

```sh
git clone --recurse-submodules https://github.com/Reidond/kedra.git
cd kedra
git checkout 98efe4d944a2bcfd27865edf6cbee0bd3ce4b94e
git submodule update --init --recursive
cargo xtask check
git diff --exit-code --ignore-submodules=none
```

Use the recorded Actions run for the exact observed runner image. A new run may
receive a changed hosted runner even with the same source/toolchain pin.

## Decision and next work

Keep the flat Rust workspace and Git-backed skill integration in ADR 0001. They
work for the measured bootstrap scope. Complete real CLI/editor/discovery and
negative-validator cases separately. R03 synthetic home review and R01/R02 image
research may proceed independently without enrolling real machines/homes.
Durable finding is linked from the Rust-workspace skill's reference notes.
