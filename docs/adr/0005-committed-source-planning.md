# ADR 0005: plan image inputs from an immutable Git commit

Date: 2026-09-08. Status: accepted for read-only source planning.

## Problem and decision

The package lists and Linux-shaped source roots had no consumer. An owner or
builder needs a concrete view of what a target means without reading every file
or confusing a working edit with a published build input.

`sysroot source plan --repo PATH --host TARGET [--json]` resolves HEAD once and
reads that commit's tree and raw blobs. It never stages, captures, exports, applies
or changes working files. Staged, unstaged and untracked edits are excluded and
remain available for the owner's normal review. It does not need network access.
Git filters, replacements and caller-supplied Git environment overrides cannot
alter payload bytes. Git remains the object engine; no custom Git implementation.

Common etc/usr/home files precede the selected host's same roots. The manifest
records each winning source path, overridden common source, Git blob, SHA-256,
Git file mode and destination. Home files map into an inert `default` baseline
namespace under `/usr/share/sysroot/home/`; this is a profile name, not a user or
UID. Enrollment and live-home adoption are still unavailable. No plugin tree is
an assembly input. Generated sysroot metadata and `/usr/etc` are reserved.

Disabled targets, identity mismatches, package options/URLs, conflicting package
intent, links/submodules, unsafe payload paths and file/directory collisions fail.
UTF-8 source paths and ordinary 100644/100755 Git blobs are the initial supported
subset. A plan is provenance, not a release signature or proof packages resolve.
The installed privileged helper must never trust this unprivileged plan as approval.

## Dependencies and limitations

Clap replaces manual CLI parsing, including non-Unicode filesystem arguments.
Serde/serde_json supply versioned output; TOML reads the existing target format;
SHA-2 supplies a standard content digest. Versions are locked in Cargo.lock.
There is no async runtime, new crate, language or check runner. Shared logic
returns typed errors; application reporting supplies context on stderr.

This read-only slice does not yet materialize an image filesystem, resolve RPMs,
classify mixed-secret home settings, or implement host enrollment/source export.
Real home adoption remains gated by R03/R04. Two-target lifecycle remains R09.

## Evidence

Six synthetic Git integration tests cover provenance/host override routing,
known SHA-256 content, untouched index/worktree/untracked state, disabled or
mismatched targets, package injection/conflicts, committed symlink refusal,
file/directory collisions and reserved paths. Windows Rust 1.98.1 Clippy and the
25 workspace tests plus one doctest passed during implementation; exact-revision
Linux evidence follows publication. See worklog for subsequent check results.

Primary interfaces reviewed 2026-09-08:
[git ls-tree](https://git-scm.com/docs/git-ls-tree),
[git cat-file](https://git-scm.com/docs/git-cat-file),
[Clap derive](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html),
[TOML deserialization](https://docs.rs/toml/latest/toml/fn.from_str.html).
