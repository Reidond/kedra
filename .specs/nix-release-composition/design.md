# ARM release composition design

State: proposed caller contract; no implementation or runtime work has started.
Reconcile exact D1 outputs at its source freeze before editing shared files.

## Normal candidate path

Keep `release.yml` and its build -> validate-candidate -> isolated signer ->
publish-stable shape. Only qemu-arm64 changes its candidate constructor:

1. Existing committed source archive, external agent/Bitwarden payloads, pinned
   tool binaries and current-source preflight resolve the reviewed base and RPMs.
2. Build a fresh unsigned Fedora foundation from those bounded inputs using the
   ordinary Containerfile and a closed assembly mode. Do not use last stable as
   the production foundation: that would hide package changes/removals.
3. Retain the complete exact foundation in the ordinary-user engine store. Use
   `sysroot system compose`, verified static replay and `kedra-lab derive-plan`
   / `derive` to apply the committed payload and fixed native stages.
4. Transfer the exact generated image into the existing public-trust derivative;
   add unchanged signed identity/material/trust files, run bootc lint, inspect
   installed bytes without executing candidate code, and publish the immutable
   unsigned builds-repository digest.
5. Existing full container validation tests that final digest with overlay none.
   Signing and final strict publication continue with their current restrictions.

`assemble.sh` accepts exactly no argument (legacy full assembly),
`--resolve-packages` (existing early RPM inventory exit), or `--foundation` (new
ARM foundation). Unknown/multiple arguments fail before transactions. A closed
Containerfile argument selects `complete` or `foundation`; default remains
`complete`. Foundation is valid only for qemu-arm64. It retains source payload,
RPM transactions, external runtimes, wrapper modes, video-bridge adjustment,
os-release branding, cache cleanup, configuration validation and RPM inventories.
It defers GLib compilation, systemd enable/mask/default links, managed skeleton
copy and QEMU initramfs generation. Verify the intermediate foundation as an input;
the final native/trust candidate owns bootc lint and installed-system acceptance.
Do not claim inherited base caches/links absent unless actually inspected.

## Caller/data contract

Proposed new standard-library host entrypoint:
`uv run usr/src/kedra/image/release/compose.py`. This is build orchestration, not
a second test runner, installer, expression language or signing authority.
It uses explicit argv and absolute pinned executable paths. stdout is one JSON
result, diagnostics go to stderr; output directories must be new private paths.

Two operations permit D3 to build packages against the exact same foundation:

| Operation | Required arguments | Result |
|---|---|---|
| `foundation` | `--target qemu-arm64 --repo PATH --source-revision SHA --inputs PATH --expected-inputs-sha256 HEX --context PATH --store PATH --binaries PATH --output-dir PATH` | `foundation.json` plus immutable retention/evidence |
| `compose` | `--target qemu-arm64 --repo PATH --source-revision SHA --inputs PATH --expected-inputs-sha256 HEX --foundation-receipt PATH --expected-foundation-receipt-sha256 HEX --store PATH --binaries PATH --output-dir PATH` | `candidate.json`, context/native plans/receipts and exact image transfer evidence |

`--inputs` is the existing canonical resolved-inputs schema1 record, supplied by
production preflight or the fixture preparer. The runner verifies its independent
hash, target/source material and actual binaries/context inventory. It never
executes a command/string carried by JSON. Production additionally enforces main
through unchanged `refresh.py`; the public builder itself can use a frozen branch
revision. Both modes refuse a different HEAD before source materialization.
Factor deterministic material construction into the existing `material.py` module
so production refresh and the fixture preparer generate identical input structure;
do not create a branch override for `refresh.py current_source()`. Branch manual
comparison can qualify produced input bytes, not the protected workflow's skip
behavior. That exact operational no-change gate belongs to TC-11 on main.

The foundation result schema is closed and contains: `schema_version: 1`,
`target`, `source_revision`, `input_material_sha256`, `foundation_image` (opaque
exact Docker image ID), `engine` (daemon identity), `rpm_sha256`,
`source_manifest_sha256`, and `retention_receipt_sha256`. The retained graph must
verify all manifest/config/layer bytes, not merely trust this orchestration receipt.
Paths are transport locations, not content identity. Store/daemon changes refuse.

The candidate result schema is closed and contains: `schema_version: 1`,
`target`, `source_revision`, `input_material_sha256`, `foundation_image`,
`engine`, `composition_identity`, `native_identity`, `native_image`,
`native_receipt_sha256`, `source_manifest_sha256`, `rpm_sha256`, `outputs`
(alias -> object identity), and `transfer` (native manifest/config digest evidence
and local Podman reference). This is the unsigned composed parent; the existing
release derivative produces the final release candidate and its distinct digest.
Existing replay/native receipts remain authoritative for their own material.

The engine and OCI platform spellings are different domains:

