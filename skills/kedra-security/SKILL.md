---
name: kedra-security
description: Review sysroot privilege, secret capture, untrusted release inputs, filesystem races, durable journals, old-version rollback compatibility and offline recovery.
---

# Trust is enforced by deterministic installed code

Agents run as the ordinary user. A future root-owned image-installed helper
accepts a narrow structured protocol and verifies eligibility independently.
It does not execute scripts, hooks, parsers or arbitrary commands from the writable
checkout. No passwordless root wrapper around a user-writable script. The current
helper refuses all operations and is not installed/setuid/sudo-authorized.

## Input and privilege review

Validate target/repository/digest/architecture, release metadata and authorization.
Reject arbitrary image references, paths, --verified claims, option injection,
user-controlled PATH, shell fragments and inherited environment substitutions.
Use explicit arguments and trusted executable/config paths. User-owned home review
state must never authorize root deployment. Same-user profile directories are
not isolation from an unrestricted same-user agent; respect sandbox/permission
controls and keep narrow credentials rather than claiming magic separation.

For home paths, allowlist adoption and defend against traversal, symlink/hardlink
escapes, special files, replacement races and permission/mode/label changes.
Canonicalizing once is not a complete TOCTOU defense. Coordinate live writers;
after-check writes and stale in-memory saves can invalidate a candidate. Use
application groups, checkpoints and durable journal transitions, not pretend a
set of separate file renames is globally atomic.

## Privacy before storage

Never capture whole agent directories, keyrings, vault data, private SSH keys,
OAuth sessions, registry tokens, transcripts or caches. A secret in a private Git
object still exists even after deletion/ignore. Mixed secret/settings formats
require a safe projection or remain unmanaged. Restrict build contexts and never
COPY the entire checkout into an OS. This public repo contains no real home data.
Redaction is a backstop for evidence, not permission to use production secrets.

## Persistent compatibility and recovery

OS rollback shares /var and may boot older code against newer journals, databases
or app configs. Version protocols and migrations; define unknown-version failures,
recovery backups and recomputable data. Test interrupted journal writes, full disk,
missing checkpoints, stale locks/concurrent invocations and rollback after newer
personal edits. Never treat unreadable state as an empty baseline to overwrite.

Distinguish staged, rebooted and healthy. Installed post-boot code—not an AI
process that must stay alive—checks results. Keep a usable administrative TTY and
boot-menu recovery path without GitHub, an agent subscription or Bitwarden GUI.
Do not promise automatic health rollback until failure-injection evidence exists.

Gates: R10 privilege/protocol, R03/R04 path/activation, R08 signer authority,
R06 credentials. Sources: PLAN.md/RESEARCH.md and docs/SOURCES.md bootc-fs,
actions-security, bitwarden-ssh, codex-auth, claude-auth, git-faq.
