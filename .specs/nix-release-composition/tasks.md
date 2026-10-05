# Release composition tasks

State (2026-10-05): source is published in draft PR34 above D1 head712927e.
Local fresh installation/update/rollback and exact-candidate container checks
pass; the genuine package contribution now also passes. Hosted direct ARM
qualification remains incomplete after the final diagnostic retry. No
production execution is authorized. The original/replacement hash manifest under
`target/nix-delivery/d2-preparation` was reconciled before adopting its20 paths.
An explicit local fixture controller supplements the Actions workflow without
inventing Actions environment identity; exact runtime evidence belongs in
verification.md.

Current task outcomes: T1–T4/T6 implementation and recorded source/material gates
pass; T5's genuine production contribution, exact-image programs and nine refusal
cases pass on frozen580/material2271. T7/T8 pass the complete local public
HVF workflow on producer1dc/compatible fixture6146d96, with actual phase waits,
security/native-artifact/data checks and cleanup. TC05 passes13/13 on exact
candidatee0fd with frozen57 harness50dca and no overlay. T9 draft publication and
source gates pass. Hosted TCG refusal/QMP shutdown passes; fresh installation
still times out. Corrected-classifier source c01ee2f/run37330502213 attempt2
reaches the bootc deployment path after physical-root cleanup, but the retained
evidence cannot locate the subsequent wait or establish child execution.
Its x86 A/B/rollback and ARM refusal/cleanup pass. Hosted ARM installation,
update and rollback remain incomplete; no further speculative run is proposed.
T10/TC11 stays not-run until separately authorized on protected main.
These source domains are deliberately distinct; see the dated verification
appendices for hashes, failures and the unavailable original host-parent wait.

| Task | Work / files | Depends on | Test requirements |
|---|---|---|---|
| T1 | Freeze D1 boundary; reconcile derived receipt and immutable VM conversion API; parent owns branch/index | D1 | TC-01, TC-07 |
| T2 | Closed foundation assembly/Containerfile mode preserving desktop full assembly | T1 | TC-01, TC-02 |
| T3 | Implement compose.py foundation/compose CLI and closed native-steps recipe using existing APIs | T2 | TC-01, TC-02, TC-03 |
| T4 | Bind deterministic recipes/tools into refresh inputs; inspect actual native material; route ARM release builder; signer unchanged | T3 | TC-04, TC-05, TC-06 |
| T5 | Implement optional typed contribution/receipt validation; confirm exact D3 producer handoff | T3 | TC-03 |
| T6 | Extend existing ARM/full-image fixture path, signature/material fixtures and A/B/A guest checks | T4 | TC-07, TC-08, TC-09 |
| T7 | Execute actual fixture-signed media verification, explicit disk fresh installation and ISO-free boot using qualified D1/native controls | T6 | TC-07, TC-08 |
| T8 | Execute forward update and retained rollback preserving modified home/data; record refusals/hold/resume | T7 | TC-09 |
| T9 | Standard checks, read-only review, parent operational docs/skills/worklog, draft stacked PR/readback | T5, T8 | TC-10 |
| T10 | Separately authorized protected-main signing/publication and unchanged repeat; do not mark complete in branch PR | production authorization | TC-11 |

T5's empty-contribution path is D2 production behavior. D3 owns real catalog
package source/build/exposure work and its installed tests. T5 qualifies the
transport with a generated real compiled CLI output; it does not invent catalog
functionality or declare D3 complete.

The implementation author must update this file with observed outcomes, exact
source/controller identity and retained safe evidence. No task may borrow old
assembly/desktop signing results to satisfy derived ARM gates. Parent coordinates
all builds/containers/VMs serially. This preparer must not modify shared docs.

## Self-review resolutions (design, 2026-10-02)

- Corrected package template references to existing Segment::Input, not the
  mutable Segment::Output that the system API refuses.
- Separated deterministic pins/recipes from realized contribution/native IDs to
  preserve no-change semantics.
- Kept fixture derivative and normal unsigned candidate digests distinct; fixture
  credentials/observer necessarily change bytes and cannot prove production keys.
- Split fresh Anaconda installation from image-builder QCOW2 boot, and recorded
  hosted ARM KVM/smoke limitation with a native manual route.
- Kept generated public fixture authority in an isolated checkout; no bypass flag
  enters production installer or signing schema.
- Actual D1 interface and runtime capability remain a prerequisite, not an
  assumed completed result. File/spec inspection supplies planning evidence only.
- Early source critic corrected transport-domain mistakes in the unhooked runner:
  engine definitions/retention use `aarch64-linux`, object IDs retain `src-` or
  `out-`, and aliases use `plan::name`'s exact ASCII alphanumeric/underscore/hyphen
  grammar. Docker `linux/arm64` remains distinct. Ruff/AST/help are syntax/parsing
  evidence only; no runtime acceptance follows from this correction.

## Prepared source milestone

T2-T8 have reviewable unadopted source replacements, including the autonomous
generated Kickstart/actual Anaconda path and ISO-free public A/B/A checks. This
does not complete those tasks: actual execution, source corrections after observed
failures, exact-source gates and negative Secure-Boot-disabled installer coverage
remain required. A/B share a verified native derivation and differ in signed
fixture marker/rank; changed-kernel/native-recipe upgrades are not claimed.
Ruff/AST pass for nine Python files and bash syntax passes for two shell files.
No image build/import, signing, VM, registry or Git operation ran in preparation.
Parent owns review/adoption/commit, standard gates, operational docs and worklog.

## Review publication boundary — 2026-10-03

T9 draft publication proceeds with an explicit incomplete T7/T8 runtime state,
under the owner's source-review priority. This changes review scheduling only:
T7/T8 and all remaining acceptance gates remain required before qualification.
The existing native producer and earlier installed cold proof retain their exact
source scopes; no old pass is transferred to the corrected media route.
