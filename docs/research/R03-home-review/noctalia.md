# Noctalia projection and disposition model

Date: 2026-09-08 (Europe/Kiev). Local model and native VM projection tests pass.
This supplements the earlier Git-line prototype and does not open real
home adoption or activation.

Implemented crates/sysroot-core/noctalia.rs, ten tests and the path-free
`r03_noctalia` Cargo example. No new dependency or package was needed.

```sh
cargo run --locked -p sysroot-core --example r03_noctalia
```

The example creates an in-memory synthetic review with a selected theme, a
local-only border preference and a visible uncommitted input preference. A later
theme write stays separate; a source receipt and serialized-state reconstruction
precede a synthetic next-baseline transition. No home path is read or written.

| Case | Result |
|---|---|
| Full-export projection excludes unknown/private fields | pass |
| Missing/malformed fields and unqualified versions | pass: rejected |
| Selected/local-only/uncommitted coexistence | pass |
| Later GUI value leaves selection pinned | pass |
| Exact ignore expiry versus persistent app ownership | pass |
| Overlapping selection/policy in both orders | pass: rejected |
| Source receipt and no repeat export | pass |
| Intermediate publication-prefix deployment | pass |
| Unrelated baseline change and owned field | pass |
| Conflicts and stale acceptance preserve state | pass |
| Canonical reconstruction and corrupt/cross-instance refusal | pass |
| Actual Noctalia 5.0.1 export/IPC projection in VM | pass: run 34179189185 |
| Filesystem persistence, source export, activation and recovery | not-run |

Windows Rust/Cargo 1.98.1 formatting, Clippy, all 48 workspace tests plus one
doctest, release build and the synthetic example passed. Linux run 34179189211
and [native VM run 34179189185](https://github.com/Reidond/kedra/actions/runs/34179189185)
pass at `d74c4c0`; the latter records `KEDRA_R03_NATIVE_PROJECTION_PASS` after
qualified full export, theme IPC change and safe projection. Raw exports are
not recorded. The projection helper enters only the
disposable R07 test derivative, never the production desktop payload.

Only safe values are persisted by the model. It does not claim to parse every
Noctalia setting or to provide a writer lock/filesystem transaction. ADR 0007
records the distinction and the required R04 acceptance boundary.
