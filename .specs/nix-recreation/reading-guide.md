# Sources to use during development

Research complete for the bounded question on 2026-10-01. The four Astra owners
performed focused source analysis, with full body reading for the shorter papers
and explicitly selected chapters for the long theses/manuals. This is not an
exhaustive Nix/nixpkgs audit or current performance/security qualification.

The stable IDs below connect to [the source manifest](source-manifest.md),
[BibTeX](sources/bibliography.bib), [citation records](sources/citations.json),
[original archive metadata](sources/archive.json) and
[supplemental document captures](sources/modern-archive.json). Page references are
one-based PDF pages; printed offsets are given where important. Cite an ID, exact
page/section and canonical URL when carrying a fact into an implementation spec.
Original PDFs prevail over extracted text/OCR. Author proposals and historical
measurements are not implementation requirements.

## Start with these

1. **S-012, thesis chapters 5 and 7:** derivations, canonical objects, reference
   closure, registration/GC races and bootstrap. This is the core implementation
   reading. Use the focused [O1 notes](findings/O1-store.md) to find exact pages.
2. **S-004, sections 5–6:** typed option/module composition, static system roots
   and the boundary between immutable construction and stateful activation.
   Use [O2 notes](findings/O2-os.md); retain the distinction from the 2008 paper.
3. **S-013 and S-019:** output content identity, trusted result provenance, closing
   builder writers, current addressing/metadata/isolation boundaries. Use
   [O3 notes](findings/O3-security.md).
4. **S-003/S-008/S-011:** test environments and CI as consumers of artifact graphs.
   Use [O4 notes](findings/O4-evaluation.md) and the established Kedra harnesses.
5. **S-022/S-023:** Rust reuse and evaluator compatibility caveats. Inspect pinned
   actual components and proof scope before treating a README or language theorem
   as a working package-manager engine.

## Supplied documents

| ID / document | Development use | Evidence starting points |
|---|---|---|
| [S-001 — Secure Nix Expression Updates (2024)](sources/papers/ba.pdf) | Source update authorization, verification placement, cached fetches and stale-but-authentic revisions | PDF19–32, §§4–6; findings remain dated to 2024 |
| [S-002 — Multi-Platform Software Package Management (2011)](sources/papers/Multi-PlatformSoftwarePackageManagement.pdf) | Toolchain/ABI/platform packaging limits | PDF36–48, §§5.17–6.1; cross compilation not established |
| [S-003 — Automating System Tests Using Declarative Virtual Machines (2010)](sources/papers/decvms-issre2010-final.pdf) | Real declarative test environments and isolated mutable nodes | PDF4–7, §§III–IV; historic direct-kernel VM route is not a firmware gate |
| [S-004 — NixOS: A Purely Functional Linux Distribution, JFP (2010)](sources/papers/nixos-jfp-final.pdf) | Module fixed point, static OS artifacts, activation and persistent state | PDF17–25, §§5.1–5.2; PDF28–35, §6/§8 |
| [S-005 — Software Deployment in a Dynamic Cloud (2009)](sources/papers/icse-cloud09-final.pdf) | Separate service descriptions, infrastructure and placement; prepare closure before activation | PDF4–5, §3.2; prototype/evaluation limits in §4 |
| [S-006 — Atomic Upgrading of Distributed Systems (2008)](sources/papers/atomic-hotswup2008-final.pdf) | Distinguish profile atomicity from controlled service transition | PDF4–5, §§5–7; proxies/draining/cohort assumptions, incomplete failure recovery |
| [S-007 — NixOS: A Purely Functional Linux Distribution, ICFP (2008)](sources/papers/nixos-icfp2008-final.pdf) | Earlier composition/activation design, kernel/module/initrd and fixed-path hurdles | PDF8–11, §§5–6; do not attribute JFP's later module framework to it |
| [S-008 — The Nix Build Farm (2008)](sources/papers/buildfarm-wasdett2008-final.pdf) | CI consumes declarative builds and platform-native test variants | PDF3–7, §2; native distribution validation remains separate |
| [S-009 — Maximal Laziness (LDTA 2008 preprint; publication 2009)](sources/papers/laziness-ldta2008-final.pdf) | Evaluator/realization separation; sharing/memoization tradeoffs if a lazy frontend is selected | PDF3–6, §2; PDF7–13, §§3–5; historical experiments |
| [S-010 — Purely Functional System Configuration Management (2007)](sources/papers/hotos-final.pdf) | Early static-system model and mutable-state escape points | See O2's full-paper evidence and limitations |
| [S-011 — Automated Software Testing and Release with Nix Build Farms (2007)](sources/papers/628612.pdf) | Exact release inputs, pure artifact generation and separate upload operation | PDF76–88 / printed65–77 only; §4 printed69–73; [OCR derivative](sources/text/628612-nix-article-ocr.txt) |
| [S-012 — The Purely Functional Software Deployment Model (2006)](sources/papers/phd-thesis.pdf) | Store identities, validity, realization, roots/GC, content addressing and bootstrap | PDF98–122 / printed90–114, §§5.2–5.5; PDF133–154, §§5.6–6; PDF177–188, §7.1 |
| [S-013 — Secure Sharing Between Untrusted Users (2005)](sources/papers/secsharing-ase2005-final.pdf) | Differentiate trusted recipe-result mappings from content-addressed bytes; residual writers and import ordering | PDF4–9, §§2.3–6.1; historical model, not today's trusted-users setting |
| [S-014 — Service Configuration Management (2005)](sources/papers/servicecm-scm12-final.pdf) | Service descriptions, effectful activation, safe reload versus restart | PDF14–15, §6.2, with O2's broader service analysis |
| [S-015 — Efficient Upgrading in a Purely Functional Component Deployment Model (2005)](sources/papers/eupfcdm-cbse2005-final.pdf) | Downstream rebuild propagation; optional transfer deltas later | PDF5–12, §§2–4; experiments PDF13–14 are historical |
| [S-016 — Nix: A Safe and Policy-Free System for Software Deployment (2004)](sources/papers/nspfssd-lisa2004-final.pdf) | Separate frontend/IR, immutable install space and selected user environments | PDF6–9 / printed84–87; substitutes and profile generations |
| [S-017 — Imposing a Memory Management Discipline on Software Deployment (2004)](sources/papers/immdsd-icse2004-final.pdf) | Runtime closure, conservative scanning and relocation limits | PDF3–7, §§3–7; hidden/compressed pointers are a limitation |
| [S-018 — Integrating Software Construction and Software Deployment (2003)](sources/papers/iscsd-scm11-final.pdf) | Earlier graph/build/deployment integration motivation | PDF5–6/10–12/15–16; Maak predecessor's overwrite/barrier ideas do not satisfy the proposed strict immutable contract |

