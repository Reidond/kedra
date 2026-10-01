# Independent engine verification plan

State: planned, 2026-10-01. Owner-authorized implementation scope; behavioral
verification **not-run**. [Cases](test-cases.md) own 14 conditions and 22 cases;
[requirements](requirements.md) and [design](design.md) are authoritative.

## Scope and levels

The gate is an ordinary-user workflow: Rust authoring → explicit source/image
admission → real C library/executable build → independent rebuild → offline
closure transfer/run → two profile versions/rollback → root-aware GC, including
corruption, concurrency, process failure and recovery. Initial execution is
native aarch64 Linux through Docker on this host. This does not qualify a new OS,
boot image, desktop, service deployment or privileged helper.

| Owning level | Cases | Method and mechanism |
|---|---|---|
| Host CLI end-to-end | TC-01–TC-20 | Automated ordinary Cargo `e2e_engine` target in the existing sysroot crate; real public CLI subprocesses, storage and Docker executor. |
| Manual | TC-21 | Standard compiler/formatter/linter plus scoped implementation review. |
| Manual | TC-22 | Real private-store interruption and recovery observations; only reached publication windows count. |

No unit/model/API/mocked/component/doctest/source-scanner tests or custom runners.
The owner contract chooses these levels. Installed-system Testcontainers/VM lanes
are unnecessary for this backend; extend them only when later installed-system
behavior needs them. No new framework, daemon or standalone fault runner is added.

## Execution sequences

### TS-01 — Real package lifecycle

Use one explicitly ignored Cargo test for the retained-image/Docker workflow; its
ignore reason states its dependencies. Order TC-01 → TC-03 → TC-04 → TC-05 →
TC-06 → TC-07 → TC-08 → TC-10 → TC-11. Reuse expensive image/source setup and use
fresh stores for destructive variants. Generate the Rust authoring consumer and
real C library/executable in a private fixture; the consumer emits typed graph
transport that the public CLI actually builds. The binary's observable version
comes from the separately linked library. Keep compiler and runtime roots distinct.

Use actual public CLI spelling implemented from design; `build --store S --plan
GRAPH --root NAME [--rebuild]` is fixed there. Source/image admission, export,
expected-digest import, profile and GC command details are finalized with CLI
implementation rather than invented here. Capture exact replay commands in the
execution report. Lifecycle success must include at least one newly compiled
output, independent rebuild, imported library and runtime image, two generations,
and receiver run with no producer mounts and networking disabled.

### TS-02 — Refusal and resource matrix

Run TC-02 → TC-09 → TC-15 → TC-16 → TC-18 → TC-19 → TC-20 against private valid
controls. Non-Docker public refusals may be ordinary non-ignored tests in the
same Cargo target. Docker-dependent variants share TS-01's explicit ignored test
or a named ignored E2E case, with distinct variant evidence. Corrupt generated
bundles and fixture stores only; recompute the outer hash for inner-validation
cases. Record positive control, intended mutation, diagnostic, unchanged sentinel
and no-valid-publication observation for every row.

### TS-03 — Concurrent use and interrupted execution

Run TC-12 → TC-17 → TC-13 → TC-14, followed by TC-22's manual publication checks.
Use generated process barriers and observed owned container/journal state before
launching conflicting GC/builds or killing the controlling process. Bound all
observer processes. Keep an unrelated owned sentinel container and verify it
survives recovery. Never stop the user's daemon or remove shared retained images.
A missing private-daemon or safe ownership fixture is a reported missing variant.

All sequences clean only exact fixture-owned resources after recording bounded,
redacted evidence. On uncertain cleanup retain the journal and fixture path for
explicit recovery. Preserve prior valid outputs and unrelated Docker state.

## Data, environments and commands

| Need | Concrete source / constraint |
|---|---|
| Source revision | Record current commit and dirty state; the spec base is `b224d5711e857f7dbcaabf7ed42870916525800c`. Existing modified status/worklog files belong to the parent task. |
| Engine executable | Actual sysroot binary built from this revision, never a stub. |
| Rust authoring | Generated standalone Cargo consumer using the public crate and typed paths, ordinary Cargo build/run, cached dependencies/offline mode when available. No bespoke expression language. |
| Compiler/runtime | Exact retained IDs and expected native platform from design; independently inspect availability before run. Missing prerequisite is blocked, never automatic pull/install. |
| Real package | Generated C shared library and caller, two printable versions, deterministic builder, separate entropy builder for divergence, held/failed/noisy builders for lifecycle faults. |
| Stores | Independent private producer/receiver/fault stores, outside sentinels, admitted archives and independently computed bundle checksum. |
| Process evidence | Bounded stdout/stderr/status, actual owned container inspection, configured timeout/limits, observed starts/stops and journals. No fabricated timing guarantee. |

