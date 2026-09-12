# Continue Kedra implementation

The owner requested full implementation through a usable, understandable installer,
with disposable VM testing. The first owner release is now published:
[desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1)
and its [signed channel](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-channel).
Use [INSTALL.md](INSTALL.md) for concrete download/verification/install commands.
All ten exact owner-media installation cases pass. First enrollment against the
anonymous public channel, repeated-enrollment refusal with unchanged status,
required desktop health, clean shutdown and final sentinel preservation also pass
in the retained generated VM. Evidence is tracked separately in
[the enrollment report](research/R08-release-protocol/owner-r1-enrollment/REPORT.md).

Published r1 remains accepted source `c660c58`, image `bb4f2b68`, candidate
34255228394. The first candidate's compressed-layer failure is historical.
Promotion 34288691672 passed owner-approved metadata signing but failed after
HTTP 500 left an empty draft. Exact-byte, key-free recovery verified all 13
downloaded assets and signed ISO assembly, then published version ID 385117864
and channel ID 385128562. Anonymous channel verification passes; both Git tags
resolve to c660c58. The failed Actions run remains failed. See the
[recovery report](research/R08-release-protocol/owner-promotion-34288691672.md).
The existing production opt-in was restored to true; signing authority and
protections did not change. Later work stays on codex/usable-system until accepted
through normal CI. Read AGENTS.md, current worklog and actual source/CI evidence.

PR 3 has since merged as accepted main `0eb1cf0`. Candidate 34293133114 passes
public build, registry/storage compatibility and the configured signing review
under the owner's standing authorization. Anonymous native verification confirms
signed image `71b928fd`; its installer is building. See
[the exact next-candidate review](research/R08-release-protocol/owner-candidate-34293133114.md).
Main stays fixed through this candidate's v2 qualification/publication. Full-target
resolution research, expired-predecessor history verification and capability
status corrections remain separate development work. No new ISO has been booted.

## Owner testing decision — 2026-09-08

Use end-to-end or manual testing only. Unit/model/mock tests, doctests, standalone
Rust synthetic test harnesses and repository self-checking code have been removed.
Do not recreate scanners for layout, source text, docs, skills or test presence.
Standard formatting, Clippy and builds remain. The explicit e2e_* Cargo targets
exercise the actual CLI home-export workflow on Linux; Python/OpenSSL exercises the
release CLI. Installed behavior is tested in disposable VMs. Older reports retain
historical unit-test results, not current instructions to run or rebuild them.

## Verified implementation

The flat three-package Rust workspace supplies committed-source planning/archives,
release/checkpoint signature and replay verification, private state, Noctalia
capture/staging/local policy, selected-field source export and source receipts.
The installed helper has a bounded root protocol with independent trust/scope checks,
durable stage/rollback records and explicit rollback hold/resume. Production trust
is provisioned and r1 is promoted through the measured recovery above. The manual release candidate workflow
separates build/sign-image/installer jobs; see build/release/authority/README.md.
The owner authorized a new dedicated release key and GitHub deployment secrets;
public authority files and the protected kedra-desktop-signing environment are
now provisioned. Main requires CI and disallows forced/deleted history. Local
recovery files are backed up in Bitwarden and retrieval is confirmed by the owner
(2026-09-08); the agent did not access the vault. Production workflow opt-in remains
enabled for the manual workflow. The owner approved both replacement image signing
and exact r1 metadata; the latter review was performed directly in GitHub. The
owner requested continuation without repeating approval questions for the already
authorized work. Do not ask again for those same approved bytes or rerun signing
to recover publication. Future signing still uses the configured protected
environment; this recovery did not relax its controls.
No workstation enrollment has occurred.

R01 native signed helper run 34228725707 at 94678f6 passes older-media enrollment against a newer fresh channel, metadata/OCI
negatives, stage/boot B, retained-A rollback, newer-data/high-water preservation and
explicit resume. Minimal VM 34218886101 also passes. The refreshed official Fedora
44 AMD64 base is pinned in build/research/inputs.json; the preceding pin became
unavailable at Quay. Historical results retain their exact older inputs.

R02 ISO 34207121856 at 85ed4ab (2,865,981,440 bytes, SHA-256
 d74e2a1eb79e8c93f52da82a8626bad43ad65498382941cf8982f07f41174ed3)
completed fresh encrypted installation on one of two generated 64 GiB disks.
Without repairs, ISO-free boot authenticates an administrative owner and reaches
niri/Noctalia. Enforcing SELinux, correct home/read-only-root mounts, no failed
system/user services and unlocked login keyring pass. The sentinel disk compares
identically after installation and shutdown. This media still uses an unsigned
localhost research origin and is not an owner release.

R07 native Noctalia writer lifecycle, private Codex packaging/runtime and logged-out
Bitwarden plus session/keyring checks pass earlier runs. Owner authentication,
Claude preinstallation terms and physical hardware remain separate. Repository
skills stay checkout-local; none are installed into personal profiles or the OS.

## Qualified development work and remaining scope

Later source `8288cc2` adds optional `sysroot update status --home`, preserving
the no-flag/helper protocol while assessing the caller's accepted Noctalia/niri
baselines and pending recovery. Linux workspace 34287224455 and actual signed
R04 A/B/A 34287224388 pass, including both group statuses, overall precedence,
absence/unavailable distinctions and killed-CLI recovery. R01 34287224413 and
R07 34287224389 also pass regressions. This feature is not in the published
c660c58/r1 image. See HOME-REVIEW.md and the R04 report.

