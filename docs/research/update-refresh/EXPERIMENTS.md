# Update/refresh experiments supplement

Status: **all cases not-run**. Date: 2026-09-07.
This is a test specification under existing R01-R11 packets, not a new completed
research report or a reason to mark those packets passed. Policy: docs/UPDATES.md.
Use fixture RPM repositories with synthetic signed packages/disposable keys and
CI-built VM artifacts, not the owner's running OS/home or production signer.

## A. Fedora reconciliation (R02/R07/R08)

| Case | Expected result |
|---|---|
| Source and base fixed; requested RPM update appears | Fresh package step detects/builds the newer target |
| Only indirect dependency changes | Closure comparison detects it despite unchanged package lists |
| Only inherited base RPM has newer repository version | Refresh includes it even when base tag did not move |
| Base digest changes; same RPM version text | Base change is not hidden by package-list equivalence |
| Repo metadata changes; selected content does not | True no-change, no new ISO/release; fresh checkpoint |
| All materialized inputs equal; timestamps differ | No useful update only if normalization is proven safe |
| Same NEVRA but package content/checksum changes | Not silently treated as the same input |
| Build cache hit would skip DNF | Test fails unless refresh forces execution |
| Required repo outage or mirror inconsistency | Retry bounded; explicit failure, never no-change |
| Resolver exits 100/0/error during check-upgrade | Updates/no-updates/failure correctly separated |
| Newest candidate unsatisfiable | No skip-broken or best-effort freshness success |
| Unexpected lower EVR, vendor change, required-package loss | Blocked pending reviewed exception |
| Changed version hold and expired hold | Accounted-for delta; expiry cannot silently remain active |
| User removes direct package | Fresh base assembly drops it unless base/dependency requires it, with reason |
| Probe/build metadata race | Actual final candidate evidence regenerated; never false projected inventory |
| Non-RPM tool source reports newer version | Existing approved pin retained; separate reviewed change |
| Docs/worklog/checkout skill only change | No new OS solely for checkout knowledge |
| Changed config, file mode or image helper | Included even with unchanged RPMs |
| Package or base signature failure | No release/signing bypass |

Record actual Fedora DNF/RPM versions, allowed repo definitions/key identities,
metadata hashes, all selected package identities and before/after final inventory.
Choose a canonical evidence format without writing a custom RPM solver.

## B. Release, checkpoint and liveness (R01/R02/R08)

| Case | Expected result |
|---|---|
| Valid candidate, all gates including same-digest ISO pass | One immutable release approved |
| Signed candidate but boot/config/ISO gate fails | Prior approved head preserved; candidate-blocked visible |
| No-change resolution after an old release | New signed freshness checkpoint may reference the old image/ISO |
| Resolution fails but job attempts success timestamp renewal | Rejected; previous last-success time retained |
| Older slow run completes after newer approval | Cannot regress image/source/checkpoint state |
| Docs-only source advances during valid image build | Same image-input intent does not obsolete it needlessly |
| Image-affecting source advances during old candidate | Old intent cannot become current without explicit policy |
| Same checkpoint generation with different bytes | Integrity/equivocation failure |
| Forged/tampered/partial metadata or swapped host release | Rejected; no permissive retry |
| Workflow run-number reset/recreation | New trusted epoch needed; no false ordering |
| Expired checkpoint, badly skewed clock | Routine stage blocked with clear reason |
| Older offline client, key transition, explicit rollback | Trust transition/recovery works without lowering high-water mark |
| Dropped/disabled GitHub schedule | 36/72-hour stale diagnostics; no "up to date" inference |
| No new releases but successful complete no-change checks | Healthy freshness, not false outage |
| Fresh solve found updates but every candidate fails | Latest-known update blockage visible, not hidden by old release |
| Release assets unavailable during pointer change | Retry or metadata-unavailable; never mixed/unsigned approval |

Use redacted run/job links, before/after checkpoint/approved state and exact digests.
Rehearse manual schedule recovery; a second cron in the same repo is not evidence
of an independent watchdog. Production key/environment configuration is not part
of this planning task.

## C. Client and local safety (R04/R06/R09/R10)

| Case | Expected result |
|---|---|
| Notify timer sees new approved image | Notification/status only; no DNF, stage, boot change or reboot |
| Inherited bootc automatic service remains active | Qualification fails; image policy/audit must address it |
| Explicit sysroot update | Exact verified digest staged, no immediate reboot |
| Ordinary reboot after authorized stage | Expected new digest and post-boot health state recorded |
| Same image already staged | Idempotent report, no duplicate destructive work |
| Different image manually staged | Preserved unless explicit replacement requested |
| Rollback then next background check | Hold retained, no automatic reinstall of rejected release |
| Private registry token missing/expired | Clear auth error; no vault export or broad token request |
| Wrong host/architecture or unpromoted run | Rejected independently by helper |
| Offline/sleeping laptop, battery/metered network | No wake/reboot; optional downloads/staging deferred by policy |
| Home unavailable or conflict, writer changes after preflight | No silent overwrite; stage/apply postponement or safe recovery |
| Power loss/full disk/unknown journal schema | Reportable recoverable state; no baseline reset |
| Timer/user agent races an authorized manual stage | Installed lock and pending identity prevent replacement |
| Local checkout dirty/missing/offline | Release check does not reset or require it |
| Optional download-only path | Actual pending/next-boot behavior documented before feature exposure |

## Evidence and implementation sequence

Start A in a disposable GitHub Actions experiment with no production publication.
Connect B to R01/R02 image/installer tests. Connect C to home/privilege research in
two VMs. Add real-device tests only after the VM gates, with explicit permission.
Write packet reports/results/ADRs and update relevant skills/worklog. Do not add
live production cron or client timers merely because this document exists.
