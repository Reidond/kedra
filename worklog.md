# Kedra worklog

Shared continuation record for Codex, Claude and other authorized contributors.
Follow the maintenance contract in `AGENTS.md`. Keep the status snapshot current;
append completed work entries without silently rewriting history. This file is
public: no credentials, transcripts, private home data or sensitive raw logs.

## Current project status

Last updated: 2026-09-08 (Europe/Kiev).

| Area | Verified status / next boundary |
|---|---|
| Phase | Implementation underway: minimal VM, strict signed-update/rollback and graphical desktop prototypes pass. No promoted owner installer yet. |
| Implemented | Three-package flat Rust workspace; committed-source plan/archive; signed release/checkpoint verification; synthetic line models; private SQLite storage and native Noctalia review commands; ordinary-user agent launchers with native Codex probes. Selected Noctalia patch export/receipts pass Linux tests; activation and privileged deployment remain unavailable. |
| Repository knowledge | One Codex/Claude plugin with 12 Kedra and 18 selected upstream skills in plugins/kedra/skills. Ordinary files; no submodule, symlinks or duplicate discovery copies. No personal installation. |
| Latest verified check | Source a2c0e63 passed workspace 34192732940: 81 Linux tests plus 1 doctest and actual CLI/source receipts. Fedora desktop 34197344807 at ba011ec passes GUI/keyring/home review and installed private Codex runtime/helpers/profile retention. Bitwarden runtime and corrected installer first boot remain pending. |
| R11 | Canonical skill files readable; Codex marketplace and fresh-profile root/crate skills/list found no Kedra entries. Claude authoring validation passed. Loaded-plugin model/editor checks not-run; distribution review blocked. |
| R03 | Synthetic line models and native three-field Noctalia capture/staging/local policy with private persistence pass. Selected-field export/receipts pass the actual Linux CLI round trip. Generic file/line integration, discard and R04 activation remain open. See docs/research/R03-home-review/REPORT.md. |
| Other R01-R10 | R01 strict signature/update/rollback prototype passes, including initial and post-rollback policy. R02 minimal VM passes in Actions and locally. R07 graphical login/session/keyring prototype passes. R08 verification has local tests; R09 source/archive tests pass. Full installer, promotion, activation, owner authentication and physical hardware gates remain open. |
| Current task | Owner requested full implementation through usable installation. Installer 34195114452 completes encryption/owner creation and boots without ISO, but first-boot health fails due to hidden home and logical-root remount. Native mount and physical-root fstab corrections are prepared alongside Bitwarden image integration; rebuilt tests are pending. No promoted owner media exists. |
| Machine effects | QEMU/OVMF and graphical test dependencies are installed in existing Ubuntu WSL2; the ordinary user now belongs to kvm. Effects are under VM-test authorization. No workstation home enrollment, vault/profile changes, host-disk formatting or workstation OS installation. |

