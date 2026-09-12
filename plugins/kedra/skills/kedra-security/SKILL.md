---
name: kedra-security
description: Review helper authority, secret capture, filesystem races, persistent journals and offline recovery.
---

# Installed trust

Read docs/ARCHITECTURE.md. Agents and home operations run as the ordinary user. The CLI requests administrator authorization for a fixed image-owned helper; no setuid bit or passwordless writable-script wrapper. The helper independently verifies fixed trust, scope, image identity and retained ordering; a valid signature is not proof of latest-upstream package freshness.

Use typed bounded requests, explicit process arguments and trusted executable/config paths. Reject arbitrary image references, claimed --verified flags, option/shell injection and inherited PATH/config substitution. User review stores and checkout code never establish root authority.

Adopt only safe paths. Defend traversal, symlink/hardlink escapes, special files, replacement races and mode/label changes. One canonicalize or hash check is not complete TOCTOU protection. Coordinate application writers and durable recovery; several atomic renames are not a global transaction.

Exclude agent auth, keyrings, vault contents, private SSH keys, tokens, transcripts and caches before capture. Deleting a secret after it entered Git is insufficient. Mixed secret/config files need a safe projection or remain unmanaged. Never COPY the entire checkout into the image.

Rollback shares persistent data. Version protocols/journals and refuse unknown or corrupt records. Test stale locks/CAS, interruption, full disk, concurrent operations and older readers with real CLI/VM flows. Bundled SQLite follows Cargo updates, independently of Fedora packages.

Before an explicit v1-to-v2 bridge switch, require legacy update status to show no pending intent, awaiting-reboot operation, staged replacement or queued rollback. Reconcile while the old helper still runs. Migration preserves hold/high-water and refuses unreconciled legacy state; the new status path must not silently repair or reset an old journal.

Retain local TTY/boot-menu recovery without GitHub, an AI subscription or the Bitwarden GUI. Distinguish staged/booted/healthy. Automatic health rollback requires separate qualification. Keep tests disposable and production private keys out of fixtures.
