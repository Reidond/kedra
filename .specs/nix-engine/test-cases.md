# Independent engine test cases

State: planned, 2026-10-01; every behavioral case is **not-run**. Source:
[requirements](requirements.md) AC-01–AC-10/NFR-01–NFR-04 and
[design](design.md), inspected at base `b224d5711e857f7dbcaabf7ed42870916525800c`.
Owner implementation authorization covers this plan; no additional approval gate.

Fourteen conditions map to 22 cases. Cases describe executable observations rather
than repeat the acceptance text. One public CLI lifecycle may cover several cases.
Every automated case has the single owning level **host CLI end-to-end**. TC-21
and TC-22 have the single owning level **manual**. Units, isolated model/API tests,
mocks and doctests are excluded by AGENTS.md; installed-system containers and VMs
add no necessary assertion for this independent backend. Docker is the actual
product executor, not a second testing layer or a new test runner.

Risk treatment: design names substantial trust, publication and execution risks
but assigns no likelihood/impact scores. All named risks receive adverse cases;
no numeric or high/medium/low score is invented. Qualifying this slice requires
these cases, not an inferred low-risk exemption.

## Conditions and derivation

| Condition / source | Technique actually applied | Derived cases |
|---|---|---|
| Independent planning, AC-01 | Decision table: same graph vs changed identity inputs; valid vs each independent graph refusal | TC-01, TC-02, TC-06 |
| Native realization, AC-02 | State transitions: admitted → building → sealed; building → failed/timed out | TC-03, TC-04, TC-17, TC-19 |
| Reuse and reproduction, AC-03 | State transitions: absent → realized → reused → independently rebuilt; valid → corrupt | TC-05, TC-06, TC-15 |
| Closure transfer, AC-04 | Decision table: valid closure; each invalid archive/reference/digest class independently | TC-07, TC-09, TC-19 |
| Offline runtime, AC-05 | Decision table: runtime archive valid/missing/altered/wrong platform, independently of daemon cache | TC-07, TC-08 |
| Generations and development, AC-06 | State transitions: version 1 → version 2 → rollback; interrupted selection | TC-10, TC-22 |
| Serialization/recovery, AC-07 | State transitions: active → concurrent wait; killed → journal → refused → recovered; invalid recovery | TC-12, TC-13, TC-14, TC-22 |
| Roots/GC, AC-08 | Decision table: admission/result/profile/transitive/explicit/unreferenced root; intact vs missing root state | TC-11, TC-12, TC-15 |
| Public Rust authoring, AC-09 | State transition: generated consumer → serialized graph → CLI realization | TC-01 |
| CLI compatibility, AC-10 | Decision table: success/refusal/child failure/Docker unavailable | TC-20 |
| Implementation boundaries, NFR-01 | Decision table: compiler/linter contracts and manual scope inspection | TC-21 |
| Resource limits, NFR-02 | Equivalence partitions/BVA for fixed sizes; state transitions for deadline and overflow | TC-17, TC-18 |
| Private store/trust, NFR-03 | Decision table: valid private root vs unsafe ownership/path/marker state | TC-09, TC-14, TC-16 |
| Canonical tree, NFR-04 | Decision table: byte/mode/name/link changes vs normalized metadata; permitted vs forbidden node type | TC-03, TC-06, TC-19 |

## Shared prerequisites and evidence guards

Use a newly generated private fixture directory, independent producer/receiver/fault
stores, generated C source, and the exact retained builder/runtime images in design.
Version 1 and version 2 print distinct fixture-selected strings and the executable
really calls a separately built shared library at a typed dependency path. An
executable with an embedded version literal alone cannot qualify runtime closure.
The generated Rust authoring program is a real consumer, compiled and run with
ordinary Cargo and a path dependency on the public engine crate; its graph drives
these CLI builds. It does not directly invoke internal functions as assertions.

Each refusal starts from a separately observed valid control, mutates one relevant
input, records the diagnostic, and confirms no valid result/profile was published.
A nonzero exit from missing Docker, a parse error in a different field, or an empty
case selection cannot qualify the intended refusal. Assertions over objects,
images, generations or containers first establish the expected nonempty fixture
population. Enumerate every listed variant in evidence; an unexecuted variant
remains not-run even if another variant passes.

