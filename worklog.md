# Kedra worklog

Shared continuation record for Codex, Claude and other authorized contributors.
Follow the maintenance contract in `AGENTS.md`. Keep the status snapshot current;
append completed work entries without silently rewriting history. This file is
public: no credentials, transcripts, private home data or sensitive raw logs.

## Current project status

Last updated: 2026-09-08 (Europe/Kiev); R03 checks ran on 2026-09-07.

| Area | Verified status / next boundary |
|---|---|
| Phase | Implementation underway: minimal VM, strict signed-update/rollback and graphical desktop prototypes pass. No promoted owner installer yet. |
| Implemented | Three-package flat Rust workspace; help/version/capability status; committed-source planning with host overrides and content provenance; operational refusals; standard Cargo CI; synthetic R03 example. No real-home manager or deployment implementation. |
| Repository knowledge | One Codex/Claude plugin with 12 Kedra and 18 selected upstream skills in plugins/kedra/skills. Ordinary files; no submodule, symlinks or duplicate discovery copies. No personal installation. |
| Latest verified check | Published R03 source e492258871f6353a0c6548a0ec5d5e318d92bdd3 passed [Actions run 34164331109](https://github.com/Reidond/kedra/actions/runs/34164331109): Linux fmt/Clippy/19 tests plus 1 doctest/release build. Matching source previously passed local Windows checks. This evidence follow-up does not claim its own future CI result. |
| R11 | Canonical skill files readable; Codex marketplace and fresh-profile root/crate skills/list found no Kedra entries. Claude authoring validation passed. Loaded-plugin model/editor checks not-run; distribution review blocked. |
| R03 | Core synthetic review prototype passed 12 tests; full gate remains blocked on Noctalia/app-owned projection, durable disposition transitions and wider path/edit safety. See docs/research/R03-home-review/REPORT.md. |
| Other R01-R10 | R01 strict signature/update/rollback prototype passes, including initial and post-rollback policy. R02 minimal VM passes in Actions and locally. R07 graphical login/session/keyring prototype passes. R08 verification has local tests; R09 source/archive tests pass. Full installer, promotion, activation, owner authentication and physical hardware gates remain open. |
| Current task | Owner requested full implementation through usable installation, including VM testing if needed. Active first slice: R01/R02 disposable Actions image and VM proof, followed by gated management implementation. |
| Machine effects | QEMU 8.2.2 and OVMF installed in existing Ubuntu WSL2 under the owner's VM-test authorization. No real-home enrollment, vault/profile changes, disk formatting or workstation OS installation. |

Evidence: [R11 report](docs/research/R11-rust-workspace/REPORT.md),
[research index](docs/research/status.json),
[bootstrap run 2](https://github.com/Reidond/kedra/actions/runs/34138882781),
[baseline run 3](https://github.com/Reidond/kedra/actions/runs/34139103852),
[documentation run 4](https://github.com/Reidond/kedra/actions/runs/34140106798),
and [update-plan run 6](https://github.com/Reidond/kedra/actions/runs/34145002985).
The snapshot summarizes those records; source, exact CI runs and case-level
reports remain authoritative for what was tested.

### Next concrete actions

Read docs/HANDOFF.md and docs/UPDATES.md. The update track starts with a disposable
CI RPM-refresh proof (slice 1), covering unchanged-base dependency updates, stale
cache, no-change and required-repo failure. Connect to signed-image/installer gates
only after those proofs. R03 home and remaining R11 discovery work can proceed
independently. Do not enable production cron or enroll a real machine/home yet.

For R03, next add a versioned synthetic Noctalia effective-settings projection
and explicit I/S/P transitions across N. Continue independent work by reading
canonical skills when native discovery is unavailable. Published R03 implementation
now has Windows and Linux evidence; real-home and activation gates stay closed.

## Work entries

### WL-20260907-01 — 2026-09-07 — Bootstrap baseline (retrospective)

- Agent / state: ChatGPT repository handoff; completed bootstrap subset only.
- Scope / base: `main` through `e807dfe219ba1d57ebb751bd40d46897af31b6d7`.
  This entry was reconstructed from existing source, the R11 report and observed
  Actions results; it is not a claim that the prior session maintained a log.
- Completed: Four flat-source Cargo packages, bootstrap CLI/helper refusal,
  `xtask`, pinned toolchain, repository-only skill links/submodule, plans,
  research packets and continuation instructions. See the source history and
  `docs/research/R11-rust-workspace/REPORT.md` for the detailed scope.
- Checks / evidence: `pass` — bootstrap run 2 records formatting, Clippy,
  metadata, eight unit/integration tests plus one doctest, release compilation,
  layout and static skill wiring. `pass` — baseline run 3 at `e807dfe...` was
  rechecked through GitHub for this update. The earlier formatting failure and
  correction are preserved in the R11 report, not erased from history.
- Remaining / blockers: Actual Codex/Claude discovery and editor behavior were
  not tested. R01-R10 remain not-run; no image, ISO, production signing or live
  system management was produced.
- Next: Use the handoff and selected research packet, not the bootstrap status
  command, as the guide to implementing the next capability.

### WL-20260907-02 — 2026-09-07 — Repository skill scope and worklog contract

- Agent / state: ChatGPT using the GitHub connector; completed documentation
  edits, with the resulting CI not yet observed at entry creation.
- Scope / base: `main`, inspected base
  `e807dfe219ba1d57ebb751bd40d46897af31b6d7`. Documentation only; no host target
  package, runtime, workflow or deployment behavior changed.
- Completed: Updated `AGENTS.md` to prohibit global/system-wide distribution or
  registration of the repository's skills and to require both agents to maintain
  this exact lowercase `worklog.md`. Updated `skills/README.md` to reflect the
  same scope. Initialized the status snapshot and retrospective baseline entry.
  `CLAUDE.md` already imports `AGENTS.md`, so it remains unchanged.
- Checks / evidence: `pass` — inspected current main, agent instructions, skill
  catalog, Claude import, research status and baseline CI through GitHub.
  `not-run` — new-revision CI at entry creation; record an observed result in a
  follow-up entry rather than borrowing the prior commit's pass.
- Remaining / blockers: These are repository instructions, not new executable
  enforcement or a claim of real agent compliance. Existing research gaps remain.
- Next: Observe the exact resulting Actions run, record its outcome and update
  the status snapshot before handing off to the next coding session.

### WL-20260907-03 — 2026-09-07 — Verify the documentation change

- Agent / state: ChatGPT using the GitHub connector; completed verification and
  handoff update. Follow-up to WL-20260907-02; its original pending observation
  remains preserved above.
- Scope / base: `main` at `1e094eca8f482b0203dcbf041c14e5883c8f4123`.
- Completed: Confirmed the published change modifies only `AGENTS.md`,
  `skills/README.md` and `worklog.md`. Read back the instructions and worklog;
  observed the exact resulting Actions job succeed. Refreshed this status
  snapshot without changing the research index or claiming new implementation.
- Checks / evidence: `pass` — GitHub comparison against `e807dfe...` confirms
  the three-document scope. `pass` — [run 4](https://github.com/Reidond/kedra/actions/runs/34140106798),
  job `101799936466`, completed all workspace/skills/format/lint/test/release
  checks and clean-source verification successfully on `1e094eca...`.
- Remaining / blockers: CI checks repository consistency, not an agent's future
  adherence to prose. Actual agent discovery/editor behavior and R01-R10 remain
  untested. This record does not imply any OS or home deployment.
- Next: Resume the selected research packet using `docs/HANDOFF.md`; read and
  maintain this worklog in the next Codex or Claude session.

### WL-20260907-04 — 2026-09-07 — Plan updates and scheduled Fedora refresh

- Agent / state: ChatGPT using GitHub and primary web documentation; completed
  planning, with resulting CI not yet observed at entry creation.
- Scope / base: `main` at `f3b44f1fd63af5805167c30e139bb5bf3f9aa3f4`.
  Documentation/repository-skill changes only; existing research gate statuses stay.
- Completed: Added docs/UPDATES.md, ADR 0002 and an update-refresh experiment
  supplement. Added detailed skill references and updated the bootc, CI and signing
  skills plus HANDOFF. Proposed 12-hour complete Fedora refresh, full RPM closure,
  no-change/freshness separation, same-digest tests/ISO and notify-only clients.
  Planned explicit staging/reboot control, pending-image preservation, rollback
  holds, repository/cache failure handling and signed per-target checkpoints.
- Checks / evidence: `pass` — inspected repository instructions/worklog/source and
  checked current bootc/DNF5/Podman/GitHub primary docs (U01-U11 in docs/UPDATES.md).
  This is documentation review, not experiment success. `not-run` — actual refresh,
  no-change proof, signer/installer/channel/client behavior and new-revision CI
  at entry creation. Fedora web docs challenged retrieval; no new exact-content
  claim relies on those failed fetches.
- Remaining / blockers: R01/R02/R04/R07/R08/R09/R10 own implementation gates.
  No schedule, signing keys, protected environment, host package upgrade, client
  unit or deployment was created. Exact metadata publication and cache/equivalence
  behavior require tests; policy timings are proposed defaults, not upstream facts.
- Next: Verify this revision's repository check and readback. Then assign slice 1
  to Codex with disposable CI fixture RPM repositories; preserve all no-loss,
  signature and repository-only skill boundaries.

### WL-20260907-05 — 2026-09-07 — Verify update-plan publication

- Agent / state: ChatGPT using GitHub; completed documentation verification.
- Scope / base: `main` at `bfb190226c5461292e06dc470c42f86f998b07af`;
  follow-up to WL-20260907-04 without rewriting its pending observation.
- Completed: Read back the published plan and worklog, confirmed the commit changes
  ten documentation/skill files only, and observed the exact bootstrap run succeed.
  Refreshed this status snapshot; no implementation/research status was advanced.
- Checks / evidence: `pass` — GitHub compare against f3b44f1 confirms no runtime,
  package, Cargo, active workflow or timer changes. `pass` —
  [run 6](https://github.com/Reidond/kedra/actions/runs/34145002985), attempt 1,
  completed successfully for bfb1902. This is repository validation, not evidence
  that Fedora refresh, client update, signing, home merge or an ISO works.
- Remaining / blockers: The new experiment supplement remains entirely not-run.
  The final worklog-only follow-up has no claimed future CI result.
- Next: Assign the disposable slice-1 refresh experiment using docs/HANDOFF.md;
  keep production scheduling disabled until its prerequisite gates pass.

### WL-20260907-CODEX-04 — 2026-09-07 — Local Rust toolchain mismatch

- Agent / state: Codex; partial (diagnosis and recovery documentation completed).
- Scope / base: `main` at `f3b44f1`; R11 local bootstrap troubleshooting.
- Completed: Confirmed Cargo requires Rust 1.98 and repository/CI pin 1.98.1.
  Active toolchain is stable, selected by RUSTUP_TOOLCHAIN, reporting rustc
  1.97.0 (2d8144b78 2026-07-07), Cargo 1.97.0 (c980f4866 2026-06-30).
  Added explicit installation/selection guidance to README.md and the Rust
  workspace skill. Kept version requirements and personal configuration intact.
- Checks / evidence: `pass` — clean initial Git state, manifests, workflow,
  rustup active-toolchain/list and compiler version inspection.
  `blocked` — `rustup run 1.98.1 cargo xtask check` reports the Windows MSVC
  toolchain is not installed; no validation steps ran. `blocked` — latest CI
  retrieval with `gh run list --limit 3` denied by network restrictions; prior
  recorded CI evidence above was not freshly verified.
- Remaining / blockers: Local full check requires installing the pinned toolchain;
  AGENTS.md prohibits automatic missing-tool installation. No commit or push.
- Next: Install Rust 1.98.1 with rustfmt/Clippy explicitly, run
  `cargo +1.98.1 xtask check`, and record its actual outcome.

### WL-20260907-CODEX-05 — 2026-09-07 — Shared plugin and removal of xtask

- Agent / state: Codex; completed local changes.
- Scope / base: main at f3b44f1. Owner requested removing submodules/copying
  relevant skills, then explicitly removed the custom Cargo check runner and
  requested Codex/Claude plugins without symlinks. Supersedes the uncommitted
  copy-synchronization approach and interrupted symlink-diagnostic patch.
- Completed: Moved 12 Kedra and 18 selected upstream skills plus support files
  into plugins/kedra/skills. Added native Codex/Claude manifests and repository
  marketplace catalogs. Removed all discovery links/copies, the submodule and
  local registration, xtask package and Cargo alias. Regenerated Cargo.lock;
  CI runs standard Cargo commands. Updated AGENTS, README, handoff, plan,
  research packet/status, skills, provenance/notices and ADR 0002.
- Checks / evidence: pass — Rust 1.98.1 fmt, Clippy with warnings denied,
  7 unit/integration tests, 1 doctest and release compilation on Windows MSVC.
  pass — Codex plugin-creator validator, Claude 2.1.263 manifest/marketplace/skill
  validation. Codex CLI 0.153.4 does not list the unregistered repo plugin in its
  marketplace query; no personal registration or installation was performed.
  pass — git diff --check. Detailed evidence: docs/research/R11-rust-workspace/plugins.md.
  not-run — new-revision CI and installed interactive plugin use.
- Remaining / blockers: Standard cargo without an explicit pin can still inherit
  RUSTUP_TOOLCHAIN=stable; README documents removing the process override or
  selecting +1.98.1. Upstream declares MIT but lacks LICENSE text; recorded in
  third-party notices. R11 remains blocked on separate discovery/editor/review
  gates. No commit or push. Submodule gitlink removal is staged; other edits
  remain uncommitted. Original upstream and intermediate copies are preserved
  under ignored target/ backups.
- Next: Review the local changes and load the shared plugin in each agent.
  Use standard Cargo commands for Rust; do not restore the removed task runner.

### WL-20260907-06 — 2026-09-07 — Remove unused skills lockfile

- Agent / state: Codex; completed.
- Scope / base: main at f3b44f1, existing uncommitted plugin migration; user
  requested deleting skills.lock.toml.
- Completed: Deleted the unused lockfile, preserved the upstream source/revision
  and all 18 selected skill names in the plugin NOTICE.md, and updated current
  documentation and skill references. Superseded ADR 0001 remains historical.
- Checks / evidence: pass — reference review and git diff --check.
  not-run — Rust tests; no Rust code, manifests or build behavior changed.
- Remaining / blockers: Existing plugin installation and research gaps unchanged.
- Next: Review the uncommitted plugin changes and load the plugin when desired.

### WL-20260907-07 — 2026-09-07 — Prepare authorized commit and push

- Agent / state: Codex; completed publication preparation.
- Scope / base: main at f3b44f1; user explicitly authorized committing and pushing
  the plugin migration, xtask removal and unused skills-lockfile removal.
- Completed: Reviewed the pending tracked and untracked scope; retained the shared
  plugin, provenance notices, updated policies/research and standard Cargo CI.
  Ignored target backups and temporary authoring scripts are excluded.
- Checks / evidence: pass — earlier Rust formatting, Clippy, 7 tests plus
  1 doctest, release build and both plugin validators remain applicable; only
  documentation changed afterward. pass — git diff --check.
- Remaining / blockers: Commit/push and resulting Actions observation follow
  this preparation record; no result is claimed in advance. Plugin installation
  and separate R11 gates remain not-run.
- Next: Commit the reviewed changes, push main and inspect the resulting CI.

### WL-20260907-08 — 2026-09-07 — Preserve concurrent update planning

- Agent / state: Codex; completed merge preparation for the authorized push.
- Scope / base: local plugin commit a872522 and origin/main b7b2962; merge is
  required because two update-planning commits arrived during local work.
- Completed: Preserved docs/UPDATES.md, the update-refresh experiment, ADR 0002
  and all remote skill edits. Moved two new skill reference files into the shared
  plugin. Reconciled HANDOFF and both agents' worklog entries. Disambiguated
  colliding local entry IDs 04/05 with CODEX in their headings; bodies remain
  historical. Renumbered the plugin ADR to 0003; earlier mentions of plugin
  ADR 0002 refer to that pre-merge name. No reset, rebase or force push.
- Checks / evidence: pass — reviewed remote diff and resolved both content
  conflicts while retaining the plugin layout and standard Cargo CI.
- Remaining / blockers: Merge commit/push and CI observation follow this record.
  Update mechanisms remain planned; plugin installation and R11 gaps unchanged.
- Next: Validate the merged plugin, commit the merge and push main.

### WL-20260907-09 — 2026-09-07 — Verify authorized publication

- Agent / state: Codex; completed push and merged-source verification.
- Scope / base: plugin commit a872522 and merge
  891cc1506851365d32747caf63b89300294ce71a, published to origin/main.
- Completed: Preserved origin/main b7b2962 and both agents' work; pushed normally
  without force. Revalidated the merged plugin/skills and observed successful CI.
  This final documentation follow-up records that observed result.
- Checks / evidence: pass — Codex manifest and Claude skill validation after
  merge; git diff --check; clean working tree after push. pass —
  https://github.com/Reidond/kedra/actions/runs/34160120958 completed successfully
  for exact merge 891cc1506851365d32747caf63b89300294ce71a.
- Remaining / blockers: This follow-up's own future CI is not claimed. Actual
  installed interactive plugin use and existing R01-R11 research gaps remain.
- Next: Load the plugin for Kedra work or continue the documented refresh
  experiment. Use standard Cargo commands; no custom task runner is required.

### WL-20260907-10 — 2026-09-07 — R03 synthetic review and R11 discovery
- Agent / state: Codex; completed requested local prototype; full research gates remain partial. Handoff finalized 2026-09-08 Europe/Kiev.
- Scope / base: main at 64c84498c898e61ff370d9f9f1ce30cbd3d00613; synthetic R03 only and read-only R11 discovery checks.
- Completed: Read contracts/source/pinned skills; initial checkout clean. Added a std-only Cargo example and 12 tests for selected line snapshots, later application writes, local-only exclusion including private-history/object audits, source drift and explicit conflicts. Generated homes/repositories only; no path-taking enrollment CLI. Added ADR 0004, R03 report/environment/results/golden fixtures/logs and R11 discovery evidence. Updated home/Rust skills, handoff/readmes/status and CI scope wording. Three packages and one lockfile retained; upstream skills unchanged.
- Checks / evidence: pass — exact-base Actions run https://github.com/Reidond/kedra/actions/runs/34160201371. pass — Rust/Cargo 1.98.1 fmt, Clippy, workspace tests (19 plus 1 doctest), release build, metadata and synthetic session; Git 2.55.0.windows.1. Final logs/source hashes in docs/research/R03-home-review. Initial Windows Git config-path failure and two test-audit/assertion mistakes corrected; initial Clippy warnings fixed without lint changes. pass — Claude 2.1.263 plugin/marketplace authoring checks and canonical skill reads. blocked — Codex 0.153.4 marketplace and isolated skills/list root/crate discovery returned no Kedra skills. not-run — interactive Claude/loaded-plugin Codex/editor and current-change Actions.
- Remaining / blockers: R03 full gate still needs Noctalia/app-owned projection, durable I/P and baseline transitions, broader path/edit safety; R04 activation/concurrent writer/crash recovery not-run. No real home, OS, vault or personal agent configuration was modified; no commit/push/global plugin installation. Synthetic publication is not deployment.
- Next: Review local diff/evidence; extend synthetic R03 with Noctalia effective settings and explicit I/S/P transitions across N, keeping real activation behind R04.

### WL-20260908-01 — 2026-09-08 — Authorized R03 publication preparation
- Agent / state: Codex; completed publication preparation.
- Scope / base: main at 64c84498c898e61ff370d9f9f1ce30cbd3d00613; owner explicitly requested commit and push of the completed R03/discovery changes.
- Completed: Reviewed tracked/untracked publication scope; fetched origin and confirmed no divergence. Preserved synthetic-only boundaries, reports, skill updates and ignored local artifacts.
- Checks / evidence: pass — recorded SHA-256 values match all tested source files; earlier fmt/Clippy/19 tests plus 1 doctest/release build remain applicable. pass — git diff --check. No unrelated pending edits were observed.
- Remaining / blockers: Commit/push and exact-revision Actions observation follow this record; no future CI success is claimed. Full R03/R11 and R04 activation gaps remain unchanged.
- Next: Commit the reviewed files, push main normally, and record the observed Actions outcome.

### WL-20260908-02 — 2026-09-08 — Verify R03 publication and Linux CI
- Agent / state: Codex; completed implementation publication and CI verification.
- Scope / base: e492258871f6353a0c6548a0ec5d5e318d92bdd3 on main, pushed normally to origin/main under the owner's explicit authorization.
- Completed: Committed the 31 reviewed R03/discovery files; source tree was clean after push. Observed successful exact-revision Actions, including all 12 synthetic R03 cases. Added this documentation follow-up and updated the report/results/status with the newly observed Linux evidence.
- Checks / evidence: pass — https://github.com/Reidond/kedra/actions/runs/34164331109 at exact e492258871f6353a0c6548a0ec5d5e318d92bdd3. Ubuntu 24.04 runner, x86_64-unknown-linux-gnu, Rust/Cargo 1.98.1 and Git 2.55.0; formatting, Clippy, 19 tests plus 1 doctest, release build and unchanged-source/lockfile check all passed.
- Remaining / blockers: This evidence-only follow-up has no claimed future CI result. Full R03/R11 and R04 activation gaps are unchanged; no home/OS deployment or global plugin installation occurred.
- Next: Publish this observed-evidence follow-up; continue the documented synthetic Noctalia and disposition-transition experiment.

### WL-20260908-03 — 2026-09-08 — Usable-system implementation
- Agent / state: Codex; in-progress.
- Scope / base: clean main at c00374cae862c669460da35471950237216a3d14; owner requested full implementation and installation guidance, with VM installation/testing if needed.
- Completed: Reconciled plans, source, research reports and exact-base Actions success (run 34164461001). Selected disposable R01/R02 image/VM path as the first installation dependency. Existing WSL2 Ubuntu exposes /dev/kvm; QEMU is absent. Windows has approximately 64 GiB RAM and 817 GiB free workspace drive space.
- Checks / evidence: pass — Git status clean at entry, main/origin inspected, exact-base Rust CI successful. not-run — image build, installer, boot, signing, desktop and physical hardware.
- Remaining / blockers: No image/installer or production management implementation exists; production trust, credential setup and hardware qualification remain unresolved. Research uses disposable keys/VMs and Actions builds.
- Next: Implement and run an isolated CI image/VM experiment; record actual results before enabling dependent production operations.
- Milestone: Prepared pinned minimal Fedora/bootc-builder inputs, a disposable boot-evidence service and an Actions QCOW2/UEFI experiment. Local shell syntax, workflow YAML parsing and git diff --check pass. Installed QEMU 8.2.2 and OVMF 2024.02 in existing Ubuntu WSL2 under the owner's VM authorization; no guest has run. Publishing the isolated research branch is necessary to execute the required Actions OS build; production promotion remains unavailable.
- R02 finding: Actions run 34164873575 at 9016832 passed container lint/build and QCOW2 build; UEFI guest reached Fedora 44/kernel 7.1.13/bootc 1.16.10. Overall VM check failed (124) because the harness passed two paths to findmnt, then left QEMU waiting. Corrected mount invocations, failure poweroff and prefixed serial marker matching. Rerun pending; full installation/signing remain not-run.
- Source-planning milestone: Implemented sysroot source plan with committed HEAD snapshots, typed target validation, package intent, host replacements, modes and SHA-256 provenance. Added six synthetic Git tests and ADR 0005. pass — Rust 1.98.1 formatting, Clippy, 25 tests plus one doctest, release build; actual desktop JSON plan emits the four existing package candidates and zero placeholder payloads. Existing staged/unstaged/untracked files are preserved in tests. Linux CI for this implementation is not-run until publication.
- R02 milestone: corrected run https://github.com/Reidond/kedra/actions/runs/34165475139 passed at cfbfc05405f13d16dbbe5a604bc116b0763a421a. Minimal image/QCOW2 and UEFI VM checks pass with enforcing SELinux; 1,323,933,184-byte disk, SHA-256 ad20491cd85267e831e5238d1b3d53025cf7c5111c56eebd066b911cab1b89ea. Source-planning revision 160f566 passed exact Linux workspace run 34165622998. Full installer and R09 lifecycle remain unpassed.
- Next experiment prepared: R01 local TLS registry, ephemeral Sigstore keys, explicit signature-negative cases, signed A-to-B boot and retained-A rollback preserving newer synthetic data. Shell syntax/YAML checks pass; execution not-run. No production trust or registry publication configured.
- R01 initial run 34166170646 failed before registry/image creation: a missing closing brace in the jq policy expression caused exit 3. Disposable key generation succeeded; cleanup removed the temporary private files. Corrected expression; no signature or boot result is claimed from this run.
- R01 transport finding: run 34166267962 at be59b4b built/signed all variants in the local TLS registry and verified A under Skopeo's strict policy. QCOW2 installation failed because the builder imports a containers-storage image ID and the fixture had only a registry policy scope. Testing a strict local-store Sigstore rule with the same key and exact original repository; default reject remains. No permissive workaround or bootc update pass.
- Owner chose public signed images/installers containing only reviewed project files. Registry/model/API/SSH credentials and private runtime/home state remain excluded.
- Build-input milestone: source archive now writes deterministic tar from committed blobs with provenance, fixed archive metadata and retained executable modes; refuses existing outputs, known credential/runtime paths and private-key markers. Nine source tests pass; full workspace has 28 tests plus one doctest, fmt/Clippy/release build pass on Rust 1.98.1 Windows. This is a bounded refusal policy, not universal secret detection.
- Desktop candidate: added plain Containerfile, image-only RPM assembly, Fedora package intent, greetd login, niri/Noctalia defaults and offline first-use guide. Prepared R07 Actions package/configuration validation; execution not-run. Initial /etc/skel seeding applies only to future new accounts, not existing live homes. No physical target or XPS identity was guessed.
- R01 milestone: https://github.com/Reidond/kedra/actions/runs/34166793087 at 7705cc36b91546c45f006cf099d7621b3f883227 passed strict signed local-store import, five negative cases, signed B stage/boot, inherited rejection on B and rollback preserving newer synthetic data. Inspection found initial/rollback A lacks spec.image.signature; strengthened the next test using bootc 1.16.10's supported install-policy drop-in and inherited rejection on A/rollback. Full installer/promotion gates remain blocked.
- R07 initial run 34167255886 at 9e2b752 resolved/installed the desktop package set, then failed because Fedora's service account is greetd, not upstream-example greeter. Corrected to observed Fedora account. No graphical-session success claimed.
- Local VM milestone: the downloaded R02 disk matched its published SHA-256 and passed UEFI/KVM boot under Ubuntu WSL2/QEMU 8.2.2 with snapshot mode and no network/host disks. A saved-script rerun records QEMU exit 0 and KEDRA_R02_BOOT_PASS; the initial shell wrapper returned 1 despite the guest pass. No workstation OS installation occurred.
- Desktop milestone: run 34167524359 at 14be822 passed package resolution, image build, niri 26.04/Noctalia 5.0.1 validators and bootc lint (three warnings remain). Prepared graphical login/PAM/keyring/IPC/service tests with a generated disposable account password, software GL, virtual audio and isolated test-marker serial port. Actual graphical runtime not-run.
- R07 graphical attempt 34168467701 at 891d6cc built the test QCOW2 and reached login-ready. Host screenshot failed before credentials were submitted: QMP no surface plus Mesa X11 shared-memory attachment errors. Next run keeps Xvfb/QEMU under one user and captures the actual Xvfb display for GL scanouts without a software QMP surface. This is a harness correction; graphical session is still unpassed.
- Release-verification milestone: implemented exact-byte P-256/SHA-256 signed release/checkpoint verification, target/approval/protocol checks, expiry/replay/high-water logic, and offline installer size/hash verification. Ten new tests plus independent OpenSSL 3.0.13 signatures pass; full local Rust 1.98.1 fmt/Clippy/38 tests plus one doctest/release build pass. CLI results explicitly do not grant deployment authority. ADR 0006 and release-verification guide describe the boundary; production signer/helper/rotation remain unimplemented.
- R01 stronger result: run 34167524353 at 14be822 passes containerPolicy on initial A, B and rollback A, with unsigned no-flag switches rejected in every state. Installed policy handoff through the tested QCOW2 route is proven; interactive ISO and promotion authority remain separate gates.
- R07 result: run 34169415857 at b3e569a passes graphical password login, niri/Noctalia IPC, portal/audio services and unlocked synthetic keyring. Screenshots were inspected. Visual follow-up removes the obstructing startup help overlay, retains an explicit help shortcut, quiets console boot chatter and enlarges the VM display; these changes await their next run.
- Visual/build cleanup submitted: explicit help shortcut with startup overlay disabled, quiet console default (journal retained), fullscreen VM sizing, and removal of observed recomputable DNF/swcatalog/ldconfig caches from the image. Research desktop runs now supersede stale branch runs; no production artifacts or keys are involved. Native config/runtime validation follows in Actions.
- Installer slice prepared: separate Anaconda Containerfile and Actions bootc-installer workflow consume a distinct desktop payload, record resource/media identities and extract boot configuration before VM testing. No unattended disk/account profile or production key is supplied. Initial origin is explicitly localhost research, so this is not owner installation media; interactive multi-disk/encryption/account/origin checks remain not-run.
- Installer interface correction: run 34171335800 at 0a5991a built desktop/Anaconda containers but rejected --bootc-installer-payload-ref. Current source confirms BIB compatibility spelling is --installer-payload-ref (prefixed form belongs to image-builder). Added a separate official GHCR v82.0.0 amd64 pin and early build-help check; existing Quay-based QCOW2/R01 evidence retains its exact pin. Installer/partition tests remain not-run.
