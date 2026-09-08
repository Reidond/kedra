# Production release preparation

The public trust-context producer and a disabled manual candidate workflow are
implemented. The dedicated public authority and protected GitHub environment
secrets are now provisioned; Bitwarden backup retrieval is still unverified.
Production execution and promotion remain unqualified. Nothing in this folder
creates a private key, enrolls a machine or promotes a release.

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

The prepared [release workflow](../../.github/workflows/release.yml) builds only
from current main after explicit opt-in and an existing protected signing
environment. It builds public trust into the final candidate, pushes the unsigned
build to a separate repository, and signs its exact digest in a job without a
checkout or candidate execution. A separate job anonymously verifies the signed
GHCR image and its installed source/public key before building an offline-checking
installer. It records the exact ISO hash/size and download parts in candidate.json.
That descriptor deliberately records fresh_install_qualified=false and cannot be
used as a promoted release manifest.

See [authority setup](authority/README.md) for the concrete files, environment,
secret names, recovery steps and current configuration evidence. The workflow has
not run with production authority. Main's source and release channels have not
changed; protection and environment secrets were configured under the owner's
authorization. The shared installer trust-copy adjustment is
being qualified through the disposable signed-ISO workflow.

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

Current manual evidence uses a disposable public key whose private key was
removed by the CLI/OpenSSL workflow. The local producer accepts the matching
fingerprint and refuses a different fingerprint without creating output. This
qualifies that earlier public-input preparation only. Later authority/keypair
and environment provisioning are documented separately; no promoted media exists.

`prepare-release.py` prepares exact **unsigned** release/checkpoint bytes after
review of candidate.json, the installed source.json, the complete ISO and a bound
qualification report. It verifies the reviewed candidate hash, source/ISO hashes,
scope/key, all required manual/E2E installation cases and an actual CLI-verified
previous channel. It advances sequence/generation and rejects older/repeated build
run/attempts. Initial preparation requires explicit `--bootstrap`. The output
signing-request.json binds the expected previous release/checkpoint hashes; the
eventual protected publisher must compare those to current channel state under
the target lock before signing/publication. This local producer cannot inspect
GitHub ordering or establish that someone actually performed a reported test.

Qualification JSON has schema_version=1, candidate_sha256, method (`manual-vm` or
`end-to-end-vm`), evidence (public report/run references), and checks. Every named
check must be `pass`: encrypted_install, unselected_disk_preserved, iso_free_boot,
owner_desktop_login, selinux_enforcing, exact_booted_image, container_policy,
home_writable, root_read_only and doctor. Review that evidence before submitting
it. Do not relabel research authority/identity as an owner candidate.

The conservative initial producer requires candidate RPM resolution within 36
hours and a still-fresh previous channel. Expired-channel recovery, no-change
renewal and rotation are not implemented by it. It makes no signing/publishing
request itself and refuses existing output. The `approval=promoted` value in the
unsigned bytes is a proposed signing payload, not an actual promoted release.

See [release verification](../../docs/RELEASES.md), [ADR 0018](../../docs/adr/0018-installer-payload-verification.md)
and the [R01](../../docs/research/R01-signatures/REPORT.md)/[R08](../../docs/research/R08-release-protocol/REPORT.md)
records for the actual signing/installer evidence and remaining authority gates.
