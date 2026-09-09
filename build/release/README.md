# Production releases

[Owner release r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1)
and its [signed channel](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-channel)
are published. Exact installation, verified publication recovery and public-channel
enrollment pass under the dedicated owner authority. See the
[recovery report](../../docs/research/R08-release-protocol/owner-promotion-34288691672.md)
and [installation guide](../../docs/INSTALL.md).

The expanded v2 publisher and expired-predecessor publication remain under
qualification; r1 retains its original v1 assets and signed bytes. The owner has
confirmed Bitwarden backup/retrieval and the protected signing environment is
provisioned. Public producers create no private keys and enroll no machines.

For the independently reviewed desktop public key:

```sh
sysroot release key --public-key release.pub
sysroot source plan --host desktop --json > source-plan.json
python3 build/release/prepare-trust.py --source source-plan.json --public-key release.pub \
  --expected-fingerprint REVIEWED_SHA256 --sysroot target/release/sysroot --output output/release-context/trust
```

The producer requires the actual committed desktop plan, Fedora 44/x86_64 and
`ghcr.io/reidond/kedra-desktop`. It validates the P-256 key with the actual sysroot
CLI and refuses a fingerprint mismatch before creating output. It copies only
public trust, strict container policy, attachment discovery and the install-time
signature requirement. The trust derivative preserves the original installed
source manifest instead of rewriting its target identity as a research fixture.

The SHA-256 identifies the key; obtaining that fingerprint from the same
untrusted download does not establish authority. A dedicated OS-release private
key and its recovery copy must be separate from the owner's Bitwarden SSH key.
Research keys and generated fixture fingerprints are never production authority.

The [release workflow](../../.github/workflows/release.yml) builds only
from current main after explicit opt-in and an existing protected signing
environment. It builds public trust into the final candidate, pushes the unsigned
build to a separate repository, and signs its exact digest in a job without a
checkout or candidate execution. A separate job anonymously verifies the signed
GHCR image and its installed source/public key before building an offline-checking
installer. Candidate schema version 2 records the exact ISO hash/size, ordered
download parts and each part's hash, plus packages.txt and provenance.json hashes.
That descriptor deliberately records fresh_install_qualified=false and cannot be
used as a promoted release manifest.

See [authority setup](authority/README.md) for the concrete files, environment,
secret names and protected environment configuration. The first production
candidate 34250485539 at accepted source 3b1bcdf passed owner-reviewed signing and
anonymous strict pull but failed ISO construction on compressed layer identity;
it is retained and not promotable. The correction preserves native OCI identity
on the first push and requires a registry/storage round trip before signing.
Replacement 34255228394 at c660c58 then passed separate image/metadata approval,
exact-media installation and publication as r1. Promotion 34288691672's failed
publisher remains failed; its exact approved bytes were recovered and verified
manually. The current opt-in and later accepted-source candidate are tracked in
worklog.md. Shared installer regression 34244387167 retains its earlier signed
build/offline-startup scope with disposable authority.

The complete release path must preserve these boundaries:

1. Build reviewed committed payloads in Actions with public trust only. Capture
   exact image/source/package/tool digests and validate the desktop.
2. Sign the final candidate digest in an isolated trusted job. Do not execute
   newly built binaries or writable repository scripts while production keys
   are available. Keep GitHub API/registry authentication separate from key files.
3. Verify and boot that signed candidate, then build the installer with the
   qualified offline payload check and strict initial-install policy.
4. Qualify the exact installer, sign immutable release metadata and checksums,
   and promote only under per-target ordering/freshness rules. Candidate signing
   alone must not create a promoted channel or claim a successful installation.
5. Preserve independent recovery material and explicit stage/reboot/rollback
   controls. A failed or stale refresh must not renew successful freshness.

The earlier public-input preparation experiment used a disposable public key whose private key was
removed by the CLI/OpenSSL workflow. The local producer accepts the matching
fingerprint and refuses a different fingerprint without creating output. This
qualifies that earlier public-input preparation only. Actual owner authority,
installation and recovered r1 publication have their separate evidence above.

