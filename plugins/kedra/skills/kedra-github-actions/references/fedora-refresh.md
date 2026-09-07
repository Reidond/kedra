# Fedora refresh implementation notes

Planning knowledge, 2026-09-07. No refresh experiment has run.
Canonical policy: docs/UPDATES.md; ADR 0002; docs/research/update-refresh/EXPERIMENTS.md.
All files in this skill remain checkout-only, not installed OS skills.

## Refresh means image reconciliation, not mirroring

Poll the Fedora 44 base and required RPM repositories twice daily (proposed cron
`23 3,15 * * *`, UTC) through the same future pipeline used for source/manual
builds. Do not implement reposync over all Fedora, a Rust RPM solver, or automatic
commits of every resolved package version. Store immutable resolution evidence
with a release. The desired package lists and exceptional holds remain in source.

Resolve the base to its actual platform digest; record multiarch index if relevant.
Check the complete target-installed package closure, not only packages/common.list.
Update inherited packages even if the base digest has not moved. Rebuild from the
Fedora base plus current intent so removed custom requests do not persist by
accidental inheritance from an old Kedra image. Show base/dependency reasons for
packages that cannot simply disappear when removed from a user list.

## DNF boundaries to prove

DNF5 --refresh refreshes metadata only if the command actually executes. Bypass
package build-layer reuse on refresh runs; --pull=always alone is not RPM freshness.
Use explicit stable repo allowlists and inspect effective config. Fail if a required
repo is unavailable: --refresh plus skip_if_unavailable=False is the relevant
upstream mechanism. Do not accept distribution overrides silently. Keep TLS and
package-signature verification enabled with reviewed Fedora keys. Repository
metadata signing and RPM signing are distinct; verify availability/versioned
option names, including pkg_gpgcheck/localpkg_gpgcheck where supported.

Normal upgrade can include fixes beyond security advisories. --best tightens
selection but does not guarantee newest versions for all transitive dependencies.
Record holds/skips and final results. Never catch an error by adding skip-broken,
skip-unavailable, allowerasing or no-gpgchecks. Distro-sync can downgrade; it is
not the routine command for this mechanism despite the user's word "sync".

DNF5 check-upgrade's 100 means updates available, 0 means none, other exits mean
failure. Handle this before shell set -e or Rust exit-code logic mistakes. Prefer
version-supported structured output to parsing colored human tables. An installed
Fedora binary may not have every option of latest docs. Probe output is advisory;
materialized build inventory and bytes are authoritative.

## Build once; do not overclaim no-change

A run can share one base digest but see different per-target repository snapshots.
Record timestamps/metadata hashes and actual final inventories rather than claiming
a global snapshot. If the solver result changes between probe/build, regenerate
diff/approval inputs and run gates against the actual final digest. Do not rebuild
again after testing. Exact transaction store/replay with retained RPMs is optional;
replay ignores/skips are not acceptable ways to conceal a mismatch.

Equivalence includes source/policy, base, package content, config and non-RPM/tool
artifacts. Comparing only NEVRA misses republished bytes; comparing only OCI digest
can create timestamp churn. No-op pruning is gated by fixtures. When uncertain,
build/test or report uncertainty instead of falsely saying no updates. A no-change
run renews freshness for the existing approved release, not its image build date.
A failed or blocked new candidate does not advance the approved digest.

## Liveness and limits

GitHub schedules may delay/drop and public inactive repositories can lose their
schedule after 60 days. A same-repo watchdog can stop too. Keep manual dispatch,
client-side 36/72-hour freshness visibility and a separately authorized external
watchdog option. Avoid dummy keepalive/worklog commits from cron. No auto-enabling
this schedule before signature/installer/promotion research and key setup pass.

Sources: docs/UPDATES.md U01-U08. These are upstream mechanisms and proposed rules,
not Fedora 44 integration results. Add actual version/digest/evidence when tested.