## Modern bridge sources

| ID / source | What it resolves | Limit |
|---|---|---|
| [S-019 — Nix 2.34.9 manual](https://nix.dev/manual/nix/2.34/) | Current input/fixed/floating addressing, string context/IFD, store verification, NAR, local store, roots, secrets and sandbox policy | Observed version, not newest claim; some features experimental; user docs do not fully specify crash/GC/signature internals |
| [S-020 — NixOS 26.05 manual](https://nixos.org/manual/nixos/stable/) | Actual option type/priority/order rules, service switch, boot intent and persistent state | Mutable stable alias is frozen by archived HTML hash; not executed behavior |
| [S-021 — Official VM integration tutorial](https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines) | Real-node driver, readiness checks, interactive workflow and cached test success | Tutorial's example pin is historical; not Mac/GPU/firmware qualification |
| [S-022 — Tvix packet](https://code.tvl.fyi/about/tvix) | Rust evaluator/compatibility interfaces and actual implementation boundary | Origin inaccessible; [declared mirror at 9bed4ce](https://github.com/tvlfyi/tvix/tree/9bed4ce6fc4c308f28df485b286a9aee467af686) inspected. No build/store realization, runtime/API fitness or original-depot equivalence established |
| [S-023 — Verified Interpreters for Dynamic Languages… (2025)](sources/papers/verified-nix-interpreters-2025.pdf) | Precise language semantics and production compatibility limits | Proof excludes paths/IO/derivations; no Rust port or mechanization check performed here |

## Claims that must not be promoted

- Input-addressed identity, immutable storage and a successful cached build do not
  prove independent reproducible rebuilding.
- Conservative scanning does not prove completeness for arbitrary encodings,
  compressed strings, dynamically constructed paths or outside-store dependencies.
- A NAR/package-tree digest does not attest all SELinux labels, capabilities,
  ownership and OS metadata.
- A profile pointer swap does not atomically restart services, reverse database
  migrations or restore writable home data.
- A Rust evaluator/library does not establish build/store realization, pure IO,
  nixpkgs compatibility or native platform behavior.
- Historical timings, counts and prototype successes do not qualify this engine.

Unreviewed future leads are recorded in the manifest's handoff section. Native
Linux scope avoids making a current cross-compilation compatibility promise. Full
Nix implementation protocols, broader package ecosystems and current security
advisory status require separate evidence if they become requirements.
