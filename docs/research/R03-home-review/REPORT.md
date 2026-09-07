# R03: Synthetic writable-home review

Status: core prototype pass; complete R03 gate blocked.
Date: 2026-09-07. Agent: Codex.
Evidence finalized and CLI argument refusal checked: 2026-09-08 Europe/Kiev.
Source: local changes on main based on 64c84498c898e61ff370d9f9f1ce30cbd3d00613.
Base CI: [run 34160201371](https://github.com/Reidond/kedra/actions/runs/34160201371),
completed/success, exact base SHA verified through GitHub CLI. CI for these local
changes: not-run; no commit or push was requested. Source file SHA-256 identities
are in environment.json. [ADR 0004](../../adr/0004-synthetic-home-review.md).

## Question and experiment

Can selected bytes survive later app writes while local-only and uncommitted
changes stay out of source, and conflicts remain explicit? The experiment uses
normal writable synthetic files, a private Git review repository and a separate
synthetic source repository. It does not enroll a real home. A fixed Cargo example
accepts no home/source path arguments and creates fresh evidence under target/r03.
The implementation is [the r03_home example](../../../crates/sysroot-core/examples/r03_home.rs).

Whole-file export fails the disposition requirement. Git index flags cannot
represent line ownership. Chosen: a pinned S tree, separate exact-line I decisions,
and source-side temporary indexes. Git handles patch creation and three-way apply;
the prototype is deliberately limited to explicit full-line replacements.

The golden session starts with B = font=12, border=2, animation=200 and
L = font=14, border=4, animation=150, three adjacent edits in one Git hunk.
Select only font=14; mark border=4 local-only; leave animation=150 uncommitted.
The app then writes font=16. S remains font=14/border=2/animation=200. Source
receives exactly S, while L keeps font=16/border=4/animation=150. A candidate
N = font=12/border=3/animation=250 conflicts and does not change L or B.
Exact files and patch are in fixtures/; session.log records the observed tree
364e40197acc06d6cdeefb7b849df46613ae15aa and synthetic source commit
5f42da166fd60c5c70cc0baee00ff199eb4a4f2f. These identify fixture objects, not a
commit to Reidond/kedra or an image deployment.

## Environment and checks

Native Windows, x86_64-pc-windows-msvc; Git 2.55.0.windows.1;
rustc 1.98.1 (48a229cea 2026-09-01), Cargo 1.98.1 (797e8a9bc 2026-08-05),
LLVM 22.1.8. Repository toolchain pin selected without changing the global default.
No added dependencies. Git SHA-1 fixture object format, fixed synthetic identity,
2000-01-01 commit dates, empty Git global config/templates/hooks, LF bytes.
The launcher clears inherited GIT_* variables per subprocess. It does not claim
defense against a malicious Git executable, same-user attacker or path replacement.

| Check | Actual | Exit | Status | Evidence |
|---|---|---|---|---|
| cargo fmt --all -- --check | No differences | 0 | pass | fmt.log (empty success output) |
| cargo clippy --workspace --all-targets --locked -- -D warnings | No warnings | 0 | pass | clippy.log |
| cargo test --workspace --locked | 19 tests and 1 doctest; includes 12 R03 tests | 0 | pass | tests.log |
| cargo build --workspace --release --locked | Workspace release targets built | 0 | pass | build.log |
| cargo run -p sysroot-core --example r03_home --locked | Synthetic session assertions passed | 0 | pass | session.log |
| cargo metadata --locked --no-deps --format-version 1 | Three edition-2024 packages, explicit flat targets, example test=true | 0 | pass | environment.json |
| Example --home synthetic-home / --help | Path argument refused before any fixture creation; help succeeds | 2 / 0 | pass | Manual invocation on 2026-09-08; fixture-directory count unchanged |
| git diff --check; source identity/layout review | No diff whitespace errors; recorded source hashes match; no first-party src/ paths | 0 | pass | Local final review |
| Current-change Actions/Linux execution | Not published or dispatched | — | not-run | Base run cannot certify new code |

## Case evidence

All test names below are in examples/r03/tests.rs and ran through Cargo, exit 0.
Expected negative operations return typed errors and leave source/live state
intact; a passing refusal is not support for the rejected operation. results.json
records the remaining packet cases separately.

| Case | Expected and actual outcome | Status | Test |
|---|---|---|---|
| One line from larger hunk; later writes | One full-hunk input; only font=14 in S; later font=16 visible unstaged; index tree and patch identical | pass | partial_line_staging_survives_later_application_writes |
| Local-only and visible edits in same file | Source exact bytes exclude border=4 and animation=150; L unchanged | pass | exact_export_no_private_objects_or_ancestry_and_pending_deployment |
| Private history isolation | Real private snapshot commit contains local values; all source objects exclude them; private commit unavailable in source; source-only parent | pass | same test |
| Exclusion before capture | Unknown synthetic auth.json marker absent from all private and source objects | pass | same test; fixed allowlist only |
| Host provenance; P vs B | Only hosts/desktop/home path updated; P records S/source commit; B unchanged | pass | same test |
| I/S overlap in either order | Explicit Classification; index unchanged; export independently rejects overlap | pass | selection_and_policy_overlap_fail_in_both_orders |
| Ignored value changes again | border=5 visible; stale I blocks export; explicitly clearing I exports only original S | pass | ignored_value_changes_again_becomes_explicit_review |
| Duplicate/ambiguous/moved anchors | Refuse duplicates, relocation, insertion/deletion and stale baseline; no fuzzy remapping | pass | duplicate_reordered_inserted_deleted_and_changed_baseline_anchors_refuse |
| Source edits selected key | ContentConflict; source HEAD/index/worktree, L and S unchanged; no P | pass | source_same_line_conflict_preserves_head_index_worktree_live_and_staging |
| Nonoverlapping source advance | Selected font plus upstream border preserved | pass | source_advance_nonoverlap_and_adopted_value_and_repeat_export |
| Upstream already adopts S; repeated export | No additional commit; publication remains pending deployment | pass | same test |
| Dirty source / source advances after preparation | Refuse without losing source work; reprepare required | pass | dirty_source_and_post_prepare_advance_refuse |
| B/L/N conflict | Markers only in private candidate; B/L unchanged; N=B gives L candidate | pass | baseline_conflict_never_writes_live_markers_or_advances_baseline |
| UTF-8 non-ASCII path; no final newline | Byte-for-byte round trip through Git patch/source | pass | non_ascii_path_and_no_trailing_newline_round_trip |
| Encoding and path restrictions; rename | CRLF/NUL/BOM, invalid UTF-8, traversal/absolute paths refused; missing renamed allowlist path returns I/O error | pass | unsupported_encoding_path_and_rename_refuse |
| Generated selected/later values | Four deterministic font variants preserve L and export only S | pass | selected_snapshot_no_loss_no_leak_across_generated_values |

## Failures and privacy review

The first targeted run failed 11 of 12 cases before Git initialization: Rust
canonicalize produced a Windows verbatim path which Git rejected for its global
config file (Git exit 128). Converting only generated subprocess path arguments
to ordinary slash form fixed it. The next run passed 10/12: the object audit had
incorrectly decoded binary Git tree objects as UTF-8, and a diff assertion assumed
adjacent removal/addition ordering. The audit now checks raw command success and
uses lossy decoding only to search ASCII synthetic sentinels; diff assertions
check the actual removed/added lines independently. No content is normalized in
the implementation. Initial Clippy failed two collapsible-if warnings (Cargo
exit 101); let chains fixed them without weakening lints. Final logs contain
the successful checks; these earlier failures are preserved here.

The source object audit includes unreachable objects, not just HEAD. Private
snapshots are generated synthetic content and are never fetched into source.
No remote is configured in either fixture repository. The term private describes
separation from export; filesystem ACL confidentiality is not proven. Logs were
reviewed and checkout/generated path prefixes sanitized. No real home data,
credentials, production keys, account transcripts or machine identifiers were used.

## Reproduce

From a Kedra checkout with the installed pinned Rust toolchain and Git:

```sh
cargo run -p sysroot-core --example r03_home --locked
cargo test -p sysroot-core --example r03_home --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

Every run allocates a fresh target/r03 directory. Inspect its live-home,
private-review, source, selected.patch, export-*.index, merge-candidate.txt and
export-diagnostic.txt. Tests retain their own separate directories. No recursive
cleanup or existing-directory adoption occurs. Do not adapt this CLI to pass a
real home. The fixture publisher is sequential research code without durable
crash recovery. Cargo checks the example normally because test=true; release
build compiles production workspace targets, not a production home manager.

## Remaining gates and next experiment

R03 is still blocked for real adoption. Not-run: Noctalia v5 effective-value
projection and comparison with persistent app-owned fields; deployment-aware
override cleanup; durable I/P reload and schema handling; policy reclassification
across N (including upstream adoption of an ignored value); general safe
insert/delete/rename support; safe discard; arbitrary Unicode/invalid-byte path
coverage; modes/labels/links/special files; permission/full-disk failures. The
prototype does not offer filtered ignored/normal review screens or a general
conflict-resolution UI. Conflicts stop; fixtures can be freshly reviewed and
recreated, but no production resolution/recovery protocol is claimed.

R04's concurrent writers, crash recovery and actual activation are not-run.
Serialized later writes prove index snapshot stability, not writer coordination
or lost-write safety during activation. No baseline is applied to live files.
Next: extend synthetic R03 with a narrow versioned Noctalia effective-settings
fixture and explicit I/S/P transition tests across N; keep activation behind R04.
R11's unavailable automatic discovery/editor checks do not block this work.

Durable findings are linked from kedra-home and kedra-rust-workspace.
Primary mechanisms: [Git update-index](https://git-scm.com/docs/git-update-index),
[Git apply](https://git-scm.com/docs/git-apply),
[Git merge-file](https://git-scm.com/docs/git-merge-file), checked 2026-09-07.
