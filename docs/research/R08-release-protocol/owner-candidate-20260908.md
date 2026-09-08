# First owner-trust desktop candidate

Status: public build and owner-approved image signing pass; ISO construction
fails on signed compression identity. This candidate is not promotable through
the current offline installer path. No installer/release metadata is promoted.
Date: 2026-09-08.

| Identity / check | Observed result |
|---|---|
| Accepted source | `3b1bcdf2e0a92e9ea2ffce872fa1d134f8cdda26`, normal PR 1 merge |
| Build | [34250485539, attempt 1](https://github.com/Reidond/kedra/actions/runs/34250485539), build job passes |
| Unsigned image digest | `sha256:8ccbdc9097ee17fdad54c95252da660c4a58f6774805b0c59f7890f19366cd72` |
| Installed source.json SHA-256 | `52a3d6f2711075bc52929cbe45f1c6b52d9c22775f1b059449bf034dea900231` |
| Public-key fingerprint | `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`, independently rechecked with native sysroot |
| Target scope | desktop / Fedora 44 / x86_64 / ghcr.io/reidond/kedra-desktop |
| Main workspace | pass — [34250468736](https://github.com/Reidond/kedra/actions/runs/34250468736) |
| Candidate source tooling | pass — pinned formatting, Clippy, actual CLI E2E, release build and OpenSSL interoperability |
| Protection preflight | pass — existing Reidond reviewer, main-only branch rule, no administrator bypass |
| bootc container lint | 11 checks pass, 1 skipped, 2 warnings described below |
| Exact image signature | pass — protected Skopeo signing job after explicit owner approval |
| Anonymous registry access | pass — unauthenticated manifest and native strict Skopeo pull preserve the exact reviewed digest/signature |
| Embedded source/trust checks | pass — installed source, key, policy, discovery and install-policy bytes match |
| Exact production ISO | fail — osbuild storage copy cannot change signed compressed layer representation |
| Fresh production installation | not-run |
| Metadata promotion / channel publication | not-run |

Public build artifact: `desktop-candidate-34250485539-1`; retained locally at
`output/owner-candidate-34250485539`. It contains source, package inventory,
image/tool input identities, build logs and public key only. The private key is
not available to the build job. The owner confirmed its separate Bitwarden backup
and retrieval before this run; no vault or SSH key was accessed by the agent.

Installed package inventory includes bootc 1.16.10-1.fc44, Skopeo 1.22.2-2.fc44,
kernel-core 7.1.13-200.fc44, niri 26.04-1.fc44, Noctalia 5.0.1-1.fc44,
greetd 0.10.3-6.fc44, foot 1.27.0-1.fc44, Git 2.55.0-1.fc44 and
gnome-keyring 50.0-1.fc44. Private Codex 0.153.4 and Bitwarden 2026.8.0 retain
their separately pinned archive/source identities in the public evidence.
The runner used Podman 4.9.3 and Skopeo 1.13.3; those are distinct from installed
Fedora tool versions.

Lint reports runtime-only DNF/SELinux build residue under /run and content under
/var without corresponding tmpfiles entries, including package caches and the
greetd portal mask. These warnings are retained rather than relabeled as passed
runtime checks. They did not prevent container lint success; actual production
installation and boot must independently verify mounted layout and session health.

The latest code regression at f8a2cb8 passes signed helper
[34249793918](https://github.com/Reidond/kedra/actions/runs/34249793918) and graphical
desktop [34249794121](https://github.com/Reidond/kedra/actions/runs/34249794121).
Their public evidence is downloaded; these runs use disposable research authority
and do not qualify the exact production candidate's image signature or ISO.

The owner explicitly approved this candidate's image signing; the agent submitted
that approval to the configured environment for the unchanged accepted main and
reviewed digest. The signing job completed successfully. An independent anonymous
GHCR request returned bytes whose SHA-256 matches the exact digest. Package API
metadata requires read:packages scope, which the current CLI token lacks; no token
scope was broadened and no visibility change was needed for that anonymous access.

The installer job completed anonymous strict pull, source/trust checks and Anaconda
image construction. The pinned image-builder v82.0.0 then failed in
org.osbuild.skopeo when copying the signed payload by image ID into media storage
under the digest-qualified name. containers/image reported that doing so would
change layer representation, forbidden by the destination digest.

The native pre-publication image was
`sha256:5a741f51a1d5c519cbfc1c0d3e660af42e17fe156f91a0eb202ace19f14b927f`.
The original unsigned registry push compressed layers and produced the reviewed
8ccbdc digest. Native storage exposes decompressed layer blobs for later copies;
the builder cannot replace those signed manifest descriptors. Earlier R02
research signed a native storage copy without this registry compression step,
so its successful installation did not establish this round-trip behavior.

The proposed correction uses `--preserve-digests` on the first public push and
requires a registry/storage/isolated-storage round trip before requesting image
signing. A disposable-key R01 case now exercises that exact native path. This
keeps one signed byte identity; it does not remove signatures, replace the
expected digest, or weaken policy. Qualification of the correction remains pending.
The failed candidate and public evidence are retained; a changed candidate needs
its own concrete signing review and exact-media installation.

Failure evidence: `output/owner-installer-failure-34250485539-1/release-evidence`,
especially verified-pull.log, payload.digest, installed-* and iso-build.log.
Independent OpenSSL verification of the public OCI attachment also passed at
`output/owner-candidate-34250485539/registry-signature`.
Sources: [containers/image storage layer representation](https://github.com/containers/image/blob/main/storage/storage_src.go),
[copy immutability checks](https://github.com/containers/image/blob/main/copy/single.go),
[compression preservation](https://github.com/containers/image/blob/main/copy/compression.go),
and [Skopeo 1.13.3 copy options](https://github.com/containers/skopeo/blob/v1.13.3/docs/skopeo-copy.1.md),
inspected 2026-09-08. The exact failing runtime is the pinned v82 builder; source
inspection is an explanation, not a successful experiment.
