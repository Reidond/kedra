---
name: kedra-github-actions
description: Maintain Kedra signed-container CI, midnight package checks and native VM workflows.
---

# Actions

Read usr/src/kedra/docs/UPDATES.md, usr/src/kedra/docs/RELEASES.md and usr/src/kedra/docs/ARCHITECTURE.md. OS image builds run in Actions. ISO construction is explicit/local through usr/src/kedra/installer/build-local.py and never uploads. Do not create GitHub Releases, machine bundles or ISO/checksum assets.

The 00:00 UTC trigger reconciles the reviewed official Fedora 44 base and complete installed RPM closure. Do not let cached DNF layers claim freshness. Required repository, signature or solver failure is an error. Changed inputs produce a candidate; identical inputs do nothing, with no checkpoint renewal.

Build jobs have public trust. Automatic isolated signing executes no checkout/candidate/repository code while production keys exist. The main-only environment has no human approval gate. Sign and verify exact OCI digest/repository, then advance GHCR stable only after current-source and ordering checks. Pin Actions/tools and minimize credentials.

Per-target releases (implemented 2026-09-25; release 36617035503 passes both targets on 2026-09-29):
- release.yml keeps one non-cancelling `release-44` group. It calls reusable `release-target.yml` independently for desktop (`ubuntu-24.04`) and qemu-arm64 (`ubuntu-24.04-arm`).
- Each target uses its own `kedra-<target>-signing` environment, secrets, builds repository and artifacts.
- The callers use `secrets: inherit`, and only the signer job declares the environment. Observed 2026-09-25 (release run 36190624411 failed closed with empty signing secrets; probe run 36191986665): a called job that declares `environment:` sees that environment's secrets as empty unless the caller inherits secrets, despite the reusable-workflow docs. The repository has no repository-level secrets, so inheriting exposes nothing else.
- check.yml adds a native `rust-aarch64` leg next to the required `rust` leg.
- Hosted arm64 runners expose no `/dev/kvm`. test-qemu-arm64.yml therefore boots its disposable disk under TCG with AAVMF Secure Boot firmware and Microsoft-enrolled vars. Main run 36617035132 attempt 2 at `bafd1884` powers off with every security/bootc marker in 300 s.
- The four x86 VM workflows boot `OVMF_CODE_4M.secboot.fd` with a copied `OVMF_VARS_4M.ms.fd`, and require the guest's `KEDRA_SECUREBOOT_PASS`. test-signed-update adds a snakeoil-keys refusal case.
- SMM under KVM on hosted runners is unverified until those runs.

check.yml uses standard Cargo tools and actual CLI/OpenSSL workflows. test-container.yml (added 2026-09-27; main run 36617035048 passes both architectures on 2026-09-29) builds each target's candidate natively with the harness's full local build (`KEDRA_LAB_IMAGE=build`, docker/BuildKit, KEDRA_LOCAL_BUILDER inputs). It then runs every container scenario and uploads report.json, junit.xml and screenshots. VM workflows keep boot-level coverage:
- test-desktop: tuigreet login, PAM keyring, doctor gate, Xwayland/Qt choosers.
- test-qemu-arm64: the bootc contract and the TCG Secure Boot boot.
- Signed update, direct GHCR, home transition and RPM refresh.
Image-content and session checks moved from these workflows into container scenarios. No unit/model/mock/doctests or repository scanners.

Observed 2026-09-29, QEMU 8.2.2 / Fedora systemd 259.9 / kernel 7.2.7:
run 36617035132 attempt 1 froze PID 1 at guest 113.669 s, before the observer
started, then consumed the full 90-minute bound. The serial line
`systemd[1]: Freezing execution.` is a fatal boot outcome, not readiness.
The same SHA, firmware and package closure pass in attempt 2; retain both
artifacts rather than rewriting the first failure. `tests/vm/qemu-arm64/boot.sh`
now detects that exact fatal line every 2 s, preserves it and terminates the
owned timeout/QEMU process before failing. There is no automatic retry and all
Secure Boot, SELinux, bootc, unit and digest assertions remain required.

Local installer changes retain pinned image-builder, labeling, offline payload verification and deliberate disk choice. Media permissive SELinux never weakens installed enforcing SELinux/signature policy. Record actual local smoke/fresh-install results separately from image builds.

Update STATUS/worklog with exact observed outcomes. Never call staged booted, signed installed or syntax qualified.

