# Kedra

**Your Linux desktop, maintained from Git.**

Kedra is a personal Fedora 44 bootc system. `sysroot` is its management command:
review changes, publish source, consume a signed CI-built image, and preserve a
writable home. One Rust/Cargo monorepo holds shared configuration, machine
overrides, deterministic tooling, and repository-local knowledge for Codex and
Claude Code. There is no BlueBuild or generic distribution framework.

## Current status

This is a **research-ready bootstrap**, not an installable OS. The workspace has
explicit `main.rs` / `lib.rs` paths, no first-party `src/` directories, one lockfile,
and a pinned Rust toolchain. The CLI implements help, version, and honest
bootstrap status. Deployment, home mutation, setup, and agent launch commands
return an unavailable error. The helper performs no privileged operation.

The check workflow validates Rust only. No signed image, ISO,
Bitwarden login, agent session, or hardware qualification is implied by a green
bootstrap check. See [research status](docs/research/status.json) and the
[Actions runs](https://github.com/Reidond/kedra/actions).

## Continue in Codex or Claude

```bash
git clone https://github.com/Reidond/kedra.git
cd kedra
cargo test --workspace --locked
cargo run --locked -p sysroot -- status --json
```

Skills and supporting files are checked in as ordinary files. No submodule
initialization, symbolic links, or Windows Developer Mode is needed.

If Cargo reports that Rust 1.97.0 cannot build a package requiring 1.98, run
`rustup show active-toolchain`. The repository pins 1.98.1, but an environment
or directory override can select an older toolchain. Install the pin explicitly
if it is missing, then select it for the check:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo +1.98.1 test --workspace --locked
```

The command-line selection takes precedence over `RUSTUP_TOOLCHAIN` and directory
overrides without changing your global default. See
[Rustup's override precedence](https://rust-lang.github.io/rustup/overrides.html).
In PowerShell, remove the override for the current shell with
`Remove-Item Env:RUSTUP_TOOLCHAIN -ErrorAction SilentlyContinue`, then check
`rustup show active-toolchain`. If it selects 1.98.1, plain `cargo test --workspace --locked`
works too. A shell prompt's version label does not establish Cargo's selection.

Start with [AGENTS.md](AGENTS.md), [the handoff](docs/HANDOFF.md), and the
[kedra-context skill](plugins/kedra/skills/kedra-context/SKILL.md). Claude reads the same
instructions through `CLAUDE.md`. Use your independently installed coding CLI
now; the future `sysroot codex` / `sysroot claude` launchers are not implemented.

## Design and knowledge

| Entry | Purpose |
|---|---|
| [PLAN.md](PLAN.md) | Agreed architecture, ownership boundaries, milestones, and acceptance scenario. |
| [RESEARCH.md](RESEARCH.md) | R01-R11 experiments, negative tests, and evidence gates. |
| [Session decisions](docs/SESSION.md) | Why the design changed and which earlier suggestions were rejected. |
| [Skills](plugins/kedra/skills/README.md) | Task-sized tooling knowledge, procedures, failure modes, and source references. |
| [Rust decision](docs/adr/0001-rust-workspace-and-skills.md) | Cargo package layout and pinned upstream Rust skills. |
| [Sources](docs/SOURCES.md) | Primary documentation and source pins; not integration-test evidence. |

The [Kedra plugin](plugins/kedra/README.md) supports Codex and Claude Code with
one shared `plugins/kedra/skills/` tree: twelve Kedra skills and eighteen selected
Rust skills, including support files. Edit these files directly. There are no
submodules, symlinks, generated discovery copies, or custom Cargo check commands.
The plugin's [NOTICE.md](plugins/kedra/third-party/rust-skills/NOTICE.md) records upstream provenance; see
[ADR 0003](docs/adr/0003-agent-plugin.md).

For Claude, launch from this checkout with `claude --plugin-dir ./plugins/kedra`.
For Codex, open this checkout and select `kedra` from the repository marketplace
`kedra-local` in the Plugins UI. See the plugin README for loading and update details.
Plugin files are prepared in the repository; personal profiles are unchanged.

## Target model

`desktop` is the first intended real machine. `xps` is a disabled future target,
not a claim about any specific Dell model. Common `etc/`, `usr/`, and `home/`
inputs are followed by explicit `hosts/<target>/` overrides. Local dotfiles,
credentials, personal agent installations, and uncommitted changes do not
synchronize automatically between machines.

This repository is public. Do not add private configuration, credentials,
vault exports, OAuth files, real home snapshots, or machine serial numbers.