| Domain | Exact representation / source |
|---|---|
| Engine SystemDefinition and ImageReceipt platform | `aarch64-linux`, `sysroot-engine/model.rs::PLATFORM` |
| Docker build platform | `linux/arm64` |
| OCI config | `os: linux`, `architecture: arm64` |
| Kedra target source | `id: qemu-arm64`, `architecture: aarch64` |
| Engine object | `src-<64 lowercase hexadecimal>` or `out-<64 lowercase hexadecimal>`, `plan.rs::object_id` |
| Engine output alias | 1-128 ASCII bytes matching `[A-Za-z0-9_-]+`, including underscore/hyphen as the first byte, `plan.rs::name` |
| Composition/native identity | 64 lowercase hexadecimal digits, without the engine object prefix |

Never conflate Docker image ID (observed archives may name OCI indexes), selected
platform manifest digest, config digest, DiffIDs, composition/native identity and
final registry digest. Transfer must verify the complete selected platform graph
and filesystem lineage. Preserve manifest digest where the transports permit;
otherwise record both graphs and prove config/RootFS/artifact equivalence before
the existing final native-manifest publication check. No image label alone proves
lineage. A transfer whose relationship cannot be proved fails closed.

## D3 package contribution extension

Optional `compose` arguments are `--definition PATH
--expected-definition-sha256 HEX --contribution-receipt PATH
--expected-contribution-receipt-sha256 HEX`, accepted as one complete group.
No contribution means empty additional outputs/files and is the D2 normal path.

`definition` is the existing Rust `SystemDefinition`, serialized by the compiled
D3 author: schema1, platform `aarch64-linux`, exact `foundation`, verified `outputs`
alias map, package requirements/removals and `SystemFile` declarations. Templates
use `SystemContent::Template(Argument::input(alias, relative_path))`; the serialized
segment is `{"kind":"input","name":"jq","path":"bin/jq"}`. Mutable
`Segment::Output` is rejected. The source adapter still injects reserved committed
Kedra provenance, home baseline and source.json; contributions cannot replace them.

The proposed D3 contribution receipt is a bounded closed record:
`schema_version: 1`, `source_revision`, `foundation_image`,
`input_material_sha256`, `definition_sha256`, `catalog_pins_sha256`,
`author_sha256`, and `outputs` (alias -> object identity). It cross-checks the
definition; it does not authenticate arbitrary caller claims. The engine verifies
every output receipt/tree/closure and the exact runtime foundation independently.
D3's workflow prepares verified packages in this same private store. If moving
between stores, use existing closure export/import with an independently selected
bundle SHA; do not teach the release runner to fetch arbitrary URLs or commands.

Before preflight, D3 supplies the canonical pinned source inventory and reviewed
author recipe/executable hashes. These are deterministic `artifacts`/`recipes`
entries. Catalog realization occurs only after foundation creation; realized IDs
and generated definition bytes are result evidence, not preflight equality inputs.
The workflow must invoke the reviewed author from the frozen source, verify its
hash and pinned source inventory, and pass actual definition/receipt hashes.
Unsupported/different pin inventories refuse. D2 does not implement catalog fetch
or author execution hooks.

D3's preparer confirmed this boundary on 2026-10-02: its public API first resolves
a deterministic graph and checked metadata, returns the recipe plus engine Plan,
and takes an explicit exact runtime foundation. Parent's Kedra adapter owns this
release-specific contribution receipt; the generic catalog does not import Kedra
release constants. Archive/tree pins and recipe-module hash are preflight inputs.
Every output value retains its engine `src-`/`out-` prefix. A bare digest and an
OCI platform string are invalid in that engine transport. Retained ImageReceipt
uses the same `aarch64-linux` spelling as SystemDefinition, not the Docker spelling.

Current static schema cannot replace arbitrary `/usr/bin` files or emit symlinks.
D3 may expose installed commands through existing supported configuration (for
example typed environment.d paths) or own a separate narrow typed exposure change.
Do not broaden overlay paths or insert arbitrary root scripts in D2. Candidate
receipt aliases let installed E2E execute the exact closure even before command
exposure is added. D3 must qualify exposure through real installed public commands.

## Deterministic native recipe and no-change

Add `usr/src/kedra/image/release/native-steps.json`, a closed schema1 record of
the four existing NativeStep values: GLib schemas; systemd enabling greetd,
NetworkManager and bluetooth, masking bootc automatic update service/timer and
selecting graphical.target; initial skeleton; QEMU initramfs with explicit sorted
virtio_dma_buf/virtio_gpu/virtio_input modules. No commands, paths to executables
or parent identity are accepted in this file. The runner injects the independently
verified current parent identity to form the existing NativeDefinition.

Hash canonical deterministic step bytes in `resolved-inputs.recipes`, together
with compose.py and its known implementation inputs; hash the pinned harness
executable in `artifacts`. Runtime parent IDs, source/run timestamps, invocation
paths, elapsed times and generated initramfs/receipt bytes are excluded from
preflight equality. Preserve existing canonical schema and field sets.

Final composed/native receipt files and actual artifacts are included in the
signed OCI image. Candidate inspection compares them to independently retained
build results. No new release-identity field or helper trust schema is needed.
Deterministic recipes are enough to trigger a changed-input build; the final
signed digest binds the resulting bytes without promising reproducible initramfs.

