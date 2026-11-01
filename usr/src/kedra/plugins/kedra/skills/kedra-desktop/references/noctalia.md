# Noctalia configuration and managed files

Noctalia v5 places curated configuration under `~/.config/noctalia/` and GUI
override state at `~/.local/state/noctalia/settings.toml`; overrides take
precedence. Do not mix v4 noctalia-shell/Quickshell JSON examples into a v5 setup.
Confirm CLI flags against the packaged version before automation.

Facts (2026-10-14, Fedora 44 noctalia-5.2.1-1; source: `noctalia config validate
--help` in the image, system/desktop.kedra, tests/container/home_files_tests.rs):

- `noctalia config validate <file>` validates one file; with no path it merges
  every `*.toml` in the config directory plus the state overrides.
- The running shell loads every `*.toml` in its directory, so update candidates
  and `<file>.kedra-new` conflict files must not end in `.toml`.
- Unknown enum values are warnings (exit 0), not failures; malformed TOML fails.
- After an update `kedra-noctalia.service` is restarted (`systemctl --user
  try-restart`, the declared reload).

The image manages only `config.toml` and `palettes/Adwaita.json`. The GUI
override file is never managed, captured or cleared: a user may keep a GUI change
local while the source baseline changes. A merged baseline value can be hidden by
an override; check the effective result in the running shell before claiming a
source change took effect. Never erase a newer GUI change to force Git authority.

Behavior worth covering with real sessions: conflicting curated/GUI values, a
baseline updated while an override remains, rollback with newer GUI changes, and a
Noctalia version change. The container case `native::home_files_update` covers
merge, check refusal, restart, conflict and resolution; the rest is not-run.

Source: https://docs.noctalia.dev/noctalia/configuration/