Only fixture-owned stores and operation containers may be corrupted, signaled or
removed. Existing Docker images/containers and retained lab sessions are preserved.
No vault, live home, installed OS, signing material or bootc storage is a fixture.
All operation stdout/stderr/status are captured with bounds. Numeric fixture values
are test inputs, never claimed product guarantees.

## Cases

| ID | Preconditions and inputs/actions | Expected observation and postcondition, with authority | Method |
|---|---|---|---|
| TC-01 | Compile/run a generated external Rust authoring consumer using public typed inputs/output paths. It emits the two-node C graph and resolved identities; invoke its planning mode twice with an unavailable Docker endpoint, then use its emitted transport with public build. | Planning succeeds without Docker and emits equal identities; actual CLI build/run consumes its typed paths and returns the fixture output. No Kedra target/release identity is supplied to the consumer. AC-01, AC-09; design Public values and identity. Static side-effect restrictions additionally reviewed in TC-21. | automated |
| TC-02 | Start with that valid graph; independently supply unknown schema, unknown wire field, duplicate serialized node name, missing input, self/dependency cycle, invalid root and unsupported multiple-output declaration through the public build entry. | Each variant refuses with a diagnostic attributable to that input before an execution container/result exists; valid control remains usable. AC-01, NFR-02; design Public values and identity. | automated |
| TC-03 | Publicly admit generated source and exact images; repeat admission after changing source mtime alone. Admit private source variants containing a regular file, executable, directory and safe internal symlink; separately hardlink, FIFO and escaping link. Request a missing image ID and an available image with unsupported volume/configuration when a fixture image is available. | Accepted source content is immutable and immediately rooted; normalized metadata preserves source ID; permitted types survive later transfer/run. Forbidden nodes/links and unsupported images refuse without registration; missing image does not pull. Images have validated platform/archive evidence. AC-02, AC-08, NFR-04; design Components and concrete choices / Public values and identity. Unsupported-image variants without a real fixture stay not-run. | automated |
| TC-04 | Build the real library/executable. During a deliberately held build, inspect the owned live Docker container; builder records UID/environment and attempts input/root writes, socket/home sentinel access and network access using generated probes. Release it normally. | Real compiler output runs and calls the library; input/root writes fail, UID is nonzero, only declared input/output mounts exist, no host home/socket is mounted, network is disabled, and security/resource settings match design. Owned executor is gone before output sealing. AC-02; design Components and concrete choices / Storage, execution and roots. An actual held container must be observed. | automated |
| TC-05 | Repeat the successful build, then request an independent forced rebuild. Separately use a graph whose builder creates differing bytes from runtime entropy, build it once and force rebuild it. | Ordinary repetition reports validated reuse; deterministic force reports actual independent rebuild and equal tree hash. Divergent force refuses and leaves the original result hash/content runnable. AC-03; design Storage, execution and roots. Record execution evidence to distinguish forced work from a cache hit. | automated |
| TC-06 | Rebuild generated graph variants changing source bytes, explicit env, dependency input and executable permission; plan variants for builder/runtime identity, platform, schema/policy and typed argv where valid. Repeat valid graph with reordered JSON keys and a different physical store directory. | Semantically identical transport/physical relocation preserves IDs; listed identity-bearing changes alter identities or reject unsupported policy/platform, while byte/mode changes alter tree hash. Source mtime alone is covered in TC-03. AC-01, AC-03, NFR-04; design Public values and identity. Do not require execution on unsupported platforms. | automated |
| TC-07 | Export executable closure, independently hash the completed bundle, import using that expected hash into an empty receiver, and run there with the producer stores/source paths made inaccessible to the run. Inspect exported closure via product results/actual archive. | Receiver outputs the library-dependent fixture string offline. Nonempty closure contains executable, library and exact runtime image bytes; compiler image/build-only sources are absent unless actual runtime reference scanning requires them. No producer or build image mount participates. Re-import valid identical content safely reuses it. AC-04, AC-05; design Transfer and trust. | automated |
| TC-08 | On separate receiver copies, remove runtime archive, alter archive bytes, or supply mismatched platform evidence; retain the actual runtime image in Docker cache. Also select a missing foundation explicitly. | Each operation refuses before child execution despite daemon cache presence; original receiver still runs. For positive load-from-archive proof use only an independently provisioned disposable daemon, never delete retained images from the user's daemon; unavailable private daemon leaves that variant not-run. AC-05; design Components and concrete choices. | automated |
| TC-09 | Copy a valid bundle and independently create wrong expected hash; unknown/duplicate member; traversal/absolute member; unsafe symlink/special node; duplicate object ID; missing library/image; changed receipt/tree/image bytes; conflicting already-stored content. Recompute outer expected hash for structural variants so outer mismatch cannot mask member checks. | Exact intended validation refuses, no selected profile or usable incomplete closure is exposed, prior receiver content remains intact, and outside-store sentinel is unchanged. AC-04, NFR-03; design Transfer and trust. | automated |
| TC-10 | Build/import both versions; select and run version 1, select/run version 2, list retained generations, rollback/run version 1. Run a declared command through develop using the same admitted environment and observe child output/status. | Each selected generation executes its corresponding library/executable version; history retains both roots, rollback returns to old output, and develop uses declared paths/environment with observable status. AC-06, AC-10; design Profiles / Public CLI. No service/data/boot claim follows. | automated |
| TC-11 | With both retained profile generations, an extra unprofiled result and an explicit pin, unpin admission/build-only roots and compare GC dry-run to explicit GC. Unpin extra result/pin, then collect again. | Dry-run changes no objects; explicit GC removes only reported unreachable engine objects/images in dependency-safe order. Both profile versions and transitive runtime foundation/library remain runnable; compiler/build-only sources can disappear after unpinning. Explicit pin preserves its object until removed. Outside sentinel and existing Docker images stay intact. AC-08; design Storage, execution and roots. | automated |
| TC-12 | Launch a public run/build held by a generated fixture barrier; verify it is active, start a second build and GC against the same store, then release the barrier. Repeat against two commands competing to realize the same output. | Contending command cannot mutate/publish/collect during active use; after release all finish consistently with one valid winner and no corrupt roots. Actual fixture process/container observation guards against an absent concurrent interval. No performance threshold is asserted. AC-07, AC-08; design exclusive store lock. | automated |
| TC-13 | While an owned builder is demonstrably active with a durable journal, terminate only its controlling CLI process. Attempt ordinary store use, then explicit recovery and rebuild. | Ordinary use refuses the unfinished journal; recovery verifies ownership, stops/removes only the operation container and staging, then normal build succeeds. Unrelated sentinel container remains running. AC-07; design executor journal. No injection into arbitrary processes. | automated |
| TC-14 | In independent fault stores use an unknown/corrupt journal, mismatched store token/owned labels, and a journal with mismatched daemon identity; attempt recovery with the actual daemon unavailable and with foreign container identity. | Recovery refuses uncertain ownership/daemon state and preserves evidence; foreign/sentinel containers and staging outside authority remain intact. Restoring fixture evidence allows TC-13 recovery. AC-07, NFR-03; design executor journal. A mismatched journal tests identity refusal; a real daemon-change experiment is recorded separately if available. | automated |
| TC-15 | Corrupt one stored output byte, receipt, reference, result root or profile generation at a time in copied private stores; request reuse, verify/run, export and/or GC as applicable. | User operations reject the damaged evidence; no cache success, execution or destructive collection is reported. Known-good store remains runnable. Missing root/reference state specifically blocks GC. AC-03, AC-08; design validated receipts/roots. | automated |
| TC-16 | Use safe private store control, then unknown marker/schema, wrong modes, symlinked management path, escaped/unsafe physical bind path, and wrong ownership where a private fixture can safely establish it. | Store use refuses each unsafe condition, touches no outside sentinel, and does not repair/accept unknown authority silently. NFR-03; design Storage, execution and roots. Wrong-owner setup that needs unauthorized privilege stays not-run. | automated |
| TC-17 | Execute real failing builder, deadline-exceeding builder/runtime, output/log producer exceeding declared policy, and builder with a surviving descendant writer. Observe started owned processes and record configured deadlines/caps. | Builder failure/deadline/output-limit breach never registers valid output; deadline termination and bounded draining/cleanup are observed, runtime status is surfaced, and descendants cannot keep writing after sealing. Log overflow follows the declared bounded drain/refusal policy; overflow alone is not assumed to be a builder failure. Existing winner is retained; uncertain cleanup retains recovery journal. AC-02, AC-07, AC-10, NFR-02; design executor lifecycle. Exact cap boundary assertions are pending TC-18; no guessed elapsed tolerance. | automated |
| TC-18 | Publicly submit valid-size and oversized transport: 8 MiB vs 8 MiB+1 bytes, 256 vs 257 graph nodes, 256 vs 257 arguments/environment entries, with otherwise valid generated data. Exercise zero/minimum partitions according to actual schema. For output bytes/file count/log/path caps, use the eventually declared limit and its immediate neighbors. | Above each stated limit refuses without unbounded capture/publication; boundary-valid input reaches the relevant next stage, not an unrelated parse failure. NFR-02; design bounded wire/tree policy. **[UNGROUNDED subcases]** output/log/path numeric caps and arguments/environment combined-vs-separate interpretation need an explicit design contract before exact BVA. | automated |
| TC-19 | Real builders emit canonical safe nodes/self references, then separate invalid hardlink/special/escaping-link outputs, unknown logical store references, missing known references and a non-self reference cycle. Include explicit runtime dependency and an undeclared-but-detectable reference in file bytes/name/symlink within the declared input closure. | Valid canonical output/runtime closure survives export/import; explicit and detected references are retained. Invalid nodes, unresolved references and non-self cycles refuse before valid publication. AC-04, NFR-04; design tree format / runtime refs. If a cycle fixture cannot reach validation, record not-run rather than an unrelated refusal. | automated |
| TC-20 | Exercise public successful management JSON, failing management diagnostic, runtime stdout/nonzero child status, missing Docker executable/endpoint, plus existing public command E2E suites. | Machine-readable management stdout remains parseable; diagnostics use stderr; child stdout/status is observable; Docker absence yields actionable refusal without auto-install/pull/backend fallback; existing command contracts pass their existing workflows. AC-10. No invented numeric exit code beyond observed child status. | automated |
| TC-21 | Run standard formatter, workspace Clippy and release build; manually inspect new public API, dependency boundary and subprocess/publication code against requirements. Review authoring consumer and fixture scope. | Standard gates pass and manual review finds no unsafe Rust/lint weakening, new daemon/language/root builder/custom runner, forbidden tests, target/release coupling or planning-side execution/capture. NFR-01, AC-01, AC-09; design Components. This is manual/compiler evidence, never a source-text assertion test. | manual |
| TC-22 | In private stores, manually interrupt real source/image/object/import/profile publication before/after durable rename and selected-profile replacement where an observable operation permits controlled interruption. Restart via public CLI and recovery; repeat only the observed states. | Readers see a complete valid prior/new record or explicit recovery/refusal, never a selected incomplete closure; old profile selection survives interruption before replacement. Unselected orphan records cannot silently become selected. AC-06, AC-07; design atomic publication / Transfer and trust. A publication window not reached remains not-run; no test-only production hook or fabricated pass. | manual |

