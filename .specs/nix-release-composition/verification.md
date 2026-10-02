# D2 verification

Source is adopted on `codex/nix-release-composition` above D1
`712927eb0180610fd1ea3e29cccd5fdc6ac16f2b`. The twenty-path preparation was
hash/mode reconciled before adoption; later-layer modules remain inactive.
Explicit local fixture support is active implementation. No runtime production
signing/main override is introduced.

Pass: adopted Python Ruff/Bash syntax/diff checks; standard public
release-material workflows for both targets, including no-change/rank/content/
scope/architecture refusals; release-interop public trust, history and sixteen
OpenSSL signature/artifact/installer workflows. Exact local fixtures:
`target/nix-delivery/d2-release-{material,interop}`.

Pass on the adopted D2 checkout above712927e: pinned Rust1.98.1 workspace
formatting, Clippy with warnings denied, ordinary CLI E2E (11 passed, six
Docker-dependent cases explicitly ignored) and workspace release build. No Rust
source was edited during those gates. Ignored cases supply no runtime coverage.

Source review caught root-owned failed installer scratch cleanup; correction
captures/rechecks its owned root, refuses all kernel mountpoints beneath it,
uses fixed narrow privileged removal and preserves failure/reporting semantics.
Actual cleanup/mount/sentinel refusal is not-run.

## Controller inputs

Read-only Docker inspection verifies cached native ARM Ubuntu24.04 image
`008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`,
Rust1.98.1 image `a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546`
and Docker image `3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0`.
Actual Docker client29.8.1 binary hash is
`55bfa076d51381e0bc28b12d0a6338b74ec5e9e0ca537df04988ea21ef48e42c`.

Pinned [uv0.12.19 release](https://github.com/astral-sh/uv/releases/tag/0.12.19)
provides the ARM GNU archive/checksum. Direct release URL returns403; official
GitHub release asset587158851 is downloaded through authenticated API and matches
SHA256 `0804e9b164c64b6914182d5920c08551958a095986f10a3731056df701126436`.
Only bounded regular ARM ELF uv/uvx files are extracted under private
`target/nix-delivery/d2-runtime-inputs`; no workstation tool installation.
Its receipt retains exact origin/hash/size. New controller execution/ABI remains
not-run; library version comparisons confer no pass.

## Remaining gates

First local invocation on deee813 fails during the real Podman package resolver:
its Netavark nftables backend cannot find `nft` in the dedicated controller.
The reviewed base was pulled and pinned agent/Bitwarden inputs were prepared;
foundation composition and installation had not started. Add the required
`nftables` package to the disposable controller and rerun from a fresh owned
fixture context. This is a retained failure, not a native-composition pass.
Terminal cleanup passes: original exit1 is retained, private scratch is removed,
no registry was created and no Podman container remains. The mount guard reports
no mounted descendants; deliberate mounted-sentinel refusal remains not-run.
Pinned executable hashes are unchanged. Safe attempt evidence is in
`target/nix-delivery/d2-runtime/attempt1-evidence`.

Before the attempt, four exact old qualification archives were retired after
full hash, receipt, owner, single-link and inactive-consumer checks. Their
manifests and evidence were retained, together with the complete canonical
foundation and D1 context/VM/media. Actual available space rose from45GiB to69GiB;
the temporary owner-write change on sealed parent directories was restored.
Receipts are in `target/nix-delivery/d2-runtime/retired-archives*.json`.

Not-run: fresh normal ARM foundation/composition/native transfer; normal
generated-authority encrypted Anaconda install; ISO-free boot; public forward
update/rollback with persistent home/data and refusals; disabled Secure Boot
installer refusal; actual cleanup/resource evidence and new source CI.
Protected-main production signing/publication/no-change repetition remain
separate from disposable fixture qualification.

D1 [PR33](https://github.com/Reidond/kedra/pull/33) both architecture workspace
checks pass on exact head712927e in
[run36941595156](https://github.com/Reidond/kedra/actions/runs/36941595156).
Earlier whole-payload workflows on8a8796c are still running; they are not
borrowed as D2/new-source results.
