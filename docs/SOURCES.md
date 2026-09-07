# Primary-source map

Source review date: 2026-09-07. Documentation is evidence for an upstream feature,
not a passed Kedra experiment. Recheck actual pinned binaries, package versions,
artifact hashes, compatibility and terms before implementation/publication.
`Rechecked` means retrieved during repository bootstrap. `Planning` means carried
from the planning documents and requiring a new version-specific check.

| ID | Primary source | Scope | Review |
|---|---|---|---|
| bootc-fs | https://bootc.dev/bootc/filesystem.html | Image versus persistent /etc and /var; do not author /usr/etc | Rechecked |
| bootc-switch | https://bootc.dev/bootc/man/bootc-switch.8.html | Digest switching and verification options | Rechecked |
| bootc-build | https://bootc.dev/bootc/building/guidance.html | Derivation, package installation, configuration | Planning |
| bootc-secrets | https://bootc.dev/bootc/building/secrets.html | Runtime credentials/private pulls | Planning |
| bootc-kargs | https://bootc.dev/bootc/building/kernel-arguments.html | Image-owned argument drop-ins | Planning |
| image-builder | https://osbuild.org/docs/bootc/ | Repository migration, installer/payload distinction | Rechecked |
| policy | https://raw.githubusercontent.com/containers/image/main/docs/containers-policy.json.5.md | Allowed key and repository identity | Planning |
| registries | https://raw.githubusercontent.com/containers/image/main/docs/containers-registries.d.5.md | Signature attachment discovery | Planning |
| podman-sign | https://docs.podman.io/en/latest/markdown/podman-push.1.html | Registry push/signing primitives | Rechecked |
| blob-sign | https://docs.sigstore.dev/cosign/signing/signing_with_blobs/ | Release manifest/ISO checksum signatures | Planning |
| actions-security | https://docs.github.com/en/actions/reference/security/secure-use | Untrusted inputs, privileges, pinned Actions | Rechecked |
| actions-runners | https://docs.github.com/en/actions/reference/runners/github-hosted-runners | Resource/privilege/KVM constraints | Planning |
| release-limits | https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases | Per-asset size constraints | Rechecked |
| git-stage | https://git-scm.com/docs/git-add | Index and partial staging | Planning |
| git-merge | https://git-scm.com/docs/git-merge-tree | Candidate tree without live worktree mutation | Rechecked |
| git-faq | https://git-scm.com/docs/gitfaq | Tracked-file local-change limitations | Rechecked |
| overlayfs | https://docs.kernel.org/filesystems/overlayfs.html | File-layer semantics, not text merge | Planning |
| inotify | https://man7.org/linux/man-pages/man7/inotify.7.html | Watcher overflow and race limitations | Planning |
| niri | https://niri-wm.github.io/niri/ | Session and configuration documentation | Planning |
| fedora-niri | https://packages.fedoraproject.org/pkgs/niri/niri/ | Distribution package, not hardware qualification | Planning |
| fedora-noctalia | https://packages.fedoraproject.org/pkgs/noctalia/noctalia/ | Distribution package/version | Planning |
| noctalia | https://docs.noctalia.dev/noctalia/configuration/ | v5 curated TOML plus GUI override state | Rechecked |
| bitwarden-ssh | https://bitwarden.com/help/ssh-agent/ | Desktop agent, socket, enable/unlock/authorize | Rechecked |
| bitwarden-cli | https://bitwarden.com/help/cli/ | Separate vault CLI/session access | Planning |
| openssh | https://man.openbsd.org/ssh_config | IdentityAgent and public-key identity selection | Planning |
| environment | https://man7.org/linux/man-pages/man5/environment.d.5.html | systemd user-service environment scope | Planning |
| gh-auth | https://cli.github.com/manual/gh_auth_login | API token, SSH Git protocol, credential-store fallback | Rechecked |
| codex-config | https://developers.openai.com/codex/config-advanced/ | CODEX_HOME and configuration layers | Planning |
| codex-auth | https://developers.openai.com/codex/auth/ | Official login and credential storage | Rechecked |
| codex-skills | https://developers.openai.com/codex/skills/ | Skill discovery and supporting files | Rechecked; URL may redirect to official ChatGPT Learn docs |
| claude-skills | https://code.claude.com/docs/en/skills | Project-local skills and links | Rechecked |
| claude-env | https://code.claude.com/docs/en/env-vars | CLAUDE_CONFIG_DIR and invocation-scoped updates | Rechecked |
| claude-auth | https://code.claude.com/docs/en/authentication | Supported login and runtime credential storage | Rechecked |
| claude-distribution | https://code.claude.com/docs/en/legal-and-compliance | Preinstallation/distribution conditions | Rechecked; not legal clearance |
| cargo-targets | https://doc.rust-lang.org/cargo/reference/cargo-targets.html | Explicit entry paths | Planning |
| cargo-workspaces | https://doc.rust-lang.org/cargo/reference/workspaces.html | Workspace inheritance/lockfile | Rechecked |
| rust-release | https://blog.rust-lang.org/releases/latest/ | Toolchain selection; record actual rustc output too | Rechecked |
| rust-skills | https://github.com/actionbook/rust-skills/tree/5c40d3ad785193231b7d0dbfb8e1eb447e5edd94 | Exact upstream source, not plugin installation | Rechecked through GitHub |
| checkout-pin | https://github.com/actions/checkout/commit/fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09 | Pinned checkout v5 source used by bootstrap | Rechecked through GitHub |

When a source contradicts a skill, record versions and an experiment. Do not
silently replace a settled requirement or report a new upstream default as
working on the Fedora package. Update this map and the relevant skill/report.
