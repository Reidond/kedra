# Kedra

**Your Linux desktop, maintained from Git.**

Kedra is a personal Fedora 44 bootc system. `sysroot` is its management command:
review changes, publish source, consume a signed CI-built image, and preserve a
writable home. One Rust/Cargo monorepo holds shared configuration, machine
overrides, deterministic tooling, and repository-local knowledge for Codex and
Claude Code. There is no BlueBuild or generic distribution framework.

## Current status

The first signed owner installer is published as
[desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1).
Exact encrypted installation, desktop health and enrollment against the public
channel pass in a disposable VM. Broader lifecycle implementation continues.
Start with the
[installation guide](docs/INSTALL.md), [current worklog](worklog.md) and
[exact research evidence](docs/research/status.json).

The implemented CLI plans/archives committed source, verifies signed releases and
channel bundles, reconstructs verified ISOs, and manages installed signed
enrollment/staging/rollback through an independently verifying root helper.
It also provides native Noctalia and niri review, selected publication, local-only
choices, discard/recovery and accepted-image baseline transitions. Real home
files remain writable. `sysroot doctor` inspects desktop health.

Actual VM evidence includes fresh encrypted installation with an unselected disk
preserved, ISO-free owner login, enforcing SELinux, graphical niri/Noctalia,
signed A/B/A transitions and retained live/selected/local home decisions.
The private Codex runtime and logged-out Bitwarden work in the qualified desktop;
owner account authentication and physical hardware are separate gates.

The owner-approved public release authority and protected GitHub signing
environment are provisioned, and the owner confirmed Bitwarden backup/retrieval.
Release 1 passed exact production-media installation and verified publication;
the next candidate is qualifying expanded signed assets and the repaired publisher.
Wider home groups, no-change refresh,
expired-channel recovery, rotation and independent targets remain unfinished.

The flat Rust workspace uses explicit main.rs/lib.rs paths, a single lockfile and
pinned toolchain. Checks use standard formatting/Clippy/builds and actual CLI or
VM workflows. The owner's policy excludes unit/model/mock tests, doctests and
repository self-scanners. See [AGENTS.md](AGENTS.md) and
[Actions](https://github.com/Reidond/kedra/actions).

Inspect a target without changing the checkout or machine:

```sh
cargo run --locked -p sysroot -- source plan --host desktop
cargo run --locked -p sysroot -- source plan --host desktop --json
```

The plan reads committed HEAD only. It lists package intent, source paths,
content hashes and host overrides; staged, unstaged and untracked edits stay
untouched and are excluded. It refuses the disabled XPS target. This is build
input inspection, not an installation command. See
[ADR 0005](docs/adr/0005-committed-source-planning.md).

`sysroot source archive --host desktop --output payload.tar` writes a new,
deterministic build input archive from the same committed snapshot and refuses
an existing output. The plain Containerfile consumes that archive in Actions.
The desktop package/configuration candidate is tracked in
[R07](docs/research/R07-desktop/REPORT.md), including exact runtime qualification.

`sysroot release verify` checks signed release records and optional installer
checksums offline. Its scope and key requirements are described in
[release verification](docs/RELEASES.md), with exact release 1 download names and
the independently confirmed public-key fingerprint.

The [R02 report](docs/research/R02-installer/REPORT.md) records exact owner ISO
installation; [R01](docs/research/R01-signatures/REPORT.md) records real signed
update/rollback. [R08](docs/research/R08-release-protocol/REPORT.md) records owner
publication and first public-channel enrollment. Each report retains the scope
of earlier research results separately.

## Continue in Codex or Claude

```bash
git clone https://github.com/Reidond/kedra.git
cd kedra
cargo test --workspace --test 'e2e_*' --locked
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
cargo +1.98.1 test --workspace --test 'e2e_*' --locked
```

The command-line selection takes precedence over `RUSTUP_TOOLCHAIN` and directory
overrides without changing your global default. See
[Rustup's override precedence](https://rust-lang.github.io/rustup/overrides.html).
In PowerShell, remove the override for the current shell with
`Remove-Item Env:RUSTUP_TOOLCHAIN -ErrorAction SilentlyContinue`, then check
`rustup show active-toolchain`. If it selects 1.98.1, plain `cargo test --workspace --test 'e2e_*' --locked`
works too. A shell prompt's version label does not establish Cargo's selection.

Start with [AGENTS.md](AGENTS.md), [the handoff](docs/HANDOFF.md), and the
[kedra-context skill](plugins/kedra/skills/kedra-context/SKILL.md). Claude reads the same
instructions through `CLAUDE.md`. On a qualified installed desktop, `sysroot codex`
opens the verified checkout with its private bundled runtime. Personal coding CLIs
remain independent. The Claude launcher is implemented but its runtime is not
packaged pending the owner's preinstallation terms choice.

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
Codex 0.153.4 did not automatically discover this repository plugin in the tested
environment; read canonical skill files directly when it is unavailable. See
[discovery evidence](docs/research/R11-rust-workspace/discovery-20260907.md) and the
plugin README for loading and update details.
Plugin files are prepared in the repository; personal profiles are unchanged.

## Target model

`desktop` is the first intended real machine. `xps` is a disabled future target,
not a claim about any specific Dell model. Common `etc/`, `usr/`, and `home/`
inputs are followed by explicit `hosts/<target>/` overrides. Local dotfiles,
credentials, personal agent installations, and uncommitted changes do not
synchronize automatically between machines.

This repository is public. Do not add private configuration, credentials,
vault exports, OAuth files, real home snapshots, or machine serial numbers.
