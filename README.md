# Kedra

Kedra is a Fedora 44 bootc desktop with niri, Noctalia and the `sysroot` management command. GitHub builds and publishes separately signed container images per target: **ghcr.io/reidond/kedra-desktop** (x86_64) and **ghcr.io/reidond/kedra-utm** (aarch64, a UTM virtual machine on an Apple Silicon Mac). Each target's `stable` tag discovers its approved image; installed updates verify and stage its exact digest. UEFI Secure Boot is required.

[Install locally](usr/src/kedra/docs/INSTALL.md) · [Update and recover](usr/src/kedra/docs/UPDATES.md) · [Signed images](usr/src/kedra/docs/RELEASES.md)

Installation media is built locally when needed. No ISO, GitHub Release or release-asset publication is part of the current workflow. [Verified status](usr/src/kedra/docs/STATUS.md) distinguishes implemented behavior from tested and published images.

## Change the system

The repository root is the image's Linux filesystem. Edit the root `etc/` and `usr/` trees (`etc/skel/` is the home baseline) and the package lists `usr/src/kedra/image/packages.list` and `usr/src/kedra/image/targets/<target>/packages.list` (`desktop` or `utm`). Explicit target files under `usr/src/kedra/image/targets/<target>/` override shared files. `usr/src/kedra/` is the development tree and never enters the image. Unknown XPS hardware remains disabled.

Actions checks packages at **00:00 UTC**, with optional on-demand runs. A changed image passes build validation, isolated automatic OCI signing and strict verification before `stable` advances. No approval or manual signing step is required. No-change does nothing: it creates no image, metadata release or renewal. Runner queues affect delivery time; installed machines never reboot automatically.

Home files stay writable. Review selected [Noctalia settings](usr/src/kedra/docs/HOME-REVIEW.md) and [niri changes](usr/src/kedra/docs/TEXT-REVIEW.md) independently of local edits. [Private agent launchers](usr/src/kedra/docs/AGENT-LAUNCHERS.md) preserve personal runtimes, profiles, MCP, skills and credentials.

## Develop

Read [AGENTS.md](AGENTS.md), [architecture](usr/src/kedra/docs/ARCHITECTURE.md) and [worklog](worklog.md). Rust uses a pinned workspace, one lockfile and explicit flat entry paths.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --test 'e2e_*' --locked
cargo build --workspace --release --locked
```

[Tests](usr/src/kedra/tests/README.md) use real CLI/process and disposable VM workflows. [Repository skills](usr/src/kedra/plugins/kedra/README.md) and the [pinned Rust skills](usr/src/kedra/plugins/rust-skills/NOTICE.md) remain checkout-local; [third-party notices](usr/src/kedra/THIRD_PARTY.md) remain intact.