`prepare-release.py` prepares exact **unsigned** release/checkpoint/checksum bytes after
review of candidate.json, the installed source.json, packages.txt, provenance.json,
the complete ISO, every download part and a bound
qualification report. It verifies the reviewed candidate hash, source/ISO hashes,
scope/key, all required manual/E2E installation cases and an actual CLI-authenticated
previous release/checkpoint history. It advances sequence/generation and rejects older/repeated build
run/attempts. Initial preparation requires explicit `--bootstrap`. The output
signing-request.json binds the exact asset names, sizes and hashes, SHA256SUMS
bytes and expected previous release/checkpoint hashes; the
protected publisher compares those to current channel state under
the target lock before signing/publication. This local producer cannot inspect
GitHub ordering or establish that someone actually performed a reported test.

Qualification JSON has schema_version=1, candidate_sha256, method (`manual-vm` or
`end-to-end-vm`), evidence (public report/run references), and checks. Every named
check must be `pass`: encrypted_install, unselected_disk_preserved, iso_free_boot,
owner_desktop_login, selinux_enforcing, exact_booted_image, container_policy,
home_writable, root_read_only and doctor. Review that evidence before submitting
it. Do not relabel research authority/identity as an owner candidate.

The producer requires candidate RPM resolution within 36 hours. It authenticates
the predecessor using `sysroot release history`, so an expired predecessor may
supply its ordering floor while invalid signatures, scope, binding, lifetime,
future timestamps or replay still refuse. The result is explicitly historical
and cannot confer fresh eligibility. The publisher repeats that authentication,
then verifies the newly signed pair with ordinary fresh `release channel` and
the predecessor's ordering floor. No clock is changed and no installed helper
freshness check is relaxed. No-change renewal and rotation remain unimplemented;
complete production expired-predecessor qualification is still not-run.
The producer makes no signing/publishing
request itself and refuses existing output. The `approval=promoted` value in the
unsigned bytes is a proposed signing payload, not an actual promoted release.

The producer takes `--packages`, `--provenance` and `--installer-parts-dir` in
addition to the source, complete installer and qualification inputs. Candidate
schema version 1 lacks the reviewed inventory/provenance/part hashes and is not
accepted for this expanded publication path. Historical candidate evidence is
preserved as produced; do not synthesize these fields for an earlier candidate.
Release and checkpoint protocol versions remain 1.

## Protected publication

The manual [promotion workflow](../../.github/workflows/promote.yml) takes a
successful current-main release.yml run/attempt, an independently reviewed
candidate.json SHA-256, the exact qualification JSON, and a bootstrap flag used
only when the channel is absent. It shares the candidate workflow's concurrency
group. Public preparation reassembles and hashes the entire ISO, verifies source
and authority, and authenticates previous signed ordering history with sysroot.
The proposed exact payloads and qualification are retained for owner review.

The protected signing job has no checkout, repository scripts, candidate execution
or production-write token. It independently checks the approved source, successful
build, candidate/qualification hashes, public authority and unchanged channel.
Its predecessor check binds the exact bundle/payload hashes; it does not execute
the repository history command or claim an independent predecessor freshness
check. Incoming checkpoint freshness checks remain strict. Historical verification
is performed independently by the public producer and the key-free publisher.
It derives the fixed checksum filenames and expected hashes from the exact
reviewed candidate, metadata, qualification and public authority, then compares
the entire checksum payload before using a key. Pinned Cosign 3.1.3 signs exact
release/checkpoint/SHA256SUMS bytes using the dedicated key; OpenSSL independently
verifies all three signatures. The key and passphrase are exposed
only in this step and its private temporary directory is removed on exit.

