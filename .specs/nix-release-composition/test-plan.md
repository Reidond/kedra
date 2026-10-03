# Release composition verification plan

Every case starts **not-run**. Only actual E2E/manual workflows and standard
tools; no source scanners, unit/model/mock/doctests or alternative test runner.
Cases have one owning level; lower-level output inspection supports that case,
not an additional purported acceptance pass. The source/controller is frozen and
hashed before/after every runtime invocation. Parent serializes heavy runtime.

| Case / owner | Level / technique | Requirements | Expected evidence |
|---|---|---|---|
| TC-01 / T2,T3 | public CLI E2E / state transition | AC-01, NFR-02 | Fresh foundation -> composition -> native -> public-trust candidate; actual source/RPM/native material and exact identity domains; full installed suite belongs TC-05 |
| TC-02 / T2,T3 | public CLI E2E / invalid equivalence classes | AC-02, NFR-05 | Unsupported assembly mode, wrong source/target/hash/foundation/receipt each refuse before final result publication; foreign sentinel and existing outputs unchanged |
| TC-03 / T5 | public CLI E2E / valid-invalid contribution pairs | AC-11, AC-12 | Generated compiled executable plus runtime closure consumed through typed alias/input reference; exact stdout from final image; wrong pin, absent alias, other foundation and reserved path refuse |
| TC-04 / T4 | manual built-material comparison / decision table | AC-03, AC-04 | Same production material constructor emits identical pinned input bytes; changed recipe/pin/tool changes bytes; generated build time/native ID does not perturb them; actual production skip behavior belongs TC-11 |
| TC-05 / T4 | candidate container E2E / exact artifact | AC-05 | Full sanctioned harness report matches final candidate reference/target, overlay null, all selected cases pass and cleanup has no failures |
| TC-06 / T4 | public material/client E2E / corrupted artifact | AC-06, NFR-04 | Actual changed receipt/source/RPM/material/config-label bytes fail bounded validation; signer source/identity schema preservation is manual review, production execution belongs TC-11 |
| TC-07 / T6,T7 | native manual installation / state transition | AC-07, NFR-01, NFR-02 | Fixture authority/media receipts, diskless verifier, explicit fresh selected-disk install, unchanged full sentinel/media hashes, ISO-free A boot, security/doctor/native artifacts and account defaults |
| TC-08 / T6,T7 | native VM E2E / trust decision table | AC-08, NFR-01 | Correct signature + Secure Boot allows verifier; unsigned/wrong-key or Secure Boot off prevents installation; disks remain unchanged on each refusal |
| TC-09 / T6,T8 | native VM E2E / A-B-A state transition | AC-09, AC-10, NFR-01 | Real registry/public client, actual A/B/A boot identities/security/artifacts, home + /var hashes/modes/owners before/after, B writes survive rollback, hold/high-water/resume and refusals preserved |
| TC-10 / T9 | standard tools + manual review | NFR-02, NFR-03, NFR-04, NFR-05 | Formatting/Clippy/release build/CLI E2E/Ruff/syntax, actual runtime timeout and cleanup observations, isolated signer review, safe evidence and exact draft branch/base readback |
| TC-11 / T10 | production Actions E2E / changed-unchanged transition | AC-03, AC-05, AC-06, NFR-04 | Separately authorized main build/test/sign/strict publish, exact stable readback, unchanged repeat without renewed rank/publication; explicitly not-run on branch |

Decision table for TC-04: same deterministic material -> unchanged; changed
recipe -> changed; changed immutable source pin -> changed; changed executable
hash -> changed; only invocation time/output image ID differs -> unchanged.
Use ordinary comparison of actual built canonical material, not a mocked GitHub
main query or a fixture that weakens `refresh.py current_source()`. TC-04 supplies
branch evidence about input construction only; AC-03's operational skip remains
not-run until TC-11 is authorized on main.
TC-03 contribution partitions include real `aarch64-linux` definitions and
`out-<hex>`/`src-<hex>` identities, valid underscore/hyphen aliases, and refusal of
the Docker `linux/arm64` spelling, bare object digests and aliases containing `.`
or `+`. These are actual engine transport boundaries (`model.rs`, `plan.rs`), not
a new producer schema. Execute through the public composer with real retained
objects; do not add isolated parser/model tests.
TC-08 trust table: valid signature + enabled Secure Boot -> verifier succeeds;
invalid signature + enabled -> refusal; valid signature + disabled -> refusal;
invalid signature + disabled -> refusal. Cases test product behavior, not source.

## Commands and receipts

These are implementation-target commands, **not commands already present or run**.
Paths in angle brackets are independently observed/pinned values. The composer
must implement the stated interface before these become executable instructions.
The fixture command may reuse the existing ghcr-update workflow entrypoint, not
introduce another general test harness.

