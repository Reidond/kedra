# Kedra: implementation plan

Revision 1.2, repository-bootstrap edition, 2026-09-07. This carries forward the
planning-session requirements and Rust revision. It is a product contract, not a
claim that the planned operating system is implemented. README.md, source, CI and
research reports distinguish the bootstrap from completed integration work.

## 1. Product and workflow

Kedra is a personal Fedora 44 bootc Linux system maintained from `Reidond/kedra`.
`sysroot` is its management command. It is not a distribution framework, alternate
package manager or custom agent platform. The everyday workflow is:

```text
Describe a desired change to a local official coding agent
 -> inspect source, machine context and existing dotfile changes
 -> edit shared/host configuration and selected programs
 -> review and publish only the authorized changes
 -> GitHub Actions builds, tests, signs and publishes the release
 -> local tooling verifies and stages its exact approved image digest
 -> reboot when authorized
 -> reconcile home configuration at the safe new-software boundary
 -> verify the running image and requested behavior
```

All OS/installer builds run in Actions. Source editing, standard build/lint tools
and end-to-end or manual experiments are allowed locally. The owner's 2026-09-08
testing decision in AGENTS.md excludes unit/model/mock tests, doctests and
repository self-checking code. Users can edit the repo without an agent.
Personal agent installation/update and local dotfile experimentation need not
trigger an OS build. A deterministic interface and recovery path must work without
an AI service, GitHub availability or a continuously running agent process.

## 2. Settled decisions and non-goals

| Area | Required design |
|---|---|
| Foundation | Fedora 44 bootc, initially x86_64, plain Containerfile |
| Build | GitHub Actions, explicit inputs/package lists, narrow scripts/Cargo tasks |
| Code | Rust edition 2024 Cargo monorepo, no first-party src/ directories |
| Tooling | One lockfile, pinned toolchain, rustfmt, Clippy, explicit entry points |
| Rust knowledge | Selected actionbook/rust-skills snapshot in a repository-local Codex/Claude plugin |
| Desktop | niri and Noctalia plus working session/audio/portals/network/keyring |
| Source | Linux-shaped etc/, usr/, home/ plus hosts/<target>/ overrides |
| Releases | Signed OCI image, installation ISO, signed manifest/checksums and build provenance |
| Home | Ordinary writable files; line/hunk review and explicit local-only changes |
| Agents | Official bundled tools private to sysroot; personal CLIs remain independent |
| Auth | Bitwarden SSH integration, official model login, separate API/registry credentials |
| Authority | Ordinary-user agent/CLI, narrow image-installed privileged helper |
| Machines | One repo, independent signed target releases/enrollment/rollback |

The TypeScript/Effect v4/Vite Plus/Oxlint/Oxfmt suggestion was superseded by the
owner's Rust decision. BlueBuild was explicitly rejected. Do not build a universal
config parser, plugin marketplace, custom Git engine, fleet control plane, custom
OAuth flow or permanent AI daemon. No automatic whole-home synchronization or
unrestricted agent vault access. No assumption that bootc rollback automatically
recovers every boot failure or reverses arbitrary database/config migrations.

## 3. Source and implementation layout

```text
kedra/
  Cargo.toml, Cargo.lock, rust-toolchain.toml, rustfmt.toml
  AGENTS.md, CLAUDE.md, PLAN.md, RESEARCH.md
  crates/sysroot/{Cargo.toml,main.rs,...}
  crates/sysroot-core/{Cargo.toml,lib.rs,...}
  crates/sysroot-helper/{Cargo.toml,main.rs,...}
  packages/{common.list,remove.list}
  etc/, usr/, home/
  hosts/desktop/{host.toml,packages.list,etc/,usr/,home/}
  hosts/xps/                         # reserved, disabled/unqualified
  build/, installer/, tests/
  plugins/kedra/skills/<topic>/SKILL.md  # single canonical skill tree
  plugins/kedra/{.codex-plugin/,.claude-plugin/}
  .agents/plugins/marketplace.json, .claude-plugin/marketplace.json
  docs/{HANDOFF.md,SESSION.md,SOURCES.md,adr/,research/}
  .github/workflows/
```

