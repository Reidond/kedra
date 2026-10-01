# Context consumer boundary

The existing exporter uses input identity over a canonical SystemPlan and separate
artifact hashes. Add strict Deserialize to exported structs and one flat engine
context module. Validate expected identity/domain, model bounds, selected outputs/
same foundation, resolved contribution/templates, object derivations/references,
canonical tree entries and payload receipt. Validate the complete foundation with
the existing OCI verifier. The Containerfile must exactly match the generated
FROM+ADD bytes; source comments or extra instructions are not accepted.

Return an opaque guard owning a private snapshot of verified files. Copy regular
input handles into that snapshot with bounds; the consumer only uses its paths or
handles. Never extract an untrusted root filesystem on the workstation. Drop removes
only the guard's private snapshot. Expected identity is caller-selected content
identity, not installed OS authentication.

Add a composition source and expected-identity argument to `kedra-lab`/harness
requests, default overlay none and reject VM/worktree/binary overrides initially.
Load archive through the existing Docker API; validate exact native ID, ONBUILD
and volume configuration. Pin the verified digest-derived local FROM tag; no
arbitrary mutable tag is trusted. Static build uses network none/pull false and
validated context, then existing lab adaptation remains separate. Cache reuse
requires an independently retained image-ID/provenance binding, not tag existence.

Use public harness commands/APIs to build/run the copied closure after deleting
generated producer source/store. The installed-unit case is composition-only,
uses System profile and actual systemd processes. Existing scenario filtering,
empty-selection behavior, ordinary suites and desktop sessions remain unchanged.

Ownership: engine verifier owns engine context/system model files; harness owner
owns image/docker/lab/vm/main/native plumbing and local dependency edge; E2E owner
owns the CLI workflow fixture. Parent owns Git/stack/docs/worklog and serialized
final gates. Freeze API contracts before dependent source edits/suites.

Source basis: current `system.rs`, `image_archive.rs`, tree.rs, container harness
README/image/docker/native; O1/O2/O3 research and prior composition review. Signed
foundation acquisition uses installed default source policy; omitting unsupported
destination signature sidecars from private OCI archive does not waive source
verification. Record exact tools/digests and observed outcomes.
