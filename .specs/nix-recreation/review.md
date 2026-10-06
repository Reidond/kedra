# Proposal review and actual checks

2026-10-01. Research/proposal only; inspected Kedra base
`b224d5711e857f7dbcaabf7ed42870916525800c` on `main`.

Six Astra workers were used: one source planner, four disjoint source owners and
one read-only plan critic. All source owners completed focused analysis of their
assigned packets. The parent preserved sources, assembled citations and wrote the
proposal/roadmap; no worker changed shared project status or the source archive.

## Critic findings and resolution

| Finding | Resolution in revised proposal/roadmap |
|---|---|
| Empty receiver could inherit undeclared loader/libc | Require store-contained runtime dependencies or a digest-bound runtime root transferred with execution metadata. Missing/wrong runtime refuses; no undeclared receiver userspace. |
| Multi-output requests could duplicate realization or expose partial siblings | First version supports one output per derivation; later multiple outputs require derivation-level coordination and validated output-set registration/recovery. |
| One sentence equated immutability with reproducible rebuilding | Selected static artifacts, independently reproduced bytes and mutable-state recovery are now separate claims. |

The final Astra readback found no remaining actionable contradiction or blocker
to presenting the architecture discussion. This is a document-review conclusion,
not implementation approval or runtime qualification. Prefix, bootstrap and
frontend/backend choices remain visible decisions for the detailed next spec.

Additional source-grounded corrections include preserving final logical output
paths inside private build namespaces, closing residual writers before sealing,
authenticating OS metadata beyond portable NAR-style trees, retaining source
authorization independently of cached bytes, and avoiding a custom TOML language
under the existing no-custom-language contract. Current Tvix reuse is scoped to
the inspected mirror/evaluator interfaces, not an available realization engine.

## Actual verification

- **pass:** initial clean Kedra and Kedranix checkouts; local/remote Kedra `main`
  equal at the inspected base. Exact prior workspace CI
  [36774345545](https://github.com/Reidond/kedra/actions/runs/36774345545) was read
  live and passed at that base; no new-revision CI result is claimed.
- **pass:** all 18 supplied PDFs copied byte-for-byte with SHA-256 metadata;
  `shasum -a 256 -c SHA256SUMS` checks all originals. An initial modern checksum
  readback exposed five alias paths using the wrong relative prefix; metadata was
  normalized, document bytes were unchanged, and the final
  `shasum -a 256 -c modern-SHA256SUMS` passes every captured payload.
- **pass:** 23 selected source packets have completed structured analysis, with
  explicit long-document coverage limits. Modern source content, access dates,
  URLs and observed versions/revisions are preserved. S-011's thirteen scanned
  article pages were visually read; searchable OCR and method metadata retained.
- **pass with disclosed capture limit:** Tvix origin was inaccessible; declared
  pinned mirror used. Its historical blog's raw HTML fetch failed, and a labeled
  web-rendered text capture is retained. No original-depot equivalence is claimed.
- **pass:** standard `git diff --check` for tracked project-status/worklog changes;
  manual review of new research documents, bibliography and source ownership.
- **not-run:** new Rust engine compilation, behavior tests, benchmarks, container
  qualification, OS image/signing pipeline, boot, installation or deployment.
  Those are future gates; no production code was changed.
- **not-performed:** commit, push, package installation or any write to Kedranix.

Next: settle the native aarch64 first-slice prefix/bootstrap/interface and write
the implementation requirements/design for slices 0–1. Broader OS composition,
x86_64 support and full `.nix` compatibility have their own declared gates.
