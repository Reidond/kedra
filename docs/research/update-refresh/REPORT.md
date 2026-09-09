# Native RPM refresh and equivalence

Recorded: 2026-09-09 (Europe/Kiev). Status: **pass for ten bounded native cases**.
Scope: section A of
[EXPERIMENTS.md](EXPERIMENTS.md), under R02/R07/R08.

The Actions-only experiment in
[build/research/refresh](../../../build/research/refresh/README.md) prepares real
signed RPM repositories and materializes disposable Fedora 44 containers with
native DNF/RPM. [Run 34286322016, attempt 1](https://github.com/Reidond/kedra/actions/runs/34286322016)
passes at source `0d82b1ff2508b3a09a62f9e8915e75a77d0506dd`. Main `c660c58` and
owner media run `34255228394` are unrelated and unchanged by this experiment.

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

| Case | Observed native result | Status |
|---|---|---|
| Baseline | Requested/dependency/inherited version 1 materialized; probe 0 | pass |
| Requested RPM update | Requested version 2 installed; fixture identity changed | pass |
| Transitive-only update | Dependency version 2 installed; requested remains version 1 | pass |
| Inherited-only update | Inherited version 1 upgraded to 2; probe returns 100 | pass |
| Metadata-only revision | Different repomd hash; identical selected RPM/file identity | pass |
| Same NEVRA, different bytes | Requested RPM retains version 1; archive and installed payload identities change | pass |
| Repeated fixed source/base/snapshot | Separate native DNF execution; equivalent fixture identity | pass |
| Required repo unavailable | Probe exit 1, Curl 37 for missing metadata; inventory unchanged | pass |
| Corrupted signed package | Install exit 1; exact downloaded RPM has BAD payload digest; inventory unchanged | pass |
| Newest requested version unsatisfiable | Install exit 1 for missing dependency >=99; inventory unchanged | pass |

All seven successful materializations return container-build exit 0. The three
expected refusals return native DNF and container-build exit 1, with no completed
equivalence record. Their before/after-failure inventory hashes all remain
`15c32cf8e46edf02f599bf578f24c6ad83be6009a8b315805318b6ebeacba3a2`.
No checkpoint or other freshness publication operation was executed.

## Checks and evidence

The native environment is Fedora 44 x86_64 with DNF/libdnf5 `5.4.4.0-1.fc44`, RPM,
rpmbuild and rpmsign `6.0.2-1.fc44`, GnuPG `2.4.9-16.fc44`, createrepo_c
`1.2.1-5.fc44`, Python `3.14.7-1.fc44` and runner Podman `4.9.3`. The exact base is
`quay.io/fedora/fedora-bootc@sha256:d4b9c5e156ab0a119962aad27c5394409094cc24348846a35c78acd8e9847a4d`.
The fixed seed image ID is
`8d9f15ef417be6e0e42ae51d7ec09f7831def9d64e1b267dddb9f8ba4de870e9`.
Full source-input hashes, image IDs, versions and public key identity are in
[environment.json](environment.json); exact case outcomes are in
[results.json](results.json).

Artifact `10079686892`, named `rpm-refresh-34286322016-1`, is 2,659,915 bytes with
ZIP SHA-256 `5bc9302989ad3b42e0a69ae118059000e198743b9e4dc20ade8fe8969e144656`.
The downloaded ZIP independently matches GitHub's digest and is retained locally
at `output/refresh-34286322016-1/`; its extracted public native evidence is in
`evidence/`. Actions retention currently ends 2026-09-22. The artifact contains
native command logs/exits, repository snapshots, public keys, generated specs,
actual selected RPM archives, before/after inventories and image inspections.
Private-directory cleanup exited 0; private signing material was never in the
artifact input path or a built image.

Manual artifact inspection confirms the native requested/dependency/inherited
transactions, the three precise failure diagnostics and their unchanged inventory
hashes. Native `rpmkeys` reports a valid header signature but BAD payload digest
for the damaged package, explaining DNF's otherwise generic package-open message.
The retained rejected archive exactly matches the corrupted repository bytes.

The baseline, metadata-only and repeated executions share fixture comparison hash
`3b3faaa2996a0af59adaf6f0a7d12dd0a0dc6bebb3457d19949494950b902eea` and inventory
hash `05ae418e9e531b9d7b6ba67ea848fa9fa92e1b5e912ceea6f25683e1ef200090`.
The metadata-only repomd hash changes from `a20c7966...` to `40a0631a...` while
selected RPM hashes stay equal. Separate execution receipts and native command
times establish that DNF ran again. Their actual container image IDs differ;
this comparison does not prove full-image equivalence or safe ISO pruning.

Same-NEVRA requested RPM SHA-256 changes from
`c24991bddea7248368056335c0ab83a8308658314f5c20b98e2db0d588976ff9` to
`4fa4993fc86fa5dced520686ce8dab693cadc75ee379c631818fa1fb6eb61b85`, with a changed
installed payload hash. Native RPM verification and archive/installed header
comparison pass for all successful candidates. No implementation repair was
needed after this first native run.

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
**transitive** newest candidate is also separate from the measured direct-package
case. Section B checkpoint/race/recovery and section C client behavior remain
outside this implementation. No timer, schedule, signer or release/channel writer
is enabled by it.

Next: extend the native fixture to the remaining resolver/input cases, then prove
complete Fedora/OS material-input equivalence before connecting any result to
signed freshness renewal or deciding to skip an image/ISO release.
