# D5 owning-source qualification

Adopted 2026-10-03 on `codex/nix-cache-recovery` above D4 draft PR36
`d591d2e0865cbaa68662ae7f50a6c398dab183dc`. The seventeen-path source delta
comes from reconciled manifest
`ed7333434da66f798a569568bc9e8bc0f4baa9cb9a8f9a2f591fec400f71c14f`.
All base/prepared hashes matched before adoption. The first current fmt check
fails on two layout-only chunks in the newer ENOSPC TMPDIR fixture; standard
Cargo formatting corrects them. No behavior or admission guard changes.

## Actual gates

- Pass: current Rust 1.98.1 workspace fmt and all-target Clippy with warnings denied.
- Pass: full ordinary CLI E2E, 13 passed and 11 ignored on macOS. Linux-only
  ENOSPC is not compiled/executed by that host result. Workspace release build
  passes in 49 seconds. Final mode-0500 host binaries are pinned in
  `target/nix-delivery/publication/d5-binaries`; sysroot SHA256 is
  `68e0e9b486ca78adacd602fec729bb68cf7285442c9b6b35b2e7547f67eadd9a`.
- Pass: repository Ruff and whitespace checks.
- Pass: real public `store substitute` rejects FIFO receipt (0.432 s, exit78),
  symlink policy (0.024 s, exit1) and hardlinked key (0.025 s, exit78), with each
  destination absent and every generated input preserved. Each process has a
  five-second outer deadline. Pinned current debug CLI hash before/after is
  `e96fbd9c0b4b7f86120227ad0ae9ef79cbd3070260abea68d3c713290b54a1cf`.
  Receipt: `target/nix-delivery/publication/d5-input-refusals/results.json`.
  This qualifies bounded metadata refusal, not valid signed cache import.
- Source review: focused independent Astra review of the immutable manifested
  packet reports no verified blocker in cache scope/key/revision/expiry/expected
  recipe, private proof before journal admission, lease lifetime or import.next
  recovery. Actual CLI/independent-consumer and synchronous build/archive callers
  were inspected. This is source evidence, not runtime qualification.

## Remaining required execution

Signed native substitution and independent-consumer cache policy/transfer, actual
stopped-reader/SIGKILL lease recovery, four native publication fault windows and
real bounded Linux ENOSPC remain not-run. The source-only preparation and compiler
checks cannot satisfy these cases. Source-input FIFO refusal above is narrower
than full archive import or native snapshot recovery.

D2 fresh media/install/A/B/A and D3/D4 native applications/lifecycle also remain
separate pending gates. Protected production signing is not exercised. See
[test plan](test-plan.md) and [manual procedure](manual-faults.md).

## Impact and review boundary

Cache policy/proof APIs are consumed by public CLI substitution and the generated
independent consumer. Bundle import rechecks authenticated exact root/hash/bytes
and expiry after staging and before the first journal/admission. Existing unsigned
explicit-hash import remains separate. Snapshot guards are consumed by verified
composition, static replay and native derivation; public `system recover` exposes
recognized abandoned cleanup. Callers finish only after synchronous Docker/context
consumers return. This inspected dependency set is a source-review floor; actual
crash/stop/ENOSPC execution remains required. Generic engine dependencies add the
existing pinned p256/base64 crates without a sysroot-core or OS authority coupling.

Additional public import refusal checks pass with the same pinned debug CLI:
FIFO bundle (0.068 s), symlink bundle (0.074 s) and directory bundle (0.083 s)
all refuse, preserve a previously admitted/verified source root and leave no
import journal. `publication/d5-input-refusals/bundle-results.json` records the
actual exits/diagnostics. The first manual add-source invocation refused an absent
store; explicit public `store init` corrected the invocation before these checks.
This is a fixture invocation correction, not a product change.

Final legacy checks pass: independent OpenSSL release interoperability and both
target release-material workflows run against the D5 source/pinned release CLI.
Receipts are `target/d5-publication-release-{interop,material}`. Two updated
external cache consumers also compile and directly reject FIFO metadata and
foreign cache scope before store access; results and executable hashes are in
`publication/d5-consumers/results.json`. The first manual scratch Cargo manifest
omitted rustix already present in the tracked generator; correcting that scratch
manifest yields both builds without changing product source. D4 binaries retain
their original hashes; these new binaries have distinct `-cache` names.

Remote D2 native failure is independently diagnosed as Quay CDN blob download
EOF while pulling the pinned amd64 builder in run37107489350/job111158768746,
before fixture keys/registry/VM. The manifest resolved; no source defect or
missing digest is established. Its concurrent ARM job remains in progress.
