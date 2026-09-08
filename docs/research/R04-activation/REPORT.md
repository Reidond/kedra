# R04: Noctalia activation and recovery

2026-09-08. Full gate: blocked. No workstation home was used.

[Linux workspace 34217852366](https://github.com/Reidond/kedra/actions/runs/34217852366)
at `4a7d02e` passes formatting, Clippy, 106 tests plus one doctest, release build
and independent OpenSSL interoperability. The new subset covers:

- Pure projected plans and comment/private-field-preserving native TOML edits.
- Pending review reservations and discard preserving other dispositions.
- Atomic related-record writes and cooperating store locks.
- Checked file prepare/exchange/no-replace, metadata retention, safe abort,
  changed-file refusal and retained late edits through an old open descriptor.
- Stale/stop-time changes, failed restart with durable reopen/resume, abort,
  explicit keep-current and resuming after validated checkpoint cleanup.
- A synthetic private value remaining in the native file while absent from the
  review SQLite files and serialized receipts.

Native lifecycle [34212238491](https://github.com/Reidond/kedra/actions/runs/34212238491)
at `3ce1c1a` passes the installed service's stop immediately after an IPC write,
persisted export and restart. Native discard run 34217852336 and signed-helper
regression 34217852250 failed before VM testing because the pinned Fedora base
manifest became unavailable (Quay HTTP 404, confirmed independently).

The official Fedora 44 tag was resolved again from registry manifest/config bytes:
index 498d7b7816044c3ef854a450c1b6873f48a870adc938c5cefa426a55a05a9bb3,
AMD64 manifest d4b9c5e156ab0a119962aad27c5394409094cc24348846a35c78acd8e9847a4d,
config 62b67de976000c36e0f38000a41cde33fa64dae0ba83106944a0b9f5dc764fdb.
It identifies Linux/AMD64, version 44.20260908.0, kernel 7.1.13-200.fc44,
created 2026-09-08T10:06:08Z. The pinned QCOW2 builder remains available with its
expected SHA-256. Requalification uses the new explicit base input; the old
successful VM evidence retains its original digest. Native discard remains pending.

The preceding workspace 34217752697 at c7ffd58 failed Clippy on a nested metadata
condition before tests. The corrected guard preserves every ownership/label check.
See [ADR 0017](../../adr/0017-noctalia-activation.md) for the state machine,
privacy boundary and recovery behavior.

Open: native mutation/read-back, actual process-kill/full-disk fault matrix,
baseline transitions across OS updates/rollback, broader application groups and
generic file/line integration. Generated filesystem tests and managed-writer
qualification do not prove a complete home manager or protection against arbitrary
same-UID/root writers. The owner installer and real-home adoption remain gated.
