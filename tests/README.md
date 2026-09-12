# End-to-end verification

Use end-to-end and manual testing only. No unit/model/mock tests, doctests, source assertions, repository self-scanners or custom test runner. Standard formatting, Clippy and release builds remain required.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
python3 tests/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
python3 tests/release-material.py --workdir target/release-material
```

Cargo E2E exercises the public CLI with real subprocesses, Git and generated data. release-interop.py uses independent OpenSSL signatures. Linux is required for installed behavior; Windows compilation alone does not qualify a desktop.

| Workflow | Coverage |
|---|---|
| test-agents.yml | Real agent launcher/runtime/profile behavior with generated profiles |
| test-ghcr-update.yml | Direct signed registry v2 enrollment/check/stage, A/B/rollback, identity recovery and critical refusals; implemented, native run pending |
| test-signed-update.yml | Signed bootc update, negative authority/replay cases and retained rollback |
| test-desktop.yml | Native graphical session, doctor, agent packaging and home recovery |
| test-home-transition.yml | Actual signed A/B/A home-baseline acceptance and rollback |
| test-rpm-refresh.yml | Native signed-RPM change/no-change and failure cases |

VM helpers live under vm/. OS image tests run in Actions. Local installer construction uses installer/build-local.py; optional --smoke checks offline verification and diskless Anaconda startup without uploading media. Manual exact-media installation must select one generated disk, preserve another sentinel disk, enable encryption, create an administrator, boot without ISO, check exact digest/policy and desktop health, then compare the untouched disk after shutdown.

Production refresh compares resolved inputs against the signed stable OCI image and requires the candidate RPM inventory to match its preflight. Native RPM fixtures exercise equality, changes and failure boundaries. No-change publishes nothing. Historical release-file interoperability remains only where needed for offline/legacy compatibility. The retired filesystem observer and GitHub ISO workflow are not production dependencies.

`test-ghcr-update.yml` uses a disposable local TLS registry mapped to the fixed GHCR hostname, generated keys and actual signed A/B VM boots. It now invokes `identity-recovery.py` to prove that identity mismatch blocks forward work while retained rollback remains available. The workflow is implemented but native execution is not-run; historical legacy-protocol VM successes do not qualify it.

Deterministic tag-race, interrupted-helper/bootc operation, legacy-state migration and native OCI-platform-mismatch cases remain explicitly not-run. The wrong-architecture identity case does not substitute for an actual wrong-platform OCI image. Shared bootc compatibility changes require native qualification before signing, even when compiler and image-input checks pass.

Keep generated logs, screenshots and results in Actions artifacts or disposable output directories; do not commit research output. Never use real homes, vault material, transcripts or production signing keys as fixtures. Historical evidence remains in Git history and [STATUS](../docs/STATUS.md).
