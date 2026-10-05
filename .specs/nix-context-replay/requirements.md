# Verified context replay

Authorized continuation: owner "do next", 2026-10-01. L3 above PR29/3350732,
branch `codex/nix-context-replay` in the existing gh-stack.

- CR01: restore complete native signed Fedora/Kedra foundation data without
  relaxing image admission or installed signature policy. Candidate thin-cache
  failure remains historical; use reviewed signed production9d6eb030 when needed.
- CR02: reusable store-independent Rust context verification requires a separate
  expected input identity, strict bounded models, complete foundation graph,
  actual artifact hashes and semantically verified config/object payload trees.
  Recomputed archive hashes alone must not admit changed payload semantics.
- CR03: consumers use a private verified snapshot, not mutable paths reopened
  after validation. Refuse links/special files, missing/extra members, unsafe modes,
  namespace collisions and inconsistent plan/receipt/reference material.
- CR04: the sanctioned harness loads the exported foundation and builds the exact
  static context with pulls/network disabled on its one selected API endpoint.
  Reject inherited ONBUILD/volumes and wrong cached tags/provenance. Normal OS
  source overlays and binary overrides do not apply to a replay request.
- CR05: after producer source/store removal, run a compiled executable with its
  runtime dependency from the exported image. Then verify a generated native unit
  through the existing container harness: native syntax check, start, successful
  status and expected output/config/identity. Ordinary suites stay applicable.
- CR06: publish reviewed implementation/evidence as the next dependent draft
  through gh-stack; preserve all installed helper/release/home contracts.

No boot, SELinux enforcement, installation, signed publication or privileged
activation is implied. Lab-tools adaptation performs its existing DNF operations
and is separate from the static offline replay; it is not an offline claim.
