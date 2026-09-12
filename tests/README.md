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
| test-signed-update.yml | Signed bootc update, negative authority/replay cases and retained rollback |
| test-desktop.yml | Native graphical session, doctor, agent packaging and home recovery |
| test-home-transition.yml | Actual signed A/B/A home-baseline acceptance and rollback |
| test-installer.yml | Separate installer construction, offline payload verification and diskless startup |
| test-rpm-refresh.yml | Native signed-RPM change/no-change and failure cases |

VM helpers live under vm/. OS/ISO builds run only in Actions. Manual exact-media installation must select one generated disk, preserve another sentinel disk, enable encryption, create an administrator, boot without ISO, check exact digest/policy and desktop health, then compare the untouched disk after shutdown.

Production refresh validates complete resolved inputs against authenticated release material and requires the actual candidate RPM inventory to match its preflight. The signed-material CLI and native RPM fixtures exercise equality, changes and authentication/failure boundaries. The older experimental full-filesystem observer was superseded and removed; its historical failure is not recorded as a passing production check.

Keep generated logs, screenshots and results in Actions artifacts or disposable output directories; do not commit research output. Never use real homes, vault material, transcripts or production signing keys as fixtures. Historical evidence remains in Git history and [STATUS](../docs/STATUS.md).