Implementation registers the ordinary `e2e_engine` test target in sysroot's
existing Cargo test declarations. Intended commands after that target exists:

```sh
cargo test -p sysroot --test e2e_engine --locked
cargo test -p sysroot --test e2e_engine --locked -- --ignored --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
uv run usr/src/kedra/tests/cli/release-material.py --workdir target/release-material
```

The default Cargo run listing an ignored lifecycle test supplies **zero** lifecycle
behavior evidence. Record the explicit `--ignored` invocation and nonzero executed
case/variant population. Compiler success is not package runtime success. Existing
release E2E cases protect compatibility; they confer no engine behavior credit.

## Entry, exit and evidence

Entry: public commands/core exist; generated fixture can be compiled with the
pinned toolchain; required retained images and native Docker executor are available;
resource contract gaps in TC-18 are settled before its exact boundary assertions.
Independent grounded cases may proceed while an environment-dependent variant is
blocked. Implementation authorization already exists.

Exit: every in-scope case and named variant has actual pass evidence, including
explicit ignored lifecycle execution and manual checks; required standard gates
pass; no new unexplained compatibility failure; owned cleanup is verified; and
worklog/status distinguish implementation from qualification. Any fail, blocked,
not-run, unreachable fault window or ungrounded boundary prevents a blanket
completion claim and is handed off explicitly. No coverage percentage or timing
budget is inferred.

For each run record source/dirty scope, actual tool versions/image IDs/platform,
command, case/variant IDs, prerequisite/control observation, result/status,
expected/actual behavior, relevant output/tree/bundle identities and cleanup state.
Use existing ignored/local evidence storage; do not create tracked raw research
reports. Parent owns worklog/status updates. Safe summary evidence, not secrets or
unbounded logs, enters those documents.

## Traceability and NFR methods

All statuses below are **not-run**; mappings alone are not evidence.

| Requirement | Cases | Method |
|---|---|---|
| AC-01 | TC-01, TC-02, TC-06, TC-21 | Automated E2E; manual side-effect review |
| AC-02 | TC-03, TC-04, TC-17, TC-19 | Automated E2E |
| AC-03 | TC-05, TC-06, TC-15 | Automated E2E |
| AC-04 | TC-07, TC-09, TC-19 | Automated E2E |
| AC-05 | TC-07, TC-08 | Automated E2E |
| AC-06 | TC-10, TC-22 | Automated E2E; manual interrupted selection |
| AC-07 | TC-12, TC-13, TC-14, TC-17, TC-22 | Automated E2E; manual publication boundaries |
| AC-08 | TC-03, TC-11, TC-12, TC-15 | Automated E2E |
| AC-09 | TC-01, TC-21 | Automated consumer-to-CLI lifecycle; manual API scope review |
| AC-10 | TC-10, TC-20 | Automated E2E and existing public compatibility cases |
| NFR-01 | TC-21 | Manual review and standard compiler/linter commands |
| NFR-02 | TC-02, TC-17, TC-18 | Automated public CLI bounds/deadlines; exact unspecified limits pending |
| NFR-03 | TC-09, TC-14, TC-16 | Automated private-store trust/refusal E2E |
| NFR-04 | TC-03, TC-06, TC-19 | Automated real source/output/transfer E2E |

## Reconciliation and open items

Reconciliation with [tasks](tasks.md) is **pending** while the parent authors it
in parallel. Every TC-01–TC-22 must have a task owner, and each task's Test
requirements must cite its cases before reconciliation is complete. Reconcile
again after any condition, case or task amendment.

Open contract item: TC-18 records ungrounded numeric output/log/path bounds and
arguments/environment interpretation, to be fixed in design by the implementing
agent within scope. Design risk scores are absent, so preserve the qualitative
risks and adverse coverage without manufacturing scores. Other unsupported or
unreachable variants retain not-run status. Deliberate scope exclusions and trust
limits are listed in [test-cases](test-cases.md#non-coverage-and-unresolved-qualification).

## Executed outcome — 2026-10-01

The first private engine is implemented. Actual counts, case/subvariant boundaries, native Linux vs Mac controller scope, review fixes and remaining not-run cases are in [verification.md](verification.md). All ten engine cases were exercised across the hardened full run and targeted fixture correction; default ignored Docker cases are not counted as default workspace passes. TC16/19 full matrices and TC22 power-loss/ENOSPC remain not-run.