Exact-candidate validation (qualified 2026-09-30, release 36702944904):
`release-target.yml` requires build → validate-candidate → isolated sign → stable
publication. Validation runs the complete sanctioned Testcontainers harness against
the immutable public builds-repository digest with KEDRA_LAB_OVERLAY=none, on the
native runner, without a signing environment or private keys. Its complete,
non-interrupted report must match target/digest, contain passed results and no
cleanup failures; only then is that digest passed to the signer. The signer
independently requires equality with build.digest and retains its current-source,
identity, ranking and namespace checks. No-change skips validation/sign/publication.
Do not substitute the independent container workflow: rebuilding the same source
SHA against refreshed Fedora repositories can test a different RPM snapshot.
A fresh Noctalia 5.2.0 candidate exposed exactly that compatibility gap. Keep test
execution out of the signer and never send test executables or raw reports to it.
The first gate run 36692694223 failed closed because Git cannot bundle a raw
commit ID without a named ref. The corrected private 0700 named-ref bundle preserves
exact image provenance and checkout state under hostile Git settings. Release
36702944904 passes both full exact-candidate suites, signing and strict publication
(all eight jobs); validation is observed, not inferred from workflow text.

ARM release-fixture findings (2026-10-02; see
`.specs/nix-release-composition/verification.md`): preserve OCI manifest bytes with
Skopeo `--preserve-digests`; a Podman push may change layer representation even
when config/RootFS match. Public fixture metadata copied into a root image needs
explicit 0644 under a private 077 producer umask. Scope any 022 umask to the
public producer/installer subprocess; private keys and parent logs remain private.
Store archives and transfer exports each have their own receipt/hash, while
config/RootFS/native/source bindings establish continuity between representations.

Anaconda 44.30 maps `xconfig --startxonboot` to the graphical default target.
Observed cold install changed only default.target despite greetd providing the
graphical-login capability; do not infer a missing provider. Explicit graphical
selection preserves all twelve native artifacts in the external-Kickstart proof.
Installer completion must run outside target chroot through a verified character
device without create/truncate: an ordinary chrooted Path.write_text created a
regular target serial file. The public defaults/embedded-marker source is fixed;
its newly rebuilt owning-source media remains unqualified until the recorded
fresh-install gate passes. External-input recovery does not qualify that route.

Candidate failure evidence (2026-10-03, PR34 run37107489350): ARM foundation
failure logs beneath private candidate work were absent from uploaded evidence,
so its cause could not be established. Candidate foundation/compose now select
`compose.py --diagnostic-output` in public evidence. Retain only bounded structured
step/tool/exit, exception source basename/line, log hashes/sizes and fixed observed
markers; never upload raw private logs or arbitrary exception messages. Preserve
the first failed command across finally cleanup, and never mask the original
exit if diagnostic writing fails. See release-composition verification.md.


Retained local registry replay (2026-10-03, Podman5.4.2): select the full existing
volume name and independently observed CreatedAt; public fixture inspection binds
local backing device/inode and no consumers before create, then exact stopped
container/mount identity before start. Use `:nocopy`; even retained NeedsCopyUp
metadata must not permit payload copy-up. The actual tiny stopped-create proof
preserves sentinel bytes/modes/inode and refuses wrong creation time/consumers
(see release-composition verification.md). Dedicated serialized controller
ownership remains required. Never select backup custody for replay; metadata
creation is expected, so describe payload preservation precisely. Fixture-only
compatible descendants still satisfy original recipe/binary admission; newer
production recipe changes require fresh material. Installer-base overrides do
not replace signed payload identity or waive TLS, deadline or capacity gates.

Hosted Docker28.0.4/overlay2 image retention (2026-10-04): the real public tiny
preflight selected a config digest while Docker save emitted a different single
OCI root manifest. Strict store add-image correctly refused78; this is not a
multiple-root or CDN failure. Keep the engine's root/graph binding. The QEMU
workflow explicitly selects containerd-snapshotter and must pass the unchanged
public preflight before full candidate work; configuration alone is not a pass.
Preserve other daemon settings, refuse existing containers before switching,
and retain bounded backend/root/config metadata plus actual cleanup exits.
Sources: `.specs/nix-release-composition/verification.md`, runs37199831847 and
37199833274, https://docs.docker.com/engine/storage/containerd/ .

Docker28.0.4 inspect Config also injects three empty strings, six false booleans
and three null fields absent from the actual exported config (2026-10-04,
QEMU runs37215763783/37215764140). The producer adjusts only those twelve typed
inspect-only defaults, refusing nondefault/other mismatches; platform, all diff
IDs and full transferred config-byte identity remain required. Do not expand
normalization from assumptions. Source/evidence: release-composition verification
and https://docs.docker.com/engine/deprecated/#non-standard-fields-in-image-inspect .
