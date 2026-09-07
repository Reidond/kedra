# Update-client plan

Planning knowledge, 2026-09-07. Read docs/UPDATES.md and ADR 0002. The bootstrap
commands still refuse update/deploy/rollback; do not confuse this note with code.
This skill is repository-only, not a system profile or global installation.

## User contract

Default background behavior checks signed release metadata and notifies. It does
not stage, run host DNF or reboot. Proposed cadence is startup and every six hours
with jitter, without waking a sleeping laptop. An explicit sysroot update --check
is the same check path; sysroot update authorizes verify/preflight/stage, not an
immediate reboot. Use exact run/target release selection when the agent deploys
its own change. The checkout need not be clean/current or even exist to update
an enrolled installed OS. Never git pull/reset local work as part of an OS check.

Digest-pinned bootc requires switch operations to advance. Verify current command
semantics; --apply can reboot, and a normal stage can change the next ordinary boot.
The upstream bootc-fetch-apply-updates service fetches and may reboot. Audit/mask
it and any competing automatic OS updater in the actual installed system, including
persistent /etc overrides. Simply installing a Kedra notification timer is not
enough to establish reboot policy. --download-only is not presumed safe prefetch
until the pinned implementation's pending/next-boot behavior is tested.

## Eligibility and state

Discover a target/architecture/Fedora-major signed checkpoint and immutable release
record. An unsigned moving URL/tag only locates metadata; it is not authority.
Independently verify signatures, scope, content hashes, approved state, protocol/
home compatibility and checkpoint freshness in the root-owned helper. Keep monotonic
checkpoint state. A same-generation changed payload fails; expired metadata cannot
authorize a new routine deployment. Clock failure is visible, not disabled expiry.
Local offline boot and explicit retained verified rollback remain possible.

Distinguish available, verified, home-preflight-complete, staged, awaiting-reboot,
booted, reconciled and healthy; add explicit paused/conflict/expired/stale/auth-error
states. Check latest upstream resolution time independently from published-image
age. Successful no-change resolution is healthy; latest candidate blocked is not
"fully current" just because the previous release is still bootable.

Keep manual pending deployments. Repeating the same stage is idempotent; a different
pending digest requires explicit replacement and safe cleanup semantics. Rollback
sets a persistent hold and does not lower trust high-water marks. Do not let an
optional auto-stage timer immediately reinstall the release the owner rolled back.
Keep checking/notifying during holds without unattended mutations.

## Home and privileges

Preflight actual managed users as those users; unavailable homes/conflicts postpone
optional staging. Do not run user checkout scripts or config validators as root.
Recheck files under the new-software activation boundary, coordinate app writers,
and retain a journal/recovery checkpoint. Staging is not safe live home apply.
Unknown journal versions after rollback fail recoverably, not to an empty home.

Initial notify-only avoids power/session surprises. Optional later auto-stage is
an explicit user policy with AC/unmetered/storage checks, no hold/pending conflicts
and completed home preflight. It still never reboots automatically. Public metadata
checks do not need Bitwarden/model auth; private pulls use narrowly scoped runtime
credentials. Installed health/recovery does not depend on an AI process.

Sources: docs/UPDATES.md U09-U11. Gates: R01/R04/R08/R09/R10, R06 for private auth.
