# R08/R10: release verification subset

Status: pass for local pure verification tests; full gates remain blocked.
Date: 2026-09-08 (Europe/Kiev). No production signing key, protected signer,
promotion, enrollment or privileged operation was configured.

Implemented release.rs and ordinary-user `sysroot release verify`; see ADR 0006.
RustCrypto p256 0.14.0 verifies P-256/SHA-256 over exact JSON bytes with SPKI PEM
keys and detached base64 DER signatures. Serde rejects duplicate/unknown fields;
records and high-water state have explicit versions. Inputs are bounded.

Local Windows Rust/Cargo 1.98.1: pass — Clippy, 38 workspace tests plus one
doctest, including ten release tests, and release build. OpenSSL 3.0.13 in Ubuntu
WSL2 independently generated 16 signatures, all verified by the Windows CLI
against a three-byte synthetic artifact. Test keys were deleted.
tests/release-interop.py reproduces this and adds CLI negative cases on Linux.
New-revision Linux CI is pending publication.

| Case | Result |
|---|---|
| Exact signed bytes and correct key | pass |
| Changed bytes, wrong key, malformed signature | pass: rejected |
| Signed duplicate/unknown fields, new schema | pass: rejected |
| Candidate, mismatched target, newer protocol | pass: rejected |
| Malformed reference/installer path | pass: rejected |
| Initial/repeat/no-change checkpoint | pass |
| Old generation/sequence or changed same-number record | pass: rejected |
| Expiry, future clock, bad binding, corrupt state | pass: rejected |
| Retained old release versus forward replay | pass |
| Explicit two-key overlap | pass (unit fixture only) |
| Installer size/content corruption | pass: rejected |
| Actual Cosign blob interoperability | not-run |
| Production signer isolation/promotion races | not-run |
| Persistent root state, interruption, helper protocol | not-run |
| Offline-machine key rotation/retention drill | not-run |

The pure result is not machine authorization. Root-side key selection, source/CI
authority, exact OCI/home binding and filesystem/journal handling need integration
and negative tests before deployment becomes available.
