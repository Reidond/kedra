---
name: kedra-research
description: Choose and execute a Kedra R01-R11 research packet, verify current primary tooling sources, record reproducible positive/negative evidence and preserve findings for later agents.
---

# Research before risky implementation

Use RESEARCH.md as the experiment contract. P0 blocks its production feature,
not every independent prototype. The goal is to resolve assumptions with small
disposable experiments, not to keep searching for a universal distro solution.
Rust/Cargo and the owner's architecture are settled.

## Packet map

R01 signature/bootc compatibility; R02 installer and runner; R03 home review/local
policy; R04 activation/writers/rollback; R05 agent runtime/profile/distribution;
R06 credentials; R07 desktop/hardware; R08 release authority/privacy/lifecycle;
R09 targets/provenance; R10 privileged protocol/persistent state; R11 Rust/skills.

Start independent R01/R02, R03, R05 and R11 tracks. R06/R07 share the desktop
fixture. R04 joins image and home prototypes. R08 precedes production signing.
R10 defines safe installed operations. R09 can use VMs before an XPS exists.

## Experiment procedure

Read the gate, fixed choices and relevant skills. Inspect existing evidence, not
just prior assistant claims. Verify current primary documentation and exact
package/CLI versions. Keep documented fact, candidate design and measured result
separate. A valid URL is not proof that the Fedora version supports an option.

Use synthetic homes, disposable keys/namespaces and VMs. OS/ISO builds belong in
Actions; local end-to-end CLI checks can work offline. Never install experimental images over
the workstation, enroll real dotfiles, or use production vault/signing secrets.
Declare missing capabilities as blockers and continue safe independent work.

Create positive AND negative cases before claiming a solution. Include interruption,
wrong inputs, concurrency, old-version readers and privacy boundaries. Record
expected/actual outputs, exit codes, exact environment and source/image identity.
Store reports in docs/research/Rxx-topic with REPORT.md, environment.json,
results.json and small sanitized fixtures/logs. Large relevant artifacts need
checksums/run links. Do not publish real transcripts or secrets for reproducibility.

## Report and preserve

Use statuses not-run, pass, fail or blocked per case. CI/static/real-agent/VM/
physical-hardware cases are different evidence categories. A green bootstrap
workflow does not pass all R11 discovery cases, much less the OS gates. Do not
invent hardware results or call a successful container build a successful boot.

End with a chosen approach/ADR or explicit remaining blocker. Update the relevant
skill with the durable finding, source/date/version and how it was tested. Keep
skills short and link details; do not overwrite settled requirements based on
optional upstream advice. Update status.json without erasing not-run cases.
The user's session should become less necessary as evidence accumulates.

Sources: RESEARCH.md, docs/research/README.md, docs/SOURCES.md. Use the report
template. An agent handoff names the exact current scope and next experiment,
not a promise of background work or a claim the entire product now works.
