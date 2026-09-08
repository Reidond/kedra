# ADR 0014: Export selected Noctalia fields as Git patches and explicit receipts

Date: 2026-09-08. Status: implemented; Linux CLI subset verified.

Extend the private Noctalia review interface with patch generation and source
receipts. Keep ordinary Git review/commit/push decisions explicit. No private
snapshot repository, SQLite record or captured runtime file crosses into source.

The exporter verifies the checkout's expected origin and builds a committed-source
plan using the existing target/provenance rules. Accepted baseline and previous
receipt commits must be ancestors of the new source state. A new host override,
missing history, unsupported file mode or conflicting selected source value is an
explicit refusal. It never changes branches or treats a working file as a commit.

Use toml_edit 0.25.13 (MIT OR Apache-2.0) only in the ordinary-user CLI to preserve
comments and unselected formatting. No custom TOML/configuration language is added.
Render only the finite selected field values and independently reproject before
giving the result to Git. A private temporary index starts from the known source
commit; standard hash-object/update-index/diff create a full-index unified patch.
The real HEAD/index/worktree remain unchanged. Only the selected source blob may
be added to source's object database. New patch files use private permissions;
existing destinations are refused.

The separate record-source operation checks a full exact commit ID, its effective
target/source path, selected values and ancestry. It requires accepted and prior
receipt history, preventing an old baseline commit from pretending to be a new
revert. A receipt is a verified local source fact, not registry promotion or image
activation. Persist the transition by compare-and-swap, preserving newer live
values and local dispositions. Selection and cached status remain available without
invoking the live application.

Eight local export cases and an exact-revision source case pass: dirty source/index
retention, comments and unselected content, all-object privacy inspection, host
routing, conflicts, empty/no-change patches and receipt ordering. The Linux-only
integration test exercises the actual CLI/SQLite/Git round trip in generated
directories without enrolling the runner's home. Run 34192732940 at a2c0e63 passes
the actual CLI round trip and all 81 Linux tests plus one doctest.

Sources: [toml_edit DocumentMut](https://docs.rs/toml_edit/0.25.13+spec-1.1.0/toml_edit/struct.DocumentMut.html),
[Git diff](https://git-scm.com/docs/git-diff), [Git update-index](https://git-scm.com/docs/git-update-index),
ADRs 0004, 0005, 0007 and 0012; versions retrieved/checked 2026-09-08.
