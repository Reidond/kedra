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


## Native signed workflows — 2026-10-03

**Pass:** existing `cache_recovery::signed_closure_refuses_untrusted_changes_then_runs_without_producer`
and `reuse::independent_consumers_authenticate_cache_before_store_admission`
each pass1/1, zero failed/ignored, in52.09s and67.71s. Both actual parent waits0;
guards54.309s/69.300s. Exact source is `2b52355764d0e82d26010af437a79c9f4a7ca4da`,
CLI SHA `63c2f608452c8eafe7031f5eacc3ee51c761fd831bf3dcc29b6d6a78aa9d229b`,
E2E SHA `8b8a33cbc446b2e9c88d2a8a75d222ee5a23d1d2970051b00eb521fa87d72d7d`.

Actual coverage includes FIFO and authentication refusals before admission,
signed import, producer-absent execution/reuse and forced rebuild in the signed
case; independent namespace/package/key/recipe ownership, producer-absent result43
and unrelated-project preservation. The independent case does not rebuild.
All543 frozen source files and binaries remain unchanged; temporary roots are
empty and owned host process groups/containers absent. Peak charged allocations
are10,312,855,552 and10,390,278,144 bytes, with the original capacity floor intact.
Aggregate `target/nix-delivery/d5-signed-cache/result.json` SHA256
`cad381235f4734043ba192007b0b2b60cbbd52ea337f6e61566d06c2cc8045e1`
binds guards and stdout/stderr. Both cases use the retained native ARM daemon;
no cold-daemon equivalence is claimed. Context lease interruption, native fault
windows and Linux ENOSPC remain separate unpassed gates.


## Interrupted context lease recovery — 2026-10-03

**Pass:** existing public `killed_context_copy_recovers_without_removing_a_stopped_reader`
passes1/1 in6.46s, actual parent wait0 (guard8.115s), on immutable source
`2b52355764d0e82d26010af437a79c9f4a7ca4da`, CLI63c2f608 and context E2E SHA
`41f350b6db221879533224f77794cdb2d4301887b4077408491b98c081a83541`.
Composition is91eff75d. Two actual SIGSTOP readers have copied92,340,224 and
116,260,864 bytes; the abandoned child wait records signal9. Public recovery
removes only abandoned state, preserves the stopped live reader and foreign
sentinel, and the survivor resumes successfully. Owned temporary root is empty,
process group absent, binaries unchanged; peak charged allocation12,245,504,000
bytes with the original capacity floor intact. No daemon mutation or VM check.
Aggregate `target/nix-delivery/d5-context-lease/result.json` SHA256
`0ae779bb60ada1f23e88c67d7019daba59f01dd8465b2ab92d0b4f4e56476c60`.
Native publication fault windows and Linux ENOSPC remain separate unpassed gates.


## Bounded Linux ENOSPC recovery — 2026-10-03

**Pass:** existing `cache_recovery::full_bounded_store_recovers_and_retries_real_import`
passes1/1, zero failed/ignored, in1.07s (guard3.390s). Exact source
`2b52355764d0e82d26010af437a79c9f4a7ca4da` is freshly compiled for native Linux
ARM64 with pinned Rust1.98.1, offline/locked Cargo and readonly source/vendor.
CLI SHA `265b0e92bb18b58038691265136298780760e97df75720f3f65369b39975276f`;
E2E SHA `6d733da3540af5be388105317c08c77be88913f350f51cfa5747d6353c71bfec`.

Actual64MiB tmpfs exhaustion refuses public source import, preserving an already
admitted root and foreign sentinel. Writable proof/TMPDIR resides on a different
device. Removing owned filler, public recovery and successful import retry pass.
Both compile/case host-attach and container waits are0; exact owned containers
are removed without volumes, process groups absent and pinned binaries unchanged.
Case peak charged allocation10,457,255,936 bytes respects the existing floor.
Aggregate `target/nix-delivery/d5-enospc/result.json` SHA256
`a9e1482de2e19d94bd0e4a90bd7c848de514a77c47e1d3d5b3264f560de9651f`
binds the source/toolchain/ELF handoff, phase guards, assertions and cleanup.

The linked readiness-sideeffect receipt explicitly records rustfmt/Clippy
component acquisition caused by the existing disposable-controller rustup shim.
The compiler itself predates that invocation; subsequent compilation uses direct
offline executables, with no workstation installation. This case qualifies
source-copy/admission failure only, not metadata publication, Docker-layer
exhaustion or power-loss durability. Four native interruption windows remain
pending; debugger readiness alone does not qualify them.
