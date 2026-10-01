# First engine implementation evidence

Publication note: the owner authorized a draft PR on 2026-10-01 after these local
checks. This dated record describes the verified working tree; subsequent exact
commit, PR and CI observations are recorded in worklog/status. Raw implementation
references in `sources/` remain optional local captures; URLs/hashes are published.

2026-10-01, Codex coordinating Astra implementation/review workers. Base
`b224d5711e857f7dbcaabf7ed42870916525800c`, local uncommitted source. This qualifies
the first private engine slice, not all Nix functionality or an OS replacement.

## Actual checks

| Check | Result / practical boundary |
|---|---|
| Preserved baseline new-command invocation | red: original CLI exits2 for `store init`; baseline binary SHA256 e6f1402b0da23bab57e5b98974beb2cef9f666cd0be7ff009e3dd73dc82bd78d |
| First coordinated engine execution | pass: seven cases, zero ignored,248.60s; subsequent independent review prompted stronger image/recovery/context mechanisms |
| Hardened full engine run | nine passed; one fixture failed before execution because its nonexistent Unix socket URL exceeded macOS path length;639.50s |
| Repaired context-only execution | pass: one executed, zero ignored,nine filtered;81.09s. Only the fixture URL was shortened; personal Docker context was never changed |
| Standard workspace CLI E2E | pass: engine seven ordinary cases/three explicitly ignored, plus two source workflow cases on macOS; Linux-only home/TPM cases are zero executed here |
| Native Linux ARM controller compile/filesystem E2E | pass: Rust1.98.1 in retained native ARM image, read-only selected compiler snapshot, vendored locked dependencies, network disabled, nonroot502. Seven ordinary cases pass/three Docker cases ignored;19.54s compile,9.48s cases |
| Format / workspace all-target Clippy / release build | pass with locked dependencies, including current CLI status inventory. These are compiler/lint gates, not runtime/boot proof |
| Independent OpenSSL release interoperability | pass using current release CLI and fresh `target/nix-engine-release-interop` fixtures |
| Existing sealed release-material workflows | pass for desktop/qemu-arm64 scope, input changes/no-change, replay/rank and tamper/architecture/target refusals in fresh `target/nix-engine-release-material` |
| Independent final implementation review | no remaining high-signal blocker found after six corrective mechanisms; read-only review does not expand runtime coverage |

All ten engine cases were executed successfully across the hardened full run and
the targeted fixture correction. This exact history is retained rather than
describing a nonexistent single all-ten-green invocation. Default Cargo ignores
the three Docker cases; those were explicitly executed separately on this Mac
controller with native ARM Linux Docker29.4.0.

The initial source-admission failure exposed Darwin cross-parent rename behavior
for sealed0555 directories. Publication was corrected without weakening byte/
reference validation. Standard release-script paths were already occupied by
earlier fixtures; fresh task paths were used. Native Linux initially requested
missing formatter/linter components through rust-toolchain.toml; selecting the
already installed exact1.98.1 Linux toolchain avoided any tool installation. Linux
vendoring fetched existing locked platform dependencies; Cargo.lock external
versions did not change.

## Observed behavior

- A generated separate Rust consumer authored the graph through the public API.
- A real C shared library and executable compiled under the declared builder,
  then ran on the separately declared Ubuntu runtime. Two versions coexist.
- Reuse and independent fresh rebuild were distinguished; divergent output kept
  the original winner and reported failure.
- Runtime exports contain the executable/library and runtime foundation, with
  compiler sources/image omitted. New receiver stores execute after producer
  paths/source directories are unavailable; network is disabled.
- Profile switching, child status7, rollback and declared-environment command
  execution are observable. Profile/engine operations do not install an OS.
- Corrupt executable/runtime evidence, wrong bundle hash, unknown members and a
  self-consistent but wrong claimed image archive refuse.
- Builder/root/input constraints, deadline/log draining, source/path/symlink
  refusal, whole-graph cycles and shared source DAG behavior were exercised.
- GC/use serialization, actual killed-controller recovery, actual interrupted
  tombstone deletion recovery and separate sentinel bytes/modes were checked.
- Private Docker config switched to an unreachable context mid-operation;
  completion/cleanup and later killed-operation recovery stayed on the recorded
  endpoint. No real user context configuration was modified.

## Review corrections and dependencies

The new public callers are the CLI adapter/main dispatch and generated Rust
authoring consumer. Internal consumers are graph/tree admission, executor,
bundle, profile and GC/recovery modules. Existing source/release commands were
checked by ordinary workspace cases and release scripts. Installed helper,
targets, signing workflow, home/native configuration and provisioning code were
not modified. This inspected caller list is a static floor, not proof about every
possible external library consumer.

Corrective review covered source root cleanup without following symlinks,
intermediate link escapes, disconnected graph cycles, capture metadata changes,
directory bounds, source-bridge runtime consistency, exact transaction names,
image archive identity graphs, recoverable collection, staged first profiles,
shared bundle traversal memoization, endpoint pinning and non-Unix cfg boundaries.
The shared source-DAG case is behavioral evidence for store traversal/GC; bundle
memoization is source-reviewed and must not be credited with that case's coverage.

## Remaining limits / not-run

Power-loss and ENOSPC publication windows, the complete TC16/TC19 authority/reference
matrices and every adverse subvariant are not-run. Cold independent Docker-daemon
load was not exercised; runtime archive graph completeness is validated and a
fresh receiver store with shared daemon was exercised. Compressed layer hashes
are checked; Docker performs decompression/DiffID verification. No claim of
general reproducibility, complete conservative scanning or hostile-owner/daemon
isolation is made.

Native x86_64, non-Unix compiler execution, native Darwin package builds, remote
Docker endpoints/caches, signed substitution, full Nix language/nixpkgs, broad
package ecosystem, declarative OS compiler and OS-backend replacement remain
subsequent phases. Firmware, boot, installation, real home/vault/account and new
signed release/deployment were not run for this engine. Existing published OS
evidence is not borrowed for those new capabilities.

## Closure self-challenge

The second review changed image/recovery/context mechanisms after a first green
run; the new cases and final readback checked those corrections. All claimed gates
finished; actual executed/ignored counts and different platform scope are above.
Unknown future consumers and unrun fault windows are explicit. The implementation
remains local; no commit, push or package/image publication occurred.