## Non-coverage and unresolved qualification

| Area | Risk band | Treatment |
|---|---|---|
| Nix language/nixpkgs, multi-output, remote signed substitution, native Darwin/cross/x86 realization, OS activation/hardware | Not scored in design | Outside requirements' initial scope; no engine result inherits existing OS qualification. |
| Arbitrary compressed/dynamic reference encodings; compromised owner/daemon; ownership/xattr/SELinux package attestation | Explicit design trust/format limitation, unscored | Concrete generated C closure only; do not extrapolate protection or closure completeness. |
| Output/log/path exact size boundaries and argument/env interpretation, TC-18 | Pending contract, unscored | Retained as ungrounded subcases until design supplies values; resource enforcement behavior still required in TC-17. |
| Unreachable manual publication window, private-daemon reload, foreign-owner or unsupported-image fixture | Environment-dependent, unscored | Record not-run/blocked with the exact missing precondition; never count ignored tests or invalid setup as a pass. |

No accepted-unverified exception is granted to AC-01–AC-10 or NFR-01–NFR-04.
The implementing agent may settle ordinary bounded-policy details within authorized
scope, update design and these subcases, and then run them. A real implementation
limitation must remain visible in final status.

## Result reconciliation — 2026-10-01

This is the planned matrix, not a blanket pass. [verification.md](verification.md) and the public E2E case names provide observed coverage. The complete adverse subvariant population was not executed; TC16/19 full matrices and TC22 power-loss/ENOSPC remain not-run. Shared source DAG behavior does not establish bundle-import DAG regression coverage.
