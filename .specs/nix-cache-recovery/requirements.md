# D5: trusted cache substitution and recoverable temporary inputs

Status: preparatory source and specification; not integrated or runtime-qualified.
Date: 2026-10-02. Owning layer: `codex/nix-cache-recovery`, after D4 freezes.
Source basis: PR32 `3ba7d1bbe0278e11a51bf17c93aec4d2b874874b`; the final D4
head must be recorded at integration. See [topology](../nix-delivery/topology.md).

## Scope

An ordinary-user consumer should reuse an explicitly authorized producer's binary
closure without running its recipe. Current bundle import validates independently
selected content, but cannot authenticate a producer. Verification snapshots and
native recipe inputs currently rely on destructors; SIGKILL leaves large copies.
D5 adds an explicit signed-file substitution boundary and lock-backed cleanup,
then qualifies real native publication and bounded-filesystem ENOSPC recovery.

Consumers supply their own keys, policy scope/revision and independently resolved
recipe through D3/D4 APIs. No Kedra target table, release key, installed helper,
production signer, live home, daemon service or registry publication is involved.
Existing expected-hash import remains a distinct content-transfer operation.

Actors: the producer exports/signs; the ordinary-user consumer chooses policy and
recipe and imports/runs; the recovery caller reclaims abandoned temporary inputs.
The parent qualification process owns generated fixture resources and evidence.

## Acceptance conditions

- **AC-C1, authenticated substitution.** Given a generated producer and consumer
  policy, substituting a signed exported runtime closure into an empty consumer
  verifies the signer, receipt validity, policy scope/revision and exact recipe,
  platform/prefix, root receipt and bundle bytes before publication. The real
  resulting executable/library workflow runs after the producer store is removed.
- **AC-C2, refusal before publication.** Unauthorized keys, wrong purpose, scope,
  policy revision, time interval, recipe/root/platform/prefix, altered bytes,
  conflicting object content and incomplete references refuse without admitting
  new objects/images, changing roots or selecting a profile.
- **AC-C3, reusable consumer boundary.** A separately compiled D4 consumer uses
  the same public engine/catalog API with its own policy and definitions. A later
  build reuses the admitted result; explicit independent rebuild still executes
  and compares bytes. Cache trust does not grant installed OS authority.
- **AC-C4, leased cleanup.** Each composition/native temporary input has a durable
  private lease and held advisory lock before any payload writer receives a path.
  Public recovery reclaims abandoned registered copies, including partial copies,
  and preserves active and SIGSTOP-held snapshots of the same or other identity.
- **AC-C5, conservative recovery.** Recovery follows no symlinks, does not recurse,
  and deletes only the closed four composition or three native filenames after
  validating every member. Unknown schema, changed inode/device, unexpected
  entries, wrong owner/mode or hardlinks are reported and untouched. Legacy
  unleased temporary directories are never adopted from their names or age.
- **AC-C6, native publication retries.** Four actual native publication windows
  are captured and killed: temporary image before recorded ID, recorded ID before
  binding, binding before final tag, and final tag before journal retirement.
  Public retry rebuilds the pre-ID case and revalidates/reuses the recorded image
  in later cases while retaining exact daemon/parent/material bindings.
- **AC-C7, actual full filesystem.** Actual ENOSPC in an isolated bounded cache or
  store filesystem fails safely during snapshot copying, transaction/binding
  writes and closure admission/root publication. After removing only the fixture
  filler, public recovery/retry succeeds and preserves unrelated valid data.
  Daemon layer-storage exhaustion requires its own bounded daemon-data fixture.
- **AC-C8, evidence and ownership.** Every fault/retry uses an independently pinned
  private single-link executable whose hash is recorded before and after that
  invocation. Capture the real state before killing; missed windows, simulated
  I/O errors and reconstructed journals confer no fault coverage. All cleanup is
  exact fixture-owned; retained default VM/container and external sentinels survive.

The success flow is export/sign, independently select policy/recipe, authenticate,
stage/verify, admit, then run. Any authorization/content exception ends before
admission. Any I/O exception preserves the prior selected root and leaves only
owned recoverable transaction state. Recovery's success flow is lock, validate,
remove exact abandoned members and retire the lease; active/unsafe records follow
the explicit skip/refusal flows instead.

## Limits and failure behavior

Records use closed versioned schemas and bounded reads; existing bundle/tree/OCI
limits remain enforced. Signed authorization is checked again immediately before
admission, so expiry during a long copy is refused. Policy is the explicit immutable
selection for that invocation; no online revocation or newest-producer guarantee
is claimed. A signature authenticates a producer assertion, not reproducibility.

The registry marker and lease must be complete before large payloads are written.
An interrupted initial metadata publication can leave a small unregistered or
malformed record; recovery reports/refuses it rather than inventing ownership.
The explicit recovery result lists removed, active and refused entries. Callers
must report refusals and must not report complete cleanup when any remain.

Excluded: Nix cache protocol compatibility, fetching/scheduling services, automatic
key discovery, production signing, power-loss claims without an actual experiment,
unit/model/mock/doctests, repository/source scanners and new custom test runners.
Preparatory source existence is not a compiler, CLI or runtime pass.
