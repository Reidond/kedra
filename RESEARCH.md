# Kedra: pre-implementation research

Revision 1.2, repository handoff, 2026-09-07. This preserves R01-R11 from the design
session. Packet status lives in docs/research/status.json; individual reports and
CI runs are the evidence. A documented upstream feature or scaffold is not a pass.

## Working contract

Preserve Fedora 44 bootc, plain Containerfile, GitHub Actions OS/ISO builds,
signed digests, writable-home line management, Bitwarden, independent personal
agents and explicit host targets. Rust edition 2024/Cargo with no first-party src/
is settled. Do not reopen BlueBuild or TypeScript/Effect/Vite Plus. Use the pinned
repository-local Rust skills without global hooks/plugins/MCP installation.

P0 blocks the named production feature, not safe independent experiments. Work in
disposable VMs/targets and synthetic homes. All OS/installer builds are in Actions;
local end-to-end and manual checks are allowed under the owner's testing policy
in AGENTS.md. Do not add unit/model/mock tests, doctests or repository self-checks.
Never test destructive bootc/home/disk operations on
the current workstation or use real vault/production signing data as fixtures.

Each packet needs a reproducible experiment, positive and negative cases, exact
versions/digests/source identity, expected/actual outcomes, exit codes and an ADR
or remaining blocker. Store results under docs/research/Rxx-topic. Use not-run,
pass, fail or blocked per case. Separate static CI, actual agent discovery, VM
boot, real-account interaction and physical hardware validation.

## Packet map

| ID | Question | Feature blocked |
|---|---|---|
| R01 | Exact signature compatibility/enforcement | P0 trusted release deployment |
| R02 | Installer, storage and CI feasibility | P0 installation |
| R03 | Line review, selected export, local-only policy | P0 real dotfile adoption |
| R04 | Activation/writers/interruption/rollback | P0 automated home deployment |
| R05 | Official agent packaging and personal coexistence | P0 bundled publication |
| R06 | Bitwarden/keyring/API/model/registry bootstrap | P0 authenticated workflow |
| R07 | Complete desktop/session and actual hardware | P0 desktop; per-device qualification |
| R08 | Release authority/privacy/rotation/retention | P0 production promotion/signing |
| R09 | Multi-target scope and independent lifecycle | P1 second-machine capability |
| R10 | Privileged interface and persistent schemas | P0 elevated operations |
| R11 | Rust tooling/layout and pinned skill discovery | P0 production Rust/agent workflow |

## R01: Prove the signature path

**Question:** Which pinned signer, registry attachment format, signature policy
and Fedora bootc stack enforce the intended trust from installation onward?
Read kedra-release-signing and its threat matrix; primary sources are bootc-switch,
policy, registries and podman-sign in docs/SOURCES.md.

Build minimal images A/B in Actions under a disposable namespace. Use temporary
test-only keys. Install A, then stage and boot B using the intended enforcing
bootc path, not just an external verification command. Record policy and discovery
configuration. Test installer/copy transport, which may not carry signatures with
the image automatically. Preserve before/after running and staged digests.

Cases: unsigned image; wrong key; allowed key/wrong repository; incorrect target
or architecture; tampered image/manifest/home binding; missing signature attachment;
valid but unpromoted candidate; malformed reference; verification option omitted;
and offline recovery using retained state. Never make the policy permissive to
get a green run. A trust failure must not advance the installed reference.

**Pass:** Only eligible signed input succeeds through actual bootc, with enforcement
retained for subsequent updates. Return codes/state demonstrate each rejection.
**Deliver:** Signer/policy ADR, Actions job, exact image/tool identities, negative
matrix, redacted logs, test-key labeling and installer-to-update trust diagram.
Dedicated-key is the initial design; R08 handles authority/rotation above validity.

## R02: Prove installer and runner feasibility

**Question:** Which pinned osbuild/image-builder route/ISO type produces safe,
usable media for the exact payload with measured Actions resources?
Read kedra-github-actions; sources image-builder, actions-runners, release-limits.

Pin builder and Fedora digest/architecture. Build minimal ISO and QCOW2 in
separate jobs. Record CPU/memory/disk peak, KVM availability, privileges, runner
identity and output sizes. Repeat with realistic desktop payload before selecting
a runner class. Do not guess free resources suffice or paid resources are required.

