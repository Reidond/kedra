# Kedra plugin for Codex and Claude Code

One shared `skills/` directory contains twelve Kedra skills and eighteen selected
Rust skills, with supporting files. Codex reads `.codex-plugin/plugin.json`;
Claude reads `.claude-plugin/plugin.json`. Everything is an ordinary file.

Use this plugin while working in a Kedra checkout. Start with the checkout's
AGENTS.md, worklog.md and docs/STATUS.md, then the `kedra-context` skill.
Repository paths in skills refer to that working checkout, not an installed
plugin cache. The sysroot agent launchers open the checkout without provisioning
these skills globally; see docs/AGENT-LAUNCHERS.md for implemented runtime scope.

## Codex

The repository marketplace is `.agents/plugins/marketplace.json`, named
`kedra-local`. On 2026-09-07, Codex CLI 0.153.4 marketplace queries and fresh-profile
skills/list probes found no Kedra skills at root or crate cwd. The current app
session did not advertise them either. Read canonical skill files explicitly when
the plugin is unavailable; a restart is not a verified remedy. See
[discovery evidence](https://github.com/Reidond/kedra/blob/main/docs/STATUS.md).
These repository files have not installed the plugin or changed personal config.

Codex CLI supports explicitly registering a marketplace with
`codex plugin marketplace add .` and installing with
`codex plugin add kedra@kedra-local`. These persist installation/config outside
the checkout: they are user opt-in, not automatic agent startup steps.

## Claude Code

From the Kedra checkout, load it for the current session:

```sh
claude --plugin-dir ./plugins/kedra
```

Use `/kedra:kedra-context` to load the context skill. The root
`.claude-plugin/marketplace.json` also supports explicitly chosen marketplace
installation. Session loading avoids a persistent user installation.

## Editing

Edit `plugins/kedra/skills/<name>/` directly. Both manifests use the same tree;
there is no generator, sync command, symlink setup or Cargo task runner.
Update both manifest versions together when publishing a plugin update.
Reload Claude plugins or start a new session; refresh an installed Codex plugin
through its normal update/reinstall flow and start a new thread.

The upstream notice records the source revision and selected skills.
[Upstream notices](third-party/rust-skills/NOTICE.md) retain attribution and the
missing LICENSE-file finding. No hooks, MCP servers or programs are bundled.
Optional upstream references are not permission to install tooling.

Sources: [Codex packaging](https://developers.openai.com/plugins/build/plugins),
[Claude plugin reference](https://code.claude.com/docs/en/plugins-reference).
