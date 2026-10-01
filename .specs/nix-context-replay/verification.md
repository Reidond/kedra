# Context replay evidence — 2026-10-01

Branch `codex/nix-context-replay`, inspected parent PR29/3350732. Codex with
Astra-only restoration, verifier, harness/E2E implementation and read-only review.

## Foundation restoration

Pass: unsigned candidate91e27148 is rejected by the installed source policy.
Selected already reviewed signed production foundation instead:
`sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71`,
source `f3d69dbaba0a50fc167efeb2e0e5d3a6caf6b2c8`, Fedora44.20260930.0/native ARM.
Skopeo1.22.3 copied anonymously under its installed default reject/sigstoreSigned
policy, preserving digests. Destination signature sidecars were omitted because
OCI archives do not support them; source signature verification remained required.
No policy override, host auth/key/socket mount, production signing or tag change.

Pass: independently verified exact root, configuration and all78 ordinary TAR
layers. Removed only the outer OCI index's generic naming annotation before
loading, retaining every signed manifest/config/layer byte. Verified again.
Private archive SHA256 `65265b34b42be2ba3d706a1d81230f899f30812eda4d457534ab3d9ad13dc333`,
6,411,493,376 bytes. Skopeo resumed a registry EOF and completed without altering
verification. Peak private copy space12,539,915,249 bytes, below16GiB.

Pass: Docker29.4 loaded exact9d6eb030 ID with no tags, nativeARM. New complete
Docker export is6,411,500,544 bytes, SHA256
`70bec2621a3026f12e452c1c1f6d78e3de522183262c72c48ba4bd06ae62dc52`.
Its exact root/config/all78 layer hashes and Docker compatibility manifest pass.
Read-only/no-network/nonroot execution reports aarch64/Fedora44; session script
mode0755/SHA9d6b1559…adcd18 matches current source. Actual image CMD `/sbin/init`,
empty user, no ENTRYPOINT/ONBUILD/volumes. Failed candidate cache is unchanged.
Local details: `target/nix-context-replay/foundation-restore/restoration-summary.json`.

## Implementation checks and review

Pass: initial workspace formatting/locked all-target Clippy; targeted engine and
harness compilation/lint, normal/composition-only scenario discovery and replay
help. No new external Cargo versions; harness adds existing local engine/rustix
edges. Public verification CLI preflight passes without Docker.

Review found and repaired producer/consumer manifest-size mismatch: a permitted
1MiB byte-array config can exceed8MiB when pretty printed twice. A shared64MiB
composition bound and bounded streaming exporter prevent a successful unsupported
export; existing small bytes/schema/input identity remain unchanged. Also repaired
cache publication interruption with owned staged tags, versioned durable journals,
atomic bindings and per-context locks. Foreign tags and unknown state still refuse.
These are source review findings; runtime coverage is recorded below when observed.

## Runtime status

Interrupted: first dev-binary native test was deliberately stopped during large
artifact hashing/build verification (SIGTERM to its verified private-fixture
child),1 preflight pass/1 interrupted case,349.12s. The fixture is retained at
`target/nix-context-replay/consumer-e2e`. This is not a product-defect or pass result.
Final run uses optimized release product binaries with the same checks enabled.

Pass: final explicit E2E2/2 in698.65s using release product binaries through the
ordinary Cargo test executable. Compiled ELF/separate runtime library, typed config
and unit export,1MiB manifest round trip, deleted producer, semantic tamper matrix,
static offline output and cached/foreign-tag refusal all pass. Retained consumer:
`target/nix-context-replay/consumer-e2e-release`, independent identity
`173705f43be7338df42e2d1dc696f7243ff969a24e43f3b333977598b591b785`.

Pass: `native::composition_unit` through release container harness,1/1 in18.47s
after image preparation121.3s, execution1790860636-24677. Actual systemd-analyze,
unit start/status, successful exit, marker/config/identity and journal output pass;
cleanup reports no failure. Report: `target/kedra-lab/runs/1790860636-24677/report.json`.
An initial run with a different artifact/cache root correctly refused the existing
tag without its binding; retry uses the established private cache root. No binding
or provenance verification was bypassed. Static image is
`sha256:c53a99f1b705c36f929b787cd9acd1bebe74225f5a8dd48b2f5c1ab56577d9c0`.

Pass: workspace formatting/Clippy/ordinary E2E and release build. Native Docker
cases are ignored in that ordinary invocation; Linux-only home/TPM cases are zero
on macOS. Existing uv/OpenSSL release-interop and release-material CLI workflows
pass in fresh replay-specific directories.

Full current-repository composition initially refused the wrapper mode: Git records
`usr/libexec/kedra-session`100644, while `image/assemble.sh:99` sets0755. Adapter
now records that existing native rule as exact foundation passthrough, retaining
raw source.json mode unchanged. Pass: current source3350732 composes20 files and
1435 observed RPM rows into context
`adb798a8e9e0f388335a3b23bb6a67741039a1dcbbfb26e8bd396315dac3627d`.
Session is exact0755 foundation passthrough, not replacement. Source/current-mode
compiler/linter and public source/system preflight E2E pass after this correction.

Pass: actual replay interruption at pre-build durable journal. Owned process58175
received SIGTERM and exited1 after50.21s; journal has image:null, pending tag absent.
Same public command retry58684 exits0 in52.93s, produces exact image
`sha256:87e488d684e9e09ff924dfe039c49777f50457a9ac5555dbbfd51c0b397f6953`,
and retains0600/uid502/single-link binding to the exact image/identity/foundation/
daemon. Journal and its owned nonce tag are absent; unrelated resources untouched.
Evidence: `target/nix-context-replay/recovery/{interruption,recovery}.json`.
This also qualifies building the complete current-source static context. No boot,
desktop health, signed deployment or configuration-derived regeneration is inferred.
Post-image-ID/post-temp-publication interruption windows remain not-run.
Draft publication is pending at this record's creation.

Cold independent daemon, boot/SELinux/install/update and signed release integration
remain separate not-run gates unless actual later evidence is recorded. Loading an
archive into this existing daemon is not a cold-daemon pass. Static replay is
offline; ordinary desktop lab adaptation remains a separate networked operation.