Use the separate Anaconda installer environment/bootc payload model. In a fresh
UEFI VM with multiple disks, require deliberate disk choice and preserve the other
disk. Test root filesystem, encryption, account/UID setup, unique machine identity
and SSH host keys, local recovery, target enrollment and real registry origin.
Do not bake fixed credentials or install automatically to the first disk.

Check Secure Boot separately: firmware setting, bootloader/kernel chain and actual
boot result. Explicitly defer or decide TPM unlock, hibernation and dual boot.
Measure ISO versus GitHub's per-asset limit; if split, reassemble and verify the
signed whole-file checksum. Verify installed A uses the R01 update path to B.

**Pass:** Reproducible CI media installs the intended digest safely and updates.
**Deliver:** Installer/storage/boot ADR, CI resource report, artifact manifest,
install transcript, permissions/cost assumptions and recovery instructions.

## R03: Prove writable-home review and local policy

**Question:** Can a small Git-backed manager preserve live app writes while
separating selected, unstaged and local-only content across baseline changes?
Read kedra-home and references/state-model.md; sources git-stage, git-merge,
git-faq and noctalia. Do not implement tracked-file ignore using index flags.

Use a synthetic live home, private review repo and separate source repo. Model B,
L, next baseline N, staging S, local-only I, published-not-deployed P and journal
separately. Demonstrate why merge(B,L,N) alone does not preserve every disposition.
Choose the representation from evidence, not a speculative large database design.

Required cases: one line from a larger hunk; app writes after staging; ignored and
published hunks in one file; overlapping selection/policy; ignored value changes
again; duplicate or ambiguous context; upstream edits same key; upstream adopts
the local value; reordered text; insertion/deletion/rename; line-ending/encoding
changes; no trailing newline; non-ASCII paths and explicit unsupported-path policy.

Compare exact-hunk policy to narrow app-owned structured fields. A stale mapping
must become visible instead of matching a different line. Display-only filtering
must not hide content that still exports. Source HEAD may advance before export;
apply/rebase selected bytes with provenance and stop on conflict. Local review
commits cannot enter pushed ancestry. Mark already-published content awaiting
image deployment. Host changes must return to host source, not shared files.

Noctalia case: project selected effective values from curated config plus GUI
overrides. Stage one setting, ignore another, preserve unrelated/newer state;
remove redundant override only when the deployed baseline and unchanged override
make that safe. Do not capture the whole state directory. Binary databases need
an application-supported safe textual view or remain unmanaged.

**Pass:** Exact deterministic fixtures, selected snapshot stable under later
writes, ignored content never exported, no automatic adoption, unresolved
classification/content conflicts visible, applicable operations idempotent.
**Deliver:** Patch/state ADR, golden/property fixtures for no-loss/no-leak,
minimal CLI experiment and recorded synthetic review sessions. Real homes remain
out of scope until this and R04 pass.

## R04: Prove activation and rollback

**Question:** How can home follow the image without applying new-schema config
to old software or losing writes across reboot/crash?
Read kedra-home, kedra-bootc and kedra-security; sources bootc-fs and git-merge.

Use two disposable app/OS versions with different config schemas. Simulate writers
that overwrite and atomic-replace files, including delayed in-memory saves. Prepare
an update, mutate after preflight, then reboot into the proposed pre-app/session
activation boundary. Candidate validators must not execute untrusted scripts with
root privilege. Handle encrypted/unavailable homes and atypical login sessions.

Inject failures before/after checkpoint, after one group file write, before journal
completion, after restart; also full disk, permissions, stale hashes, competing
invocations and prior incomplete transactions. A pre-write hash check is not
sufficient against delayed writers; an individual atomic rename is not a group
transaction. Compare controlled logout/login with explicit stop/reload contracts.

Roll back after newer personal edits and after state migration. Old sysroot/app
readers must handle or safely reject newer schema, never treat it as empty. Do not
promise a filesystem snapshot is semantically compatible with older application
versions. Include deletion of a baseline file that is locally modified.

**Pass:** No live conflict markers, no silent lost writes, coherent groups, no
baseline advancement after failed activation, recoverable interruptions, usable
TTY/recovery even for critical compositor failures.
**Deliver:** Activation state machine, journal/schema ADR, writer/session contract,
crash-injection tests and rollback matrix. Automatic health rollback is separate.

## R05: Prove agent coexistence and distribution

