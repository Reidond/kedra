# Kedra

Kedra is a Fedora 44 bootc desktop with niri, Noctalia and the `sysroot` management command. GitHub builds and publishes separately signed container images per target: **ghcr.io/reidond/kedra-desktop** (x86_64) and **ghcr.io/reidond/kedra-utm** (aarch64, a UTM virtual machine on an Apple Silicon Mac). Each target's `stable` tag discovers its approved image; installed updates verify and stage its exact digest. UEFI Secure Boot is required.

[Install locally](docs/INSTALL.md) · [Update and recover](docs/UPDATES.md) · [Signed images](docs/RELEASES.md)

Installation media is built locally when needed. No ISO, GitHub Release or release-asset publication is part of the current workflow. [Verified status](docs/STATUS.md) distinguishes implemented behavior from tested and published images.

## Change the system

Edit `packages/common.list`, `hosts/<target>/packages.list` (`desktop` or `utm`) and the Linux-shaped `etc/`, `usr/`, `home/` trees. Explicit host files override shared files. Unknown XPS hardware remains disabled.

Actions checks packages at **00:00 UTC**, with optional on-demand runs. A changed image passes build validation, isolated automatic OCI signing and strict verification before `stable` advances. No approval or manual signing step is required. No-change does nothing: it creates no image, metadata release or renewal. Runner queues affect delivery time; installed machines never reboot automatically.

Home files stay writable. Review selected [Noctalia settings](docs/HOME-REVIEW.md) and [niri changes](docs/TEXT-REVIEW.md) independently of local edits. [Private agent launchers](docs/AGENT-LAUNCHERS.md) preserve personal runtimes, profiles, MCP, skills and credentials.

## Develop

Read [AGENTS.md](AGENTS.md), [architecture](docs/ARCHITECTURE.md) and [worklog](worklog.md). Rust uses a pinned workspace, one lockfile and explicit flat entry paths.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
```

[Tests](tests/README.md) use real CLI/process and disposable VM workflows. [Repository skills](plugins/kedra/README.md) remain checkout-local; [third-party notices](THIRD_PARTY.md) remain intact.