Every first-party binary/library explicitly declares main.rs/lib.rs directly in
its package directory. No root src/ or nested crates/*/src/. Modules and crate
integration tests remain ordinary Rust. Do not rewrite vendored examples. The
shared core stays small/pure; the helper never depends on the user CLI, agent
executor, writable hooks or development tooling. Async/framework dependencies require a real
need. Rust checks and actual agent discovery are separate R11 evidence.

The bootstrap includes read-only CLI status and a helper that refuses operations,
not an installer or privileged protocol. Initial tools are pinned, not guaranteed
forever. The eventual non-RPM tool lock records exact artifacts/hashes separately
from Cargo.lock and the skills pin. No speculative production Containerfile or
permissive release workflow is introduced just to make the tree look complete.

Assembly is common files first, then the selected host. Same-path replacement is
explicit/reported. Prefer native application includes; do not deep-merge arbitrary
formats. Ignore source-control placeholder files such as .gitkeep in payloads.
Do not COPY the entire repository into an image.

`etc/` goes to image /etc; `usr/` to /usr. `home/` is a source representation whose
resolved content goes to /usr/share/sysroot/home/<user>/. It is not a direct live
home overwrite. Record every managed path's source path/revision/target/hash/mode/
application group so reviewed changes return to the correct source.

Usernames/UIDs, disk identifiers and monitor connectors are established through
setup/inventory. Old examples using andrii are examples, not hard-coded identity.
The proposed checkout default is ~/src/kedra and is configurable.

### Ownership on the installed system

| Location | Owner/purpose |
|---|---|
| /usr/bin/sysroot | Image-owned management command |
| /usr/libexec/sysroot/agents/ | Private bundled executables, not on normal PATH |
| /usr/share/sysroot/ | Build/target manifests, public trust, home baselines |
| /var/lib/sysroot/ | Root-owned enrollment and deployment/recovery journal |
| User source checkout | Ordinary Git work, not trusted root-executable source |
| Live dotfiles | User/app-writable configuration |
| ~/.local/state/sysroot/home/ | Private review, prior baseline, local policy/checkpoints |
| ~/.local/state/sysroot/agents/ | Dedicated writable management-agent state |
| Personal agent paths | Independently owned unless a safe subset is adopted |

bootc persistent /etc retains local changes through its merge semantics; /var is
shared between deployments. Prefer image-owned /usr defaults where supported,
provide runtime directories through appropriate systemd facilities, surface /etc
drift and do not author /usr/etc. Transient /etc needs a persistence design and is
not an initial shortcut. These upstream facts do not implement our home manager.

## 4. Build, signing and release contract

Record source SHA, workflow run/attempt, target, architecture, resolved Fedora
base digest, RPM inventory, builder/tool versions, non-RPM hashes, Rust toolchain,
Cargo.lock digest and skill pin. A source commit against moving repositories does
not uniquely determine an image. Retain exact built digests for redeployment;
bit-for-bit rebuilds require pinned/retained package inputs too.

Build all enabled targets initially. A second VM target can test independence
before XPS hardware exists. Each final registry digest proceeds through:

```text
unprivileged PR checks (no production secrets)
 -> accepted source/manual/scheduled build
 -> resolve and build target inputs
 -> lint/config/static checks
 -> isolated signing of candidate digest
 -> boot/update/negative tests of that exact candidate
 -> installer containing the same OS payload
 -> sign manifest/checksums
 -> promote only successful target releases
```

The signing job must not run image contents or arbitrary newly changed scripts
while holding production keys. Pin Actions, minimize credentials and treat build
artifacts/PR content as untrusted. Protect trust-critical workflows/helper/policies
separately from routine package edits. No workstation SSH credentials in CI.

A signed candidate is not a promoted release. Serialize promotion per target and
prevent an older slow run from replacing a newer approved head. Reruns have new
identities. The signed release binds target/architecture/source/run/sequence,
image digest, home provenance, compatibility requirements and ISO checksum.
Choose an unambiguous encoding and schema in R01/R08/R10.

Expected output per promoted target:

```text
ghcr.io/reidond/kedra-<target>@sha256:<digest>
kedra-<target>-44-<build>.iso
release.json + signature/bundle
SHA256SUMS + signature/bundle
packages.txt and selected SBOM/provenance format
```

Use a dedicated OS-release signing key and independently trusted public material.
Bitwarden SSH identity is not the release key. Commit no private keys/passwords.
Prove bootc's actual enforcing signature path, repository identity and attachment
discovery with pinned tools; do not treat standalone Cosign verification as proof.
Container signing does not establish Secure Boot or automatic rollback.

Negative inputs include unsigned/wrong-key/wrong-repository images, wrong target/
architecture, tampered/missing manifests or attachments, unpromoted candidates,
stale replays and forged caller verification. Explicit rollback is distinct from
routine forward release selection. Key rotation must cover an offline client and
retained old releases. Retention/GC must not destroy required recovery artifacts.

The ISO is install/recovery media; normal updates consume OCI images. Research
uses the current osbuild/image-builder path, separating an Anaconda environment
from the OS payload. Explicit disk choice, encryption/account/recovery setup and
correct installed registry origin are required. No automatic first-disk erase.
Measure Actions resources and actual media size; each GitHub Release asset is
currently limited to less than 2 GiB, so splitting must preserve the signed
whole-ISO checksum. Secure Boot/storage/hibernation/TPM/dual-boot claims are separate
research decisions, not implied by a successful container build.

Repository visibility is public at bootstrap. Decide eventual artifact visibility
and audit intended dotfiles/MCP metadata before publication. Private GHCR needs
its own runtime credential/bootstrap strategy. A private registry does not make
plaintext keys safe or grant third-party redistribution rights.

## 5. Deployment, interruption and recovery

State transitions are explicit:

```text
available -> verified -> home-preflight-complete -> staged
 -> awaiting-reboot -> booted -> home-reconciled -> healthy
```

Error/cancelled/recovery-required/rollback states and journals are versioned.
Resuming after power loss or an agent restart must not repeat destructive work.
The installed post-boot checker is deterministic; no AI process must survive.

Local root enrollment identifies permitted target/repository/architecture/channel.
The image self-manifest is separate. The helper verifies eligibility itself and
stages an exact digest using enforced bootc policy. For digest-pinned bootc,
ordinary upgrade is a no-op; sysroot update resolves an eligible release then
switches to its digest. A desktop must reject an XPS image even with a trusted key.

Home preflight must not apply new config while old program versions are running.
Research how to inspect/validate candidate baselines without executing untrusted
code as root. Recheck live files at the actual new-software/session activation
boundary. An OS/home change is not one globally atomic transaction: coordinate
application groups with checkpoints and a durable journal.

The helper accepts narrow typed inputs, not shell strings or writable-checkout
scripts. User-owned review state does not establish trust. Validate environment,
paths, executable identity and concurrent operations. Old OS code may encounter
new persistent schemas after rollback; fail safely and offer recovery, not an
empty-state assumption. Preserve local administrative/TTY/boot-menu access even
when the network and Bitwarden GUI are unavailable. Automated health-triggered
rollback is deferred until explicitly proven.

## 6. Writable-home management

The repository is the versioned baseline, not every live home byte. Use normal
writable files and a private Git review workspace separate from the source repo.
Track prior accepted baseline B, current live L, new baseline N, staged selection,
local-only policy, publication state and recovery journal. Three-way content
merge alone does not preserve all these independent dimensions.

| User decision | Required behavior |
|---|---|
| Stage/publish lines | Export exactly selected content; later writes stay unstaged |
| Leave uncommitted | Keep active local change visible in review |
| Keep a hunk local | Preserve this difference, hide normally, expose in ignored review, never export |
| App-owned setting | Preserve that local structured field under explicit policy |
| Discard selected change | Recheck newer writes, checkpoint and revert only the selection |

All may coexist within one file. Git provides index/partial staging and candidate
merges; it does not provide tracked-file line ignore policy. Do not use
skip-worktree/assume-unchanged/.gitignore or a display-only filter as the solution.
OverlayFS operates at filesystem-object level, not line-level reconciliation.

Anchor local-only hunks to baseline/content/context, not absolute lines. A changed
value becomes new visible work unless a persistent app-owned field rule applies.
Ambiguous/overlapping upstream context returns to review. Structured adapters are
narrow, only for formats actually needed. Binary databases require safe textual
export or remain unmanaged. Do not invent a universal config language.

Capture adopted safe paths only. Unknown files can be untracked candidates but
are never auto-staged. Exclude secrets before snapshotting, including agent auth,
transcripts/caches, vault/keyring data and SSH private keys. Mixed-secret settings
need projection or exclusion. A secret in a private Git object is still retained;
ignore/deletion after capture is insufficient. Keep local review commits out of
pushed ancestry. Track published-but-not-deployed edits to prevent repeat export.

Compute candidates away from live configuration. Never write conflict markers
into the app's files. Preserve a conflicting app group coherently; never advance
its baseline after a failed apply. Validate with appropriate software versions,
coordinate writers, recheck hashes, checkpoint, journal, apply and restart/reload.
Watchers are hints and may miss events. A hash check alone does not prevent a
later stale in-memory app save; stop writers when necessary. Separate atomic file
renames do not make a whole group atomic.

Test local/upstream deletion, rename, mode/label changes, file-directory swaps,
symlink/hardlink/path escape, special files, newline/encoding issues, disk full,
permission failures, concurrent invocations, interruption and app state migration.
An upstream removal must not silently erase locally changed content. Rollback uses
the same no-loss rules and handles newer persistent state, not blanket snapshots.

Noctalia v5's GUI override state can supersede curated TOML. Review a selected
safe effective-settings projection, not the entire state tree. Export only chosen
values. Remove redundant overrides only after publication is deployed and the
live override still matches the reviewed value. Verify packaged version semantics.

## 7. Agents, personal tools and authentication

Future interfaces include sysroot setup/doctor/status/context, codex/claude,
home status/diff/stage/ignore/export, update/deploy/rollback. They are proposed
contracts, not currently operational commands. Keep machine-readable status and
clear exit codes without an excessive command taxonomy.

Bundled official agent binaries live privately under /usr/libexec/sysroot/agents.
Only sysroot exposes them by default. Never shadow ordinary codex/claude commands,
install global aliases, or alter the user's personal install directories. User
versions can update immediately with their own installer; the OS does not repair,
downgrade or remove them on upgrade/rollback.

Runtime and config scope are independent. Default sysroot codex/claude uses the
bundled version and dedicated management profile. --runtime user selects a personal
executable but not implicitly personal configuration. --config-scope personal opts
into personal settings. --host selects editing scope, not execution location or
local deployment permission. Define wrapper/upstream argument forwarding safely.

Resolve a configurable verified checkout, preserve dirty state and coordinate
concurrent agents. Offline existing checkouts remain editable. Missing personal
runtime fails clearly instead of silent fallback. Report effective binary/version,
profile, skill/MCP sources and executing versus edited host. Agent startup is not
permission to push or reboot; the actual user task governs authorization.

Dedicated profiles are writable private runtime state. CODEX_HOME and
CLAUDE_CONFIG_DIR are building blocks, not security sandboxes. Do not change real
HOME. Test external skill discovery, keyring identities, hooks, child environments
and cross-version state. Separate personal-runtime management profiles when
needed. Disable bundled Claude updates only per bundled invocation, never globally
or for user runtimes. Keep official login methods; no custom OAuth/token sync.

Personal installation declarations, MCP, skills and safe preferences may each be
local or selectively tracked. Track install intent, not binary caches. No blanket
adoption of .codex/.claude. Existing other dotfiles repositories retain ownership.
Repository instructions are shared through AGENTS.md and CLAUDE.md import, not
machine-wide restrictions on every personal coding session.

First-party knowledge skills and the pinned upstream Rust skill tree are exposed
locally to both agents, without hooks/plugins/MCP auto-setup, global installs or
automatic updates. Keep supporting references and source provenance. Update skills
with measured durable findings. Documentation guidance is not a permission grant,
a runtime dependency guarantee or proof of model obedience.

### Credential flows

Bitwarden Desktop SSH is the default integration candidate. User sign-in, agent
enablement, unlock and signing prompts stay explicit. Configure session socket
propagation and public-key identity selection; never export vault-held private
keys or replace them because the vault is locked. Native Linux currently documents
~/.bitwarden-ssh-agent.sock; validate packaging/session details.

Git uses SSH; gh API calls use a token; model access uses official agent sign-in;
private OCI pulls use bootc-compatible registry credentials; image signing uses a
protected CI key. These are distinct. Provide/test Secret Service where needed;
Bitwarden is not automatically a universal keyring. Detect gh plaintext fallback.
Codex keyring and Claude's documented Linux credential files require explicit
profile/storage testing. Keep credentials out of captures, logs and build inputs.
No broad BW_SESSION is passed to an agent. A same-user profile directory is not
isolation from unrestricted same-user code. Any narrow credential helper needs
its own limited contract; no general vault executor.

## 8. Multiple machines

One repo has shared defaults and host differences. Each enabled target owns an
image repository, installer, signed manifest, root enrollment, home history and
rollback state. Publishing a shared change can produce candidates for all targets;
each machine chooses independently when to deploy. Local-only edits, personal
installs, secrets and OAuth do not synchronize. Only chosen source content travels.

Desktop is first. XPS stays disabled until an exact model/components are known;
use synthetic VM targets for architectural tests. Verify actual desktop hardware
rather than invent connector/device/disk IDs from remembered context. Report
cached offline status as last-known. Optional later SSH remote management calls
the same helper; no fleet server is required for the first version.

## 9. Implementation milestones

| Phase | Work | Exit condition |
|---|---|---|
| M0 | R01-R11 small research proofs | Critical assumptions measured, not merely documented |
| M1 | Minimal signed image/installer/update | VM installs A, accepts signed B, rejects invalid inputs, boots A again |
| M2 | Desktop/session/keyring/Bitwarden | Core session works with SELinux enforcing and recovery access |
| M3 | Enrollment/trust/staging/journal | Interrupted, stale, cross-target requests are recoverable/safe |
| M4 | Writable home and export/apply | No-loss/no-leak line policy, conflicts/writers/rollback fixtures pass |
| M5 | Agent launch/auth/orchestration | Both official CLIs complete an authorized change without personal-tool damage |
| M6 | Independent targets/hardware/lifecycle | End-to-end and recovery tests pass before daily-driver migration |

## 10. Acceptance scenario

Install A in a disposable VM; establish recovery access, Bitwarden SSH, GitHub API
and official agent logins. Install a personal agent separately and keep its own
MCP/skills untracked. Create three local edits in one managed file: publish one,
leave one visibly uncommitted, keep one local-only. Include a GUI-written Noctalia
setting. Ask an agent to add a program and export only approved changes, follow
its exact Actions build, and stage B by verified digest.

After an authorized reboot verify digest, program and config, while preserving
both local categories and keeping credentials/transcripts out of Git/images/ISO.
Exercise a conflict, concurrent writer, invalid signature, wrong host, interrupted
activation and rollback after newer edits. Update a second target independently.
Recover offline with the Bitwarden GUI closed. Only then qualify the real desktop;
actual XPS hardware remains a separate test.

## 11. Evidence and references

RESEARCH.md is the experiment backlog. docs/research reports and CI identify what
actually ran. docs/SESSION.md preserves rejected alternatives. docs/SOURCES.md
maps primary upstream documentation and review status. Skill procedures are
entry points, not a substitute for version-specific evidence. Update the relevant
skill/report/ADR together when an experiment changes a prior assumption.
