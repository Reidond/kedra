# Build and publish a release

All OS/ISO production runs in Actions. [Published r2](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r2) remains available; new source does not replace it until qualification and promotion complete. See [install](../../docs/INSTALL.md), [verification](../../docs/RELEASES.md) and [observed status](../../docs/STATUS.md).

## Refresh and candidate

The release workflow schedules package refresh at **00:00 UTC** on main. It also supports manual dispatch:

```sh
gh workflow run release.yml --ref main
gh run list --workflow release.yml --limit 5
```

Schedules can queue, skip or stop after repository inactivity. The trigger is not a promise that artifacts exist at exactly midnight.

Before building an OCI image, the workflow resolves the official Fedora 44 base to an immutable digest and runs a disposable package preflight against that base. It records the complete installed RPM header/payload identities, image-affecting source inputs, compiled/external artifacts, external pins and build-recipe identity. The prior release's signed SHA256SUMS authenticates its provenance and package records for comparison. Package versions alone are insufficient; failure is never no-change.

Missing legacy comparison material conservatively requires a candidate. Published r1 and r2 lack this new resolved-input schema, so the first midnight refresh builds a candidate; its later qualified promotion establishes the baseline for future no-change checks.

A changed candidate is built with public trust and must match the preflight's resolved package identities before it proceeds. It is pushed to the separate build registry, signed by an isolated protected job and anonymously verified before the ISO is produced. Keep final native OCI digest identity across registry/storage/media transfers. The candidate artifact contains numbered ISO parts and evidence, including exact source, package inventory, provenance and the whole ISO's size/hash; it does not include a second complete ISO file. Reconstruct and verify the complete ISO for qualification. A candidate is not an approved installed update.

A proven unchanged result skips OCI and ISO production, reuses the existing release and prepares checkpoint renewal. Predecessor signatures/material are rechecked independently; protected signing remains required before key-free publication. A failed comparison or blocked candidate cannot renew successful freshness.

## Review and promote

The existing `kedra-desktop-signing` environment requires owner review, main-only deployment and no administrator bypass. `KEDRA_RELEASES_ENABLED=true` is opt-in, not a substitute for environment protection. Keep the dedicated key separate from SSH and outside checkouts; [authority configuration](authority/README.md) gives the exact names.

Before approving image signing, inspect the exact source/run/attempt, digest, public authority, inventory and source-check result. Then qualify that exact ISO in a disposable two-disk VM. No unattended disk selection or default password is allowed.

Create a qualification JSON using schema_version 1, the actual candidate_sha256, method `manual-vm` or `end-to-end-vm`, public evidence references and a checks object. Each of these must record a genuinely observed `pass`:

- encrypted_install
- unselected_disk_preserved
- iso_free_boot
- owner_desktop_login
- selinux_enforcing
- exact_booted_image
- container_policy
- home_writable
- root_read_only
- doctor

Do not copy a passing result from another ISO or invent evidence. The producer binds the qualification and current-main successful candidate run/attempt; RPM resolution must be recent enough for publication. It refuses unsupported schema-1 candidates rather than backfilling missing v2 inputs.

Open **Actions → Promote qualified desktop installer → Run workflow**, supply the successful candidate run/attempt, independently checked candidate.json SHA-256 and exact qualification JSON. Leave bootstrap false for an existing channel. The promotion workflow serializes against production release work and repeats source/predecessor checks.

Public preparation proposes exact release/checkpoint/checksum bytes. The protected no-checkout signer independently checks the approved request and signs those bytes without executing repository scripts or candidate code. The key-free publisher verifies all signatures/assets, downloads every draft asset for byte comparison, publishes the immutable version, then replaces and verifies the single channel bundle.

## Assets and recovery

R2 was published at 2026-09-12 12:19:49 UTC by key-free recovery of the original approved bytes from failed promotion 34325343190. All 17 assets, downloaded ISO parts, signed checksums and current channel were verified; no signatures were regenerated. That Actions run remains failed. Version ID 385328126 names source 0eb1cf09c0eab5f4488a780552f51582d3e92bdf; the current channel was replaced and verified, with the release opt-in restored true. See [status](../../docs/STATUS.md).

A schema-2 version includes release/checkpoint signatures, public authority, candidate/source/qualification records, packages.txt, provenance.json, ISO parts and signed SHA256SUMS. packages.txt is the actual installed RPM inventory; provenance is Kedra's explicit format, not SPDX/SLSA certification or a complete reproducibility claim. R1 retains its original 13 v1 assets.

Publication uses numeric release/asset IDs and authenticated draft discovery. A published-only tag lookup cannot prove a draft is absent. Versioned releases/assets are never overwritten. A transient missing channel fails closed; replacement is not atomic.

On uncertain/partial publication, preserve the publication artifact, exact signed bytes and remote IDs. Inspect current source, draft inventory and channel before explicit recovery. Do not blind-rerun, delete a draft or re-sign to hide failure. Retain promoted OCI digests, attachments, signed metadata and installer recovery media.

Expired predecessor history may supply authenticated ordering via `sysroot release history`; newly signed incoming checkpoints must still pass normal freshness and replay validation. Key rotation and full interruption/race coverage remain separately qualified. [STATUS](../../docs/STATUS.md) distinguishes implementation from actual production execution.
