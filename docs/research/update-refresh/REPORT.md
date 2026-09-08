# Native RPM refresh and equivalence

Prepared: 2026-09-09. Status: **not-run**. Scope: section A of
[EXPERIMENTS.md](EXPERIMENTS.md), under R02/R07/R08.

The Actions-only implementation in
[build/research/refresh](../../../build/research/refresh/README.md) prepares real
signed RPM repositories and materializes disposable Fedora 44 containers with
native DNF/RPM. No Actions execution, image, native result or no-change conclusion
is claimed by the prepared files. Main `c660c58` and owner media run
`34255228394` are unrelated and unchanged by this experiment.

## Question and implemented approach

Can a fixed source/base select changed requested, transitive and inherited RPM
content, distinguish metadata-only changes from selected package bytes, refuse
broken required inputs, and prove that its DNF step actually ran?

The fixture uses one exact seeded Fedora image and a fixed requested package
list. Native signed RPM snapshots independently change a requested package,
dependency or inherited package. Other snapshots retain selected RPM bytes while
changing repository metadata, republish different payload bytes under the same
NEVRA, omit required metadata, corrupt a previously signed payload, or offer an
unsatisfiable newest direct requested version. Every image build bypasses cache;
each DNF process writes a new receipt outside the image layer.

The evidence format binds the seed image, source intent, full native installed
inventory, selected fixture RPM archive hashes and installed payload hashes/modes.
Native RPM compares archive and installed header/payload identities and verifies
the installed files. Metadata hashes are independent evidence. The exact source,
run/attempt, tool image, DNF/RPM packages, repository definitions/public key,
before/after inventories, actual exits and image inspections are retained at
runtime. No private signing material is eligible for artifact upload.

## Case status

| Case | Expected native result | Status |
|---|---|---|
| Baseline | Requested/dependency/inherited version 1 materialized | not-run |
| Requested RPM update | Requested version 2; changed fixture identity | not-run |
| Transitive-only update | Dependency version 2; changed fixture identity | not-run |
| Inherited-only update | Inherited version 2; native probe returns 100 | not-run |
| Metadata-only revision | Different repomd hash; identical selected RPM/file identity | not-run |
| Same NEVRA, different bytes | Same NEVRAs; changed archive and installed payload identity | not-run |
| Repeated fixed source/base/snapshot | Another executed DNF step; equivalent fixture identity | not-run |
| Required repo unavailable | Native error; unchanged installed inventory; no equivalence/freshness | not-run |
| Corrupted signed package | Native DNF refusal; unchanged inventory; no equivalence/freshness | not-run |
| Newest requested version unsatisfiable | Native DNF refusal with production best/refresh semantics | not-run |

The expected failure must occur at its specific native boundary. Missing tools,
unsupported CLI options, absent execution receipts or unrelated infrastructure
failures cannot count as successful negative cases. A resolver success that skips
the requested newest candidate must be reported as a failed expectation.
Each refusal also requires its specific native diagnostic: missing file-repository
metadata (Curl 37), the exact unsatisfiable dependency, or the named package's
signature refusal correlated with retained bytes and native RPM bad-payload
verification. Unrecognized diagnostics remain failed expectations for review.

## Checks and evidence

No native execution yet. The runtime entrypoint is the development-branch
`Research RPM refresh` workflow. Its sole artifact is
`rpm-refresh-<run-id>-<attempt>`, containing `output/refresh-evidence/`.
Local review/syntax checks are recorded separately in the handoff/worklog; they do
not change the case statuses above. The committed environment/results files are
preparation records; replace or supplement them only from actual retained evidence.

## Remaining boundaries

This is a **synthetic RPM fixture** comparison. It does not establish full Fedora
closure refresh, whole-image equivalence, safe timestamp normalization, a no-op
ISO/release decision, production checkpoint renewal or boot health. Required
Fedora packages outside the synthetic closure are held by the fixed seed image;
their original package archives are not re-resolved against Fedora mirrors.

The remaining section-A cases include changed base identity, timestamp
normalization, mirror inconsistency, lower EVR/vendor/loss exceptions, version
holds/expiry, direct removals, a probe/build metadata race, non-RPM pin policy,
docs/config/mode/helper changes and base signature refusal. An unsatisfiable
**transitive** newest candidate is also separate from the prepared direct-package
case. Section B checkpoint/race/recovery and section C client behavior remain
outside this implementation. No timer, schedule, signer or release/channel writer
is enabled by it.

Next: run the prepared workflow on the development branch, inspect exact native
outputs (including failures), retain public evidence hashes/run links, and update
the case results before using any finding in a production design.