**Question:** Can official bundled CLIs live privately while personal installs,
updates, credentials, MCP, hooks and skills remain independently usable?
Read kedra-agents and launch-matrix; sources codex/claude config/auth/skills and
claude-distribution. A download URL is not redistribution clearance.

Create an agent test image with pinned official artifact hashes. Install a
different personal version using a supported user mechanism. Test bundled or user
runtime with management/personal config as explicitly selected. Record executable,
version, discovery roots, auth-store identity, MCP/hooks and files written without
publishing credentials or transcripts. No HOME replacement shortcut.

Run from outside/nested/dirty/moved/offline checkout, with missing binaries,
conflicting synthetic personal skills and concurrent sessions. Preserve user Git
changes and upstream argument forwarding/exit codes/signals. Missing requested
user runtime is an error, not silent bundled fallback. Editing host B on A never
implicitly deploys B locally.

Update the personal runtime alone; update/rollback the bundled runtime alone.
Check invocation-scoped DISABLE_UPDATES does not affect user launches or leak into
personal behavior. Test cross-version state and shared keyring identifiers;
CODEX_HOME/CLAUDE_CONFIG_DIR are not complete sandboxes. Repo skill lookup outside
these directories needs actual tests, not assumptions.

Review supported binary packaging, authenticity, required notices and preinstallation
conditions before publishing bundled agents. Keep official CLIs and login flows
unmodified. Do not use custom OAuth/session copying to avoid a setup prompt.

**Pass:** Plain codex/claude stay personal; sysroot defaults private; personal
updates/MCP/skills work without repo adoption; management never overwrites personal
state; an acceptable distribution route is documented.
**Deliver:** Matrix/report, packaging ADR, artifact pin installer, launcher
argument/environment contract, safe write traces and regression tests.

## R06: Prove credential bootstrap

**Question:** Can a new session push source and inspect/deploy releases through
separate credentials without private-key export or broad vault access?
Read kedra-bitwarden; sources Bitwarden/OpenSSH/environment/gh/agent-auth/bootc-secrets.

In a disposable session test Bitwarden logged out, locked before first unlock,
unlocked, relocked and restarted. Verify SSH socket from GUI terminal, TTY,
systemd user service and both launchers. Select only a public-key identity backed
by the vault key. Do not generate a replacement or suppress signing/host-key
prompts just to make automation pass.

Test Secret Service/keyring startup with the chosen niri/login environment,
supported GitHub/model sign-in, refresh, logout and profile identity. Detect gh
plaintext fallback. SSH Git protocol is not GitHub API auth; Bitwarden Desktop
is not a general Secret Service or the separate bw vault CLI session.

For private OCI images, test a minimal runtime pull credential at the correct
bootc root path, expiry/rotation and cold boot before user/Bitwarden starts.
Choose interactive-only versus unattended pulls explicitly. Do not put broad
BW_SESSION, OAuth or registry tokens into the agent environment or git source.

Test offline startup/repair. Recovery cannot require an agent subscription or
Bitwarden GUI. Use synthetic credentials for leak fixtures; never store production
secrets and promise to redact them later.

**Pass:** Authorized push, Actions lookup and intended pulls work through the
correct independent identities. No private key/token/vault state reaches source,
review objects, image layers, ISO or logs. Missing auth gives a clear setup step.
**Deliver:** Credential-flow diagram, idempotent setup, session configuration,
auth report, privacy tests and offline recovery instructions.

## R07: Prove Fedora desktop and hardware

**Question:** What complete package/session set works on actual Fedora 44 and the
chosen hardware rather than merely appearing in upstream docs?
Read kedra-desktop and Noctalia notes; sources Fedora/niri/Noctalia docs.

Resolve actual packages and inspect their versions/services/defaults in CI. Test
niri session, login/TTY recovery, systemd/D-Bus environment, Noctalia effective
settings, portals/file picker/screen sharing, Xwayland apps, PipeWire/WirePlumber,
network/Wi-Fi, Bluetooth, lock/idle and Secret Service. Do not mix v4 shell commands
with v5 TOML or assume latest docs match the package. Keep SELinux enforcing.

VMs test boot and generic integration. Obtain a permitted redacted hardware
inventory for the desktop; remembered GPU/display context is not a live probe.
Test actual acceleration/scaling/modes, audio input/output, USB, suspend/resume,
networking and firmware update path. Exclude serials and unrelated private data.
Workarounds need a host-specific reason and removal condition.

