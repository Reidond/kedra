---
name: kedra-security
description: Review helper authority, secret capture, filesystem races, persistent journals and offline recovery.
---

# Installed trust

Read docs/ARCHITECTURE.md. Agents and home operations run as the ordinary user. The CLI requests administrator authorization for a fixed image-owned helper; no setuid bit or passwordless writable-script wrapper. The helper independently verifies fixed trust, scope, image identity and retained ordering; a valid signature is not proof of latest-upstream package freshness.

Use typed bounded requests, explicit process arguments and trusted executable/config paths. Reject arbitrary image references, claimed --verified flags, option/shell injection and inherited PATH/config substitution. User review stores and checkout code never establish root authority.

Adopt only safe paths. Defend traversal, symlink/hardlink escapes, special files, replacement races and mode/label changes. One canonicalize or hash check is not complete TOCTOU protection. Coordinate application writers and durable recovery; several atomic renames are not a global transaction.

The verified image cache (`/var/lib/sysroot/verified-oci`; see kedra-release-signing) is validated before every copy, because skopeo follows symlinks when it writes `index.json` and manifests with `os.WriteFile`. Any of these makes the helper remove the whole layout without following links, then download again:
- symlinks, unknown entries or temporary names other than containers/image `oci-put-blob<digits>`;
- files that are not root-owned, are group- or other-writable, or have multiple links;
- metadata that cannot be parsed or is unexpected.

Pruning replaces `index.json` atomically before deleting blobs. Stale refs go before every copy, so a failed transfer (for example ENOSPC) cannot keep them; unreferenced blobs go only after a successful copy, so a retry reuses completed layers. Only the helper writes the cache, while holding the management lock; an interrupted helper's surviving skopeo process inherits that lock.

Exclude agent auth, keyrings, vault contents, private SSH keys, tokens, transcripts and caches before capture. Deleting a secret after it entered Git is insufficient. Mixed secret/config files need a safe projection or remain unmanaged. Never COPY the entire checkout into the image.

Rollback shares persistent data. Version protocols/journals and refuse unknown or corrupt records. Test stale locks/CAS, interruption, full disk, concurrent operations and older readers with real CLI/VM flows. Bundled SQLite follows Cargo updates, independently of Fedora packages.

The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md); a legacy bridge is not a delivery requirement. Retained compatibility code still refuses unreconciled legacy operations. This scope correction never authorizes clearing journals, rollback holds or high-water state, or silently repairing unknown schemas.

UEFI Secure Boot (owner decision, 2026-09-25) is required at install time and by doctor. It is never a boot-time hard stop, so local recovery stays available.

Scope limits:
- It covers firmware → shim → GRUB → kernel, plus kernel lockdown. The initramfs, kernel command line and composefs root are not signed.
- On QEMU `virt`/Arm there is no SMM, so a malicious guest kernel could alter the variable store.
- The UTM host controls `Data/tpmdata` and the VM.

The efivar reader (`crates/sysroot-helper/firmware.rs`) makes bounded reads, requires exactly 5 bytes (4 attribute bytes and the value) and requires the efivarfs magic. SecureBoot must be 1 and SetupMode 0. Ordinary users can read efivarfs variables and `/sys/kernel/security/lockdown`: both are 0644 in kernel v7.0 source, and an unprivileged doctor passed in the aarch64 TCG rehearsal.

Guest-agent exposure: Fedora 44 qemu-ga filters nothing by default. The utm drop-in's `--block-rpcs` is defense in depth. It is a block list, so RPCs added later by qemu-ga stay enabled.

Serial-console exposure (2026-09-25): the utm PL011 carries the UEFI firmware console, GRUB (one-second menu, no superuser; `fedora-bootc:44` `grub-static-pre.cfg` and `01_users.cfg`) and a login prompt. Whoever reaches it has physical-console access: firmware setup can disable Secure Boot, and the unsigned command line accepts `systemd.debug_shell=ttyAMA0` for a root shell after unlock. UTM's `TcpServer` mode listens unauthenticated on 127.0.0.1 (UTM v5.0.6 `serialArguments`), reachable by other local accounts, network-capable apps and containers without Automation consent. `installer/utm/kedra-utm.py create` therefore defaults to UTM's built-in terminal and adds TCP only with an explicit `--serial-port`. A GRUB password would be a separate owner decision.

Retain local TTY/boot-menu recovery without GitHub, an AI subscription or the Bitwarden GUI. Distinguish staged/booted/healthy. Automatic health rollback requires separate qualification. Keep tests disposable and production private keys out of fixtures.
