# Kedra plugin for Codex and Claude Code

The `skills/` directory contains the twelve Kedra skills with supporting files.
The eighteen selected upstream Rust skills are the separate repository-local
`rust-skills` plugin in `usr/src/kedra/plugins/rust-skills/`. Codex reads
`.codex-plugin/plugin.json`; Claude reads `.claude-plugin/plugin.json`.
Everything is an ordinary file.

These skills are for developing this repository only. Start with the checkout's
AGENTS.md, worklog.md and usr/src/kedra/docs/STATUS.md, then the `kedra-context`
skill. Repository paths in skills refer to the working checkout. The skills are
not part of the OS, installer, home baseline or the sysroot agent launchers.

## Codex

The repository marketplace is `.agents/plugins/marketplace.json`, named
`kedra-local`. The checked-in `.codex/config.toml` enables `kedra@kedra-local`
and `rust-skills@kedra-local` once the project is trusted. Codex loads plugins
only from its cached copy (`$CODEX_HOME/plugins/cache/`), and its command-line
plugin commands do not discover the repository marketplace by themselves, so
register once per machine from the repository root:

```sh
codex plugin marketplace add .
codex plugin add kedra@kedra-local
codex plugin add rust-skills@kedra-local
```

The first command records this checkout's absolute path in the user's Codex
configuration; the others copy the plugins into the cache. Codex refreshes the
cache only when a plugin's `version` changes, or when `codex plugin add` runs
again. Earlier discovery evidence (Codex CLI 0.153.4, 2026-09-07) is in
[STATUS](https://github.com/Reidond/kedra/blob/main/usr/src/kedra/docs/STATUS.md).

## Claude Code

The checked-in `.claude/settings.json` declares the `kedra-local` marketplace
(`.claude-plugin/marketplace.json`) and enables `kedra@kedra-local` and
`rust-skills@kedra-local`. Once the folder is trusted, both plugins load in
place from the working tree; edits take effect in a new session or after
`/reload-plugins`. Use `/kedra:kedra-context` to load the context skill.

## Editing

Edit `usr/src/kedra/plugins/kedra/skills/<name>/` directly. Both manifests use
the same tree; there is no generator, sync command, symlink setup or Cargo task
runner. Any change to the skills bumps `version` in both manifests together, so
Codex refreshes its cached copy; then start a new session or thread.

The `rust-skills` plugin's notice records the source revision and selected
skills. [Upstream notices](../rust-skills/NOTICE.md) retain attribution and the
missing LICENSE-file finding. No hooks, MCP servers or programs are bundled.
Optional upstream references are not permission to install tooling.

Sources: [Codex packaging](https://developers.openai.com/plugins/build/plugins),
[Claude plugin reference](https://code.claude.com/docs/en/plugins-reference).
