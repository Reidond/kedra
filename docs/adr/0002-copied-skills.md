# ADR 0002: Shared Codex/Claude plugin and standard Cargo

Date: 2026-09-07. The owner requested removing submodules, copying relevant
skills, then removing the custom Cargo task runner and making Codex/Claude
plugins without symlinks. Supersedes ADR 0001's xtask, submodule and discovery
link decisions. Other Rust, OS and repository-only scope choices remain.

The Windows checkout had core.symlinks=false: discovery entries were link-text
files, and read_link failed with os error 4390. Requiring symlink privileges or
maintaining synchronized discovery copies adds unnecessary setup.

Use a single ordinary-file skill tree at plugins/kedra/skills/. It contains
twelve Kedra skills and eighteen selected actionbook/rust-skills directories
with all local support files. Both native plugin manifests use that tree.
The checkout contains Codex and Claude marketplace catalogs pointing to the same
plugin. There are no discovery copies or synchronization mechanism. Edit skill
files directly; plugin installation is a separate user choice, not an incidental
repository operation.

Record upstream revision 5c40d3ad785193231b7d0dbfb8e1eb447e5edd94 and selection
in the plugin's third-party/rust-skills/NOTICE.md. Preserve upstream README/metadata declarations in the plugin's
third-party directory and record the absent LICENSE file in NOTICE.md. No hooks,
installers, MCP configuration or agent executables are copied. References to
omitted integrations do not authorize installation.

Remove the xtask package, Cargo alias, custom layout/skill validators and their
tests. CI runs Cargo fmt, Clippy, test and release build directly against three
packages. Explicit flat source paths and safe Rust remain review requirements;
we no longer claim a custom validator enforces them. Plugin authoring checks
remain independent of building the Rust programs.

See [local evidence](../research/R11-rust-workspace/plugins.md). Historical
four-package CI results remain historical; actual plugin use in both agents,
editor behavior and OS redistribution review remain separate research gates.
