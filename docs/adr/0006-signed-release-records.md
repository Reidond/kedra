# ADR 0006: exact-byte signed releases and freshness checkpoints

Date: 2026-09-08. Status: accepted for verification; signer authority and installed
state transitions remain gated by R08/R10 experiments.

## Decision

Use JSON records signed over their exact file bytes with ECDSA/P-256/SHA-256.
Public keys use SPKI PEM and detached signatures use standard base64 of ASN.1 DER.
These are standard OpenSSL/Sigstore encodings. RustCrypto's p256 verifies signatures;
Kedra implements no cryptographic primitive or custom canonical JSON language.
Serialization order and whitespace are part of the signed bytes. Readers reject
duplicate/unknown fields and unknown versions.

An immutable promoted release binds project, target, architecture, Fedora major,
repository, source revision, workflow/run/attempt, sequence, exact OCI digest,
home provenance hash, installer name/size/SHA-256, approval and minimum protocol.
A candidate signature does not grant promotion. The workflow identity in a signed
record is a signer statement: verification alone does not prove the job happened
or authorize that signer to issue it. CI authority remains a separate gate.

A separately signed checkpoint binds the record's SHA-256 to the same scope, a
monotonic generation, issue/expiry and last successful resolution time. Maximum
lifetime is initially seven days; future clock tolerance is five minutes. A
successful no-change check can refresh a checkpoint pointing to the same release.
Corrupt/unknown-version state is never an empty default.

Independent trust high-water state retains generation/digest, highest release
sequence/digest, scope and resolution history. Older values, same-number changed
records and backwards resolution history fail. Rechecking identical records is
idempotent. Offline verification of an old retained release is distinct from
routine forward eligibility; rollback must preserve the high-water state.

## Interfaces and boundaries

The pure library verifies bytes and returns a typed result plus proposed next
trust state. It performs no network/process/privileged operation or state write.
An installed helper must load keys, enrollment and state independently and must
not accept a caller's verification flag. That integration is not yet implemented.

`sysroot release verify` is an ordinary-user offline verifier. It optionally
hashes an installer, checking size and SHA-256. JSON output explicitly says that
channel freshness and deployment authority were not established. Use an
independently trusted public key; a key beside an untrusted artifact does not
establish its publisher. Inputs are bounded before parsing.

Trust distribution/rotation, persistent journaling, protected signer jobs,
promotion races, OCI-to-home binding validation and installer integration remain
required. The unit key-overlap fixture is not a production rotation drill.

## Evidence

Ten Rust tests cover exact-byte signatures, malformed/duplicate/unknown input,
target/candidate/protocol refusals, replay, no-change freshness, expiry/clock
errors, rollback high-water preservation, explicit key overlap and ISO corruption.
OpenSSL 3.0.13 independently generated 16 signatures accepted by the Windows
verifier. Linux CI also runs CLI wrong-key/tamper/artifact cases. Private fixture
keys are deleted; none is a production default.

Sources reviewed 2026-09-08:
[RustCrypto p256](https://docs.rs/p256/0.14.0/p256/ecdsa/index.html),
[Sigstore blob signatures](https://docs.sigstore.dev/cosign/signing/signing_with_blobs/),
PLAN.md, docs/UPDATES.md and R08/R10.
