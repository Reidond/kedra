# Production release preparation

The public trust-context producer is implemented. Production signing authority,
the release workflow and promotion are not configured yet. Nothing in this folder
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

The remaining release path must preserve these boundaries:

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
qualifies public-input preparation only; it does not establish production key
storage, CI secret provisioning, a signer or promoted media.

See [release verification](../../docs/RELEASES.md), [ADR 0018](../../docs/adr/0018-installer-payload-verification.md)
and the [R01](../../docs/research/R01-signatures/REPORT.md)/[R08](../../docs/research/R08-release-protocol/REPORT.md)
records for the actual signing/installer evidence and remaining authority gates.
