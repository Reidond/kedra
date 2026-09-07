# Kedra updates and scheduled Fedora refresh

Date: 2026-09-07. Status: **implementation plan; not enabled or integration-tested**.
This supplements PLAN.md sections 4-5. Defaults below are proposed Kedra policy,
not claims about upstream defaults. Existing R01-R11 gates still apply. See
[ADR 0002](adr/0002-updates-and-fedora-refresh.md),
[experiment cases](research/update-refresh/EXPERIMENTS.md), and the repository-only
[kedra-github-actions](../plugins/kedra/skills/kedra-github-actions/SKILL.md),
[kedra-bootc](../plugins/kedra/skills/kedra-bootc/SKILL.md) and
[kedra-release-signing](../plugins/kedra/skills/kedra-release-signing/SKILL.md) skills.

## 1. The experience

There are two independent loops, neither requiring a continuously running AI:

```text
Fedora base and package repositories
  -> scheduled GitHub Actions reconciliation
  -> candidate image -> tests -> signed approved release + ISO

Machine's enrolled Kedra target
  -> periodically check approved releases
  -> notify user -> authorized stage -> next authorized reboot
  -> reconcile writable home -> verify running deployment
```

The workstation never runs a scheduled host DNF/RPM upgrade. It consumes a complete
Kedra release by digest. A normal update needs neither a local OS build nor a Git
pull in the user's checkout. Repository/package edits and periodic dependency
refresh both feed the same release path.

## 2. Proposed policy defaults

| Concern | Initial policy |
|---|---|
| Fedora refresh | Twice daily at 03:23 and 15:23 UTC; manual dispatch also available |
| Source changes | Accepted image-affecting changes to main use the same pipeline immediately |
| Scope | Fedora 44 base and installed RPM closure, including indirect dependencies |
| Repositories | Explicit reviewed allowlist: fedora and updates; no testing/Rawhide/COPR implicitly |
| Dependency mode | Normal same-major upgrade, not routine distro-sync/downgrade |
| Published release | Changed candidate only after all required gates, including its installer |
| No-change run | Record successful resolution/freshness, reuse the existing release and ISO |
| Automatic machine behavior | Check/notify only; no automatic staging initially |
| Machine check cadence | On boot/session availability, then every 6 hours, with up to 30 minutes jitter |
| Reboot | Never automatic by default; no force on an active session |
| Optional future staging | Explicit opt-in only after home/session preflight and pending-slot tests pass |
| Pausing/rollback | Keep checking, suspend automatic mutations and warn; never immediately undo rollback |
| Stale refresh warning | More than 36 hours without a successful complete target resolution |
| Serious freshness warning | More than 72 hours; distinguish outages, blocked candidates and no updates |
| Channel checkpoint lifetime | Proposed 7 days; expired metadata cannot authorize a new routine deployment |
| Retention | Initially no automatic deletion of promoted image digests, signatures or release metadata |

The 12-hour schedule is a polling target, not a security-patch SLA. Mirrors, CI
queues, failed validation and the owner's chosen reboot time add delay. Long
unsupported/EOL operation is not solved by repeating a failing build.

GitHub schedules can be delayed/dropped, run on the default branch, and are
automatically disabled after 60 days without repository activity for public
repositories. Off-hour minutes reduce one load source but do not guarantee runs.
A second cron in the same disabled repository is not an independent watchdog.
Use client-side freshness checks plus an explicit manual recovery/dispatch path;
consider an external watchdog only as a separately authorized service. No fake
keepalive commits or broad machine-side GitHub write token. [U01]

## 3. What is refreshed, and what is not

The Fedora base's moving 44 tag is a discovery input. Resolve it to a verified,
architecture-specific immutable digest once for the run; record the index digest
as well when present. A moved base can matter even when its RPM version list is
unchanged. Base trust is a separate reviewed input to R01/R08; a Kedra signature
does not retroactively prove the upstream base was authentic.

