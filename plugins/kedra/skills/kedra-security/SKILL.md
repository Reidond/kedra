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

The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md); a legacy bridge is not a delivery requirement. Retained compatibility code still refuses unreconciled legacy operations. This scope correction never authorizes clearing journals, rollback holds or high-water state, or silently repairing unknown schemas.

Retain local TTY/boot-menu recovery without GitHub, an AI subscription or the Bitwarden GUI. Distinguish staged/booted/healthy. Automatic health rollback requires separate qualification. Keep tests disposable and production private keys out of fixtures.
