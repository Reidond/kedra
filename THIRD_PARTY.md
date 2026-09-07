# Third-party source and distribution boundaries

`vendor/rust-skills` is a Git submodule referencing actionbook/rust-skills at the
revision in skills.lock.toml. The upstream tree and its notices are retained.
Repository-local links expose skills as instructions; no hook/plugin/MCP setup is
executed. No blanket license conclusion or distribution approval is implied by
this reference. Review upstream licensing/notices before copying its contents
into OS releases, and record findings in R11.

Bundling Codex, Claude Code, Bitwarden and other non-Fedora binaries requires
versioned source/authenticity and distribution review in R05/R06/R08. Keep the
official CLIs unmodified and preserve supported sign-in. A public download URL,
a private registry or an existing subscription is not proof of redistribution
permission. No such binaries are included in this bootstrap.

The owner has not selected a first-party distribution license in this bootstrap.
Do not invent one on the owner's behalf as part of an unrelated implementation.
