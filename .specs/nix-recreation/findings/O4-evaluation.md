# O4 — Evaluation, CI, platform boundaries and qualification

Research date: 2026-10-01. Inspected Kedra base: `main` at `b224d5711e857f7dbcaabf7ed42870916525800c`. This is a research/proposal result, not implementation, migration, a benchmark, or a qualification pass. Kedra is the target; Kedranix is reference only and a possible later consumer after rewriting. Source ownership is static: only S-002, S-003, S-008, S-009, S-011, S-021 and S-023 were analyzed. Other-owner conclusions below are identified as handoffs.

## Recommendation

Put a shareable, typed derivation/store contract below Kedra policy. A reviewed Rust authoring API can produce that contract without introducing a new configuration language. Keep full Nix-language compatibility a separately scoped adapter and product commitment. The existing architecture explicitly excludes a custom configuration language; adopting one would require changing that owner decision. Writing ordinary Rust configuration code is also not equivalent to a pure or sandboxed evaluator: it can read files, use the network and execute processes unless isolated.

The reusable contract should distinguish **evaluation** (produce and validate requested build descriptions), **realization** (obtain/build their outputs), **composition** (create an OS artifact), and **activation** (change a running or booted system). This division is a design inference from S-009's description/build split, S-008/S-011's CI layers and Kedra's current release boundary. It does not assert that arbitrary Nix evaluation is free from realization: O3's modern-manual handoff documents import-from-derivation.

## Per-source evidence

### S-002 — Multi-Platform Software Package Management