In a disposable Fedora build environment, refresh repository metadata and update
the complete installed package set, then ensure the common and target package
requests/removal policy are satisfied. This includes kernel, bootc, systemd, Mesa,
firmware and libraries when present; checking only packages/common.list would miss
most inherited/dependency updates. Build from Fedora plus source each time, not
from the previous Kedra image indefinitely. A removed package declaration must not
survive solely because it was installed in last week's derivative.

A dependency may remain because the base or another requested package requires it.
Show that reason; removal from a list is not permission to erase essential base
components or run broad autoremove. Use an explicit reviewed removal policy.

Keep non-RPM agent binaries, Bitwarden downloads, build tools, Rust and Cargo
inputs pinned. Their update checks may propose ordinary reviewed dependency PRs;
the Fedora refresh must not secretly download their latest installers. No personal
Codex/Claude update, Flatpak update, Toolbx package update, user MCP/skill update or
firmware flashing is part of this loop. Repo skills and the Rust-skills submodule
remain checkout-only and are not image/rootfs/home payloads.

Fedora-major changes (44 -> a later release), new RPM repositories/signing keys,
package holds, unexpected downgrades and vendor changes require a source/policy
review. Default updates include bug fixes and enhancements as well as security
fixes. Advisory/CVE information improves priority/release notes; incomplete
advisory data must be reported as unknown, not as proof there are no security
issues. Holds need an owner, reason and expiry; expired holds block promotion.

## 4. Refresh workflow

Use one future release.yml orchestrator for scheduled, manual and accepted-source
runs. Existing check.yml stays an unprivileged repository gate. Do not create a
separate package-sync bot that commits a version bump for every Fedora RPM.
Store resolved RPM versions/checksums and transaction evidence with the release;
Git stores installation intent and reviewed exceptional pins.

### A. Capture the request

Record immutable source SHA, trigger, workflow/run/attempt, target list, target
architecture, relevant source-input hash and policy version. Build all enabled
targets initially. xps remains disabled; use disposable VM targets for tests.
Trusted manual dispatch can request a target/refresh/diagnostic rebuild, but not
arbitrary refs, repositories, shell commands or bypasses of release gates.

### B. Resolve current Fedora inputs

Resolve the base, allowlist repositories, preserve TLS and RPM signature checking,
and fetch fresh metadata. Force required-repository errors to fail the check;
never report a skipped or unreachable updates repo as no changes. Verify actual
DNF5/Fedora option names and effective configuration. Package signature checks
and repository-metadata signatures are distinct; do not blindly enable a metadata
signature mechanism an upstream repository does not publish. [U02, U04]

The default refresh/build path uses strict best-candidate upgrade behavior and
explicit install/removal intent. DNF5's --best prevents some silent older-candidate
fallbacks but does not prove every transitive dependency is at the newest version.
Record solver choices; fail or explicitly account for held/skipped results. Never
add --skip-broken, --skip-unavailable, --allowerasing or --no-gpgchecks to conceal
an unresolved update. Routine distro-sync is inappropriate because it may also
downgrade. [U02-U05]

A no-cache package stage must actually execute with refreshed metadata even when
the base digest, package-list text and repository source commit are unchanged.
--refresh inside a cached RUN instruction has no effect if that instruction never
runs. Initially avoid package-layer caching; reuse immutable downloads/Rust caches
only where keyed and revalidated. Later cache optimization needs a negative test
that changes the repository while keeping source and base fixed. [U08]

### C. Decide whether the target changed

A preliminary probe is optional optimization, not authoritative deployment input.
It must evaluate the prior complete target image/installed closure and the intended
new source/base, not just a bare base or package names. DNF5 check-upgrade uses
exit 100 for updates and 0 for none; other exits are errors, not no updates. [U06]

The first prototype should prefer a real disposable refresh build over a clever
incomplete probe. Establish no-change behavior using the materialized package
result and complete declared inputs before optimizing. Compare against the last
**promoted** target release, not merely the last failed/signed candidate.

