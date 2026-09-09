# R04: Native home activation and recovery

Updated 2026-09-09. Full gate: blocked. No workstation home was used.

Optional caller-home status passes Linux workspace 34287224455 and actual signed
[A/B/A run 34287224388](https://github.com/Reidond/kedra/actions/runs/34287224388)
at `8288cc263331f64625e131776a799b38d0bd0f85`. The public CLI distinguishes absent,
unsafe/corrupt/incompatible/busy/profile state, matching accepted baselines,
required reconciliation and actual killed-CLI recovery. Independent live,
selected, local and published choices remain intact. Both groups and overall
precedence are checked: after niri accepts B, Noctalia remains on A and keeps
overall reconciliation required; after explicit rollback handling both match A.
The no-flag/helper contract stays unchanged, and no private home data is sent
to root. The fixture's narrow, visudo-checked helper grant is guest-only; it
does not qualify interactive authentication or change production sudo policy.
Artifact 10080447739 independently matches ZIP SHA-256
`c6547af784d8d4a934485d226df9932e2ae2252bf58eb4576d2f3d7a35e7a25b`;
local native evidence is `output/caller-home-8288cc2/r04/evidence`.
R01 34287224413 and R07 34287224389 also pass the existing regressions at this
source. See [HOME-REVIEW.md](../../HOME-REVIEW.md) for the assessment's limits.
The separately qualified owner ISO remains on `c660c58` and lacks this new flag.

Actual signed graphical A/B/A passes
[34243567959](https://github.com/Reidond/kedra/actions/runs/34243567959) at `f45b55a`.
The distinct image digests are A
`sha256:c6260c5408917d76a8773233736103e3b5c3e6c72bf194813b8d8019ffb1b719`
and B `sha256:ce494cbe010cbce6b35f7999564ecd977f25cede4e4440c611b24038098fab48`.
Public fixture source A is `f45b55a28025d42d5f84ba06563b0e2ee7c9aea9`, selected
publication P is `b235ed44af04d316f1ff1dbe8bcb5d9d1285852f`, and actual B is
`cc56cbcbd8aa863e7e43310bc8c70bd01d2a3b4e`.

On A, the ordinary CLI prepares independent live/selected/local/published state
and stages signed B (39.677 seconds). On booted B, activate-plan/apply retires P,
accepts B, preserves the later width=5 edit and pinned width=4 selection, reanchors
local gaps=14 against incoming gaps=18, and adds the B cursor default; native
acceptance passes at 16.819 seconds. After real retained-A OS rollback, the CLI
refuses the conflicting home plan without changing live data or accepted B.
Explicit unstage/keep-local resolves that conflict, then A activation preserves
both local choices and removes the unchanged B-only cursor default. Acceptance
passes at 17.084 seconds; high-water sequence 2 and rollback hold remain intact.
All three boots retain enforcing SELinux/containerPolicy and a real graphical
session. Evidence is `output/r04-run-34243567959`, with per-boot serial logs.

This qualifies the named niri image-transition/rollback subset using disposable
authority and generated public source commits. It does not qualify older binary
schemas, power loss/full disk, arbitrary file groups, owner authority or hardware.

Current installed niri baseline acceptance passes
[34238949306](https://github.com/Reidond/kedra/actions/runs/34238949306) at `9802b49`.
The actual CLI resolves the installed source/history, refuses a stale plan, accepts
the installed baseline and preserves the later native file. The full
discard/recovery/doctor/session sequence also passes. The baseline marker appears
at 30.434 seconds and the session pass at 79.033 seconds in the retained serial log.
The unchanged fresh-login driver path passes 34240940795 at `967fd93`.

Actual signed A/B/A [34240940931](https://github.com/Reidond/kedra/actions/runs/34240940931)
at `967fd93` builds and signs distinct A and B graphical images, enrolls A,
establishes real selected/local/published/live user choices and stages B. B boots
with enforcing SELinux/containerPolicy, but its test login fails before home
acceptance: the greeter remembers kedra-test, while the driver types that username
again into the password field. `accept-b/login.png` shows the remembered prompt.
The driver now has an explicit repeated-login option used only after the first
boot. Corrected run 34243567959 at `f45b55a` passes the full named sequence above.

Niri exact discard and native reload pass at 5e238c7 in
[34237287511](https://github.com/Reidond/kedra/actions/runs/34237287511), with no
accepted baseline advance. The actual VM covers stale-plan refusal, pinned-line
restoration, unrelated edits, owner/group/mode/SELinux preservation and a relative
include. SIGKILL at file publication passes abort/resume/keep-current, including
later-edit abort refusal and independent S/B/checkpoint retention. The complete
desktop and doctor pass. Evidence is
`output/r07-run-34237287511/r07-desktop-evidence-34237287511-1/vm/serial.log`.
Workspace 34237287689 also passes actual CLI/release workflows at this source.
Installed-baseline acceptance is qualified separately above (ADR 0022); actual changed-
image home acceptance remains not-run. The preceding 7d14ada stopped on a rustix PID conversion
method error; the method was corrected without relaxing peer checks. The source
preview native regression 34235455502 was cancelled when superseded.
No unit, mock, model or repository-scanner tests were added.

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
