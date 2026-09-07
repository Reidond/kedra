# ADR 0002: CI Fedora refresh and explicit client deployment

Date: 2026-09-07.
Status: Proposed implementation policy; planning only. Research gates remain open.
Scope: Details the already agreed CI-built, signed, per-machine bootc update model.
Canonical plan: [UPDATES.md](../UPDATES.md).

## Decision proposed

Use two independent loops: one GitHub Actions release orchestrator refreshes Fedora
inputs every 12 hours and on accepted image-source changes/manual request; each
machine checks its own signed release channel and notifies by default. Authorized
sysroot update stages the exact approved digest without forcing a reboot.
No workstation host DNF cron, package layering or mandatory agent service.

The proposed schedule is 03:23/15:23 UTC. Refresh both the Fedora 44 base and the
full target-installed RPM dependency closure. Use only approved stable repos and
fresh metadata, strict repository availability and signature checks. Normal
upgrade is not distro-sync. Reconcile install/removal intent from source and build
from the resolved Fedora base rather than indefinitely extending the last Kedra
image. Pins/holds require reviewed exceptions; non-RPM tool updates are separate.

No-change detection must consider materialized packages and complete image inputs,
not only base tags, Git commits or direct package names. Do not let a cached DNF
RUN bypass network refresh. Initially prioritize correctness over clever probing;
measure and prove cache/no-op optimizations in fixtures. Do not mirror all Fedora,
commit every RPM version to main or invent a Rust dependency resolver. Use DNF/RPM.

One materialized candidate digest goes through signing, tests, same-digest installer
and release approval. Store immutable release records plus fresh signed per-target
channel checkpoints. No-change checks can point to the same approved release;
failed or blocked candidates cannot be relabeled as shipped. Channel generations,
expiry, target binding and a root-owned high-water mark protect routine selection;
explicit rollback preserves that high-water mark and records an update hold.
Exact encoding/publication/trust implementation remains R01/R08/R10 evidence work.

Checks occur about every 6 hours with jitter on online machines. Default mode is
notify, not stage. Optional future auto-stage requires explicit policy, successful
home preflight, power/network/storage checks and no competing pending deployment.
Never silently replace a manually staged image. Reboot stays a separate user
choice, while staging clearly declares that the next ordinary reboot may activate
it. Audit/mask inherited automatic bootc fetch/apply/reboot services. A second
updater must not bypass the Kedra approval path.

Detect stale upstream-refresh health separately from the age of the current image.
Proposed warning/critical thresholds are 36/72 hours; channel checkpoints expire
after 7 days for new routine deployment. GitHub cron is best-effort and can be
disabled by inactivity. Use machine-side warnings/manual recovery; a second cron
in the same repo is not an independent guarantee. No fake activity commits.

## Alternatives not selected

- Local rpm-ostree/DNF updates: bypass the agreed image definition/trust lifecycle.
- Only rebuild when the Fedora base digest moves: misses later RPM updates.
- Rebuild twice daily without a tested no-change concept forever: unnecessary
  downloads/reboots and misleading version churn; correctness-first prototypes
  may temporarily do this while equivalence is being proven.
- Sync every RPM into a private mirror or commit nightly package version PRs:
  disproportionate to two machines; retain actual release evidence instead.
- Automatically stage/reboot immediately after CI succeeds: ignores local home
  conflicts, pending deployment choice, laptop conditions and authorization.
- Follow one mutable latest tag: loses exact tested artifact and target approval.
- Security-only upgrades: incomplete advisory coverage/dependency fixes; record
  advisories and prioritize expedited normal refresh instead.

## Consequences and gates

No new source change is required for same-major Fedora updates. The pipeline becomes
an operational dependency and needs freshness/error visibility. The owner retains
control over installation/reboot; that means automatic release generation alone
cannot promise when patches are active. Ordinary personal tools remain independent.

The existing R01/R02/R04/R07/R08/R09/R10 packets own implementation proofs; the
supplemental [cases](../research/update-refresh/EXPERIMENTS.md) make them concrete.
No actual workflow/timer/client behavior changes in this planning commit. Third-party
and first-party skills remain repository-only and are never globally installed.
