# Derived boot qualification

Source starts from PR32/3ba7d1b; D1 implementation is frozen before runtime.
Only public CLI/manual and the existing native QEMU harness qualify behavior.

Current source gates: pass — scoped Rust formatting, full workspace formatting/
Clippy, assigned Python Ruff and build-disk Bash syntax. Read-only Astra source
review found one qualification gap: the fixed guest observer cannot identify a
circular final fixture digest itself. Disposition: for every actual cold/warm
boot the host compares its bootc image reference/digest and native provenance
with the retained image.json's exact fixture ID/imported manifest/native tuple.
The observer's passed flag alone never qualifies lineage.

| Case | State | Evidence |
|---|---|---|
| DB-T1 public refusal | not-run | Complete tuple/target/overlay/binary override cases pending. |
| DB-T2 conversion/material | not-run | Actual immutable fixture and Podman/disk conversion pending. |
| DB-T3 generated initramfs boot | not-run | Host matching reference/digest/native tuple plus guest BLS/kernel/initrd evidence pending. |
| DB-T4 security/desktop | not-run | Actual firmware/lockdown/SELinux/AVCs/Metal/screenshot pending. |
| DB-T5 restart/preservation | not-run | Named instance warm restart and owned cleanup pending. |
| DB-T6 standard gates/publication | partial | Format/Clippy/Ruff/syntax pass; source commit/release binary/E2E/PR pending. |

No new image, VM boot, installation, signature admission, production release or
update qualification is claimed by source implementation. Runtime records stay
under ignored retained artifacts, with pinned controller/script hashes.
