# Verified release status

This records observed qualification, not a promise that every implemented path has shipped. Historical logs and reports remain in Git history; routine build logs belong to Actions artifacts.

| Scope | Observed result |
|---|---|
| Published installer | `desktop-44-x86_64-r1`, source `c660c58d9bbbbe34119f6ea35a03528485455848`, candidate [34255228394](https://github.com/Reidond/kedra/actions/runs/34255228394). Exact encrypted two-disk installation, ISO-free desktop, enforcing SELinux, writable home, read-only root and untouched data disk passed. |
| Publication | Promotion [34288691672](https://github.com/Reidond/kedra/actions/runs/34288691672) failed after signing with HTTP 500. Exact-byte key-free recovery published verified version/channel assets. That failed run remains failed. |
| Enrollment | Published r1 anonymous channel enrollment, repeated-enrollment refusal with unchanged state and later reboot persistence passed in the retained disposable VM. |
| Next candidate | [34293133114](https://github.com/Reidond/kedra/actions/runs/34293133114), source `0eb1cf0`, passed signed image/ISO construction and separate fresh encrypted installation. It was not promoted at the last recorded observation. |
| R2 publication attempt | [34325343190](https://github.com/Reidond/kedra/actions/runs/34325343190) passed preparation and protected signing, but publication failed and left an empty r2 draft. R2 is not published; preserve the signed artifact and inspect the draft before explicit recovery. |
| Update and home | Signed update/retained rollback and replay/hold regressions passed [34296984697](https://github.com/Reidond/kedra/actions/runs/34296984697). Real signed A/B/A home transitions passed [34296369019](https://github.com/Reidond/kedra/actions/runs/34296369019). Desktop regression passed [34296984700](https://github.com/Reidond/kedra/actions/runs/34296984700). |
| Source checks | Latest accepted-main workspace [34688808033](https://github.com/Reidond/kedra/actions/runs/34688808033) passed at `76abf57cff5caad48777152a996502472da887bb`. Current changes pass the local checks below; new-source Actions remains not-run. |

Current source includes channel discovery and home-status features absent from r1. Midnight resolved-input refresh, protected renewal, fixed-channel enrollment/staging and repository consolidation are locally implemented/reviewed and ready for commit. These changes are not retroactively part of r1.

Local checks pass: Windows Rust 1.98.1 formatting, Clippy with warnings denied and release build; OpenSSL 3.6.1 release interoperability; signed material CLI no-change, five changed-input cases including same-NEVRA payload change and four authentication refusals; Python AST parsing of 26 files, WSL Bash syntax, delegated production/test YAML validation, whitespace and independent cross-reviews. The Cargo E2E command passed with **zero Windows cases**. See [WL-20260912-04](../worklog.md#wl-20260912-04--2026-09-12--complete-local-production-integration) for the completed integration record.

Refresh uses an authenticated resolved-input baseline before OCI/ISO production. R1 lacks that schema: its first midnight refresh builds a candidate, and later qualified promotion establishes the baseline required for no-change checkpoint renewal. The new implementation is not itself evidence of an observed production renewal.

Current-source external checks are not-run: Linux/native Actions, scheduled Fedora resolution, protected renewal/publication, renamed VM/ISO workflows, installed --channel enrollment/staging and fresh ISO installation/forward update. The empty r2 draft blocks conflicting sequence-2 promotion until explicit exact-byte recovery. Physical hardware/Secure Boot, key rotation, all interruption/full-disk/older-schema cases, arbitrary home groups and second-machine lifecycle remain unqualified. Claude bundling still requires its separate terms decision. Owner account/model/vault authentication is not simulated by logged-out startup checks.

The initial r1 checkpoint expires **2026-09-15 23:05:17 UTC**. Fetch the current channel; expired metadata never becomes current merely because an ISO remains downloadable. Previously verified local boot and retained rollback remain separate recovery paths.
