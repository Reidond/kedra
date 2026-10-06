# Completion review

Reviewed frozen native source and its CLI/harness callers on2026-10-01, against
the NA01–08 requirements. Astra's final independent read-only review found no
remaining concrete blocking defect. This is a bounded source review; execution
results are in [verification](verification.md).

| Changed seam | Inspected dependents | Result |
|---|---|---|
| Public NativeDefinition/Step/Plan/Receipt API | harness native_derivation, compiled external Rust CLI fixture, installed receipt consumer | Updated; real generation/readback and installed workflows pass. Other engine APIs retain their existing signatures. |
| Docker archive import | composition::prepare | Updated timeout only for complete import; fresh empty mutual-TLS daemon replay passes. |
| Docker publication/temporary-tag helpers | static composition and native derivation | Static delegates retain exact composition namespace/labels; native delegates use their separate namespace. Actual static interruption/retry passes. |
| Native source applicability/Context identity | main's sole scenario Context construction, native test selection, lab/image preparation | Updated; complete native variable pair required before provision; ordinary listing still14 cases; native System case executes. |
| Fixed native root recipe | core rendering, harness uploaded files/build, artifact inspector | Explicit root/workdir, offline/no-bind boundary, exact parent layer prefix, single-link generated material and bounded tmpfs inspector are present. |

The caller search is a floor: it covers the current Rust source/direct CLI wiring,
not unknown external library consumers or dynamic future integrations. Workspace
Clippy/compiler and actual public authoring workflows also check these seams.

Review corrections are incorporated and rechecked: explicit recipe user/workdir,
exact parent layer lineage, selected/generated hardlink checks, approved systemd
directory ancestry, kernel module hyphen equivalence, dracut's exact dangling
root-link handling and existing user-bus fixture. The ordinary package executor,
RPM transactions, production signer/helper and live-home authority were not expanded.

| Completion step | Result / evidence |
|---|---|
| Dependencies, code/conventions and adversarial review | pass; table above, parent diff inspection and final independent Astra review. |
| Actual gates | pass; final fmt/Clippy/ordinary E2E/release/legacy CLI/driver Ruff, explicit native CLI and installed case; ignored listings count as no execution. |
| Documentation impact | README, architecture, ENGINE/SYSTEM/NATIVE/PACKAGES, harness README, STATUS, worklog and Rust/research/security skills updated. PLAN product/signing/home requirements are unchanged. |
| Learnings | Observed native boundaries recorded in three project skills, manifests0.3.12 and changelog; force-kill snapshot cleanup limitation retained in development observations. |
| Changelog/hypothesis decision | Fields/date/file claims inspected manually under the owner's standard-tool-only policy. These changes synchronize operational evidence; no new improvement metric or workflow hypothesis is asserted. |
| Close self-challenge | No further concrete source defect found. Re-reading the actual receipt/report exposed no mismatch; cold/fault JSON and image events are assessed at each completion, with exact observed states. |
| Backend restart | n/a; no application server/backend changed. All installed checks use disposable harness containers. |

The close challenge changed the documentation to name SIGKILL snapshot residue,
separate static publication fault evidence from native generation/cache reuse,
and correct release-executable provenance after the workspace build replaced a
cached dependency-feature variant. The final strict audit caught that drift;
private pinned-artifact cold/cache/native readback now passes before publication.
Actual native receipt, report outcome/count/identity and cold/recovery observations
were read back before recording; no percentage-complete or source-text test is used.
Broad tests were not repeated after documentation-only edits.

Unexamined/unqualified surfaces: native derived-image kernel boot, Secure Boot/
SELinux, installer/update, production signer integration, x86 native generation,
power-loss/ENOSPC, unknown external clients and native-cache publication faults.
These require their own actual workflows. Adjacent static successes do not prove
those surfaces; the list is bounded by declared scope rather than claiming a
complete global failure model.