A separate publisher receives signatures but no private key. It repeats native
signature/freshness and complete ISO verification, independently verifies the
checksum signature and every named asset, and creates the versioned release
`desktop-44-x86_64-rSEQUENCE` as a draft, uploads and checks the complete asset
inventory, downloads every draft asset and compares its exact bytes before
publication. Versioned releases are never overwritten. Finally
it replaces only `channel.json` on `desktop-44-x86_64-channel` and verifies exact
readback. The bundle contains schema_version=1 plus release and checkpoint, each
with the original payload string and detached signature string. Clients must
verify both documents and their own retained replay history. A bundle is discovery
data, not an extra signature format.

### Inventory, provenance and checksums

The versioned release includes the following additional assets:

- `packages.txt`: the exact build-time `rpm -qa | sort` inventory read from
  `/usr/share/sysroot/packages.txt` in the signature-verified payload. It lists
  installed RPM versions; separately bundled, non-RPM software is outside this
  inventory's scope.
- `provenance.json`: Kedra's small, explicit `kedra-candidate-provenance` JSON
  format, schema version 1. It records candidate scope/source/run/attempt/image,
  installed source and package hashes, RPM-resolution time, whole ISO identity,
  and the digest-pinned base/builder arguments and observed builder version used
  by the installer job. It is drawn from that job's actual outputs. It is not
  SPDX, CycloneDX, SLSA or an independently attested external build statement, and
  does not claim a complete dependency graph or reproducible build.
- `SHA256SUMS` and `SHA256SUMS.sig`: lexically ordered ASCII SHA-256 lines with
  two spaces before each fixed filename and LF endings. They cover the whole
  ISO, every numbered download part, release/checkpoint JSON, candidate/source/
  qualification JSON, inventory/provenance and public authority files. The
  complete ISO is reconstructed from the parts; it is not uploaded above the
  per-asset size limit. The detached signature is base64-encoded DER
  ECDSA/P-256 over SHA-256, the same encoding as the release/checkpoint signatures.

Checksums omit themselves and signature files to avoid self-reference. The
signing request is a review record, not independent authority; the channel
contains the separately signed release/checkpoint pair. No additional signature
or provenance field changes installed client release authorization.

After obtaining the public key's fingerprint independently and downloading the
versioned assets, the checksum signature can be checked with standard tools:

```sh
sysroot release key --public-key release.pub
# Compare the reported fingerprint to the independently trusted value.
openssl base64 -d -in SHA256SUMS.sig -out SHA256SUMS.sig.der
openssl dgst -sha256 -verify release.pub -signature SHA256SUMS.sig.der SHA256SUMS
# Assemble the whole ISO using the signed release metadata as documented in docs/RELEASES.md.
sha256sum --check SHA256SUMS
```

The final checksum command expects both the downloaded files and the assembled
ISO in the current directory. It does not replace signature, scope, freshness or
replay verification by `sysroot release assemble` and the installed helper.

A transient missing channel asset fails closed. A failure after creating a
versioned draft/release is deliberately not repaired by a blind rerun: inspect
the retained `desktop-publication-RUN-ATTEMPT` artifact, uploaded asset inventory,
source and current channel before any explicit recovery. No immutable version
may be replaced to make a rerun pass. The mutable channel release is incompatible
with enabling repository-wide immutable releases for every future release; keep
that channel mutable. Versioned assets are append-only by this workflow.

Draft discovery uses authenticated, paginated release listings, refusing duplicate
matches or a lookup that exceeds 60 seconds or 100 pages of 100 releases. GitHub's tag lookup
returns published releases, so a 404 from that endpoint cannot establish that a
draft is absent. Draft metadata and asset inventory are checked by numeric release
ID; publication also updates that exact ID. Uploads use that release ID and
downloads use the returned, validated asset IDs, without resolving a tag again
for transfers. Download streams enforce the checked byte size and a 15-minute
deadline, killing/reaping the CLI on failure. The read-only preparation token's view
is limited by its access. It does not establish absence of hidden drafts: the
publisher repeats discovery with its existing `contents:write` token before any
creation, and refuses visible conflicting drafts without changing permissions.