[Verified preserved primary PDF](https://github.com/qknight/Multi-PlatformSoftwarePackageManagement/raw/master/Multi-PlatformSoftwarePackageManagement.pdf). Joachim Schiele, diploma thesis, Universität Tübingen, 26 March 2011; 69 PDF pages. Focused coverage, not exhaustive: §§4.3–4.4 (printed10–11 / PDF11–12), §§5.17–5.18 (printed35–36 / PDF36–37), chapter6 (printed37–47 / PDF38–48), chapter7 introduction (printed48 / PDF49), and chapter8 (printed59 / PDF60). Read from preserved text and selected original PDF text ranges.

- **Supporting; high confidence:** Its Evopedia deployments show that shared application source still needs platform-specific toolchains, dependency packaging and distribution adapters. The native Mac example needs bundled libraries; the Windows experiment encounters loader/name-resolution problems. Evidence: §§6.1.5–6.1.7, printed42–45 / PDF43–46.
- **Supporting; high confidence:** The libpng examples distinguish changing a dependency from mutating an existing store object; preserving prior artifacts protects rollback while dependents select revised inputs. Evidence: §§5.17–5.18, printed35–36 / PDF36–37.
- **Limitation; high confidence:** Chapter8 explicitly says cross-compilation was not investigated sufficiently. Its 2011 Nix/Cygwin/Windows and Mac observations cannot establish current support or a modern build/host/target model. Chapter7 proposals are proposals, not implemented results. No current legal, security or package-manager advice is taken from its dated comparisons.

**Inference:** Portable Rust libraries do not make Linux artifacts runnable on macOS, remove ABI dependencies, or supply OS activation. Separate execution-platform identity from output-platform identity and validate actual load/run behavior.

### S-003 — Automating System Tests Using Declarative Virtual Machines

[Verified preserved primary PDF](https://nixos.org/~eelco/pubs/decvms-issre2010-final.pdf). Sander van der Burg and Eelco Dolstra, ISSRE 2010; full ten-page paper read, including limitations and evaluation. Page references are one-based PDF pages.

- **Central; high confidence:** Declarative machine/network definitions plus an imperative driver provision the real services, users and topology needed for repeatable system tests. Tests and their environments can be build-graph artifacts. Evidence: §§III–IV, PDF4–7.
- **Supporting; high confidence:** The implementation separates reusable immutable software from per-VM mutable storage, and uses isolated control sockets instead of fixed host ports. Evidence: §III-B, PDF5. Its host-store-sharing/direct-kernel-boot implementation is historical, not a suitable substitute for Kedra firmware/install gates.
- **Limitation; high confidence:** The system targets NixOS guests and noninteractive tooling; its small-network model runs VMs on one host and does not establish large-scale distributed testing. The reported timing experiment used five runs on a specified 2010-era host. Evidence: §V PDF7–8, §VI PDF9, §VII PDF10. Those timings are not a forecast for today's Kedra loop.

**Inference:** Preserve the configuration-to-test-environment connection, but implement it through Kedra's already sanctioned Testcontainers/QEMU workflows. Explicit readiness, observable outcomes and isolated mutable state matter more than adopting the historical test-driver language.

### S-008 — The Nix Build Farm: A Declarative Approach to Continuous Integration

[Verified preserved primary PDF](https://nixos.org/~eelco/pubs/buildfarm-wasdett2008-final.pdf). Eelco Dolstra and Eelco Visser, WASDeTT 2008; full nine-page paper read.

- **Central; high confidence:** Build/test variants and their toolchain dependencies are described as functions and derivations. Nix supplies dependency placement and platform-appropriate execution; CI consumes those descriptions. Evidence: §2 PDF3–7, especially the PatchELF variants and distribution VM examples.
- **Supporting; high confidence:** A successful Nix-environment build does not prove that source builds with a distribution's native paths/tools; native RPM/Debian builds therefore use generated VM environments. Evidence: §2 PDF5–7.
- **Limitation; high confidence:** Configuration-space search and failure isolation are future work, not demonstrated automatic intelligence. Evidence: §4 PDF8. Package counts and C++/ATerm implementation details describe 2008, not current Hydra or Rust performance.

**Inference:** Reuse GitHub Actions as the CI consumer. Do not add a CI server, fleet manager or automatic configuration-space explorer to the core.

### S-009 — Maximal Laziness — An Efficient Interpretation Technique for Purely Functional DSLs

[Verified preserved primary PDF](https://nixos.org/~eelco/pubs/laziness-ldta2008-final.pdf). Eelco Dolstra; LDTA 2008 preprint dated 16 February 2008, later ENTCS publication in 2009 (event and publication years differ). Full fifteen-page paper read. DOI is omitted here because it was not independently checked in this packet.

- **Central; high confidence:** Expression evaluation yields a graph describing imperative build actions; evaluating an unused package need not construct its requested build graph. Evidence: §2 PDF3–5.
- **Supporting; high confidence:** Maximal sharing plus memoization can reuse equal closed-term evaluations. Closure updating is a distinct implementation approach. Naive substitution still performs badly; closed-term optimization is essential in the paper's experiments. Evidence: §§3–4 PDF7–11, §5 PDF12–13.
- **Limitation; high confidence:** Memoization does not automatically solve non-strict function arguments, low-sharing inputs can grow memory, and the reported speedups concern an old interpreter and hardware. The simplified semantics deliberately omits some error cases. Evidence: §2.2 PDF5–6, §4.3 PDF11, §5 PDF12–13. This is not proof that hash-consing every Rust value is optimal or that the historical semantics fully defines current Nix.

**Inference:** If compatibility is later required, represent suspended computations and their environments deliberately; distinguish evaluator closures from a store's transitive runtime closure. Benchmark a real workload before choosing aggressive memoization, and bound evaluation resources. A Rust authoring frontend can instead request explicit graph roots and avoid implementing a lazy language at all.

### S-011 — Automated Software Testing and Release with Nix Build Farms

[Verified preserved original proceedings PDF](https://pure.tue.nl/ws/files/2239914/628612.pdf). Eelco Dolstra and Eelco Visser, VVSS 2007, printed65–77 / one-based PDF76–88. Only this article was analyzed. All thirteen pages were rendered with `pdftoppm` at 180dpi and visually read. A native macOS Vision OCR derivative was also produced at `/tmp/kedra-o4-research/vvss-ocr.txt`; page markers map to the original PDF. The original is authoritative. OCR punctuation/code syntax can be inaccurate; claims below were independently checked in the page images. This resolves the original archive's missing article text.

- **Central; high confidence:** The prototype consists of a generic job supervisor plus Nix-driven build scripts. The release-page derivation creates artifacts; a separate script performs uploads. Evidence: §4 printed69–73 / PDF80–84, particularly printed70–72.
- **Central; high confidence:** Release records replace moving source references with exact revisions and hashes, making the input description reusable. Evidence: §4, “Reproducing releases,” printed73 / PDF84.
- **Supporting; high confidence:** The authors explicitly limit claims from Nix toolchain builds to that environment; platform-native deployment requires separate evidence. They also report serialized prototype jobs and poor utilization, so the paper is not evidence of an advanced scheduler. Evidence: §5 printed74–75 / PDF85–86.
- **Limitation; high confidence:** Its channel naming, release retention, uploading and impure-install suggestions are historical mechanisms, not recommendations for Kedra's signed release/security policy. Automatic exploration of configuration variants remains future work (§6 printed75–76 / PDF86–87).

**Inference:** Retain an exact-input receipt and artifact/result status per target. Build success must not confer publication or activation authority. This article adds these implementation details beyond S-008 and is not treated as its duplicate.

### S-021 — Integration testing with NixOS virtual machines

[Verified live official tutorial](https://nix.dev/tutorials/nixos/integration-testing-using-virtual-machines), accessed 2026-10-01; undated mutable page. Authors credited on page: @olafklingt and Domen Kožar; editor Valentin Gagarin. Entire tutorial read; no linked manual was reanalyzed.

**Central; high confidence:** `testers.runNixOSTest` combines named machine configurations and a Python test script, using QEMU and readiness/outcome checks. It supports multiple machines and an interactive driver. Evidence: Introduction, “The testers.runNixOSTest function,” “Interactive Python shell,” and “Tests with multiple virtual machines.” Successful results are cached, so invoking an unchanged test can reuse evidence rather than execute it again (“Re-running successful tests”). The example still references `nixos-23.11`; it is not a current recommended pin. The page describes Linux prerequisites, with macOS support called undocumented in footnote1, and has an acceleration caveat. It does not qualify Kedra's Mac runtime.

**Inference:** Reports must say executed versus reused. Repeated timing, flaky-network, security-boundary and hardware checks need deliberate fresh runs with actual environment identity. No tutorial command was executed here.

### S-023 — Verified Interpreters for Dynamic Languages with Applications to the Nix Expression Language

[Verified author-hosted PDF](https://robbertkrebbers.nl/research/articles/nix.pdf). Rutger Broekhoff and Robbert Krebbers, *Proc. ACM Program. Lang.* 9, ICFP, article268, August2025, 30pages; DOI [10.1145/3747537](https://doi.org/10.1145/3747537). Focused technical reading: §§1–5, §§7–8; mechanization implementation is not independently proof-checked.

**Central; high confidence:** The Rocq proof relates a defined operational semantics to its interpreter, covering termination, faults and divergence (§4.4, PDF19–20). It is not a proof of equivalence to the production package manager. Against Nix2.25.0, 108 of 182 language tests are supported and 103 agree (§5 PDF21–22). Five disagreements concern call-by-name inefficiency, cycle detection and lazy `with`. Paths, filesystem I/O, derivations and several specialized builtins are excluded. The frontend uses OCaml parser/elaborator glue. The paper also corrects older binding/divergence semantics (§7.2 PDF25–26). Lazy sharing, broader differential testing and remaining semantics are future work (§8 PDF27).

**Inference:** Treat this as a precise semantic reference and warning about compatibility scope; neither a Rust port nor a subset passing examples inherits the proof.

## Proposed shareable Rust boundary

The following are architecture recommendations, not APIs implemented or performance demonstrated in this research.

| Boundary | Proposed responsibility | Kept outside |
|---|---|---|
| Typed graph model | Sources, builder executable/arguments/environment, declared input/output references, platform constraints, requested roots, schema/versioned canonical representation | Network, process spawning, secrets, machine mutation |
| Frontend | Trusted Rust authoring and translation to validated graph; optional separately scoped Nix adapter | Store authority or implicit root privileges |
| Store/realizer | Validate graph, resolve input closure, obtain/build outputs, verify and register immutable artifacts | OS service restart policy, personal home edits, release approval/signing keys |
| OS composer | Target-specific rootfs/configuration/boot artifact derivations | Live machine activation |
| Kedra adapter | Target selection, release identity, signed OCI policy, bootc/helper integration, persistent-state boundaries | Generic library policy tied to one user's home or repository |
| CI adapter | Submit requested graph roots to existing Actions jobs; collect exact-input receipts and results | New scheduler service or fleet protocol |

Keep two closure concepts explicit: an evaluator closure captures lexical bindings; an artifact closure is the transitive set of store objects needed to build/run something. Avoid passing untyped filesystem strings where an output reference is intended. An illustrative `ArtifactRef`/`OutputRef` plus structured command arguments can preserve dependencies until lowering. Nix compatibility would additionally need its actual string-context propagation and coercion behavior.

**O3 handoff, not independently reanalyzed by O4:** Nix2.34.9 `language/string-context` treats a string as characters plus dependency context, including output references and derivation-deep closure; discarding context loses guarantees. `language/import-from-derivation` allows evaluation to pause for realization and documents `allow-import-from-derivation=false`. Cite O3's finalized source ledger for exact URLs. Therefore the proposed restricted frontend should accept declared immutable reads only, refuse undeclared ambient filesystem/environment/network inputs, and disable evaluator-triggered builds initially. This is an intentional compatibility restriction, not a statement about all Nix behavior.

A typed Rust interface alone enforces none of those ambient-I/O restrictions. Trusted authoring code needs a documented trust boundary; untrusted authoring needs an isolated process and a deny-by-default input interface. Keep fetch operations explicit and digest-bound, separate from ordinary builders. Process isolation, input validation and store integrity remain necessary even if the evaluator itself is memory-safe.

## Platform and ABI contract

Model at least: **build** = where build tools execute; **host** = where the resulting program executes; **target** = code-generation destination for tools such as compilers. These definitions are a proposed compiler-toolchain contract here; S-002 is insufficient evidence for current Nixpkgs cross-compilation semantics. Ask the coordinator before adding an authoritative current cross-compilation source.

Cache/build identity must include relevant architecture, OS, ABI/libc, toolchain/sysroot and platform constraints. Declare executable build tools separately from output/runtime dependencies. Cross compilation is not foreign-program execution: any configure-time execution must use a declared emulator or a valid target runner, or refuse. Rust core reuse on macOS and Linux does not imply the same sandbox, dynamic linker, filesystem naming behavior, OS image format or activation backend. Initially qualify Kedra's declared Linux targets using matching builders; keep Mac as an authoring/control host unless native Mac builds are explicitly added. Avoid promising Windows support from a 2011 experiment.

## Proposed E2E/manual qualification matrix

All rows below are **not-run** for the proposed replacement. They extend the existing sanctioned harness, not a new runner. Assertions concern public CLI results and actual installed-system outcomes. No unit/model/mock/doctests or source/manifest/skill scanners are proposed.

| Area | Real workflow and required evidence | Existing layer |
|---|---|---|
| Evaluation/realization boundary | Public plan command with unavailable network and a builder that would create an observable file; planning must not run it. Undeclared input attempts refuse; requested roots omit unrelated variants. | Host CLI E2E with disposable inputs/processes |
| Dependency/closure fidelity | Build a real tiny linked program and revised library; execute both generations, export/import required artifacts, remove unavailable original build tree, then execute imported program. Wrong/missing references refuse. | Host CLI E2E + Linux containers |
| Identity and reproducibility | Same declared inputs in independent clean stores/builders; compare resulting artifact bytes and execution. Change source/toolchain/feature input; verify revised identity and correct reuse. Separate cached success from fresh reproduction. | CLI E2E + existing Actions builders |
| Build containment | Builder attempts undeclared file/network access, writes outside output, interruption and partial output publication; verify denial, no valid partial artifact, bounded cleanup and safe retry. | Real sandbox/container workflow; kernel-specific isolation separately in VM |
| Platform/ABI | Native builds for each supported Linux architecture and actual execution on matching runners. Later cross build must run target output and demonstrate correct host build tools; mismatch refuses. | Actions Linux builders + container/VM target |
| Installed composition | Start actual systemd/session/services, verify generated config through application behavior, dependencies, home preservation and scoped reload. | Existing Testcontainers scenario/native harness |
| Boot/activation/recovery | Exact candidate firmware boot, Secure Boot/lockdown, enforcing SELinux, VT login/PAM, installer, update/switch/rollback, interruption and retained mutable-state recovery. | Existing disposable QEMU/VM workflows |
| Desktop GPU/input | Render visible animated content with observed renderer, inspect captures, actual physical input/audio and native window behavior; record host/device/backend. | Retained native QEMU manual workflow; real hardware separate |
| Release trust | Wrong authority/target, stale candidate, missing/corrupt signature and cache artifacts refuse; exact source/artifact identity survives validation into isolated signing and strict readback. | Existing release E2E/Actions/VM gates; no production keys in local fixtures |
| Performance | Explicit workload, input counts, tool versions, cold/warm/cache state, memory, evaluation vs build vs transfer/boot timings; multiple fresh observations and retained failures. | Public workflows/manual measurements after implementation |

Current AGENTS, ARCHITECTURE, STATUS and the `kedra-research` skill establish the layer split. The container README's “Native QEMU status” paragraph still says unimplemented even though adjacent text and newer STATUS/worklog document completed native work; it is stale and was not used as status evidence. This research does not rerun or inherit prior runtime passes for the new proposal.

**O2 handoff, not independently reanalyzed by O4:** S-004's cheap historical VM path bypasses firmware/installation through direct kernel loading and a shared store. S-020 describes QEMU and systemd-nspawn tests and simulated hardware, while `build-vm` does not bring the host's data/home state by default. The coordinator should cite O2's exact sections when using those details; neither simulated hardware nor a disposable clean home proves the corresponding physical-device or migration workflow.

## Remaining decisions and handoffs

1. Select Nix-style semantics versus an explicit compatibility target. Default recommendation: typed graph/Rust authoring first, with full Nix language/nixpkgs compatibility out of initial scope.
2. O3 owns exact current string-context, restricted/pure evaluation and import-from-derivation citations. O1 owns store compatibility/Tvix assessment. O2 owns module composition/activation semantics. O4 does not upgrade their claims independently.
3. A current primary cross-compilation specification would improve the platform contract if it becomes a compatibility promise. Candidate handoff: `https://nix.dev/tutorials/cross-compilation` (observed tutorial navigation; not fetched/analyzed).
4. The coordinator reports S-011 OCR preserved at `sources/text/628612-nix-article-ocr.txt` with method metadata. Temporary PDF extraction/rendering is in `/tmp/kedra-o4-research`; only the two O4 findings files were written inside the repository by O4.
