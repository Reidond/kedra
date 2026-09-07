---
name: kedra-github-actions
description: Build Kedra CI, target matrices, image/ISO/VM experiments, runner measurements, signed artifact provenance, safe release jobs and checks that do not expose secrets.
---

# CI and installer pipeline

All OS and installer builds run in GitHub Actions. Local Rust tests and synthetic
home tests are allowed; local OS rebuilding is not the intended workflow.
Bootstrap check.yml only validates Rust and skill wiring. It is not a release
pipeline and has no signing, GHCR publishing or workstation access.

## Job boundaries

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
refreshes silently stale. A failed target never advances its channel; per-target
promotion is serialized. Reruns have distinct identities. Retain exact digest,
source, toolchain, Cargo.lock, package inventory, builder and source-skill pins.

## Installer research

Upstream moved bootc-image-builder into osbuild/image-builder. Evaluate a pinned
bootc-installer route with separate Anaconda environment and signed OS payload.
Do not bake installation tooling into the everyday desktop unnecessarily. Prove
interactive disk selection in a multi-disk VM, encryption/account setup, recovery,
correct registry origin, target enrollment and the first signed update. Never
use an unattended first-disk erase default or default credentials.

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
image-builder; read kedra-release-signing and kedra-research.
