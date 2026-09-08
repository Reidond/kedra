# R05: launcher and distribution research

Date: 2026-09-08. Launcher Linux tests: pass. Full gate blocked.

Private Codex image packaging passes [34195980696](https://github.com/Reidond/kedra/actions/runs/34195980696)
at `9aa59ad`. The actual enforcing Fedora desktop VM passes
[34197344807](https://github.com/Reidond/kedra/actions/runs/34197344807) at `ba011ec`:
`sysroot codex` resolves the installed private 0.153.4 runtime, version/help/login-help
and bundled helpers run, and seeded personal profile content survives. Native
runtime files, pinned corresponding source and reviewed notices are now included
in the image. This does not prove authenticated model turns or close Claude terms.

[Run 34181692426](https://github.com/Reidond/kedra/actions/runs/34181692426)
at `082f8d44ebd23f542975135dd2900b21d76fe3af` passes fmt/Clippy, 64 workspace
tests plus one doctest, release build and OpenSSL interoperability. This includes
seven Linux launcher unit cases and the process/lock/exit-status integration case.

Implemented the Linux ordinary-user wrapper, explicit runtime/configuration choice,
offline checkout identity checks, editing target context, version-specific private
profiles, advisory checkout locking through exec, direct argument forwarding and
broad vault-session exclusion. Source/Git operations remain in the CLI package;
the privileged helper cannot launch agents. See [launcher guide](../../AGENT-LAUNCHERS.md).

Seven Linux selection/profile/environment tests plus an actual CLI process test
cover lock inheritance, exit status and synthetic vault exclusion. Windows
fmt/Clippy and existing workspace tests also pass. Local Cargo initially selected an inherited
1.97.0 override; explicit installed `+1.98.1` checks use the pinned toolchain.

Official Codex package 0.153.4 was downloaded into research storage and its
SHA-256 matches the release asset: `a822187e1a2420c61c5926721bfbd878701ed95547c9bb0d4de4498a16ba1821`.
The package layout has `bin/codex`, code-mode host, rg, bubblewrap, zsh and
`codex-package.json`; an isolated WSL invocation reports `codex-cli 0.153.4`.
No personal profile/config was edited. Public image packaging is not yet enabled.
Required license/notices and bundled component source obligations must be retained.

The pinned Codex login storage source derives its keyring account from the
canonical CODEX_HOME path. That supports distinct identities but does not prove
Secret Service behavior or every external discovery path. No real account login
or model request was run. Claude native packaging and actual-runtime tests remain
not-run. Its official preinstallation policy requires publisher agreement to
Anthropic's Commercial Terms, unmodified binaries, all native authentication
methods, and direct end-user billing. Owner agreement has not been established;
no Claude binary has been published as part of Kedra.

GitHub's artifact-attestation endpoint returns 404 for the package archive;
no SLSA attestation success is claimed. The release separately supplies Sigstore
bundles for Codex, its code-mode host and bubblewrap. A new R05 workflow verifies
those exact executables against the release workflow identity using pinned
Cosign 3.1.3, then probes bundled 0.153.4 and personal 0.153.3 under a generated
home in a network namespace without network access. This native workflow is pending.
Only safe result JSON/version/signature evidence is uploaded, not binaries,
profiles, transcripts or auth state. Public OS packaging remains disabled.

Native run 34182176074 at `3be3030` verifies all three Sigstore bundles, but fails
the personal-profile assertion. Upstream Codex initializes argument-zero runtime
helpers before parsing help/version. The wrapper now probes versions with a
disposable private CODEX_HOME/CLAUDE_CONFIG_DIR while keeping HOME unchanged.
The corrected native test preserves a seeded personal config and checks personal
files before/after management launches; explicit personal launches may perform
normal upstream runtime writes. Corrected execution is pending. A local `/tmp`
probe also shows Codex's deliberate refusal to put argument-zero helpers there;
version output still succeeds. No real account or model request was involved.

Corrected [run 34182776503](https://github.com/Reidond/kedra/actions/runs/34182776503)
at `43873cb` passes all three Sigstore bundle checks and four native runtime/scope
combinations: bundled 0.153.4 and personal 0.153.3, each with management or personal
configuration. Version/help/login-help return success; management launches retain
seeded personal config/files and all probes preserve dirty checkout state. These
probes do not run a model turn, authenticating login, MCP server or actual skill
discovery. Those parts of the matrix remain open.

Remaining: official binary write/discovery traces, full
runtime/profile matrix, personal-version updates and OS rollback, signal/PTY and
long-lived child behavior, credential identities, redistribution prerequisites,
and repository-only skill discovery. A cooperative lock does not coordinate
arbitrary editors or provide a same-user security sandbox.

Component review follow-up (2026-09-08): the pinned native package reports
ripgrep 15.2.0 at `e89fff89ac` with PCRE2 10.45 and zsh 5.9.0.3-test. Its pinned
source identifies the zsh base `77045ef899e53b9598bebc5a41db93a548a40ca6` and the
exec-wrapper patch. Fetching now retains Codex LICENSE/NOTICE, bubblewrap COPYING,
that patch, and hash-pinned zsh, ripgrep and PCRE2 notices. The complete pinned
Codex source archive includes the vendored bubblewrap source and build inputs.
This is a component-notice collection, not a claim that all transitive source and
distribution prerequisites are closed. Claude preinstallation still requires
the owner's agreement to the published Commercial Terms; no agreement is inferred.

Sources retrieved 2026-09-08:
[Codex release](https://github.com/openai/codex/releases/tag/rust-v0.153.4),
[pinned login storage](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/login/src/auth/storage.rs),
[Codex license](https://github.com/openai/codex/blob/rust-v0.153.4/LICENSE),
[Codex notices](https://github.com/openai/codex/blob/rust-v0.153.4/NOTICE),
[Claude preinstallation](https://code.claude.com/docs/en/legal-and-compliance),
[Claude signed manifests](https://code.claude.com/docs/en/setup#binary-integrity-and-code-signing).
