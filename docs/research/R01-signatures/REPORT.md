# R01: enforced signature path through bootc

Status: **blocked** at full installation-policy/authority gates, 2026-09-08
(Europe/Kiev); the first strict transport/update/rollback subset passed.

Latest regression [34242602458](https://github.com/Reidond/kedra/actions/runs/34242602458)
at `3d29689` passes older-media enrollment, metadata/native signature refusals,
signed B staging/boot, retained A rollback, replay/hold preservation and explicit
resume. Downloaded public serial evidence is `output/r01-run-34242602458`.
Production authority and rotation are still unqualified.

Follow-up [34226367288](https://github.com/Reidond/kedra/actions/runs/34226367288)
at `71916b9` passes older-media enrollment: the running signed A release is
verified separately from a fresh channel naming newer B. Enrollment caches A's
verified identity while retaining B's newer high-water state. The actual VM emits
`KEDRA_R10_OLDER_ISO_ENROLLMENT_PASS`, then passes metadata/OCI refusal, B stage/boot,
retained A rollback, replay/hold retention and explicit resume. This is an
end-to-end installed-helper result using disposable signing authority.

The installed-helper extension passes
[34208979891](https://github.com/Reidond/kedra/actions/runs/34208979891) at `bb53ce7`.
Cosign 3.1.3 signs exact release/checkpoint bytes with the disposable Skopeo key;
the Rust helper independently verifies them, stages B with native enforcement,
recognizes B after boot, queues signed retained A, preserves replay high-water and
the rollback hold, and resumes explicitly. Metadata and native image negative
cases pass. This adds R08/R10 integration evidence without production signing
authority or interactive installer trust handoff. See the R10 report and ADR 0016.

The stronger [run 34167524353](https://github.com/Reidond/kedra/actions/runs/34167524353)
at `14be822` also passed. Initial A, booted B and rolled-back A all reported
containerPolicy; unsigned switches without the explicit enforcement flag were
rejected on all three. The supported install drop-in resolves the missing-setting
boundary described below. Production authority/rotation/installer cases remain open.

[Run 34166793087](https://github.com/Reidond/kedra/actions/runs/34166793087)
at `7705cc36b91546c45f006cf099d7621b3f883227` passed strict local-store import,
all five rejection cases, signed B staging/boot, inherited-policy rejection under
B and rollback retaining newer data. Global policy remained reject. Both registry
and local-store verification required the test key and original repository.

Review of the successful logs found an additional installer boundary: initial A
had no `spec.image.signature`, and returning to A restored that missing setting.
The test did not claim inherited enforcement after rollback. The next version
adds bootc 1.16.10's supported `[install] enforce-container-sigpolicy = true`
drop-in and requires `containerPolicy` on every boot, including unsigned-image
rejection without the explicit flag on initial A and after rollback. This is a
stronger test prompted by observed state, not a relaxation of verification.

Run [34166267962](https://github.com/Reidond/kedra/actions/runs/34166267962)
at `be59b4b` built all variants, signed/pushed them through verified TLS, removed
the selected attachment, and verified A with Skopeo's strict policy before copy.
The QCOW2 builder then failed during bootc installation: it imports from a
containers-storage reference by local image ID, which the fixture's global
reject policy denied. No VM update result was produced.

Next experiment explicitly requires the same Sigstore key and exact original
repository for containers-storage imports. The global default remains reject;
no permissive policy or disabled TLS/signature flag is introduced. This tests
whether signatures survive that builder handoff. The first run's separate jq
syntax failure (34166170646) occurred before registry/image creation and is fixed.

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

The five cases below passed in run 34167524353:

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
