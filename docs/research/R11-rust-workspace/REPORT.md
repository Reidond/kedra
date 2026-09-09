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

## Repository-only Codex discovery audit — 2026-09-09

Agent: Codex. Read-only audit began on `codex/usable-system` at `95e0e1c`,
using the current ordinary-file plugin and repository marketplace. This follow-up
explains the [2026-09-07 discovery result](discovery-20260907.md); it does not
replace the historical bootstrap experiment or turn native skill loading into a
pass. Main remains the separate accepted candidate source `c660c58`.

The pinned runtime is Codex CLI **0.153.4**, source
`3d2ee51ca2d5db578f328aa75e20aa22c0197c9a`, as recorded in
`build/agents/inputs.json`. Actual execution of the existing native Windows binary
confirmed `codex-cli 0.153.4`. Top-level, plugin, plugin-add, marketplace-list and
app-server help all returned exit 0. These commands used a generated disposable
CODEX_HOME without copied credentials; the runtime refused to create PATH aliases
under the Windows temporary directory, and help still succeeded. The temporary
profile was removed. No marketplace registration, plugin installation, model
request or authentication was performed. Help output was inspected as stdout;
no persistent runtime evidence artifact or filesystem isolation trace is claimed.

The source explains two distinct expectations in the earlier probe:

- `codex plugin list` supplies an empty additional-root list to the plugin
  manager. Its roots are configured marketplace sources and eligible curated
  catalogs; it does not add the current checkout automatically. The marketplace
  command uses the same empty-root approach. Thus a filtered query for the
  unregistered `kedra-local` catalog returning no available entry is not proof
  of a malformed Kedra manifest. See pinned
  [plugin command](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/cli/src/plugin_cmd.rs#L261),
  [marketplace command](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/cli/src/marketplace_cmd.rs#L208)
  and [root selection](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/core-plugins/src/manager.rs#L3507).
- Standalone repository skills are discovered under `.agents/skills` between
  the working directory and repository root. The canonical
  `plugins/kedra/skills` tree is outside that scan. A marketplace entry marked
  `AVAILABLE` exposes an installation choice; it does not load that tree.
  Plugin loading requires an active installation. This accounts for the prior
  fresh-profile `skills/list` returning only built-ins. See pinned
  [skill roots](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/ext/skills/src/host_roots.rs#L137)
  and [installed-plugin requirement](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/core-plugins/src/loader.rs#L867).

Current official [skill discovery documentation](https://learn.chatgpt.com/docs/build-skills#where-codex-loads-local-skills)
describes the repository scan. The [plugin packaging documentation](https://developers.openai.com/plugins/build/plugins#how-local-marketplaces-work)
distinguishes a repository marketplace from installation into the user's plugin
cache and configuration. That installation would conflict with this project's
repository-only/no-copy contract when performed as an incidental discovery fix.

There is a supported **App Server** mechanism,
`skills/extraRoots/set`, which replaces process-level extra roots without
persisting them. It is documented in the
[App Server reference](https://learn.chatgpt.com/docs/app-server) and present in
the previously generated 0.153.4 `SkillsExtraRootsSetParams` schema. A host could
point it directly at the absolute checkout's `plugins/kedra/skills` directory.
The pinned [root resolver](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/ext/skills/src/host_roots.rs#L63)
treats extra roots as process-wide standalone user-scope skills, not a
repository-scoped installed plugin. Sharing that process with unrelated
checkouts would not preserve Kedra's intended scope.

This endpoint is not an exercised native CLI/model workflow. The inspected native
help exposes no `--plugin-dir` or `--skills-dir` option, and
[`skills.config`](https://github.com/openai/codex/blob/3d2ee51ca2d5db578f328aa75e20aa22c0197c9a/codex-rs/config/src/skills_config.rs#L70)
is an enable/disable selector for discovered skills, not an extra-root setting.
No supported direct CLI configuration route for loading this canonical tree was
established by this audit. Building a custom App Server client or daemon to bridge
the gap is outside this bounded task and is not a discovery fix to introduce
incidentally.

**Decision / next step:** retain the AGENTS-directed canonical-file reading
approach in the Kedra checkout. Do not create skill copies, symlinks, profile
registrations or a custom client to manufacture an automatic-discovery pass.
If a supported native session path becomes available, qualify actual root/crate
skill use and supporting-file resolution through that workflow. Current status:
`pass` for native help observation and source analysis; `not-run` for the extra-root
endpoint and loaded-plugin/model workflow. Overall R11 remains incomplete.
