# R06 credential bootstrap

2026-09-08: native package/startup subset passes; full gate blocked.

[Fedora VM run 34199985702](https://github.com/Reidond/kedra/actions/runs/34199985702)
at `4dc3355` passes package preparation, installed Bitwarden startup and the default
socket environment in the graphical/systemd and TTY-login paths. Its captured
logged-out Bitwarden window was inspected. The ordinary generated user launched
the unchanged upstream runtime without a sandbox-disable flag or setuid change.
Codex, Noctalia, session services and synthetic keyring checks also pass. This is
startup/propagation evidence, not vault authentication or SSH signing evidence.

The reviewed official Bitwarden Desktop 2026.8.0 x86_64 RPM has SHA-256
`5537e0ae5b1d3a2a3aff560362f6689d9f45c4584bb271e3cedff482f01f237a`
and size 102,553,277 bytes, matching its GitHub release asset digest. Its identity
is `bitwarden 2026.8.0 x86_64`, with GPL-3.0 package metadata. Matching client
source revision is `f60a4606b7581c338d18091a98640ee71392f015`; the 44,242,002-byte
source archive has SHA-256
`77dbd88a1a78dbc7c23cc0e54d85cb6446f76387a67a68beb8e2d2361c0d7edb`.

Read-only local inspection passes: original files are regular files below
`/opt/Bitwarden`, plus icons and a desktop entry. The native launcher resolves
its installation location and retains upstream process-isolation setup. RPM
scripts would create alternatives, conditionally make the Chromium sandbox
setuid and update AppArmor. Image preparation extracts data without executing
those scripts, retains mode 0755 and relocates the native app to `/usr/lib/bitwarden`.
Upstream runtime bytes and Electron/Chromium notices are retained. Client source
and license files travel in the image. See [packaging details](../../../build/bitwarden/README.md).

Prepared integration defaults new graphical, systemd-user and TTY login sessions
to the documented `$HOME/.bitwarden-ssh-agent.sock`, preserving a nonempty explicit
alternative. These files do not enable or unlock the SSH agent. A generated R07
account test will assert socket environment propagation, start the logged-out
native app with its sandbox and capture its window. Syntax checks pass; actual
package execution, image build and this new native test are **not-run**.

Locked/unlocked/relocked/restarted signing, selected public-key identity, owner
GitHub API/model authentication and refresh remain **not-run**. The existing R07
generated-password login verifies unlocked Secret Service separately; Bitwarden
is not that service. Public image selection avoids private-registry bootstrap for
the intended release, but production signing/promotion is unfinished. No owner
vault, private key, token, global agent configuration or workstation home was used.

Sources: [official release](https://github.com/bitwarden/clients/releases/tag/desktop-v2026.8.0),
[SSH setup](https://bitwarden.com/help/ssh-agent/),
[license map](https://github.com/bitwarden/clients/blob/f60a4606b7581c338d18091a98640ee71392f015/LICENSE.txt).