Evidence: [R11 report](docs/research/R11-rust-workspace/REPORT.md),
[research index](docs/research/status.json),
[bootstrap run 2](https://github.com/Reidond/kedra/actions/runs/34138882781),
[baseline run 3](https://github.com/Reidond/kedra/actions/runs/34139103852),
[documentation run 4](https://github.com/Reidond/kedra/actions/runs/34140106798),
and [update-plan run 6](https://github.com/Reidond/kedra/actions/runs/34145002985).
The snapshot summarizes those records; source, exact CI runs and case-level
reports remain authoritative for what was tested.

### Next concrete actions

Check the corrected Anaconda build, then test deliberate disk choice,
encryption, administrative account creation and first boot using two disposable VM disks. Join
the interactive installer to R01's strict signed-origin policy before promotion.
Implement the narrow helper protocol and home filesystem coordination on the
tested pure models/storage; do not enroll the workstation. RPM refresh/no-change,
agent packaging, owner credential setup and physical qualification remain open.

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
- Installer build result: run 34173909609 at eca177c produced a 3,490,482,176-byte legacy ISO, SHA-256 67f3628bb66ea63b729b3b52dd1c49f67852e23faf1260b71e10c4656820c7f9; it was not booted. Inspection found forced text mode, broad clearpart defaults and legacy ostreecontainer import. Switched the experiment to upstream v82.0.0's preferred generic ISO/native bootc path with explicit interactive defaults and graphical/rescue entries. No SELinux-disable flag or preset disk/owner account was added to the installed OS. New build and multi-disk tests pending.
- R03 Noctalia model milestone: added a three-field safe projection of qualified native full-export TOML, pinned selection, exact local-only and persistent app-owned policy, source publication chains and non-mutating next-baseline planning. Ten tests, canonical reconstruction and the path-free example pass; all local Rust checks now pass 48 tests plus one doctest. Filesystem crash durability/source export/live activation remain unimplemented. Added a native Noctalia IPC/export probe to the disposable R07 derivative; that run is pending.
- Release verifier publication evidence: exact 315389786db7b1519c5590ce9583ba1d972b163c passed Linux run 34170739316, including 16 OpenSSL signatures and negative CLI cases. This does not configure production signing or machine authorization.
- Privilege-boundary refactor: moved Git/source archive implementation and its tests into the ordinary-user sysroot package. Shared core now has no ambient filesystem opens or process execution; artifact verification consumes a caller-provided stream. Three packages remain. pass — local fmt/Clippy/48 tests plus one doctest/release build; public CLI behavior unchanged. This prepares the helper boundary without enabling privileged operations.
- Persistent-state slice prepared: the existing helper package now has a flat Linux-only library target for owner/link/type/inode-checked private SQLite stores, bounded/schema/checksum validation, CAS revisions and retained history. Added generated-directory concurrency/corruption/process-interruption tests. Windows compile/fmt/Clippy pass but do not execute Linux storage; Actions validation is pending. The helper binary still refuses operations. No new package, real state enrollment or home activation.
- Linux storage check 34179085588 at aa752c9 reached Clippy and found that rusqlite 0.40.2 limit setters return fallible results. Added error propagation for all three; no lint or safety guard was weakened. Storage runtime cases remain pending the corrected Linux run.
- Verified follow-up: exact d74c4c0 passes Linux run 34179189211, including eight storage cases (56 workspace tests plus one doctest) and OpenSSL interoperability. Desktop run 34179189185 passes login/session/keyring and native Noctalia export/IPC projection. The inspected settings screenshot still has a cramped virtual scanout; display sizing remains open. Updated research/status handoff to separate tested subsets from unavailable management operations.
- Installer download: generic ISO run 34176407860 at 76dc82a produced 2,540,959,744 bytes, SHA-256 8734723fb87db17a129d4293858a0638164ee4db898990453fadd03eed788205. Initial artifact transfer truncated; scoped HTTP range resume is in progress. Interactive disk/encryption/account/first-boot cases remain not-run.
- Local installer result: transfer resumed and SHA-256 verified. UEFI boot of the generic ISO failed before Anaconda with systemd SELinux permission errors; screenshot inspected, no disks selected, sentinel qemu-img comparison pass. The pinned v82 generic pipeline omits installer filesystem labeling. Prepared a standard osbuild SELinux stage insertion with drift refusal, packaged-label check and a diskless enforcing-userspace smoke test. Corrected Actions results pending. Graphical VM screenshot access was repaired; an earlier progress message claiming Anaconda had started was corrected immediately after inspecting the failure.
- R05 launcher slice: prepared ordinary-user Linux launchers with runtime/profile selection, verified offline checkout, explicit editing target, version-specific private state, cooperative lock through exec and exact argument/exit forwarding. Broad vault sessions are excluded; HOME is preserved; nested explicit personal launches restore saved config controls. Seven Linux unit cases and one CLI process case await Actions; Windows pinned fmt/Clippy/48 tests plus one doctest/release pass. Official Codex 0.153.4 package checksum/version pass in isolated research storage; no bundled binary entered an image or personal profile. Claude preinstallation terms and actual runtime/profile/distribution evidence remain open.
- R05 wrapper result: exact 082f8d4 passes Linux run 34181692426, including seven launcher unit cases and the actual exec/lock/exit-status test (64 workspace tests plus one doctest). Prepared pinned official Codex 0.153.4/0.153.3 package probes and per-binary Sigstore verification in a network-isolated generated user environment. The archive attestation API returned 404; no attestation pass is claimed. Native probes, component notices/source review and Claude public-preinstallation agreement remain pending; no agents have entered a public Kedra image.
- R02 correction passes: run 34180796587 at da140ff labels installer systemd as init_exec_t and reaches diskless userspace with enforcing SELinux. Corrected 2,541,139,968-byte ISO SHA-256 is 85753341b9070938590d9e9c9a70bd7f67d44e4ba685e0be5627eb390566b74e; download is in progress for UEFI/UI/install tests. No physical installation or signed-origin handoff is claimed.
- R05 native finding: run 34182176074 at 3be3030 passes Codex/code-mode-host/bubblewrap Sigstore verification but fails the personal-profile assertion. Codex startup can create runtime helper files before parsing help/version. Version discovery now uses disposable config directories without changing HOME; native tests distinguish management preservation from explicitly selected personal runtime writes. Corrected checks pending; no account/model request or binary publication occurred.
- R07 display finding: recorded niri output reports show QEMU's preferred mode is 640x505 while 1280x768 is an available mode. The disposable test derivative now selects 1280x768 at scale 1 and asserts logical dimensions before screenshots. The ordinary desktop has no new monitor override; runtime validation is pending.
- R05 native correction: exact 43873cb passes run 34182776503: Codex/code-mode-host/bubblewrap Sigstore checks and all four 0.153.4 bundled/0.153.3 personal runtime/config combinations. Version/help/login-help succeed without changing seeded personal settings during management launches or dirty checkout state. This does not prove model turns, authenticated login, MCP/skill discovery or full distribution eligibility. Corrected installer range download is nearing completion; UEFI/UI installation remains next.
- R02 local follow-up: corrected ISO checksum passes and UEFI reaches userspace, but Anaconda/rescue-shell operations are denied; the direct shell remains getty_t. No disks selected or installation performed. Pinned upstream Lorax explicitly sets its separate installer environment permissive and creates an install-user WebUI account. ADR 0010 adopts that media-only contract, retains labeling/auditing and strict signature gates, and requires enforcing SELinux in the separate desktop payload and after real installation. Strengthened the smoke to require Anaconda service/log startup. New Actions results pending; early userspace pass is not reclassified as installer success.
- Diagnostic-only boot: temporarily appending enforcing=0 to the existing disposable VM's boot entry reaches Anaconda's language screen; no install was started. This is UI investigation, not acceptance of the media or installed OS. Missing .buildstamp made Anaconda display its bluesky fallback; added standard Main metadata with Kedra/Fedora version and source-commit timestamp, retaining the research/pre-release flag. Also corrected timeout diagnostics in the new smoke. Updated media build pending; actual installation uses rebuilt media without the temporary kernel override.
- Display harness correction: R07 run 34182940311 passes the 1280x768 guest-mode assertion, but its screenshot still occupies the small host window. Local xdotool geometry proved QEMU's GTK window remained 640x505 because Xvfb has no window manager to honor fullscreen; resizing that owned window to 1280x800 reveals the complete Anaconda summary. Added xdotool as a VM-test dependency and scoped R07 window resizing to the spawned QEMU PID, with matching 1280x768 capture/guest sizes. New screenshot qualification pending. Diagnostic Anaconda summary shows no disks selected and root disabled; no installation started.
- Native installer findings: diagnostic UI correctly leaves both disks unselected; lsblk maps vda to KEDRA-INSTALL-ONLY and vdb to KEDRA-KEEP-DATA. Selected only vda and entered a generated encryption passphrase for a plan; Begin Installation was not clicked. Actual Anaconda 44.30-2 source always requires networking for bootc and treats a seen locked-root directive as an admin. Prepared two source-hash-guarded, media-only property adaptations with native getter cases (ADR 0011). Bootc 1.16.10's future fetch check is opt-in; no skip-fetch, TLS or signature-policy change is needed. New build/VM results pending; actual disk writes and installed enforcement remain untested.
- R07 capture result: exact afb8a67 passes run 34184975810; inspected 1280x768 Noctalia settings screenshot fills the correctly sized QEMU window with readable controls. Login, services, keyring and native projection still pass. Research-only window geometry is recorded; physical display/audio/suspend qualification remains open. Current installer build at cfc956d is pending; the diagnostic VM remains at a storage plan without starting installation.
- Diagnostic VM closed: qemu-img comparison passes after stopping the storage-planning session; the unselected sentinel disk is identical to its original. No installation was started. R05 component review identified bundled ripgrep 15.2.0/PCRE2 10.45 and zsh 5.9.0.3-test. Added pinned upstream notices and retained bubblewrap source/license plus the zsh patch to the fetch output; native CI validation is pending. This does not close public distribution or Claude terms requirements. Installer run 34185915639 remains in ISO assembly.
- Installer build milestone: run 34185915639 at cfc956d passes Anaconda startup, labels, both guarded native property adapters and separate installer/desktop SELinux configuration checks. New ISO is 2,550,966,272 bytes, SHA-256 f6240416ff5adae95b98e34d3f093329228c72588abb527aecfced1871209db5; transfer is in progress. R05 run 34186921224 at 116a658 passes pinned notices/signatures and the native runtime/profile matrix. The owner has been asked about Anthropic's required Commercial Terms; no acceptance is inferred while unanswered.
- R03 implementation in progress: connecting the three-field Noctalia state model to private SQLite review commands, with ordinary-user execution, installed baseline provenance/hash checks, bounded effective export and pinned selection across later writes. Explicit border defaults match the true/true values observed in R07. Source export and live activation remain separate unfinished work. Local/native tests are pending; no current-workstation home has been adopted.
- R03 local checks: initial CLI test correctly exposed the obsolete expectation that the entire home namespace was unavailable; updated it to assert that activation remains unavailable. Pinned Windows fmt/Clippy/49 tests plus one doctest/release build pass. Six new Linux persistence/input cases and the native R07 command/GUI sequence are pending Actions. ADR 0012 records the implementation boundary; no production-home or source-export success is claimed.
- Linux compile finding: run 34188055729 at 866708b rejects LowerHex formatting for sha2 0.11's digest array in the new Linux-only bridge. Replaced it with the bytewise formatting already used elsewhere; corrected Linux results pending. The ISO download completed and its SHA-256 passed. A fresh local VM initially could not access /dev/kvm after WSL restarted; added the ordinary WSL user to the existing kvm group under the VM-test authorization. QEMU and the checkout harness continue to run unprivileged.
- R03 native result: exact 3d108e9 passes workspace 34188179270 (71 Linux tests plus one doctest) and desktop 34188179252. The generated-account native sequence initializes review, pins light while live becomes auto, and verifies exact-local and app-owned transitions. Generic lines/source export/discard/activation remain unfinished. Local QEMU now explicitly uses X11/software GL, avoiding WSL's Wayland display; no host-app window is needed.
- R02 installation result: fresh ISO 34185915639 boots UEFI without overrides and passes unselected-disk, deliberate vda-only/encryption and offline/admin UI cases. Began installation only after the generated owner had wheel membership. Deployment failed GetBlob import because /var/tmp uses a 1.6 GiB installer filesystem; the encrypted target still had 59 GiB free. Stopped VM and passed sentinel comparison. Added version/hash-guarded selected-disk scratch binding with seven inert lifecycle cases (18 total native adapter cases); local exact-source checks pass. ADR 0013 records failure/cleanup behavior. Rebuilt-media install and first boot remain pending.
- R03 source-export work in progress: added format-preserving selected-field patches, exact retained-commit plans and history-bound source receipts. Eight Windows source/export cases pass, including dirty source/index retention, host routing, source conflicts, all-object privacy checks and stale-receipt refusal. The actual Linux CLI/SQLite/source round trip and full updated checks are pending. Scratch-fixed ISO build 34190162895 at 8ae2828 passes with all three guarded adapters; its 2,552,576,000-byte ISO SHA-256 is 28f33970ea3af4702b9c68d0d567ebcff4c824bb2eb1d136d53f9e4c55fba832. Transfer is in progress for a fresh installation.
- R03 export local result: pinned Windows fmt/Clippy/58 tests plus one doctest/release build pass. Added the actual Linux CLI source/SQLite integration fixture, cached status and offline selection/export/receipt paths; Linux execution is pending. ADR 0014 documents the selected-blob-only Git boundary and ancestry checks. Generic line integration, discard, activation and privileged deployment remain unfinished.
- R03 Linux export result: exact a2c0e63 passes workspace 34192732940, including the actual CLI/SQLite/source-commit round trip (81 Linux tests plus one doctest). Noctalia source export/receipt is implemented and tested; generic line integration/discard/activation remain open. Codex private image-input packaging is being prepared separately and is not yet in an image.
- R02 scratch follow-up: fresh 34190162895 media completes bootc image import, GRUB installation and finalization; native log records Installation complete. Anaconda then aborts on scratch rmdir with EROFS because bootc remounted the Btrfs physical root read-only. Added the same validated-target read-write remount that native PrepareBootcMountTargetsTask performs next, only on EROFS, before retrying nonrecursive cleanup. New lifecycle cases cover this and a failed remount. Owner creation and installed boot are still not-run; no full installer success is claimed.
- R02 cleanup diagnosis: manually tested the exact remount/rmdir on the failed disposable VM; it succeeds and the target is writable for Anaconda's remaining setup. This is diagnostic repair, not a completed installer run. Stopped the VM and passed the sentinel comparison. All 20 inert adapter cases pass against the exact native sources; fresh rebuilt-media validation follows.
- R05 packaging preparation: added a closed-layout private Codex tar with repeated archive/member identity checks, exact Sigstore identity verification, component notices and pinned corresponding source. Added the MIT text for locked Ratatui 0.30.2. The R05 workflow will validate packaging before image integration; image-context and Fedora runtime-probe changes remain uncommitted while the independent cleanup-fixed installer builds in 34195114452 at 923a282. No personal/global agent or skill installation, model call, or Claude terms acceptance occurred.
- R05 image integration: run 34195980696 at 9aa59ad passes deterministic package preparation, native signatures and the runtime/profile probes. Wired its private Codex archive into both desktop and installer contexts and added a generated ordinary-user Fedora VM probe for private runtime/helpers and personal-profile retention. Syntax checks pass; actual image build/runtime checks are pending. Claude remains excluded pending the owner's answer.
- R02 rebuilt-media milestone: cleanup-fixed run 34195114452 at 923a282 passes build, labels and Anaconda startup. ISO is 2,552,686,592 bytes, SHA-256 24161c57cac073137bc4730684a106815fc8d654b65483c52e46c88b89fa0e91. Transfer is in progress; this does not yet prove installation or first boot.
- R06 packaging in progress: inspected official Bitwarden Desktop 2026.8.0 RPM/source identities and native layout. Prepared ordinary-runner extraction without RPM script execution, relocation of unchanged app files into the immutable image, retained source/notices and separate login/systemd/TTY SSH socket defaults that preserve explicit personal agents. Added Fedora startup/screenshot/propagation checks. Local syntax checks pass; package execution and actual VM runtime remain not-run pending completion of the current independent Codex builds. No vault/profile changes or owner credentials were used.
- R05 actual-image result: run 34197344807 at ba011ec passes Fedora graphical login/session/keyring, native Noctalia review and the installed private Codex runtime/helpers/profile-retention probe. Exact marker is retained in output/r07-run-34197344807/vm/serial.log. The companion Codex installer is still building. R06 local generated-directory checks pass the native systemd environment generator and login shell for absent/default and explicit personal sockets; systemd 255 keeps an explicitly empty socket empty, while the graphical/login wrapper supplies a default. Fedora runtime remains pending.
- R02 fresh acceptance in progress: run 34195114452 ISO download and whole SHA-256 pass. Fresh isolated VM boots without overrides; serials identify vda KEDRA-INSTALL-ONLY and vdb KEDRA-KEEP-DATA. Both start unselected. Deliberately selected vda with encryption, created generated kedra-owner with wheel membership, and began installation offline. Image deployment is running; first boot and sentinel comparison remain pending.
- R02 first-boot finding: Anaconda completed installation; clean shutdown and sentinel comparison pass. Booting the retained disks without the ISO unlocks LUKS and authenticates the owner with enforcing SELinux. Health fails: the owner directory exists behind the separate home subvolume, and physical-root fstab options trigger a failed remount of logical overlay /. Prepared the missing native separate-mount loop before useradd and an installer-only physical-root /sysroot normalization, preserving read-only policy (ADR 0015). All 23 inert native adapter cases and seven fstab cases pass; fresh-media validation remains pending. A repair on the generated VM is diagnostic only.
- R05 companion installer build 34197344823 at ba011ec passes assembly/startup. It was not installed and shares the newly diagnosed R02 issues. Bitwarden and first-boot fixes can now be queued without interrupting that completed build.
- R06 actual-image result: desktop 34199985702 at 4dc3355 passes package preparation, the logged-out Bitwarden window, GUI/systemd/TTY default socket propagation and existing Codex/Noctalia/keyring checks. The captured native login window is visually verified. Authenticated vault/signing states remain not-run. R02 corrected ISO 34199985719 passes assembly/startup: 2,864,662,528 bytes, SHA-256 7f18c52e51ac5f9ec27d7e5cdc5d7096a0a74e79ab7f8dba7bedcd396b7a4086; download and whole checksum pass, and a fresh VM test is starting.
- R02 diagnostic result: after copying only the generated hidden owner home into its selected subvolume and normalizing the fstab root entry, reboot reaches the intended niri/Noctalia desktop. Native niri validation, enforcing SELinux, zero failed system units and unlocked synthetic keyring pass; bootc reports the unsigned localhost research origin as expected. Clean shutdown and sentinel comparison pass. This validates the cause, not a fresh installation; 34199985719 must pass independently.
- R10 implementation in progress: prepared a bounded stdin helper protocol, fixed root-owned trust/state paths, independent signature/freshness/scope checks, native bootc observations, durable intent/receipt journaling, explicit pending replacement and retained rollback hold (ADR 0016). The CLI uses explicit sudo authorization. No production trust/key exists and no workstation enrollment/staging occurred. Windows fmt/Clippy/67 tests plus one doctest/release build pass; Linux execution and native signed-helper tests remain pending. A protocol test caught ignored extra fields on a Serde unit variant; an empty struct variant now rejects them.
- R10 Linux result: source 10e8cfd passes workspace 34203674917, including all 90 Linux tests plus one doctest and the shared trusted-reader refactor. Prepared the native R01 helper experiment using generated public trust, Cosign-signed metadata/checkpoints, exact running enrollment, metadata/OCI negatives, pending retention, actual B staging/boot, retained A rollback, high-water/hold preservation and explicit resume. This is pending execution with disposable signing keys only.
- R02 finalizer path correction: native SetSystemRootTask resolves the actual OSTree deployment below the physical filesystem, so the new post step must not treat /mnt/sysroot itself as the deployed OS. Corrected it to use native ostree --print-current-dir with strict descendant/layout checks. Twelve text/path cases pass locally. The running 34199985719 installation still uses the previous finalizer and cannot qualify this correction; rebuilt media is required.