The comparison includes base/platform digest, image-affecting source/policy,
selected RPM NEVRA and content identity, external artifact hashes, binary outputs,
resolved configuration and builder/recipe identity. NEVRA means name, epoch,
version, release and architecture. Use RPM comparison rules for diffs, not string
or semver ordering. A changed base, requested package removal, rebuild of identical
NEVRA with changed bytes, or non-RPM artifact is not hidden by equal version text.

Do not decide equivalence from OCI digest alone: timestamps, DNF history, logs
and build annotations may create a different digest without a useful update.
Conversely, excluding volatility must not hide real config/mode/xattr changes.
The no-change detector needs explicit inputs and fixtures. When equivalence is
not established, conservatively build/test a candidate or report uncertainty;
never falsely record no updates. Source worklog/docs/repo-skill changes alone
should not force a new OS; preserve full source provenance separately from an
image-relevant input hash.

### D. Build once and promote that exact artifact

A detected change produces a complete new image in Actions. Assert final requests,
repository allowlist, RPM signatures, inventory, dependency health and boot assets.
If discovery and build see different repository snapshots, the actual build result
wins: recompute its diff/evidence and rerun gates. Do not sign a probe's projected
package list as if it described a later unconstrained solve. Initial per-target
resolution timestamps may differ; do not claim a global Fedora snapshot.

Publish a uniquely identified candidate digest and sign it through the isolated
trusted path. Validate config, boot/update/rollback and required negative cases.
The candidate's contents never execute in the job holding production signing
keys. Build and test the installer against that same digest, then publish immutable
release metadata and a signed approval. Testing one digest then rebuilding for
release is prohibited. Transaction-store/replay with retained RPMs is a possible
later optimization, not a requirement to mirror all Fedora or build a package
manager; replay mismatch must fail rather than ignore differences. [U07]

Each promoted release has its ISO, image/signatures, package inventory/diff,
home provenance and checksums. No-change runs reuse them. An ISO failure leaves
the previous approved release in place and reports an image candidate awaiting
its required gate. An expedited security refresh uses manual dispatch and the
same gates, not an unsigned or untested bypass.

## 5. Publication, freshness and channels

The initial user channel is `44` per target/architecture, not a global latest
release across all machines. A convenience OCI tag can point to the current
approved image, but clients deploy its digest from signed metadata, not tag text.
Candidate tags do not constitute approval.

Use two logical records:

**Immutable release record:** target, architecture, Fedora major, image repository
and digest, source/build identity, materialized package/config/artifact hashes,
ISO and signature references, home/protocol compatibility, test evidence and
release sequence. Re-signing or rereleasing changed bytes creates a new identity.

**Signed channel checkpoint:** scope, monotonic checkpoint generation, issued/expiry
times, approved release-record hash/locator, image digest, latest observed refresh
status/time and latest successful resolution time. It can point to the same release
after a valid no-change check without publishing another OS image or ISO. A failed
check must not renew its last-success field or claim current packages. Reporting
failed/blocked status must preserve the last approved release reference.

A successful resolution that found updates is distinct from successful promotion.
Expose `updates-found/building`, `candidate-blocked`, `no-change`, `promoted`,
`resolution-failed` and `refresh-stale` rather than one ambiguous last-update date.
Only a completed fresh resolution can support a no-change statement. Clients
should show when new upstream packages were observed but validation prevented
shipping them. Advisory-only changes may update freshness/notes without an OS build.

GitHub Releases can store immutable release assets and per-target channel bundles;
GHCR stores OS images/signature attachments. Exact bundle encoding and atomic
publication are R01/R08 work. A moving lookup URL/API result is discovery only.
Validate embedded signed scope/hashes, never unrestricted redirect URLs. If channel
assets are transiently missing/mismatched during publication, retry or report
unavailable; do not accept partial/unsigned metadata. Do not use the repository's
single GitHub `latest release` endpoint as a multi-host channel authority.

