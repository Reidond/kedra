# Next owner candidate: exact image-signing inputs

Recorded 2026-09-09 (Europe/Kiev), Codex. This record reviews image signing inputs
for the next candidate. It does not qualify a new ISO or promote release 2.

## Accepted source and public build

[PR 3](https://github.com/Reidond/kedra/pull/3) merged after required checks
[34292927880](https://github.com/Reidond/kedra/actions/runs/34292927880) and
[34292924150](https://github.com/Reidond/kedra/actions/runs/34292924150) passed at
`a8da90465f5119f2f0df532d50c98d280e05d4c2`. Accepted main is
`0eb1cf09c0eab5f4488a780552f51582d3e92bdf`; its workspace
[34293092301](https://github.com/Reidond/kedra/actions/runs/34293092301) also passes.
Main is held fixed for candidate
[34293133114, attempt 1](https://github.com/Reidond/kedra/actions/runs/34293133114).

The production build passed source checks, public-input preparation, image
construction and complete registry/storage/isolated-storage identity checks.
Build job `102283842482` completed successfully at 2026-09-09 00:08:27 UTC.

| Reviewed input | Exact value |
|---|---|
| Target | desktop / Fedora 44 / x86_64 |
| Accepted source | `0eb1cf09c0eab5f4488a780552f51582d3e92bdf` |
| Image digest | `sha256:71b928fd53a593ece7d08ad84cf68b6dfd366a6606fd1bd0f38b09a7a95c381a` |
| Installed source manifest SHA-256 | `c55f7142c78d6240ed08f2a3b852d720142e844df6186879054e627b699e113f` |
| Independent public-key SPKI DER SHA-256 | `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e` |
| Actual public PEM file SHA-256 | `0eed364e4f141c11f28b1ee22f8b307535eaa16705972b860d3a795ec6ae8ffb` |
| Build evidence artifact | `10082301634`, `desktop-candidate-34293133114-1`, 64,086 bytes |
| Downloaded artifact ZIP SHA-256 | `335cc48f67167e0cd77ffca38de469cf37251e62bf6b13f4c2a72e1b72994859` |

The complete artifact archive was independently hash-verified before extraction.
The source manifest names the accepted commit and expected target/repository.
An anonymous request to the builds repository returned the 12,684-byte manifest
whose SHA-256 equals the reviewed image digest. The native production guard
retained that same digest through registry and isolated container storage.
Observed host tools are Podman 4.9.3 and Skopeo 1.13.3. No local OS build was used.

## Differences from published release 1

The RPM inventory, external agent/Bitwarden evidence, image/installer pins,
public key, helper binary and declared configuration files are unchanged from
candidate 34255228394. The new `sysroot` executable implements the already
native-qualified caller-home status assessment. Its SHA-256 is
`5684dc6721da7db196b0235a908348fabc6366ea535bff33924a367d91fd989f`.
The unchanged helper SHA-256 is
`a70447a4c25ceef756668621c739e571f1963d1a093addcbe27c94fe5d7eaa5f`.

The source manifest's commit identity changes, so the generated payload archive
also changes. Other source-manifest fields compare equal. This observation is
not whole-image equivalence or a no-change release decision. The actual resolved
packages.txt SHA-256 is
`bf504505b65c6aa771f7ef6faf3c1c12fcfd3a13ad0c9912ac88ece9279db13b`.

The same two known bootc container-lint warnings remain: runtime-directory
contents and `/var` entries without tmpfiles declarations. Lint reports eleven
checks passed, one skipped and two warnings. Earlier release 1 passed the exact
installation/health cases with these warnings; this new media still needs its
own qualification.

The accepted source also prepares candidate schema 2, resolved packages and
explicit provenance, signed SHA256SUMS, and numeric-ID publisher recovery.
Those release-pipeline changes still require this candidate's full native
execution. Later development history-verification and full-target research
changes are outside the accepted source and this signing request.

## Authority and remaining boundary

Readback confirms protected main, the existing sole-owner/main-only signing
environment `21492153153`, no admin bypass, and `KEDRA_RELEASES_ENABLED=true`.
The environment public PEM equals the committed public key and the native
fingerprint matches the independently established authority. The owner's
standing authorization covers continuing the already requested project work;
the configured protected review remains in place for these concrete inputs.

At this review point, image signing is waiting at that existing environment.
No new ISO, first boot, metadata signature or release 2 is claimed. The next
steps are isolated image signing, anonymous signature verification, exact v2
ISO qualification, protected metadata publication and an update of the retained
enrolled release 1 VM. Published release 1 and its channel remain unchanged.

Local continuation evidence: `output/owner-candidate-34293133114/`, including
the verified archive, extracted native build records and anonymous manifest.

## Signing outcome

After the concrete review above and another exact-main/pending-environment
readback, the configured review was submitted under the owner's standing
authorization. GitHub accepted deployment `6339675374` at source `0eb1cf0`.
The isolated sign-image job passed. The installer job is now building the ISO.

Independent anonymous verification also passes: the owner repository serves the
exact reviewed `71b928fd` manifest, and native OpenSSL 3.6.1 verifies the attached
P-256/SHA-256 signature with the established public key. The verified payload
binds the complete digest above to
`ghcr.io/reidond/kedra-desktop:candidate-34293133114-1` and the expected signature
type. Local `registry-signature/` records retain the public manifest, signed
payload, detached signature and verification result. This does not establish
new ISO or installed-system qualification; those results remain pending.
