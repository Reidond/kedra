# Noctalia configuration projection (R03/R07 research)

The v5 documentation consulted in the session places curated configuration under
~/.config/noctalia/ and GUI override state at
~/.local/state/noctalia/settings.toml. Overrides take precedence. Verify this
against the exact packaged version; do not mix v4 noctalia-shell/Quickshell JSON
examples into a v5 setup. Current docs describe `noctalia config export` and
`noctalia config validate`; confirm actual CLI flags before automation.

The desired manager view is selected effective settings from curated baseline
plus GUI overrides. It is not arbitrary runtime state capture. A user may stage
appearance, leave a layout edit uncommitted, and keep volume application-local.
The exported patch changes only the chosen source settings. A GUI override
becomes redundant only once the new baseline is active; remove it only if its
live value still matches the reviewed value. Never clear all overrides to force
Git authority or erase a newer GUI change after preflight.

Required fixtures: conflicting curated/GUI values; publishing one of several GUI
changes; a second GUI write during review/after staging; baseline updated while
override remains; rollback with newer GUI changes; unknown/secret-like fields
excluded; configuration schema/version changes. R03 owns projection semantics,
R04 safe activation, R07 actual shell behavior. Documentation is not a test pass.

Source: https://docs.noctalia.dev/noctalia/configuration/
