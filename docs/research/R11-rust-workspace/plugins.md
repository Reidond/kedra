# Local plugin migration — 2026-09-07

Source: uncommitted changes on main, inspected base f3b44f1. Agent: Codex.
Decision: ADR 0003. Overall R11 remains blocked; R01-R10 remain not-run.

The owner replaced the submodule/link integration and custom xtask check with
an ordinary-file Codex/Claude plugin. Three Rust packages remain. The plugin has
30 skill directories: 12 Kedra and 18 selected Rust skills. Both manifests use
the same tree, with no generated discovery copies or synchronization.

| Case | Result | Evidence |
|---|---|---|
| Previous Windows links | fail | core.symlinks=false; read_link failed with os error 4390 |
| Previous symlink creation probe | blocked | Administrator privilege required |
| Rust formatting | pass | cargo +1.98.1 fmt --all -- --check |
| Clippy | pass | cargo +1.98.1 clippy --workspace --all-targets --locked -- -D warnings |
| Unit/integration tests | pass | cargo +1.98.1 test --workspace --locked: 7 tests |
| Doctest | pass | 1 sysroot-core doctest |
| Release build | pass | cargo +1.98.1 build --workspace --release --locked |
| Codex manifest | pass | plugin-creator validate_plugin.py against plugins/kedra |
| Claude manifest | pass | claude plugin validate ./plugins/kedra |
| Claude marketplace and skills | pass | claude plugin validate on the root marketplace and plugin skills directory |
| Upstream file preservation | pass | 92 files in 18 skills match the preserved clean snapshot byte-for-byte |
| Ordinary-file layout | pass | 30 skills; no plugin symlinks/junctions or superseded integration paths |
| Installed interactive plugin discovery | not-run | No personal installation or model session launched |
| New-revision Actions | not-run | Changes have not been committed or pushed |

Native Windows MSVC with Rust/Cargo 1.98.1. The Codex validator used an existing
Python 3.12 and cached PyYAML import path; no dependency was downloaded or installed.
An initial invocation failed on missing PyYAML before validation. This successful
rerun was a local authoring check, not a new project tool or CI requirement.
Claude Code was 2.1.263; Codex CLI was 0.153.4. The CLI's filtered marketplace
query returned no installed/available entries for the unregistered repository
marketplace. Native manifest validity does not prove interactive installation.

The original clean upstream checkout and superseded generated discovery copies
were preserved under ignored target/ backups. The submodule gitlink was removed
from the index; source additions remain for normal review. Earlier worklog and
toolchain troubleshooting were preserved and updated.

Upstream README/metadata declare MIT but the pinned revision lacks LICENSE.
Retained notices document the gap; redistribution review remains unresolved.
Source-layout enforcement is now review plus standard Cargo metadata, not the
removed validator. Historical REPORT.md/results.json describe the earlier CI
experiment and are not overwritten with local results.

## Published follow-up

Plugin commit a872522 was merged with concurrent update-planning changes at
891cc1506851365d32747caf63b89300294ce71a and pushed to main. The plugin ADR is now
0003; the concurrent update-plan ADR retains 0002. Both plugin/skill validators
passed after merging. [Actions run 34160120958](https://github.com/Reidond/kedra/actions/runs/34160120958)
completed successfully for that exact merge. This follow-up does not assert an
unobserved CI outcome for its own later documentation commit.
