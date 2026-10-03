# D5 design

Status: prepared modules are unregistered; ignored shadow replacements and an
original/replacement SHA-256 manifest live in `target/nix-delivery/d5-preparation`.
Actual source adoption, compilation and runtime gates remain pending.
Requirements: [requirements.md](requirements.md).

## Signed-file substitution

Use the existing bundle transport and admission transaction. A new pure cache
module authenticates bounded exact receipt bytes through an existing-crypto
primitive, checks consumer policy, and produces a private-field capability:

```rust
verify_cache_receipt(payload, signature, public_key, policy, expected, now)
    -> Result<VerifiedCacheReceipt>
```

`CachePolicy` contains schema, scope, revision, half-open validity interval and
1..=64 unique lowercase SHA-256 SPKI fingerprints. `CacheReceipt` contains schema,
the `sysroot-engine-binary-cache-v1` purpose, matching scope/revision, a validity
interval contained by policy, exact bundle hash/length and one full root receipt.
The root's resolved recipe must equal the consumer's independently resolved
`ResolvedBuildSpec` byte-for-byte after typed serialization. Derivation identity
is recomputed from that recipe; runtime image, tree digest and unique non-self
references remain explicit. Namespace/platform arrive through the selected recipe
and existing engine validation, not through Kedra release constants.

The signed bundle hash binds all transitive receipts/data/runtime images. One
root is intentional: reject extra roots, while existing dependency traversal
requires every bundled member to be reachable. No new archive format is needed.

Use the already pinned workspace `p256`, `base64` and `sha2` dependencies directly
in the engine. A small private verifier preserves the existing exact-payload
ECDSA/P-256/SHA-256, SPKI PEM, base64-DER signature and SPKI-DER fingerprint
encodings without importing `sysroot-core` or its Kedra release/domain modules.
This deliberately repeats the small codec/verification boundary to preserve the
independent crate boundary; it does not implement cryptographic algorithms.
Receipt payload is bounded to 8 MiB, signature to 1024 bytes and key to 4096 bytes.
No release/core source or existing release limits change. Independent OpenSSL
signatures verify interoperability through the real public cache workflow.

Integration adds `Store::substitute` as the admission boundary. Authenticate the
receipt and expected recipe, snapshot/hash the input bundle, run all existing
manifest/tree/reference/OCI checks, then call
`VerifiedCacheReceipt::validate_bundle(roots, root, sha, bytes, now)` **before**
the import journal or first object/image admission. This private method checks
current validity and exact root/receipt/hash/length. Do not call ordinary import
first and compare its result afterward. No fallback accepts an invalid signature.
Return the existing import result plus authentication diagnostics at the adapter
boundary if useful; never serialize/reload the verified capability as authority.

The CLI remains a thin file adapter with explicit store/bundle/receipt/signature/
public-key/policy, explicit independent `--scope` and expected graph/root inputs.
Resolve graph/root locally and
pass that exact recipe to verification. A caller-selected output ID alone is not
the recipe-selection UX. Transport and signing are external; fixture signing uses
generated OpenSSL keys. Automatic network fetching is outside this slice.
For D4, first resolve the catalog's source/image policy, then bind cache scope to
the consumer's chosen namespace/package mapping before opening the store. A
resolved recipe alone contains no publisher or catalog namespace authority.
Retained valid local winners keep the existing store semantics; this substitution
policy does not retroactively revoke or automatically delete admitted objects.

Import journal publication uses the locked store's exact `transactions/import.next`
name: write/fsync the complete record, hard-link without replacement to
`import.json`, fsync the directory, remove the extra link and fsync again before
admission. Recovery removes a single-link unpublished temporary only when the
final journal is absent. A two-link temporary must match the exact final inode,
valid schema, store token and stage name before its extra link is removed.
Unknown aliases, conflicting final records and malformed authoritative records
refuse. This repairs the direct-write path where ENOSPC could otherwise leave
truncated authoritative JSON that retry could never parse.

## Managed temporary inputs

`ManagedSnapshot::create(parent, SnapshotPurpose::{Composition,Native})` creates
a unique owned 0700 directory under `parent/.sysroot-snapshots-v1`. Registry and
lease files are single-link mode 0600. Preserve the prior caller-parent contract:
owned and not group/other writable (0755 remains valid), without chmod or following
the final path component. Child access and
cleanup use retained no-follow directory descriptors and named inode checks.