Serialize approval/checkpoint writes per target/channel; do not cancel active
promotion. Serialization alone does not establish event order. Under that lock,
recheck current desired source inputs and approved sequence, reject obsolete run
results, and reject same-generation/different-payload equivocation. Re-run attempts
are distinct; workflow recreation/reset needs a versioned epoch, not comparing
unrelated run_number values forever. A docs-only commit with identical intended
image inputs need not invalidate an otherwise current candidate.

Clients retain a root-owned highest accepted checkpoint generation per trust epoch;
normal operations cannot lower it. Freshness expiration bounds replay/freeze for
new updates and requires a sane clock or an explicit clock-error state. Keeping
an existing booted deployment working offline does not require fresh metadata.
Explicit rollback may select a retained previously verified release without
resetting channel high-water marks. Key rotation/revocation and very old offline
installations remain R08 tests; do not invent a new cryptographic protocol in the
CLI to paper over that gate.

## 6. Client-side mechanism

Proposed interfaces; none is implemented by this planning change:

```sh
sysroot update --check    # Check signed channel and describe changes; do not stage.
sysroot update           # Verify/preflight/download/stage after authorization; no reboot.
sysroot deploy --run ID  # Resolve this run's approved target release; not any signed candidate.
sysroot status --json    # Distinguish running, pending, available, held and stale states.
sysroot rollback         # Explicit safe rollback flow; separate reboot authorization.
```

A lightweight future systemd timer checks metadata after startup and every six
hours with jitter, without waking a sleeping laptop or making host RPM changes.
Notify through the user session when available, otherwise retain readable status.
Do not give a background user agent root rights or an unlocked vault. Checking a
public channel does not require cloning source, a model login or Bitwarden unlock.
Private registry/channel access must use the separately scoped credential design.

Default mode is `notify`. An optional `stage` mode is a later explicit policy,
not initial behavior: require unmetered network and laptop AC by default, enough
storage, no conflicting pending deployment/hold, verified metadata, and successful
home preflight for all affected managed users. An unavailable encrypted home or
conflict postpones staging. It never silently picks a winner or overwrites a
manual pending digest. `download-only` is not assumed harmless across bootc
versions: prove whether it affects pending/next boot before exposing prefetch.

Mask/disable the inherited bootc-fetch-apply-updates timer through image-owned
policy, and audit other enabled automatic OS update/reboot units. Upstream's
fetch/apply service may reboot; bootc switch --apply may also reboot. Do not call
those paths from a notification timer. Verification/signature defaults alone do
not establish Kedra's approval or reboot policy. Check effective installed units
and /etc overrides on upgrade, and visibly report drift. [U09-U11]

For authorized staging: resolve eligibility, independently verify in the installed
root helper, acquire the deployment lock, check disk/current pending state, prepare
home candidates as the user, download and stage the exact signed image, and record
its identity. Recheck eligibility at the final staging boundary. A nonconflicting
stage prepares the next boot: any later ordinary reboot may activate it under the
installed bootc behavior. Clearly state this when staging; no automatic restart is
not the same as no next-boot change. [U10-U11]

After boot, verify the actual image digest, reconcile home against new program
versions before affected apps start, and run deterministic health checks. Save
successful/failed states for a resumed agent or human. No conflict markers or
silent journal reset; rollback does not erase persistent personal changes.
Keep paused, offline, held, denied, home-conflict, metadata-expired and recovery
states observable. Periodic timers do not make progress claims while powered off.

## 7. Failure, holds and operations

