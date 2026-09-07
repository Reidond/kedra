# R01: enforced signature path through bootc

Status: **not-run**; first experiment prepared 2026-09-08 (Europe/Kiev).

`Research signed updates` creates short-lived Sigstore keys and a TLS-protected
CNCF Distribution registry in an ephemeral Actions runner. The listener binds
only to loopback; no GHCR packages, production keys or credentials are involved.
TLS and image-signature verification are both retained. Only public keys and the
test CA enter the fixture images. Private files are outside build contexts and
artifact paths and are removed when the runner script exits.

The exact Fedora and builder inputs are inherited from the passing R02 experiment.
Distribution's amd64 manifest is pinned to
`sha256:7518da9b12dd746278282a729dee2e65eabdeb449db4d0b28d46ef6e90308f58`, resolved
from the official `registry:3` registry API on 2026-09-08. Tool versions, public
policy, image digests and serial logs are captured in each Actions artifact.

## Intended cases

All cases are not-run until observed:

1. Verify signed A with containers/image before local builder-store copy.
2. Boot A at the expected final registry digest; reject unsigned, wrong-key,
   wrong-repository, removed-attachment and malformed references through
   `bootc switch --enforce-container-sigpolicy`; verify installed/staged identity
   remains unchanged after each rejection.
3. Stage signed B by digest; confirm B is staged while A remains booted and the
   bootc spec records containerPolicy enforcement.
4. Boot B, verify its digest and retained synthetic data. Omit the enforcement
   flag on an unsigned switch and verify the inherited policy still rejects it.
5. Stage rollback, boot retained A and verify newer data created under B survives.

The VM uses only generated disks, a read-only case-reference disk and synthetic
data under `/var/lib/kedra-research`. Its small test service performs the guest
operations and shuts down. The workflow never executes bootc on the runner host.
This service, its test host mapping and test trust must never enter a promoted OS.

## Boundaries still open

The initial QCOW2 install's policy handoff is distinct from the pre-copy signature
check. Interactive installer transport/enforcement remains R02. Target/architecture
and promoted-release eligibility require the independent R08/R10 protocol; raw
bootc signature validity alone does not enforce Kedra release approval. Tampered
metadata, candidate promotion, rotation, replay, interrupted staging and physical
hardware remain untested. This experiment cannot by itself pass all of R01.

Primary sources reviewed 2026-09-08:
[Sigstore key generation](https://github.com/containers/skopeo/blob/main/docs/skopeo-generate-sigstore-key.1.md),
[containers policy](https://github.com/containers/image/blob/main/docs/containers-policy.json.5.md),
[attachment discovery](https://github.com/containers/image/blob/main/docs/containers-registries.d.5.md),
[Distribution TLS setup](https://distribution.github.io/distribution/about/deploying/),
[bootc 1.16.10 state schema](https://github.com/bootc-dev/bootc/blob/v1.16.10/crates/lib/src/spec.rs),
[switch](https://bootc.dev/bootc/man/bootc-switch.8.html) and
[rollback](https://bootc.dev/bootc/man/bootc-rollback.8.html).
