# Kedra

Kedra is a Fedora 44 bootc desktop with niri, Noctalia and the `sysroot` management command. Install a signed ISO, then update the system from signed container images. Home files stay writable; selected Noctalia settings and niri changes can be reviewed independently of local edits.

[Install Kedra](docs/INSTALL.md) · [Update and recover](docs/UPDATES.md) · [Verify releases](docs/RELEASES.md)

The published installer is [desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1). It passed encrypted installation and desktop boot in a two-disk VM. [Verified status](docs/STATUS.md) distinguishes shipped and implemented behavior. Physical hardware and Secure Boot are not qualified.

## Change the system

Edit `packages/common.list` for shared packages, `hosts/desktop/packages.list` for desktop additions, and the Linux-shaped `etc/`, `usr/`, `home/` trees for configuration. Host files override shared files explicitly. `hosts/xps` remains disabled until its hardware is known.

GitHub Actions builds all OS images and installers. Release refresh is scheduled for **00:00 UTC** and can be dispatched manually. Package changes produce a candidate; signing and promotion retain protected review and exact-media qualification. A scheduled trigger is not an immediate installation or reboot. See [release operations](build/release/README.md).

`sysroot codex` uses a private bundled runtime. Personal agents, settings, MCP, skills and credentials remain independently owned. See [agent launchers](docs/AGENT-LAUNCHERS.md), [Noctalia review](docs/HOME-REVIEW.md) and [niri review](docs/TEXT-REVIEW.md).

## Develop

Read [AGENTS.md](AGENTS.md), [architecture](docs/ARCHITECTURE.md) and the current status in [worklog.md](worklog.md). Rust uses one workspace, lockfile and pinned toolchain, with explicit flat entry paths.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
python3 tests/release-interop.py --sysroot target/release/sysroot --workdir target/release-interop
```

[Tests](tests/README.md) exercise real CLI and disposable VM workflows. [Repository skills](plugins/kedra/README.md) are checkout-local development knowledge; they are not installed into the OS or personal profiles. Third-party distribution details are in [THIRD_PARTY.md](THIRD_PARTY.md).