The XPS-specific report waits for exact model, CPU/GPU, camera, Wi-Fi/audio and
dock. Battery/lid/power and external displays need independent physical tests.
Another distro's vendor support is not a Fedora/Kedra qualification.

**Pass:** Versioned desktop recipe passes VM checks and explicit current-desktop
hardware tests; untested devices remain identified honestly.
**Deliver:** Bill of materials, session ADR, hardware inventory/compatibility
report and physical checklist. No future-laptop assumption blocks the VM proof.

## R08: Prove release authority and lifecycle

**Question:** Which source/artifacts can be signed/promoted and how does trust
survive races, key changes, privacy choices, outages and cleanup?
Read kedra-release-signing/security/github-actions; sources actions-security,
policy and distribution docs. Cryptographic validity alone is not freshness.

Prototype untrusted PR and isolated trusted signer using disposable credentials.
Attack signer-script changes, forged workflow artifacts, malicious build output
and altered release metadata. The signer must not run newly built code with its
keys. Pin Actions and use minimal permissions; do not trust event provenance alone.

Race two target builds and finish the older one last. Rerun one, remove expected
artifacts, replace channel tags, replay valid old metadata and forge a caller's
claim of approval. Channel/install state must not silently regress. Define review/
approval authority for routine versus trust-critical source changes. Do not
force-push or change repository security settings as incidental bootstrap work.

Exercise old-key -> overlap -> new-key with an offline machine. Restore from
recovery material; simulate registry outage/missing tags/retention. Preserve
metadata, signatures and required digests together. Decide artifact privacy,
audit deliberately embedded dotfiles/MCP endpoints, document third-party notices,
vulnerability response, RPM refresh and Fedora-major upgrades. Public repo status
is known; intended future artifacts still need a deliberate privacy decision.

**Pass:** Untrusted work cannot sign; approved identity is unambiguous; stale,
cross-target and tampered releases fail; old clients transition trust; cleanup
retains recovery; publication does not leak private information.
**Deliver:** Threat model, promotion/visibility/distribution ADRs, rotation drill,
retention/update policies and security fixtures.

## R09: Prove independent targets and provenance

**Question:** Can one repo route shared/host changes correctly without mixing
executing host, editing target, deployed target or local home state?
Read kedra-machines and kedra-home.

Build two VM targets from one source commit with different config and local-only
home edits. Publish a shared keybinding, host-only monitor config and an edit to
a path with both shared and host override definitions. Provenance must route the
export and show all affected hosts. Ambiguous scope cannot be guessed from a
filename. Private review state never travels to the other host.

Enroll targets separately. Reject a cross-target image despite an allowed key.
Run an agent on A editing B and prohibit accidental local B deployment. Leave B
offline while A advances, then update B and reconcile its own home conflicts.
Report cached state as last-known. Exercise independent rollback.

**Pass:** Only intentionally shared source travels when each machine updates;
local and host-only changes do not leak; wrong target fails; no machine branches,
clone-per-host repos or fleet server are required.
**Deliver:** Enrollment/provenance schema, two-target fixtures, source-export
rules and example truthful agent context. Real XPS hardware is separate R07 work.

## R10: Prove privileges and persistent protocols

**Question:** What narrow Rust interface safely stages releases and recovers after
interruption or an older OS reading newer state?
Read kedra-security/rust-workspace; language and package layout are settled.

Define typed operations, machine-readable outputs, stable exits and authorization
before expanding CLI taxonomy. The user CLI and private home state cannot establish
root trust. The installed helper never executes writable source code or depends
on agent/development orchestration. Choose necessary parser/persistence/OS bindings in
an ADR, not an invented language or Git engine.

Test malformed reference, option/shell injection, environment/PATH substitution,
attacker-controlled checkout/hook, traversal, symlink/hardlink replacement, altered
permissions, stale locks, concurrency and false --verified claims. Confirm the
helper verifies signature/eligibility independently. Memory safety is not proof
of path-race, authorization or crypto correctness.

Version journal/manifests. Simulate new-writer/old-reader rollback, incomplete
migration, corrupt state, missing checkpoints and full disk. Identify recoverable/
recomputable versus irreplaceable records. Never parse failure as an empty home.
Interrupt the agent before/after commit, push, promotion, staging, reboot and
health reporting; resume from deterministic state without repeated destruction.
Installed post-boot checks remove any need for an AI daemon.

