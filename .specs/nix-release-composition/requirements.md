# ARM release composition requirements

State: design only, 2026-10-02. D2 implementation is deferred until the parent
freezes and publishes D1 and grants the next branch boundary. Inspected source:
PR32 `3ba7d1bbe0278e11a51bf17c93aec4d2b874874b`; current shared branch is D1.
Only this spec directory belongs to this preparer. Parent owns worklog/status/Git.

## Problem and scope

Current production candidates are assembled by the shell image recipe; the
engine's composition/native output is not consumed by the signed release path.
This is an inspected missing connection, not a measured production incident.
Prior native-generation tests do not establish install/update behavior. D2 connects
the existing ARM backend to the normal Fedora bootc candidate pipeline and
qualifies a real derived image with disposable signing, installation and A/B/A
boots. Desktop retains its existing builder: current engine/replay are ARM-only.

Actors: public candidate builder; ordinary-user catalog author; isolated production
signer; disposable fixture controller; generated VM account; owner/operator.
AI services do not participate in any product flow.

## Acceptance

### Candidate construction

- **AC-01:** Given committed qemu-arm64 source, pinned Fedora base, resolved RPM
  material and pinned tools, when the public candidate builder runs, then a fresh
  foundation is composed and natively generated through existing public APIs,
  with actual material/source/foundation provenance in its output receipt.
- **AC-02:** Given a mismatched input hash, foundation, target, source revision,
  package closure or native identity, when the builder consumes it, then it fails
  before final candidate publication and preserves unrelated state. A partial
  build never becomes a valid result merely because a tag exists.
- **AC-03:** Given unchanged resolved input bytes and verified prior stable,
  when production preflight runs, then it emits unchanged and schedules zero
  candidate/sign/publication work; invocation times, output IDs and generated
  initramfs bytes do not participate in that input comparison.
- **AC-04:** Given a changed native recipe, composition implementation or pinned
  catalog input, when production preflight runs, then it observes changed inputs
  even if source RPM names/versions did not change.
- **AC-05:** Given a completed candidate, when release validation runs, then the
  full container harness validates that exact final digest without worktree
  overlay before the isolated signer receives it. Final installed identity,
  material, RPM rows and composition/native receipts match the reviewed build.
- **AC-06:** Given an invalid final material binding, stale main source, failed
  candidate test or unsupported signing scope, when release advances, then it
  refuses signing/publication. The existing signer retains no checkout or
  candidate execution and the release workflow identity does not change.

### Disposable installed qualification

- **AC-07:** Given fixture-signed media for the complete derived ARM image and
  two generated disks, when the controller performs a fresh installation, then
  only the explicitly selected disk changes, and an ISO-free boot verifies exact
  target/digest, Secure Boot, enforcing SELinux, generated artifacts and a
  writable newly seeded home. Sentinel/media integrity is retained.
- **AC-08:** Given unsigned/wrong-key payload or Secure Boot disabled, when
  installation verification runs, then Anaconda cannot start installation and
  both generated disks remain unchanged.
- **AC-09:** Given installed signed A and writable home/data, when the public
  update workflow performs A to B to retained A, then each boot matches its
  signed digest and generated artifact receipt, all existing home/data bytes
  survive including writes made on B, and rollback hold/high-water/resume work.
- **AC-10:** Given an untrusted candidate, occupied pending slot or lower rank,
  when the public update client attempts staging, then it refuses while preserving
  current/staged/rollback state, high-water and writable home/data.

### Package extension point

- **AC-11:** Given an independently hashed typed SystemDefinition containing
  verified package output aliases and generated configuration over the exact
  foundation, when the candidate composer consumes the contribution, then the
  resulting image contains the verified closures and the receipt reports aliases
  and object identities. Deterministic source/recipe pins feed preflight separately.
- **AC-12:** Given a contribution that substitutes a pin, targets another
  foundation, collides with reserved Kedra ownership or references an absent
  output, when it is consumed, then composition refuses without accepting its
  caller-supplied hashes or provenance strings as installed authority.

## Nonfunctional contracts

- **NFR-01:** Zero production keys, protected signing environments, real GHCR
  writes, workstation installs/reboots or live-home changes in branch qualification.
- **NFR-02:** One frozen source/controller artifact per qualification invocation;
  record executable hashes before and after and all image identity domains.
- **NFR-03:** Zero unit/model/mock/doctest or repository scanner additions. Use
  public CLI, sanctioned container/native VM workflows, standard compiler/linter
  checks and actual manual installation only.
- **NFR-04:** Preserve identity schema 2, resolved-inputs schema 1 and the exact
  `.github/workflows/release.yml` identity. Fail closed; no fallback unsigned path.
- **NFR-05:** Every new external process has a finite timeout; each failed stage
  is terminal for that invocation, with zero automatic retries. Cleanup is limited
  to resources recorded as owned by that invocation.

## Explicit limits

D1's exact derived boot bridge must be qualified before integration. Full desktop
engine support, RPM replacement, automatic home activation, key rotation,
production promotion/merge and hardware qualification are excluded. Protected
main-only signing, real publication and production no-change repetition remain
not-run until separately authorized on actual main. This branch can qualify the
same public builder and fixture protocol; it cannot claim those production gates.