```sh
uv run usr/src/kedra/image/release/compose.py foundation \
  --target qemu-arm64 --repo <frozen-checkout> --source-revision <commit> \
  --inputs <resolved-inputs.json> --expected-inputs-sha256 <sha256> \
  --context <bounded-context> --store <private-store> \
  --binaries <pinned-binaries> --output-dir <new-foundation-output>

# D3 later builds its pinned packages in the same store, against the emitted
# exact foundation, then emits the existing typed definition and its receipt.
uv run usr/src/kedra/image/release/compose.py compose \
  --target qemu-arm64 --repo <frozen-checkout> --source-revision <commit> \
  --inputs <resolved-inputs.json> --expected-inputs-sha256 <sha256> \
  --foundation-receipt <foundation.json> \
  --expected-foundation-receipt-sha256 <sha256> --store <private-store> \
  --binaries <pinned-binaries> --output-dir <new-compose-output>

# Optional complete package group added to compose for TC-03/D3:
# --definition <system.json> --expected-definition-sha256 <sha256>
# --contribution-receipt <contribution.json>
# --expected-contribution-receipt-sha256 <sha256>
```

Existing public calls used inside the composer remain real CLI surfaces:

```sh
<sysroot> store init --store <private-store>
<sysroot> store add-image --store <private-store> --image <exact-foundation-id>
<sysroot> system compose --repo <frozen-checkout> --target qemu-arm64 \
  --store <private-store> --foundation <exact-foundation-id> \
  --output-dir <new-context>
<sysroot> system verify --context <new-context> \
  --expected-identity <composition-id> --workdir <private-verification-dir>
<kedra-lab> derive-plan --target qemu-arm64 --image composition:<new-context> \
  --overlay none --composition-identity <composition-id> --native-plan <native.json>
<kedra-lab> derive --target qemu-arm64 --image composition:<new-context> \
  --overlay none --composition-identity <composition-id> --native-plan <native.json> \
  --derivation-identity <native-id>
```

Run final immutable-candidate installed checks on native ARM using the existing
harness; `ref:` must resolve to the just-built final candidate in the selected
daemon/fixture registry, not another build of the same source:

```sh
KEDRA_LAB_TARGET=qemu-arm64 KEDRA_LAB_OVERLAY=none \
KEDRA_LAB_IMAGE=ref:<immutable-final-candidate-reference> \
cargo test -p kedra-container-tests --test container --release --locked

# Proposed closed mode on the existing Actions-only workflow fixture entrypoint:
bash usr/src/kedra/tests/vm/ghcr-update/run.sh --target qemu-arm64 \
  --candidate-a <verified-A-candidate.json> --candidate-b <verified-B-candidate.json>

# Existing public installer, from the isolated fixture checkout/native Linux ARM:
uv run usr/src/kedra/installer/build-local.py \
  --image ghcr.io/reidond/kedra-qemu-arm64@sha256:<fixture-A-digest> \
  --output-dir <new-fixture-iso-directory>
```

The fixture flow must retain separate stages so actual Anaconda-installed A disk
can be handed to update tests; it cannot silently substitute its usual image-builder
QCOW2. Exact D1 public VM attach/boot commands are filled at D1 freeze from its
implemented help/API. Host actual disks are never accepted. Hosted ARM lacks KVM;
do not claim the installer's `--smoke` pass when using manual native diskless boot.

Standard gates after implementation, parent-serialized:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
uv run ruff check usr/src/kedra/image/release usr/src/kedra/tests/vm/ghcr-update
uv run usr/src/kedra/tests/cli/release-interop.py --sysroot target/release/sysroot \
  --workdir <new-release-interop-dir>
uv run usr/src/kedra/tests/cli/release-material.py --workdir <new-release-material-dir>
```

Preserve exact source and binary hashes, Docker/Podman/Skopeo/builder/firmware
versions/digests, foundation/native/final fixture image domains, all phase reports,
safe serial/screenshots, disk/media hashes and owned cleanup outcomes. Capture
no credentials. Refusal success needs the expected reason and unchanged protected
state, not merely a nonzero exit. Enumerated runtime cases must be nonempty.

## Deliberate non-coverage

- TC-11 production signing/publication remains not-run: certain/high risk boundary
  in design, protected main/environment and separate authorization required.
- Desktop native integration remains excluded: medium/high foundation risk;
  existing desktop builder must continue passing its ordinary workflow.
- Key rotation and physical hardware remain excluded: low/critical trust and
  medium/high platform risk respectively; fixture keys/VMs do not qualify them.
- Power loss/ENOSPC/cache substitution recovery belongs D5; D2 covers terminal
  failures and bounded owned cleanup, not a new crash-recovery implementation.
- Unit/mock/model/source-text tests are deliberately absent under repository
  policy. Static inspection is never substituted for boot/install/update evidence.

Traceability: T1-T10 own TC-01 through TC-11 as listed in tasks.md. All AC-01 to
AC-12 and NFR-01 to NFR-05 have an owning case. Runtime statuses will be recorded
only from observed results; this document records planned verification.