The registry contains an immutable versioned random-token marker, permanent
management lock, `snapshot-NONCE` directories and `lease-NONCE.json` records.
A lease binds schema, registry token, nonce, purpose and directory device/inode.
First-use initialization writes/fsyncs marker and lock inside a new private small
initializer directory, then publishes the complete registry without replacement.
Concurrent first users consume the same complete winner; an incompletely written
registry never appears at the authoritative name. A killed initializer can leave
a small unregistered private directory, which is never adopted by filename alone.
The lease file is locked, written and synchronized, followed by directory fsyncs,
before returning a writable payload path. The held file lock is the liveness
signal; SIGSTOP and concurrent readers retain it. No PID/age threshold is used.

Closed payload members:

| Purpose | Allowed regular single-link mode 0600 members |
|---|---|
| Composition | `Containerfile`, `composition.json`, `foundation.tar`, `payload.tar` |
| Native | `Containerfile`, `native-driver.py`, `native-plan.json` |

Native integration must create its three files explicitly at mode 0600 instead
of inheriting ordinary `fs::write` creation modes. Only known files may be copied;
there are no subdirectories. Keep the guard alive until every consumer has finished;
detached consumers cannot outlive it. The native BuildKit call uploads the explicit
context bytes before the guard is retired; qualify this actual lifetime.
Because BuildKit preserves those input modes in its tar, the native implementation
identity advances to `kedra-native-v2-private-context`. Its native plan must be
resolved anew; old bindings and recorded v1 identities are not silently reused.

`recover_snapshots(parent)` takes the short registry lock and tries each operation
lock without waiting. It returns `{ removed, active, refused }`; unsupported or
unsafe entries retain their data. Validate the full listing and all member
metadata before deleting anything. Remove exact payload files, fsync, remove the
matching empty directory, fsync, then unlink the sibling lease and fsync again.
The lease outlives the directory, allowing retry after interrupted retirement.
Never recursively delete a snapshot and never touch native durable cache state.

Creation/recovery take registry then operation locks; creation takes a fresh
operation lock. Finish holds its operation lock then waits for registry, while
recovery tries operation locks nonblockingly and releases registry promptly.
Thus recovery cannot wait for the finishing operation while preventing its finish.
`finish(self)` returns cleanup errors; `Drop` provides best-effort cleanup only.

Initial marker/lease allocation failures occur before a path reaches payload code.
Synchronous creation failure attempts exact empty-directory/lease cleanup. A kill
during initial metadata publication may retain small incomplete state and is
reported/refused. No big payload can legitimately appear before the durable lease.
Legacy `.sysroot-context-*` and `.context-*` directories outside the registry remain
untouched. The known historical 6.4 GiB leftovers need their previously captured
exact ownership evidence; this implementation does not manufacture that evidence.

## File ownership and integration order

Prepared now, solely owned by D5: new `sysroot-engine/cache.rs`, `snapshot.rs` and
this `.specs/nix-cache-recovery/` directory. They are not compiled by active D1.
No existing module, manifest, CLI, test or harness file changes until D4 freezes.

Compatibility/rollback: existing expected-hash bundle import and old context input
formats are unchanged. The new lease registry is separately versioned; old binaries
do not own its cleanup. Reverting D5 must preserve that registry and require a
compatible explicit recovery operation, never a broad delete during rollback.
Unknown cache policy/receipt or lease schemas refuse rather than being downgraded.

Parent adoption later: engine lib/Cargo, pre-admission bundle hook, context snapshot replacement,
native temporary guard, composition/recovery adapter, thin sysroot/lab CLI wiring,
existing E2E extensions, Cargo.lock, operational docs/worklog/status/skill updates.
The shadow sysroot Cargo change makes the existing workspace rustix dependency
available under cfg(unix), enabling bounded no-follow/nonblocking cache metadata
reads on macOS and Linux; Windows still refuses unsupported engine commands.
Record actual D3/D4 API names and frozen source at that boundary. No executor
or scheduler change is required for explicit substitution followed by normal build.

## Risks and review decisions

| Risk | Decision / acceptance evidence |
|---|---|
| Import publishes before authorization | One pre-admission hook after all staged verification; TC-C2 observes unchanged store roots/data. |
| Cache keys inherit root authority | Dedicated explicit consumer fingerprints/scope; no installed key lookup or target constants; TC-C3 standalone consumer. |
| Collector deletes another reader | Persistent lock held during full lifetime; TC-C4 includes same/different identity and SIGSTOP. |
| Cleanup escapes through alias/replacement | No-follow descriptor access, exact inode binding, allowlist and pre-delete full validation; TC-C5. |
| ENOSPC tears durable state | Existing `.next` publication retained; actual bounded-filesystem failure/retry TC-C7; initial metadata uncertainty refuses. |
| Old static faults mislabeled native | Four newly captured native windows with exact per-invocation executable hashes; TC-C6/TC-C8. |

This design does not establish power-loss durability, independent reproducibility
or production release eligibility by assertion. Those require their own evidence.
