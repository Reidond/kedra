# Composition verification — 2026-10-01

Scope: dependent `codex/nix-system-composition`, inspected parent `7329ef1` (PR28).
Codex with Astra-only scoped core/adapter/E2E workers and read-only critic. No
installed authority, signed release schemas, workstation activation or OS build
pipeline replacement is part of this slice.

## Observed checks

- Pass: prior release CLI copied to `target/nix-system-baseline/sysroot`, SHA256
  `f94ec6ac1c88c96971a1dadc19e4b577c0cdd94baf2ed8a8bf7e3743c01a0981`;
  `system --help` refuses unsupported command with exit2 before implementation.
- Fail, repaired: first new E2E compile used LowerHex on sha2 0.11's output array;
  changed only fixture digest formatting to byte-wise hex.
- Pass: first full explicit native workflow, two cases, 202.60s, using
  `cargo test -p sysroot --test e2e_system --locked -- --include-ignored --test-threads 1`.
  This predates the final hardlink guard and added passthrough/symlink checks.
- Pass: workspace formatting, all-target locked Clippy with warnings denied.
- Pass: final explicit native run after hardlink guard and added symlink/
  executable passthrough checks, two cases, 263.45s. Same public command as above;
  neither native test was ignored. Existing `/usr/bin/true` bytes/mode independently
  match the retained foundation, are reported as passthrough and omitted from tar.
  `/etc/os-release` foundation symlink refuses before context publication.
- Pass: workspace ordinary E2E (engine7/source2/system1); native Docker cases
  explicitly ignored there, Linux-only home/TPM cases zero on macOS. Release build
  passes. Final workspace fmt/all-target Clippy pass. Existing release-interop and
  release-material public CLI workflows pass under uv in fresh task directories,
  with OpenSSL3.6.4. No Python sources or dependency versions changed.

Native fixtures: Docker29.4, macOS ARM controller/Rust1.98.1, exact Fedora44 ARM
`sha256:e402cca673711fee025f9ce21c6c08b1bd25ee26b3c482119c8675fbaaedbc85`,
Rust builder `sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546`
and explicit wrong Ubuntu runtime `sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3`.
Images are retained local inputs; no new host tool/package install or pull.

## Workflow coverage

SC-T1–T5 use the public CLI, generated committed repositories, compiled external
Rust authoring and actual native processes. The workflow builds/runs a real shell
package under Fedora, observes actual RPM material, emits a typed systemd reference,
exports deterministic contexts and checks complete archive content/hashes/modes/
ownership/timestamps. Dirty/untracked/ambient Git inputs are excluded. Native home
baselines remain source review material, and no home apply occurs. A changed config
produces a different input identity while prior context bytes remain equal.

Refusals cover source provenance/target mismatch, explicit replacement ambiguity,
prefix/reserved/traversal paths, unknown/missing typed references, required/removed
RPM policy, wrong Fedora foundation, mixed runtime foundations, corruption and
existing destination preservation. Review prompted strict foundation ancestor/
symlink checks and a hardlink-count guard. The latter is a preventive ABI-alias
rule; no builder-dependent hardlink corruption was demonstrated.

Manual read-only observation of retained Kedra foundation
`sha256:91e27148af14acca963b1604170407ae98813820908a23d8532b3e88364b0589`
finds `/usr/libexec/kedra-session` mode0755 and SHA256
`9d6b15593c5857ad2f4507966adcd68f979d4bc4c39050073448604753adcd18`, equal to
current committed payload. This is an exact script prerequisite observation,
not full Kedra composition or installed-session qualification.

Blocked: retaining that full Kedra foundation through public `store add-image`
refuses exit78: its exported archive lacks or mismatches descriptor blob
`4a41d6120b733411b658ed22006753ee75f326d7c4f277787b72dab4ed660ee4`.
The failed stage is removed and no image receipt/context is admitted. Read-only
Astra diagnosis confirms a 25,088-byte archive with only the correct root manifest;
the configuration and all 78 ordinary OCI TAR layers (6,415,209,984 expected bytes)
are absent. Explicit `--platform linux/arm64` export also refuses. Evidence is local
`target/nix-system-archive-diagnostic/inspection.json`. Exact digest pull from the
already observed public candidate repository succeeds with "up to date", but
repeated admission still refuses the same missing configuration blob. No context
is created. Independent complete-archive/cache restoration remains a follow-up;
validation was not weakened. The minimal Fedora fixture
archive/native workflow above passes, but current full-Kedra composition remains
unqualified until this retained-image evidence issue is resolved.

## Review and qualification limits

Pass: read-only Astra review of frozen source materialization, canonical RPM
observations, typed replacement/runtime closure checks and no-replace publication.
Source/API checks establish implementation boundaries; executable E2E establishes
only the named flows.

Not-run: actual context replay in a clean independent daemon, loading/building the
exported context, container installed-unit execution, full current-Kedra composition,
GLib/schema/initramfs regeneration, service enablement, home seeding, SELinux,
boot/update/install, signed release integration, power-loss/ENOSPC publication
windows and an actual raced-in output directory. These require the existing
container/VM harness or separate manual qualification. Package execution here
uses the original verified store; tar inspection does not qualify execution of
the exported context. No portable OS/runtime or activation pass is inferred.

Input-addressed plan/compose identity binds resolved bytes/modes, foundation/archive,
RPM material, output tree receipts and source provenance. Actual context artifact
hashes are recorded separately in the manifest; no self-referential hash is used.
