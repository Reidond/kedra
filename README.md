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

The check workflow validates Rust and skill wiring only. No signed image, ISO,
Bitwarden login, agent session, or hardware qualification is implied by a green
bootstrap check. See [research status](docs/research/status.json) and the
[Actions runs](https://github.com/Reidond/kedra/actions).

## Continue in Codex or Claude

```bash
git clone --recurse-submodules https://github.com/Reidond/kedra.git
cd kedra
cargo xtask check
cargo run --locked -p sysroot -- status --json
```

For an existing checkout, initialize the exact pinned submodule with
`git submodule update --init --recursive`. Do not use `--remote`.

Start with [AGENTS.md](AGENTS.md), [the handoff](docs/HANDOFF.md), and the
[kedra-context skill](skills/kedra-context/SKILL.md). Claude reads the same
instructions through `CLAUDE.md`. Use your independently installed coding CLI
now; the future `sysroot codex` / `sysroot claude` launchers are not implemented.

## Design and knowledge

| Entry | Purpose |
|---|---|
| [PLAN.md](PLAN.md) | Agreed architecture, ownership boundaries, milestones, and acceptance scenario. |
| [RESEARCH.md](RESEARCH.md) | R01-R11 experiments, negative tests, and evidence gates. |
| [Session decisions](docs/SESSION.md) | Why the design changed and which earlier suggestions were rejected. |
| [Skills](skills/README.md) | Task-sized tooling knowledge, procedures, failure modes, and source references. |
| [Rust decision](docs/adr/0001-rust-workspace-and-skills.md) | Cargo package layout and pinned upstream Rust skills. |
| [Sources](docs/SOURCES.md) | Primary documentation and source pins; not integration-test evidence. |

First-party skills live in `skills/`. The complete upstream
[actionbook/rust-skills](https://github.com/actionbook/rust-skills) source is pinned
under `vendor/rust-skills/`. Both are exposed through per-skill links in
`.agents/skills/` and `.claude/skills/`. No global installation, plugin hooks,
automatic MCP setup, or permission changes are performed.

## Target model

`desktop` is the first intended real machine. `xps` is a disabled future target,
not a claim about any specific Dell model. Common `etc/`, `usr/`, and `home/`
inputs are followed by explicit `hosts/<target>/` overrides. Local dotfiles,
credentials, personal agent installations, and uncommitted changes do not
synchronize automatically between machines.

This repository is public. Do not add private configuration, credentials,
vault exports, OAuth files, real home snapshots, or machine serial numbers.