## Fixture installation and update path

Extend the existing Actions-only `ghcr-update` workflow with a closed ARM/full
candidate mode, preserving the existing desktop cases. Build real A/B images
through the same candidate composer. Use generated fixture keys and TLS registry
restricted to the disposable runner/VM. Use production-shaped repository/target
identity only inside that isolated registry; network routing must resolve it to
the fixture before any write. No production token is provided. Both identities
bind actual source/material bytes. B has a changed declared default/artifact and
higher fixture rank; B's real native receipt must differ from A as intended.

Fixture-only immutable derivative adds observer, test CA and generated public
trust before fixture signing; final A/B fixture digests are reported distinctly
from unsigned normal candidates. Test code has no production authority. Reuse
D1's qualified immutable conversion, Microsoft-enrolled ARM firmware/NVRAM and
exact boot artifact observations; no generic guest root command channel.

For Anaconda, invoke the existing local installer against the fixture-signed A
digest from a disposable fixture checkout with generated public authority and
matching fingerprint. Record the authority substitution; do not alter the real
checkout or add a production verifier bypass. Keep image source manifests bound
to the actual candidate source. Run its public build entrypoint on native Linux
ARM, perform separate diskless verification, then manual bounded installation
onto one new disk with another full-hash sentinel. The existing `--smoke` path
requires KVM; hosted ARM runners have none, so use the D1/native QEMU route for
the manual diskless/install flow, recording that distinction rather than relaxing
`smoke.py` or claiming its pass.

After ISO-free A boot, create real account home files/preferences and `/var`
application data; record content/hash/mode/owner. Invoke public enroll/check/stage
to B, boot B, verify those values, change them, then invoke retained rollback and
boot A. Verify B's newest home/data persists, baseline is not reapplied, exact
booted image/artifacts/security are A, and high-water/hold/resume retain B ordering.
Unsigned/wrong key/repository/withdrawn signature/lower rank/pending preservation
remain real registry/client cases, not synthetic helper state tests.

## Failure and resource behavior

Inputs and outputs are private, bounded and reject symlink aliases; reuse the
engine/harness verified readers rather than new tar parsers. Refuse existing
output destinations. Each process has a finite deadline; build/retain/compose and
derive stages default to 2400s each, simple inspection to 300s. Reconcile with the
workflow's overall timeout after first measured run. No automatic retry. Explicit
operator retry reuses only verified engine/harness cache state in a new result
directory. Production main-advance or RPM snapshot mismatch terminates the run.
Old stable remains unchanged until normal strict publication succeeds.

Cleanup operates only on recorded invocation resources after process exit. Preserve
foreign/default VM/container/media, journals needed for explicit recovery and safe
public evidence. No raw generated credentials, registry auth or private keys enter
artifact uploads. Fixture keys live in a separate 0700 directory and are deleted
on terminal cleanup. Production keys remain exclusively in the isolated signer.

## Files and boundaries

Production edits after grant: `image/{assemble.sh,Containerfile}`,
`image/release/{compose.py,native-steps.json,refresh.py}` and
`.github/workflows/release-target.yml`. Candidate material inspection may add
narrow validators to `image/release/material.py` without changing schema.

Fixture edits: `.github/workflows/test-{qemu-arm64,ghcr-update}.yml`, existing
`tests/vm/ghcr-update/{run.sh,prepare.py,check.py,Containerfile,variant.Containerfile}`
and small ARM fixture support under that existing directory if needed. Reuse D1
VM bridge. Changes to D1-owned VM files require coordinated re-review after freeze.
Parent owns root metadata/docs/skills and all Git operations. No installer/helper
policy weakening, new crate/test runner or ordinary execution privilege change.

## Alternatives and risk

Keeping full shell assembly then deriving again is smaller, but does not prove
the native stage supplies missing artifacts; foundation mode avoids that ambiguity.
Replacing Fedora/RPM resolution or generalizing x86 now increases scope without
helping the first ARM release. Calling composition code inside the signer violates
the existing authority boundary and is rejected.

| Risk | Likelihood / impact | Control |
|---|---|---|
| Foundation lacks required Fedora state | medium / high | full derived container and actual D1 boot/install/update |
| Transfer changes image identity or loses layers | medium / high | complete graph retention and explicit identity-domain receipt |
| Repeated no-change churn | medium / medium | compare deterministic inputs only; production repeat stays not-run |
| Branch fixture accidentally contacts production | low / critical | no production token; isolated DNS/TLS namespace checks before writes |
| Existing home overwritten on transition | low / critical | real A/B/A modified home and data assertions |
| ARM install tooling/TCG limits | medium / high | separate native manual installation; no borrowed QCOW2 install pass |
| Main protected signing unavailable on branch | certain / high | deliberate production qualification boundary |

Rollback means retain prior signed digest/signatures and use the existing held
bootc rollback; no schema migration is introduced. Builder failure leaves stable
unchanged. Source changes can return ARM construction to the prior full builder
only through a reviewed source revision and its normal validation/signing gates.
