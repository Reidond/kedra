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

TPM disk unlock (owner request, 2026-09-26): `sysroot setup tpm-unlock` (crates/sysroot/setup.rs) is a post-install, ordinary-user command. Preflight is unprivileged and read-only; only `/usr/bin/sudo -- /usr/bin/systemd-cryptenroll` with explicit arguments changes the LUKS2 header, with the caller's terminal inherited so systemd-cryptenroll itself prompts for the passphrase. Facts behind it, observed 2026-09-26 on `quay.io/fedora/fedora-bootc:44` 44.20260925.0 (systemd 259.9, util-linux 2.41.5, cryptsetup 2.8.8, kernel 7.2.7) and in upstream source:
- No crypttab option or karg is needed. systemd v259 `src/cryptsetup/cryptsetup.c` calls `crypt_activate_by_token_pin_ask_password(..., type NULL, ...)` (any token) whenever no key file is set and `use_token_plugins()`, before the passphrase prompt; token plugins are off if `tpm2-measure-pcr`/`fido2-cid` or `SYSTEMD_CRYPTSETUP_USE_TOKEN_MODULE=0` are set. A key file (`rd.luks.key`) skips tokens, so the preflight refuses `rd.luks.key`/`luks.key`/`rd.luks.options`/`luks.options`.
- The base initramfs is built `--no-hostonly` (`lsinitrd` Arguments) and contains `systemd-cryptsetup`, its generator, `libcryptsetup-token-systemd-tpm2.so` and `libtss2-*`, but no `etc/crypttab`. The root unlock is generated from `rd.luks.uuid=` (the generator strips a `luks-` prefix), so the preflight requires that karg; editing `/etc/crypttab` does not affect the root unlock.
- Anaconda writes `/etc/crypttab` under `umask(0o077)` (`pyanaconda/modules/storage/devicetree/fsset.py`, `FSSet.write`), so an unprivileged preflight cannot read it. The volume is identified from `lsblk --json --paths` mounts (`/`, `/sysroot`, `/var`) through the `crypt` mapping to one `crypto_LUKS` version 2 container, cross-checked against `/dev/disk/by-uuid/<uuid>` and `/proc/cmdline`.
- `drivers/char/tpm/tpm-sysfs.c`: `tpm_version_major` and `pcr-<bank>/<n>` are 0444, with uppercase hex PCR values. The preflight refuses an all-zero/all-F SHA-256 PCR 7 (firmware did not measure Secure Boot), because such a policy protects nothing.
- systemd-cryptenroll 259 (`src/cryptenroll/cryptenroll.c`) silently adds a signed PCR 11 policy if `tpm2-pcr-public-key.pem` exists in /etc, /run or /usr/lib/systemd, and a pcrlock policy if `pcrlock.json` exists in /run or /var/lib/systemd. Empty `--tpm2-public-key=` and `--tpm2-pcrlock=` disable both. An identical existing PCR policy without a PIN is kept as is ("already enrolled"; with a PIN it is re-enrolled). `--wipe-slot` runs only after a successful enrollment, excludes the new slot and never removes the last slot.
- `--replace` failure mode (v259 `cryptenroll-tpm2.c` `enroll_tpm2`, lines 459-540, checked 2026-09-26): the policy digest compared by `search_policy_hash()` comes from `tpm2_calculate_sealing_policy(PCR values, pubkey, use_pin, pcrlock)` and excludes the SRK. After a TPM clear or a replaced UTM `Data/tpmdata` with the same PCR 7, cryptenroll seals under the new SRK, finds the matching token, logs "This PCR set is already enrolled, executing no operation", exits 0 and keeps the old slot, which can no longer be unsealed. `sysroot` detects this (the single TPM slot afterwards was already listed before) and exits 78 telling the owner to `--remove` and then enroll. New enrollments use `CRYPT_ANY_SLOT` while old slots still exist, so a genuine re-seal always gets a slot index not listed before.
- Removal needs no TPM (v259 `cryptenroll.c` `run()`): without an enroll type, `prepare_luks(&cd, NULL)` loads the header without the volume key, then `wipe_slots()` runs. `cryptenroll-wipe.c` refuses if no slot would remain, then destroys slots one by one and continues on failure, so a failed wipe can be partial. `sysroot setup tpm-unlock --remove` therefore checks only the owner, terminal and LUKS2 volume, not Secure Boot, the TPM or the kernel command line, so revocation works after the TPM or Secure Boot is turned off.
- Slot listing (`systemd-cryptenroll <dev>`, `cryptenroll-list.c`) prints `SLOT TYPE` then `<n> <type>`, where type is password (no token), recovery, pkcs11, fido2, tpm2, other or conflict; "No slots found." goes to stderr. Reading it needs root, so `sysroot doctor` cannot report enrollment and does not try.
- Experiment (2026-09-26, disposable fedora-bootc:44 container, swtpm 0.10.2, generated 64 MiB LUKS2 file): the exact arguments enrolled a token recording PCR 7/sha256 without pubkey or pcrlock. Replace, PIN replace and remove gave the slot lists the verifier expects, a wrong passphrase changed nothing and wiping the last slot was refused. The libcryptsetup token plugin cannot reach a socket swtpm (it looks for `/sys/class/tpmrm`), so boot unlock was not exercised.
Scope: PCR 7 only, because PCRs 4/8/9 change with every image update and Kedra has no UKI for signed PCR 11 policies. The initramfs and kernel command line are unsigned and outside PCR 7, and GRUB has no password, so console access yields a root shell after TPM unlock unless `--with-pin` is used. In VMs the host's TPM state (UTM `Data/tpmdata`) yields the key regardless of PCRs or PIN. If PCR 7 changes (Secure Boot, db/dbx, shim SBAT), the passphrase prompt returns; re-seal with `--replace`. After a TPM clear or new UTM `Data/tpmdata`, PCR 7 is unchanged, so use `--remove`, then enroll again. Boot unlock on UTM and hardware is not yet qualified.

Guest-agent exposure: Fedora 44 qemu-ga filters nothing by default. The utm drop-in's `--block-rpcs` is defense in depth. It is a block list, so RPCs added later by qemu-ga stay enabled.

Serial-console exposure (2026-09-25): the utm PL011 carries the UEFI firmware console, GRUB (one-second menu, no superuser; `fedora-bootc:44` `grub-static-pre.cfg` and `01_users.cfg`) and a login prompt. Whoever reaches it has physical-console access: firmware setup can disable Secure Boot, and the unsigned command line accepts `systemd.debug_shell=ttyAMA0` for a root shell after unlock. UTM's `TcpServer` mode listens unauthenticated on 127.0.0.1 (UTM v5.0.6 `serialArguments`), reachable by other local accounts, network-capable apps and containers without Automation consent. `installer/utm/kedra-utm.py create` therefore defaults to UTM's built-in terminal and adds TCP only with an explicit `--serial-port`. A GRUB password would be a separate owner decision.

Retain local TTY/boot-menu recovery without GitHub, an AI subscription or the Bitwarden GUI. Distinguish staged/booted/healthy. Automatic health rollback requires separate qualification. Keep tests disposable and production private keys out of fixtures.
