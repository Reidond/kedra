# R05 launch and coexistence matrix

| Runtime | Profile | Checkout | Update owner |
|---|---|---|---|
| Bundled | Dedicated management | Verified Kedra source | Signed OS release |
| User | Separate management | Verified Kedra source | User installer |
| User | Personal (explicit) | Verified Kedra source | User installer |
| Plain codex/claude | Personal normal behavior | Current directory | User installer |

For each case record actual executable/version, discovery roots, credential-store
identity, MCP/hooks invoked, files written and inherited environment. Do not put
credential values or full transcripts in reports. Repeat with empty profiles,
synthetic conflicting personal skills, moved/dirty/offline checkout, nested crate
working directory, concurrent sessions and missing executable.

Test a newer personal runtime while the bundled runtime stays old, then upgrade
and roll back the image. Personal installation remains unchanged. Management
config isolation does not automatically isolate OS keyring entries. Changing
HOME may hide the user's SSH socket and violate expectations; do not do it.

The wrapper must distinguish its flags from upstream flags and forward prompt
arguments without shell interpolation. Preserve exit code/signals and interaction.
Opening an agent grants no new publish/deploy/reboot permission. A user task can
authorize those actions, subject to the deterministic helper and vault prompts.

An agent must not depend on its process surviving reboot. Persist approved workflow
identity/digest and let installed non-AI checks finalize status. On restart inspect
state, not guess whether a previous push, staging operation or reboot completed.
