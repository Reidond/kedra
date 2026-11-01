---
name: kedra-bitwarden
description: Integrate Bitwarden Desktop SSH signing, the bundled Bitwarden package, user bootstrap, niri session/keyring, GitHub API authentication, agent login and private registry pulls without exporting secrets.
---

# Credential flows are separate

The required experience is Bitwarden SSH out of the box, not an automatically
unlocked vault. Ship the native Desktop app and session integration; user sign-in,
SSH-agent enablement, unlock and signing authorization remain explicit. Never
export a private key or generate a replacement because the vault is locked.

## Bundled package and SSH integration

Bitwarden Desktop is the `bitwarden` catalog package
(system/packages/bitwarden.kedra) on both targets: the unchanged
official Linux tarball per architecture (`bitwarden_2026.8.0_x64.tar.gz`,
`_arm64.tar.gz`; the two differ only in the desktop-napi module name),
SHA-256-pinned in packages.lock.json. Bitwarden publishes no signatures: compare a
new archive with the GitHub release asset digest before pinning (docs/PACKAGES.md
"Updating a pin"). The `bitwarden` command is the catalog launcher
/usr/share/kedra/catalog-bin/bitwarden; the desktop entry
/usr/share/applications/com.bitwarden.desktop.desktop names the exact store output
in Exec= and Icon= (StartupWMClass com.bitwarden.desktop). The package build runs
`ldd` over bitwarden-app, desktop_proxy and the native module in the builder root
and declares the Electron libraries as `runtime_requires`; the container scenario
image-contents.yaml repeats `ldd` on the installed image and refuses special
permission bits. Store files are root-owned 0555/0444, so chrome-sandbox is never
setuid and Chromium uses its namespace sandbox, which needs unprivileged user
namespaces (Ubuntu 24.04 hosts confine them through AppArmor; harness README).
The scenario bitwarden.yaml starts it logged out in the nested session; a
logged-out start of the tarball layout on x86_64 hardware or VM is not-run.
The official RPM (inspected 2026-09-08, Desktop 2026.8.0) installs to
/opt/Bitwarden with conditional setuid/AppArmor scriptlets; Kedra no longer uses
it, and never run those scriptlets on the workstation.

The native Linux agent documents $HOME/.bitwarden-ssh-agent.sock. Kedra defaults
SSH_AUTH_SOCK to it, keeping an existing value, in etc/profile.d/kedra-bitwarden.sh,
usr/lib/environment.d/60-kedra-bitwarden.conf and usr/libexec/kedra-session. Test
it in the graphical session, GUI-launched terminals, systemd user services, TTY
and sysroot launchers: environment.d applies to the user service manager's
environment, not to every process.

Select the intended GitHub identity with a public-key file and SSH IdentityAgent,
IdentityFile and IdentitiesOnly. The private half stays with Bitwarden. This
selection is not a sandbox preventing an unrestricted same-user agent from other
operations. Do not enable agent forwarding by default. Preserve signing prompts.
Never suppress host-key verification to make a Git test work.

## Independent authentication paths

Git fetch/push uses SSH. gh workflow/release APIs use separate GitHub token auth.
An SSH Git protocol setting does not authenticate the API. gh's credential-store
fallback may be plaintext; setup must detect it. `gh auth login --web
--git-protocol ssh --skip-ssh-key` is a candidate command after verifying the
selected version, not permission to upload or generate keys.

Codex/Claude use official sign-in. Provide/test Secret Service rather than assume
Bitwarden is that store. Codex offers keyring storage; Claude documents Linux
credential files. Configuration directories and keyring identities need actual
tests. OAuth sessions are local runtime state, not dotfiles to synchronize into
Git or the vault. Private GHCR pulls need their own root-side bootc-compatible
credential path, expiry/rotation policy and cold-boot behavior.

A dedicated release-signing key is yet another identity in protected CI. Do not
reuse the user's Bitwarden key. Do not give a general BW_SESSION to coding agents
(the launchers remove it): it authorizes vault CLI decryption, whereas the Desktop
SSH socket is for SSH agent operations, not a broad secret API. A narrow secret
helper may be designed for specific tasks; no general vault executor is part of
this project.

## Validate and recover

Cover logged-out/locked/unlocked/relocked/restarted app, session entry points,
auth refresh, plaintext fallback, cold private pulls and offline repair with
synthetic data; owner vault login and SSH signing stay manual. Credentials are
excluded before home capture and image assembly; no tokens in args, logs or
transcripts. Recovery remains available without the GUI or network. Primary
references: [Bitwarden SSH](https://bitwarden.com/help/ssh-agent/),
[Bitwarden CLI](https://bitwarden.com/help/cli/),
[OpenSSH identity configuration](https://man.openbsd.org/ssh_config),
[systemd environment scope](https://man7.org/linux/man-pages/man5/environment.d.5.html),
[GitHub CLI authentication](https://cli.github.com/manual/gh_auth_login),
[Codex authentication](https://developers.openai.com/codex/auth/),
[Claude authentication](https://code.claude.com/docs/en/authentication) and
[bootc runtime secrets](https://bootc.dev/bootc/building/secrets.html).
