# Replacement owner-trust desktop candidate

Date: 2026-09-08. Status: public build and registry/storage compatibility pass;
the protected image-signing job is awaiting exact owner review. No replacement
image signature, ISO, fresh installation or metadata promotion is claimed.

| Identity / check | Observed result |
|---|---|
| Accepted source | `c660c58d9bbbbe34119f6ea35a03528485455848`, normal PR 2 merge |
| Build | [34255228394, attempt 1](https://github.com/Reidond/kedra/actions/runs/34255228394), build job passes |
| Exact native and published digest | `sha256:bb4f2b68996a4fc260972d9654440f62ee078bcf92a0996a8f7e34ea0133118b` |
| Installed source.json SHA-256 | `4451de9c4363a4d3a7a3f789816b6ec9d169b14283aeb3a5a322c045fe745208` |
| Public-key fingerprint | `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`, independently checked with native sysroot |
| Target | desktop / Fedora 44 / x86_64 / ghcr.io/reidond/kedra-desktop |
| Accepted main CI | pass — [34255226140](https://github.com/Reidond/kedra/actions/runs/34255226140) |
| Candidate tooling | pass — pinned format, Clippy, CLI E2E, release build and OpenSSL interoperability |
| Environment protection | pass — sole Reidond reviewer, main-only, administrator bypass disabled |
| Native registry round trip | pass — first push preserves digest; pull and isolated-storage copy retain the same digest |
| bootc lint | 11 pass, 1 skipped, 2 retained warnings |
| Owner image signing | pending owner review |
| Signed anonymous pull / ISO / installation / promotion | not-run |

The first candidate failed after successful signing because ordinary registry
publication compressed native OCI layers. This replacement preserves the native
manifest on the first push. Its build checks a GHCR pull and storage-by-ID copy
into a separate installer store before exposing a signing job. Both candidate.digest
and storage-roundtrip.digest contain the exact digest above; native image-inspect
agrees. No signature check, policy rule or digest binding was weakened.

The corresponding disposable signed copy experiment passes R01 run
[34253906774](https://github.com/Reidond/kedra/actions/runs/34253906774) at 7e846f0,
including strict signature pull/storage copy, negative cases and three real
update/rollback VM boots. That evidence qualifies the copy correction; this
production candidate still requires its own signature and complete v82 ISO test.

Public artifact `desktop-candidate-34255228394-1` is downloaded to
`output/owner-candidate-34255228394`. Source revision/hash, public-key fingerprint,
native digest and round-trip logs were independently inspected. The RPM inventory
is identical to the first candidate's inventory, including bootc 1.16.10-1.fc44,
Skopeo 1.22.2-2.fc44, kernel-core 7.1.13-200.fc44, niri 26.04-1.fc44 and
Noctalia 5.0.1-1.fc44. The runner uses Podman 4.9.3 and Skopeo 1.13.3.
Private Codex and Bitwarden retain their existing pinned inputs.

The same two packaging warnings remain: runtime-only DNF/SELinux build residue
and /var entries without matching tmpfiles rules. They are not a failed lint
result or a substitute for installed health checks. Fresh owner-media installation
must verify the exact booted digest, SELinux, mounts, services and doctor.

The owner's earlier signing approval applies only to failed candidate 34250485539.
This run is waiting in the existing kedra-desktop-signing environment, ID
21492153153. Main still matches the accepted source above. The requested action
is isolated signing of this exact candidate; it does not promote metadata or
install anything on the workstation. Dedicated key recovery remains confirmed by
the owner; no vault, SSH key or personal profile was accessed.
