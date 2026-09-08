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
and promotion are not configured. No workstation enrollment has occurred.

R01 native signed helper run 34226367288 at 71916b9 passes older-media enrollment against a newer fresh channel, metadata/OCI
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
Actual desktop 34223972271 at a5cd96e passes stale-plan refusal, discard, metadata
and selection retention and installed doctor. Later 34226367519 passes discard,
Codex and Bitwarden but times out at bus checks after audio output. New 94678f6
adds bounded portal/keyring calls and markers, inherited error traps and a required
portal response in doctor. R07 34228725414 is pending. Broader crash/recovery and
baseline-transition cases remain open; no unit/meta harnesses were added.

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

1. Inspect current source/CI, including the bounded native bus checks and signed
   helper regression. Fix observed failures without
   weakening runtime signature/path/state validation or adding unit/meta tests.
2. Preserve signed-installer evidence; production authority/promotion, recovery
   and an owner installer are still required after the successful research install.
3. Complete real home activation/recovery behavior and generic file/line integration.
4. Build and qualify production release authority, immutable signed artifacts,
   target-bound promotion/freshness, offline recovery and understandable install
   instructions. Keep production signing keys outside research.
5. Complete RPM/no-change/failure cases and independent targets. Owner vault/model
   authentication, the pending Claude terms choice and actual hardware require
   their own evidence; do not invent them or guess future XPS hardware.

Only generated guest disks are used. QEMU/OVMF are installed in existing Ubuntu
WSL2 under the owner's authorization. Never attach/format host disks, enroll the
workstation home, alter owner vault/profile state, or install repository skills
globally. Maintain worklog.md and exact-source reports at milestones.
