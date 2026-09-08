---
name: kedra-bitwarden
description: Integrate Bitwarden Desktop SSH signing, user bootstrap, niri session/keyring, GitHub API authentication, agent login and private registry pulls without exporting secrets.
---

# Credential flows are separate

The required experience is Bitwarden SSH out of the box, not an automatically
unlocked vault. Package the native Desktop app and session integration; user sign-in,
SSH-agent enablement, unlock and signing authorization remain explicit. Never
export a private key or generate a replacement because the vault is locked.

## Intended SSH integration

The native Linux agent documents $HOME/.bitwarden-ssh-agent.sock. Configure and
test SSH_AUTH_SOCK in the graphical session, GUI-launched terminals, systemd user
services, TTY and sysroot launchers. environment.d applies to the user service
manager's environment, not magically every process; prove session propagation.
A Bitwarden Flatpak would have different packaging/sandbox details; native RPM is
the initial research candidate, not a tested installed outcome.

Desktop 2026.8.0 inspection (2026-09-08, R06): the official native RPM puts
regular application files in /opt/Bitwarden and its launcher resolves its own
path. Build preparation relocates unchanged runtime files to /usr/lib/bitwarden
for bootc, retaining notices/source. Never run its conditional setuid/AppArmor
RPM scripts on the workstation. The generated enforcing Fedora VM must prove
native sandbox startup and socket propagation; syntax/layout checks alone do
not pass authentication. See build/bitwarden/README.md and the R06 report for
exact source/digest and pending runtime evidence.

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
selected version, not permission to upload/generated keys.

Codex/Claude use official sign-in. Provide/test Secret Service rather than assume
Bitwarden is that store. Codex offers keyring storage; Claude documents Linux
credential files. Configuration directories and keyring identities need actual
tests. OAuth sessions are local runtime state, not dotfiles to synchronize into
Git or the vault. Private GHCR pulls need their own root-side bootc-compatible
credential path, expiry/rotation policy and cold-boot behavior.

A dedicated release-signing key is yet another identity in protected CI. Do not
reuse the user's Bitwarden key. Do not give a general BW_SESSION to coding agents:
it authorizes vault CLI decryption, whereas the Desktop SSH socket is for SSH
agent operations, not a broad secret API. A narrow secret helper may be designed
for specific tasks; no general vault executor is part of this project.

## Validate/recover

R06 tests logged-out/locked/unlocked/relocked/restarted app, session entry points,
auth refresh, plaintext fallback, cold private pulls and offline repair with
synthetic data. Credentials are excluded before home capture and image assembly;
no tokens in args/logs/transcripts. Recovery remains available without the GUI or
network. Sources: docs/SOURCES.md bitwarden-ssh, bitwarden-cli, openssh,
environment, gh-auth, codex-auth, claude-auth, bootc-secrets.