**Pass:** Authorized narrow operations work; adversarial requests cannot execute
root code/weaken trust; interruptions stay reportable/recoverable; unknown schemas
fail safely; humans can administer without AI/network.
**Deliver:** Protocol/privilege ADR, adversarial fixtures, compatibility tests,
recovery commands and minimal status contract.

## R11: Prove Rust layout and skill integration

**Fixed:** Rust edition 2024, no first-party src/, explicit flat entry points,
one lockfile, rustfmt/Clippy, actionbook/rust-skills at
5c40d3ad785193231b7d0dbfb8e1eb447e5edd94, a shared Codex/Claude plugin with ordinary skill files and no
hooks/MCP/global installation. The bootstrap now scaffolds these choices; it does
not prove all agent/editor behavior. Read ADRs 0001/0003 and kedra-rust-workspace.

**Workspace cases:** Record exact rustc/cargo/toolchain. Run metadata --locked,
fmt check, Clippy workspace/all-targets with warnings denied, tests/doctests and
release build in Actions. Validate explicit entry targets/lint opt-in and reject
nested/root src/. Test negative layouts and unintended targets/dependencies, not
only happy-path compilation. Inspect rust-analyzer behavior separately. Exclude
vendor examples and target artifacts from first-party layout restrictions.

**Skill cases:** Review the selected snapshot against the plugin NOTICE.md provenance
and existing upstream notices. Validate both plugin manifests, marketplace source
paths, skill frontmatter and skill-relative support files. Confirm ordinary files
with no symlinks, submodules, duplicate discovery trees or custom check runner.
Clone/move/offline cases must not trigger unpinned auto-install. Standard Cargo
commands cover Rust; plugin checks are separate authoring checks, not build gates.

Start each real official CLI at repo root and a crate directory using separate
empty test profiles, then synthetic personal conflicts. Prove actual discovery
of Kedra skills and upstream router/domain-cli/coding/ownership/error/unsafe topics.
Open relative support files through plugin and canonical paths. Record unsupported
optional plugins/tools. Referencing a browser/MCP/subagent is not proof it exists.
Do not run upstream setup/hooks or copy example permissions. Prove no personal
profile/global skill writes and no toolchain/lint/source-layout takeover during
a Rust task. Model obedience remains subject to code review/CI, not certification.

Review upstream notices/licensing before any OS redistribution. A recorded revision
identifies source, not a security audit. Update the selected files, provenance
and plugin version coherently and retest. Actual runtime profile isolation remains R05 coverage.

**Pass:** Standard Cargo checks pass and explicit source layout is reviewed; both CLIs identify pinned
sources/support references without unintended global/personal mutation or hook execution;
editor findings recorded. Static bootstrap success alone is not overall R11 pass.
**Deliver:** Toolchain/dependency decision, workspace/CI evidence, plugin validation,
redacted discovery traces, notices findings and explicit remaining unsupported
cases. Update skills/status without turning R01-R10 into implied passes.

## Execution order and evidence

First independent tracks: R01/R02, R03, R05 and R11. R06/R07 use a disposable
desktop. R08 precedes production signer/promotion. R04 joins home and OS proofs.
R10 defines the production boundary using findings. R09 proves two targets before
any XPS purchase; future physical qualification is independent.

Use docs/research/REPORT.template.md. Every case records question, candidates,
selected experiment, exact source/tools/architecture/digests/runner, expected and
actual outcome, exit code, failure analysis, privacy review, reproducible commands,
Actions reference and resulting ADR or blocker. Logs/screenshots must be sanitized
before publication; secrets are never fixtures. A documentation link is not a test.

## Assignment template for Codex/Claude

Read AGENTS.md, docs/HANDOFF.md, PLAN.md, this packet and relevant skills. Preserve
settled choices. Work on Rxx only with small disposable experiments. Verify exact
primary-source versions and actual inputs. Use synthetic data; OS artifacts are
CI-built; do not alter the current workstation, real home or vault. Write success
and failure tests, record evidence/status and propose an ADR. Update the skill
with established findings. Finish with actual results and a concrete blocker/next
step, not a claim the whole Kedra system is implemented.
