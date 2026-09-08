# Continue Kedra implementation

The owner requested full implementation through a usable, understandable installer,
with disposable VM testing. Public artifacts may contain reviewed project files
only. Work is on codex/usable-system; main is unchanged. Read AGENTS.md, the current
worklog snapshot and latest entries, then the relevant source/research evidence.

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
and promotion are not configured. A disabled manual release candidate workflow now
separates build/sign-image/installer jobs; see build/release/authority/README.md.
The owner authorized a new dedicated release key and GitHub deployment secrets;
public authority files and the protected kedra-desktop-signing environment are
now provisioned. Main requires CI and disallows forced/deleted history. Local
recovery files are backed up in Bitwarden and retrieval is confirmed by the owner
(2026-09-08); the agent did not access the vault. Production workflow opt-in remains
unset; no promoted media exists.
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

## Work now being qualified

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

1. Inspect current source/CI and qualify the prepared production candidate and
   protected promotion workflows. Actual signed graphical A/B/A already passes
   34243567959; shared signed installer regression 34244387167 also passes its
   build/offline-startup scope. Do not repeat or relabel these as owner-media tests.
2. Preserve signed-installer evidence; production authority/promotion, recovery
   and an owner installer are still required after the successful research install.
3. Complete text discard/activation, new-baseline transitions and wider file/group integration.
4. Build and qualify production release authority, immutable signed artifacts,
   target-bound promotion/freshness, offline recovery and understandable install
   instructions. Keep production signing keys outside research.
5. Complete RPM/no-change/failure cases and independent targets. Owner vault/model
   authentication, the pending Claude terms choice and actual hardware require
   their own evidence; do not invent them or guess future XPS hardware.

Prepared promote.yml has public prepare/publish jobs around a protected no-checkout
Cosign signing job. Exact candidate, installed source, whole ISO and qualification
are bound to owner review; versioned drafts publish before the single signed-pair
channel bundle. Actual production execution and interrupted-publication recovery
remain not-run. See ADR 0023; current main/opt-in and Bitwarden backup status must
be checked before enabling. Public key files are committed at a6cd08f.

Only generated guest disks are used. QEMU/OVMF are installed in existing Ubuntu
WSL2 under the owner's authorization. Never attach/format host disks, enroll the
workstation home, alter owner vault/profile state, or install repository skills
globally. Maintain worklog.md and exact-source reports at milestones.
