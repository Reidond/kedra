# R04: Noctalia activation and recovery

2026-09-08. Full gate: blocked. No workstation home was used.

Native interruption now passes
[34230166262](https://github.com/Reidond/kedra/actions/runs/34230166262) at `3267e03`.
The actual installed CLI is killed with SIGKILL on native file publication.
Public recovery commands successfully abort to the prior value, resume to the
selected value, and preserve a later real Noctalia edit after exact abort refuses
it. Every case retains selection and native metadata and returns an active
Noctalia service. Keep-current restarts/validates the writer before clearing the
journal and preserves the private checkpoint. The same run passes default private
state initialization and the complete desktop/doctor/portal/keyring checks.
Serial evidence is `output/r07-run-34230166262/vm/serial.log`. This is process
termination evidence, not a power-loss/full-disk or every-phase interruption pass.

Native discard/session subset now passes
[34223972271](https://github.com/Reidond/kedra/actions/runs/34223972271) at `a5cd96e`.
The actual sysroot CLI rejects a stale plan, stops the real Noctalia writer,
discards only the unselected field value, preserves selection and native
owner/group/mode/SELinux label, restarts the application and records completion
with no remaining checkpoint. The installed doctor command also passes in that
desktop session. No test-only projection executable is used.

The preceding native 34222698950 at d8a76a9 refused a metadata mismatch before
publication. Candidate files now explicitly preserve group and SELinux label
through their owned descriptor, then retain the final equality check. The earlier
Fedora global timeout drop-in is accepted only at its verified root-owned path
with exactly its observed directive; user overrides still refuse.

The owner removed unit/model/mock/doctests and repository self-checks on
2026-09-08. The isolated results below are historical. Ongoing qualification uses
actual CLI/native VM workflows and manual testing only. Other interruption phases,
broader baseline transitions and generic files remain open.

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

Open: actual process-kill/full-disk fault matrix,
baseline transitions across OS updates/rollback, broader application groups and
generic file/line integration. Generated filesystem tests and managed-writer
qualification do not prove a complete home manager or protection against arbitrary
same-UID/root writers. The owner installer and real-home adoption remain gated.
