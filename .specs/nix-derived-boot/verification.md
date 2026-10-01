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

## First actual current-desktop attempt

Pinned current sourcefcfae2b composes20 files in76.23s, native planning18.09s and
eight public refusal cases pass. Existing retained runtime checks pass.
Composition0f584dba00438433b4d600268b1e0876ded7ed84b4f8b2e0783255263343dc46;
planned native identity7598286845b52a928bc273a1fb2707b19d002876f4955cb5dbafe4d1a305bea8.

Actual generation completes but admission fails305.03s: enabled greetd is solely
an Alias=display-manager.service link, while receipt collection/validation require
a link named greetd.service. Actual image shows all three selected services
enabled; alias omission is a native model/collector defect, not a failed systemd
operation. Keep greetd declared and repair precise alias semantics before retry.
Owned failed journal/tag/image remains diagnostic; diagnostic container removed.
No disk/VM was created and no boot/security/desktop pass is claimed.

Safe evidence: `target/nix-delivery/derived-boot/attempt1-summary.json` and
`native-enable-diagnostic.json`. Frozen source/binary hashes stayed unchanged.
Default state bytes are unchanged, but its recorded QEMU/TPM PIDs are absent and
initial liveness was not captured. Therefore preserved running-state is not
asserted; this qualification invoked no VM commands. Capture actual process
identity/liveness before the next named-VM attempt.

Correction source gates pass: selected-target aliases are collected/validated
only as valid top-level same-type links to declared enabled units; nested
dependency links retain matching names, removals require selected disable targets,
and default.target cannot stand in for enable evidence. Read-only Astra review
found no further concrete defect. Full workspace formatting/Clippy/release build
and extracted-driver Ruff pass; runtime requalification is pending. The fixed
observer additionally retains bounded root AVC observations for actual review.
