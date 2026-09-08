---
name: kedra-github-actions
description: Build Kedra CI, target matrices, scheduled Fedora package refresh, image/ISO/VM experiments, runner measurements, signed provenance and safe release jobs without exposing secrets.
---

# CI and installer pipeline

All OS and installer builds run in GitHub Actions. Local Rust tests and synthetic
home tests are allowed; local OS rebuilding is not the intended workflow.
check.yml builds/lints Rust and exercises actual CLI home/release workflows.
It has no signing, GHCR publishing or workstation access. Owner policy forbids
unit/model/mock/doctests and repository self-scanners.

## Scheduled Fedora refresh

Read [the refresh notes](references/fedora-refresh.md), docs/UPDATES.md and ADR 0002
for the proposed 12-hour refresh, no-change/freshness policy and concrete negative
cases. These are plans, not enabled automation. One future orchestrator handles
scheduled/manual/accepted-source changes. Fresh metadata and full installed RPM
closure matter even when the Fedora base digest and source package list do not
change. A cached RUN can skip DNF entirely; --refresh inside it is insufficient.

Keep fedora/updates as the proposed reviewed allowlist, fail required-repo errors,
preserve signature checks and audit solver results. Normal upgrade is not routine
distro-sync. Do not mirror every Fedora package or commit nightly RPM-version
updates. Never silently update non-RPM pins, personal agents or repository skills.
No-change checks renew a signed freshness record pointing at the same release/ISO;
failed resolution or validation is not no changes. Client freshness checks must
expose stopped schedules, including GitHub's public-repository inactivity limit.

These skills are checkout-only. Do not copy them into image payloads, home baselines
or global/shared agent profiles. CI is permitted to read them as repository source.

## Job boundaries

Prepared 2026-09-08: release.yml is a manual, disabled main-only candidate path.
It preflights an existing owner reviewer and main-only deployment rule, then
separates build, no-checkout image signing and anonymous strict-pull/installer
jobs. Production execution is not-run; public authority, environment secrets,
recovery and promotion are not configured. Read build/release/authority/README.md
before enabling anything. An environment name does not configure protection;
GitHub may auto-create an unprotected environment. Source: GitHub environment
API/docs linked there; R08. Actual Skopeo signing remains qualified through
disposable R01/R02/R04 authority only.

Follow-up 2026-09-08: owner-authorized public authority and protected environment
are now provisioned; backup retrieval and production execution remain pending.
The prepared promote.yml shares candidate concurrency and rechecks current source
plus prior channel hashes. Public jobs prepare/verify media; a no-checkout protected
job signs exact bytes with pinned Cosign 3.1.3. Versioned drafts precede the single
channel bundle; partial version publication is retained for explicit inspection,
not overwritten by a blind retry. R08/ADR 0023 records syntax-only preparation;
do not mark production isolation, races or recovery passed from code inspection.

Use read-only PR checks with no production secrets. Pin third-party Actions by
full commit SHA; minimize token permissions and disable persisted checkout
credentials when unnecessary. Treat PR code, built image contents, logs, workflow
artifacts and input strings as untrusted. Do not interpolate untrusted values into
shell source. A privileged workflow_run or pull_request_target chain can elevate
untrusted content; prefer an explicit reviewed trust boundary.

The eventual path is build -> validate -> sign candidate digest in an isolated
trusted job -> test that exact candidate -> make installer -> sign metadata and
checksums -> promote eligible target release. The signing job must not execute
newly built code or writable repo scripts with production keys. Approving a
package change and changing trust-critical workflows/helper are different risks.

Build all enabled targets from one accepted commit initially. Resolve and record
base/artifact inputs deliberately. Never let cached DNF layers make scheduled
refreshes silently stale. A failed target never advances its approved image;
per-target promotion is serialized AND rechecks source intent/event order. Reruns
have distinct identities. Retain exact digest, source, toolchain, Cargo.lock,
package inventory, builder and source-skill pins. Source-skill provenance is not
permission to install those skills into the OS or force an OS release per doc edit.

## Installer research

Measured 2026-09-08: the legacy Quay builder at a686afe passed QCOW2 tests but
rejected the README's `--bootc-installer-payload-ref` option. The current v82.0.0
bootc-image-builder compatibility source uses `--installer-payload-ref`; the
prefixed spelling belongs to image-builder. Check `build --help` first. The
selected generic-ISO container is ghcr.io/osbuild/image-builder, pinned separately
in installer/inputs.json. Its canonical build command supports bootc-generic-iso
and the prefixed option. Keep older compatibility/QCOW2 evidence attached to its
actual pin; do not interchange these interfaces.
Source: osbuild/image-builder v82.0.0 cmd/image-builder/bib_cmd.go and R02 report.

Upstream moved bootc-image-builder into osbuild/image-builder. Evaluate a pinned
bootc-installer route with separate Anaconda environment and signed OS payload.
Do not bake installation tooling into the everyday desktop unnecessarily. Prove
interactive disk selection in a multi-disk VM, encryption/account setup, recovery,
correct registry origin, target enrollment and the first signed update. Never
use an unattended first-disk erase default or default credentials.

The local 2026-09-08 R02 installation of ISO run 34185915639 failed GetBlob import
with ENOSPC in /var/tmp: installer writable capacity was 1.6 GiB despite 59 GiB
free on the encrypted destination. ADR 0013 adds a hash-guarded, media-only scratch
bind after deliberate disk approval and native root cleanup. It preserves bootc
arguments and refuses cleanup failures; actual corrected installation is pending.
Sources: containers/image storage_src.go/internal/tmpdir and bootc v1.16.10
require_dir_contains_only_mounts. Do not treat more VM RAM as the installer fix.

Measure the actual runner: disk/memory/privileges, /dev/kvm, architecture and image
identity. Avoid ubuntu-slim for privileged filesystem image work. Split heavy
image, VM and ISO jobs; do not guess a paid runner is necessary or that standard
resources suffice. GitHub Release assets currently must be below 2 GiB each;
measure output and design signed whole-ISO reassembly when splitting is required.

## Evidence and failure

Pin everything material and record actual outputs. Container build/lint, QEMU
boot, signed update and physical GPU/camera tests are distinct. UEFI Secure Boot
gets a separate result. Do not enable production publishing before R01/R02/R08.
Sources: docs/SOURCES.md actions-security, actions-runners, release-limits,
image-builder and docs/UPDATES.md U01-U08. Read kedra-release-signing and
kedra-research; record actual findings and worklog outcomes without marking planned
refresh cases passed merely because repository checks succeed.