| Event | Required outcome |
|---|---|
| Fedora mirror missing/inconsistent | Bounded retry, then explicit failure; existing release untouched |
| Resolver cannot satisfy newest permitted set | Record solver failure; no silent skip or test bypass |
| Unexpected downgrade/required-package loss | Block and require reviewed exception/source change |
| CI/boot/config/ISO test fails | Candidate remains unapproved; notify owner with a redacted diff/evidence |
| No package/source/base change | Successful heartbeat/checkpoint only; no dummy commit or ISO |
| GitHub schedule disabled/dropped | Client freshness warning; manual re-enable/dispatch or authorized watchdog |
| Token/key expires | Clear auth/trust failure; no unsigned fallback |
| Machine offline/battery/metered | Postpone optional downloads/staging; no forced reboot |
| Another digest already staged | Preserve it; require explicit replacement/cancellation policy |
| User rolls back or pauses | Record a persistent hold; do not immediately restage the unwanted release |
| Same signed generation with changed payload | Reject as integrity/equivocation error |
| Expired channel checkpoint | No new routine stage; current OS and local verified rollback remain available |

For repeated failures, use a bounded, deduplicated issue/notification per target
rather than one new issue each run. Failure-reporting credentials do not get
signing authority. Avoid automatically committing worklog changes from cron:
CI results/metadata are machine evidence; agents summarize material work in
worklog.md when acting on it. Provide a manual refresh command through GitHub's
workflow UI or scoped gh workflow run invocation; do not grant machines a broad
write token merely to compensate for an unreliable schedule.

Keep promoted releases/signatures initially. Add reviewed retention only after
accounting for the latest/previous installer, explicit pins, rollback chains,
compatibility bridge releases and offline machines. Large artifacts and failed
candidates need measured size/cost limits and bounded temporary retention; never
GC active runs or delete recovery inputs solely by tag age.

## 8. Implementation slices and exit gates

1. **Refresh proof (R02/R07/R08):** Disposable CI resolver/builder with fixture
   repository state A/B; prove source/base-fixed RPM changes are detected, no-change
   is genuine, dependencies are included and required-repo failure is not success.
   Record exact DNF/RPM/base/repo/tool identities. No production keys/publication.
2. **Release proof (R01/R02/R08):** Signed A/B images, negative verification,
   immutable release identity, same-digest installer and approval/freshness records.
   Test publication ordering, replay, cold client and failed candidate handling.
3. **Client proof (R04/R09/R10):** Notify-only timer and staged state in two VMs;
   mask inherited reboot service, enforce target/signatures, preserve pending slot,
   handle offline/clock/expired metadata/holds and incompatible home state.
4. **Enable production schedule:** Only after the trust, boot, installer and
   credential gates pass; start with notify-only clients. Optional auto-staging
   is a separate opted-in milestone. Review runner costs and maintenance recovery.

The current commit only documents these slices. It does not enable a schedule,
create signing keys, stage images, alter timers or pass any new research gate.

## Primary sources reviewed for this plan

Documentation retrieval is not version-specific integration evidence. Verify the
actual Fedora 44 tools in experiments. Some latest docs may differ from packages.

- U01: https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule
- U02: https://dnf5.readthedocs.io/en/latest/dnf5.8.html
- U03: https://dnf5.readthedocs.io/en/latest/commands/upgrade.8.html
- U04: https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html
- U05: https://dnf5.readthedocs.io/en/latest/commands/distro-sync.8.html
- U06: https://dnf5.readthedocs.io/en/latest/commands/check-upgrade.8.html
- U07: https://dnf5.readthedocs.io/en/latest/commands/replay.8.html
- U08: https://docs.podman.io/en/latest/markdown/podman-build.1.html
- U09: https://bootc.dev/bootc/man/bootc-fetch-apply-updates.service.5.html
- U10: https://bootc.dev/bootc/man/bootc-switch.8.html
- U11: https://bootc.dev/bootc/man/bootc-upgrade.8.html

The fedora/updates allowlist is proposed policy; resolve and inspect actual repo
configuration in R07. Fedora documentation pages challenged this session's web
fetcher, so no new claim about their exact current contents is based on that fetch.