Native RPM refresh experiment 34286322016 at `0d82b1f` passes all ten prepared
snapshot cases. Its equality result is limited to the generated RPM fixture;
whole-target material equivalence and checkpoint renewal remain unfinished.
See research/update-refresh/REPORT.md. No production refresh schedule is enabled.

Noctalia plan/apply/discard/recover commands are implemented with narrow native
field edits, writer coordination, checked file replacement and durable recovery.
Actual desktop 34230166262 at 3267e03 passes stale-plan refusal, discard,
metadata/selection retention and SIGKILL-at-publication recovery. Abort restores
the prior file; resume completes the selected version; keep-current preserves a
later real Noctalia edit after abort refuses it. Keep-current now starts and
validates the app before clearing pending state. Default XDG private home state,
bounded portal/keyring calls, doctor, Codex and Bitwarden also pass. Power loss,
full disk, other interruption phases and baseline transitions remain open.

Ordinary niri file review/export is implemented. Workspace 34233086757 at 86bb96b
passes the actual CLI/Git workflow: adjacent selected/local lines, later file
edits, source/index preservation, receipts, insertion/deletion and symlink refusal.
Native niri run 34233086974 passes. `home file init --reviewed-safe` explicitly
adopts only .config/niri/config.kdl; whole live bytes are compared through Git stdin
and never saved as Git snapshots. Selected/source/publication state stays separate
from accepted image B. Source reconciliation passes workspace 34235455512 at
3c948aa. Native discard/reload and actual killed-CLI abort/resume/keep-current pass
34237287511 at 5e238c7, including pinned decisions, later edits, metadata and
relative includes. That run also passes the full desktop/doctor checks. Installed-
baseline activate-plan/apply passes 34238949306 at 9802b49, requiring exact root-
installed provenance and public source history (ADR 0022). Actual A/B/A run
34240940931 initially failed the fixture's remembered-login handling. Corrected
f45b55a run 34243567959 passes actual signed B acceptance and retained-A rollback
with explicit conflict resolution, preserved live/selected/local/publication
state, replay high-water and rollback hold. Wider path/group integration,
old-binary schemas and additional interruption phases remain open. See TEXT-REVIEW.md and
ADRs 0019-0022; no unit/meta tests were added.

Signed-payload ISO 34222699188 at d8a76a9 passes the complete local installation:
offline signature precheck, deliberate encrypted target choice, owner creation,
ISO-free desktop boot, enforcing SELinux, native portal/keyring/mount health,
exact booted digest and inherited containerPolicy. The installed helper accepts
trust/scope/policy and reports unenrolled. Both clean shutdowns return QEMU exit 0
and the unselected disk compares identical. All local VMs are stopped. See R02
and ADR 0018 for exact hashes. The outer ISO and authority remain research-only.

Release assemble at 94678f6 verifies signed metadata then streams ordered download
parts into private temporary output. It checks full size/hash before publishing
the signed filename without replacing existing output. Actual Windows and Linux
CLI/OpenSSL E2E pass (workspace 34228725596); the actual 2.86 GB R02 ISO also passes
Windows two-part reconstruction with separate disposable authority. No owner
release/promotion is implied. See docs/RELEASES.md.

## Next actions

1. Preserve the completed r1 installation, publication and enrollment evidence.
   A later no-ISO reboot passes exact enrollment/order-state persistence, doctor,
   clean shutdown and sentinel preservation. Qualify the first owner forward
   update with the next exact accepted candidate; no new owner image is staged.
2. Accept the repaired development pipeline through normal CI, then qualify a
   later candidate schema 2 and its expanded signed checksum/inventory/provenance
   outputs. The current r1 has its original 13 v1 assets; do not backfill or
   overwrite them. Full native v2 publisher execution remains not-run.
3. Build whole-target material evidence and protected no-change renewal. The
   initial public checkpoint expires 2026-09-15 23:05:17 UTC; an expired channel
   must not be reported as fresh. Keep owner keys outside research.
4. Complete wider home/file groups, remaining interruption/compatibility cases,
   expired-channel recovery and signing-key rotation.
5. Complete independent targets and physical qualification. Owner vault/model
   authentication, the pending Claude terms choice and actual hardware require
   their own evidence; do not invent them or guess future XPS hardware.

Promote.yml keeps public prepare/publish jobs around a protected no-checkout
Cosign signer. R1 exercised actual authority and one exact-byte recovery after
uncertain draft creation. The development repair uses authenticated draft listing,
numeric release/asset IDs, exact Git tag checks and bounded readback; it preserves
existing-version refusal and replaces only the verified mutable channel asset.
Independent source review passes, but the complete repaired v2 path, broader
publication races, renewal and rotation remain unqualified. Preserve the existing
owner backup and environment controls; do not infer another setup/approval step
from the historical preparation notes. See ADR 0023 and build/release/README.md.

Only generated guest disks are used. QEMU/OVMF are installed in existing Ubuntu
WSL2 under the owner's authorization. Never attach/format host disks, enroll the
workstation home, alter owner vault/profile state, or install repository skills
globally. Maintain worklog.md and exact-source reports at milestones.
