# Build layout

OS image builds run in GitHub Actions; on-demand ISO construction runs locally from a reviewed signed image and never uploads. `Containerfile` uses a generated context: compiled CLI/helper, resolved payload, private agent binaries, Bitwarden and `assemble.sh`. Shared files are overlaid by explicit target files; live home is never overwritten during image assembly.

Each enabled target builds natively on its own runner from `output/<target>-context`: `desktop` (x86_64, OCI `amd64`) on `ubuntu-24.04` and `utm` (aarch64, OCI `arm64`) on `ubuntu-24.04-arm`. The closed target table lives in `release/material.py` (Python) and `crates/sysroot-core/targets.rs` (Rust); any other target/architecture pair is refused.

`build/inputs.json` (schema 2) constrains the official Fedora base tag and pins one bootc-image-builder platform manifest per OCI architecture under `.platforms.<amd64|arm64>.builder`. Production and test runs resolve the base index to the exact no-variant platform for their architecture and record it before building; retired upstream digests are not reused blindly. `agents/inputs.json`, `bitwarden/inputs.json` and `installer/inputs.json` pin other external inputs. Keep hashes and provenance; package repositories can change independently of source.

Builder provenance (observed 2026-09-25 from `quay.io/centos-bootc/bootc-image-builder:latest`, index `sha256:2b52843e…2f0b`): the amd64 pin `afeffdb5…` and the arm64/v8 pin `a4779fc2…` are sibling platform manifests of the same upstream revision `a686afed6dde14fa5444a3d3be0f269acc783470` (both created 2026-06-18). Both manifest hashes and config labels were checked against the registry. The builder only creates disposable VM test disks, never owner install media.

See [release operations](release/README.md), [installer internals](../installer/README.md) and [end-to-end checks](../tests/README.md). Do not run OS assembly on the workstation.
