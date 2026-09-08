# R08/R10: release verification subset

2026-09-09 owner-media milestone: separately approved candidate 34255228394 at
accepted `c660c58` passes native signing/registry-storage compatibility, ISO
construction and all ten exact-media manual installation cases. Installed bootc
uses the exact owner digest with containerPolicy; source/key identity, doctor,
mounts and retained sentinel pass. See the
[candidate record](owner-candidate-34255228394.md) and
[public installation evidence](../R02-installer/owner-34255228394/REPORT.md).
Metadata promotion/enrollment, lifecycle renewal and rotation remain open.
The historical preparation and earlier research results below retain their scope.

2026-09-08 protected-publisher preparation: promote.yml now separates public
exact-media preparation, no-checkout metadata signing and key-free publication.
It binds independently reviewed candidate/qualification bytes, successful current
main build/attempt, dedicated public authority and unchanged prior signed channel.
The publisher verifies the complete ISO and native signatures again, uploads a
versioned draft, then publishes a single channel bundle with exact-byte readback.
Existing tags/drafts refuse blind replacement. See ADR 0023 and build/release/README.md.
Python AST, YAML and Bash syntax pass; pinned native Cosign 3.1.3 help supports the
selected bundle signing interface. Workspace 34247944518 at a6cd08f passes the
existing native channel/OpenSSL and actual CLI checks. Actual production execution,
publication interruption/races, renewal, expired recovery and rotation are not-run.

Owner-authorized authority setup, 2026-09-08: a separate encrypted P-256 key is
generated, with public fingerprint
`a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`.
Official pinned Skopeo/Cosign containers generate/use it without network; an
independent OpenSSL check verifies a public setup confirmation. Its encrypted key
and passphrase are GitHub Actions environment secrets in kedra-desktop-signing,
restricted to main and the owner reviewer with no admin bypass. Main requires
the Actions rust check and rejects forced/deleted history. Public authority files
are prepared for source. No production key enters the disposable research jobs.
The owner confirmed Bitwarden backup and retrieval on 2026-09-08; the agent did not
access recovered private material or the vault. Production candidate execution,
exact media qualification, metadata publication/renewal and rotation remain not-run.
See build/release/authority/README.md for exact tool digests and configuration.

2026-09-08 follow-up: workspace 34242602461 at `3d29689` passes the real CLI's
P-256 fingerprint against independent OpenSSL SPKI DER hashing, private-key
input refusal and the existing signature/ISO-assembly workflows. Installed
Cosign/Skopeo/bootc interoperability also passes R01 34242602458.

The prepared `.github/workflows/release.yml` separates build, image signing and
installer jobs. It is main-only/manual, requires an opt-in variable plus existing
owner-review/main-only environment controls, and has no public authority files
at that preparation point. The signing job has no checkout/artifact execution; the installer job must
anonymously verify the exact digest, installed source and public-policy files.
Its output remains a candidate pending exact-media installation and promotion.
Syntax checks pass; production execution, recovery/key provisioning, signer
isolation under actual authority, promotion races, freshness and rotation are
not-run. Read-only GitHub inspection found no environments and unprotected main;
no configuration was changed. See build/release/authority/README.md.

Historical prototype results follow; removed unit tests are not current policy.

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

Publication follow-up: exact source `315389786db7b1519c5590ce9583ba1d972b163c`
passed [Linux workspace run 34170739316](https://github.com/Reidond/kedra/actions/runs/34170739316),
including the OpenSSL interoperability script's 16 valid signatures and CLI
wrong-key/tampered-manifest/corrupt-installer refusals.

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