Each new draft body includes the promotion run, attempt and exact signing-request
hash. A create request is submitted once, with a 60-second CLI timeout. If it
returns an error or times out, one bounded
authenticated lookup may continue only with a unique, empty draft matching that
operation's exact tag, source, title, complete marked body and unpublished state.
An absent, ambiguous, changed or populated result stops for explicit recovery.
Pre-existing drafts always refuse a new attempt, including a rerun with the same
requested release. There is no create retry, version overwrite or automatic draft
deletion. This handles only an uncertain acknowledgement within the current
invocation, not general interrupted-publication recovery.

Observed 2026-09-09: the earlier accepted v1 publisher's create returned HTTP 500
in run 34288691672 while GitHub retained empty draft 385117864 for
`desktop-44-x86_64-r1`. Authenticated list and ID reads returned that draft while
the tag endpoint returned 404. That observation motivates this development
repair; it does not qualify the repaired v2 publisher or authorize automatic
adoption of the earlier draft. Sources:
[GitHub release listing/access](https://docs.github.com/en/rest/releases/releases#list-releases),
[release lookup by ID](https://docs.github.com/en/rest/releases/releases#get-a-release)
and [release updates](https://docs.github.com/en/rest/releases/releases#update-a-release).

Immediately before each mutation, the publisher rechecks the accepted source and
the applicable draft/channel metadata and asset inventory. Version assets are
never deleted or replaced. The sole replacement path is the current channel:
it deletes only the exact old `channel.json` asset ID whose bytes were already
downloaded and verified, checks that exact removal and unchanged surrounding
metadata, then uploads once to the same release ID. Any unexpected asset, changed
release or uncertain mutation stops for explicit recovery. Final readback binds
the new channel's release ID, asset ID and full byte hash. This preserves the
existing fail-closed availability gap during replacement; it is not an atomic
server-side transaction with independent writers. See
[GitHub release asset APIs](https://docs.github.com/en/rest/releases/assets).

Before publishing a version or the initial channel draft, the real Git tag must
be absent or resolve to the accepted source; annotated tags are peeled with a
bounded lookup. After publication it must resolve to that exact source.
`target_commitish` alone is insufficient because GitHub ignores it when the tag
already exists. A mismatch is never repaired by moving the tag. Later updates to
the existing mutable channel retain its historical Git tag; only its verified
discovery asset advances. See [Git references](https://docs.github.com/en/rest/git/refs#get-a-reference)
and [annotated tags](https://docs.github.com/en/rest/git/tags#get-a-tag).

Expanded v2 preparation/signing/publication under actual production authority,
uncertain-create continuation, expired-predecessor publication, interrupted-publication recovery and races remain
not-run. Syntax parsing and native signer
help inspection do not qualify those behaviors. Native Windows and Linux CLI/OpenSSL checks
exercise history authentication and unchanged incoming expiry/refusal semantics;
Linux workspace 34294737483 and actual R01 34294737470 pass at 67b4b14. The guest
accepted history only as ordering, refused expired request 6 above enrolled floor
2, then passed fresh B staging/boot and retained-A rollback with high-water/hold
preservation. R07 34294737457 also passes its existing desktop regressions. See
[native evidence](../../docs/research/R08-release-protocol/REPORT.md#native-history-qualification--2026-09-09).
These do not qualify production expired-predecessor publication. No-change renewal and key
rotation remain separate unfinished work.

Sources: [Sigstore blob signing](https://docs.sigstore.dev/cosign/signing/signing_with_blobs/)
and [GitHub draft-first release publication](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository?tool=cli),
checked 2026-09-08; exact native Cosign 3.1.3 help and authority setup are recorded
in the R08/worklog evidence.

See [release verification](../../docs/RELEASES.md), [ADR 0018](../../docs/adr/0018-installer-payload-verification.md)
and the [R01](../../docs/research/R01-signatures/REPORT.md)/[R08](../../docs/research/R08-release-protocol/REPORT.md)
records for the actual signing/installer evidence and remaining authority gates.
