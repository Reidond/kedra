# Specification and implementation review

Current state (2026-10-06): implementation is authorized and present, with local
frontend, real Fedora build/rebuild, independent API and installed catalog passes.
The original review below is historical; current qualification is in test-plan.md.

## Original specification review (2026-10-05)

Status: revised following the owner's approval on 2026-10-05. Standalone language
and inline-first direction are approved; implementation remains unstarted. This
is a single-agent Codex side-conversation review, with no subagent interaction.

## Scope and source evidence

The revision changes only `.specs/nix-package-dsl/`, STATUS and worklog in the
isolated PR38 worktree. The entry README retains exact delivery-source links and
separates those APIs from the older main PR base. No main-thread checkout, package
list, Cargo manifest, engine, workflow or installed system is changed.

The initial embedded-Rust/author-crate design is superseded by direct owner
approval. This is not an attempt to reinterpret the earlier no-new-language rule:
the spec records the narrow decision change and gives future implementation tasks
for reconciling the corresponding operational guidance. It does not implement a
parser merely because the direction was approved.

## Review resolutions

1. **Primary authoring outcome:** replace the Rust author-crate/sidecar default
   throughout the requirements, design, examples, tasks and tests. jq/SQLite and
   a small C pilot must carry actual build logic/resources inline; a conceptual
   ability to embed code is insufficient if ordinary recipes still require wrappers.
2. **Language scope:** define explicit v1 grammar, fields/types, local imports,
   namespaces and bounded package-template expansion. No hidden Rust/shell eval,
   network import, environment lookup, loop or plugin escape.
3. **Literal stability:** document indentation/newline/escape behavior, formatter
   preservation and file modes/paths. Opaque shell strings are never interpolated
   by the frontend; template binding is a separate typed operation.
4. **Existing argument limits:** current engine plan validation bounds arguments
   to 32 KiB. Scripts lower to admitted resource files and execute by typed input
   path; TC-14 requires a valid longer script without widening engine limits.
5. **Preparation ownership:** inline patches/overlays use an explicit owned
   preparation node. Immutable archive sources stay read-only; unsafe paths,
   missing replacement targets and fuzzy patch application refuse. Generated nodes
   count toward graph bounds.
6. **Phase identity:** build/preparation resources and post-build config resources
   are separated. A config-only template edit changes contribution identity rather
   than forcing an unrelated binary rebuild or creating a self-output cycle.
7. **Role-resolution order:** builder/runtime dependencies contribute before exact
   images are frozen; symbolic roles lower only after independent resolution.
   Libraries remain dependency nodes, not invented runnable packages.
8. **Fedora and release authority:** retain the independent target/repository/base
   policy, actual complete RPM material and failure-on-drift. Generic contribution
   still binds exact frontend/input/resource/policy/result identities; signers run
   no language or recipe code. Material does not promise permanent RPM availability.
   Resource object IDs are computed from independently accepted bytes and scoped
   per invocation; a producer cannot expand standing source permissions.
9. **Migration and versioning:** explicit source-format/version selection rejects
   mixed authority and unknown versions without fallback. Old readers and signed
   provenance survive. Both targets need actual Fedora parity; ARM source-build
   results do not qualify x86 execution.
10. **Traceability:** new AC-11/12/13 cover self-contained packages, resource identity
    and language tooling. TC-12/13/14 and T2/T3/T6/T7 own those outcomes; every AC/NFR
    appears in the matrix and every case has an owning task/level.

## Check boundaries

Source/API inspection confirms current data/node/argument bounds. Other limits
in NFR-02 are proposed defaults, not measured performance. Syntax examples are
reviewed illustrations and have not been compiled by a nonexistent parser.
The grammar outline and schema rules together define proposed behavior; the
future E2E cases must prove the complete frontend-to-build workflow.

Review applied scope/security, overengineering, failure handling, compatibility,
affected-file ownership and task/case coverage checks. AI-service/web-specific
items do not apply. Repository E2E/manual-only policy overrides generic unit/mock
recommendations. Documentation and ordinary manual ID/diff inspection are used;
no repository scanner or new test runner was created.

The final local review and diff checks are recorded in the worklog amendment.
All actual language/build/resolver/VM cases remain not-run. Automatic PR workspace
CI exercises unchanged product code and does not qualify this future language.
No new project-wide implementation learning or global skill change is claimed;
these task-specific decisions belong in this spec.

## Closing challenge

The review changed the proposal at the inline-script transport, preparation
ownership, formatter semantics and build-vs-contribution identity boundaries.
No remaining design blocker is being hidden behind a claimed implementation pass.
Not inspected by execution: parser behavior, actual new lowering, resource ceilings,
new Fedora migration and installed outputs, because this PR contains no code for
those operations. They have explicit future case owners rather than assumed results.

## Implementation review (2026-10-06)

The owner explicitly authorized language and migration implementation. The scope
now includes the frontend/intent, engine resource admission, source reader, release
preflight/contribution, authored definitions, public workflows and documentation.
This replaces the original documentation-only scope recorded above.

Dependency inspection is a floor from concrete references, not proof of every
dynamic caller. Opened dependents and disposition:

| Changed contract | Dependents checked | Disposition |
|---|---|---|
| parse/compile/format and typed intent | catalog_language.rs, catalog.rs, source_packages.rs, direct Rust consumer | updated; public CLI and real API/build checks |
| source identity and SourceFile | engine tree/store admission, language lowering, packet emitter | checked/updated; exact identity and actual store admission agree |
| managed snapshot purpose | system recovery, catalog emission and formatter | updated; atomic publication and closed recovery members preserve winners |
| source format discriminator | source plan/archive, assembly, material/refresh and image workflows | updated; real committed old/new readers and exclusive format refusals |
| pin/material schema2 | refresh.py, release-target.yml, contribution and composer | updated; generic roles/packages, exact observed RPM checks; signer schema unchanged |
| legacy catalog API/schema1 | catalog list/resolve/contribute, immutable legacy.json, material.py | updated; explicit historical reader retained |
| command/config exports | system definition/composer, installed catalog harness | updated; actual alias/PATH/library/service check passes |
| package-only edits | container/desktop/QEMU/home/GHCR workflow path filters | updated; new input descriptor/policy/definition paths trigger existing workflows |

Review fixes: bounded owned frontend process and aggregate resource limits; conflict
diagnostics carry related locations; source preparation remains an isolated node;
formatter scratch uses a recoverable lease; generic lowering contains no Kedra
target whitelist; contribution checks complete actual foundation/compiler RPM
material, and archive acquisition helper enters preflight recipe identity.

The specification was checked against actual public behavior by requirement, and
source/literal identity against the existing engine serializer. Ordinary compiler,
Ruff and public E2E checks support convention compliance; no unit/mock test or
repository scanner was added. Operational docs and narrowly revised architecture/
agent guidance follow the implemented owner decision; both plugin manifests are
bumped. Durable findings are recorded only in worklog.md.

Remaining qualification is explicitly recorded in test-plan.md. Protected
production release is main-only and was not run from this branch; existing
Noctalia/installer/native gates are not waived by these package results.
