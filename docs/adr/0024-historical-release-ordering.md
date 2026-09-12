# ADR 0024: Authenticate expired predecessor ordering separately

Date: 2026-09-09. Status: implemented on development; native publisher qualification pending.

## Context

The first production channel has a finite expiry. Both preparation and publication
used ordinary fresh-channel verification for its predecessor. Once that checkpoint
expired, even a newly resolved, fully qualified replacement candidate could not
advance the channel. Installed clients already retain ordering floors separately
and apply freshness to incoming metadata; they need no expiry exception.

## Decision

Add a distinct `sysroot release history` command and `VerifiedHistory` type.
The verifier authenticates exact release/checkpoint signatures, bounded schemas,
scope, binding, valid lifetime, resolution timestamps and the normal future-clock
tolerance. An independently retained previous state also enforces generation,
release sequence, same-number hashes and resolution floors. The actual current
clock is always used. Present expiry is reported separately.

History returns `ordering_state`, `historical_only: true`, and explicit false
freshness/deployment-authority fields. It cannot return a `VerifiedUpdate` or
`next_trust_state`. It neither updates installed trust nor stages an image.
The ordinary update verifier retains its unconditional expiry refusal.

Use history only for the predecessor in release preparation/publication. Keep
genuine matching media qualification, recent candidate resolution, accepted-main
identity, increasing sequence/generation and unchanged channel/asset checks.
The newly signed pair must pass ordinary freshness verification against the
authenticated predecessor's complete ordering floor.

The isolated signer retains its existing exact prior bundle/payload bindings and
strict incoming issue/expiry/resolution checks. It executes no repository verifier
and receives no new unbound authority summary. No privileged request, root trust
schema, key, helper policy or production protection is changed.

## Evidence and limits

Native Windows Rust 1.98.1 formatting, Clippy and release build pass. Actual
CLI/OpenSSL 3.6.1 E2E passes expired historical verification, strict incoming
refusal, fresh higher continuation, signature/scope/schema/binding/lifetime/future
negatives and ordering regressions. The new CLI also authenticates the actual
published r1 pair as historical-only data with its established public key.
The R01 guest workflow now exercises the same public CLI boundary and an expired
higher-sequence request through the installed helper; its new run remains pending.

This is not same-image checkpoint renewal, key rotation, proof that an old release
is current, or a completed production expired-channel recovery drill. Full
publisher qualification still requires an actual matching accepted candidate and
genuine installation evidence. See [release instructions](../RELEASES.md) and
[R08](../research/R08-release-protocol/REPORT.md).
