# Build layout

OS image builds run in GitHub Actions; on-demand ISO construction runs locally from a reviewed signed image and never uploads. `Containerfile` uses a generated context: compiled CLI/helper, resolved payload, private agent binaries, Bitwarden and `assemble.sh`. Shared files are overlaid by explicit target files; live home is never overwritten during image assembly.

`build/inputs.json` constrains the official Fedora base tag/architecture and pins the VM builder. Production and test runs record an exact immutable base resolution before building; retired upstream digests are not reused blindly. `agents/inputs.json`, `bitwarden/inputs.json` and `installer/inputs.json` pin other external inputs. Keep hashes and provenance; package repositories can change independently of source.

See [release operations](release/README.md), [installer internals](../installer/README.md) and [end-to-end checks](../tests/README.md). Do not run OS assembly on the workstation.
