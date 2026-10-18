# Welcome to Kedra

Kedra is Fedora 44 with the niri window manager and Noctalia desktop controls,
updated as a whole signed image. At the login screen, enter the account and
password you created during installation. The Super key is usually the Windows key.

| Shortcut | Action |
|---|---|
| Super+Return | Open the Foot terminal |
| Super+Shift+/ | Show the shortcut overlay |
| Super+D or Super+A | Open the application launcher |
| Super+Shift+D | Open the Fuzzel launcher |
| Super+Comma | Open Noctalia settings |
| Super+S | Open desktop controls |
| Super+Q or Alt+F4 | Close the focused window |
| Super+arrows | Move focus |
| Super+Ctrl+arrows | Move the window or column |
| Super+PageUp/PageDown | Change workspace |
| Super+F / Super+Shift+F | Maximize column / fullscreen |
| Super+V | Toggle floating window |
| Super+O or Super+Tab | Overview; type to search applications |
| Super+L | Lock the screen |
| Print | Screenshot a selection |
| Super+Shift+E | Log out, with confirmation |

## First steps

Run management commands as your ordinary account, never with sudo; the installed
helper asks for administrator authentication when it needs it.

    sysroot doctor          # checks the desktop session; changes nothing
    sysroot update status   # running, staged and rollback images

To stop typing the disk passphrase at every boot, run
`sysroot setup tpm-unlock --dry-run` to check this machine and print the exact
commands, then `sysroot setup tpm-unlock` in a terminal. It asks for the current
disk passphrase and seals a new disk key in the TPM, bound to the Secure Boot state
(PCR 7). The passphrase stays enrolled, and boot asks for it whenever the TPM
refuses, for example after a firmware or Secure Boot change: then run the command
again with `--replace`. After a TPM reset, `--replace` keeps the old slot: run
`--remove`, then the plain command. `sysroot setup tpm-unlock --remove` undoes it,
even with the TPM or Secure Boot off. Without `--with-pin`, anyone at this
machine's console can reach the unlocked disk, because the boot menu and kernel
command line are not protected; in a virtual machine, whoever can read the VM's TPM
state can recover the key.

## Updates

    sysroot update check    # is a newer signed image available?
    sysroot update stage    # stage it for the next boot; does not reboot

This system follows only its own target's signed image (ghcr.io/reidond/kedra-desktop
for x86_64, ghcr.io/reidond/kedra-qemu-arm64 for the aarch64 virtual machine).
bootc verifies the image signature while staging, and the helper refuses an image
older than the running one. Nothing updates or reboots automatically. Reboot when
you choose, then run `sysroot update status --home` and `sysroot doctor`.
`sysroot update rollback` selects the previous image for the next boot and holds
further updates until a later `sysroot update stage --resume`.

## Your configuration files

Files under `~/.config/` are normal writable files. The niri and Noctalia
configuration files are managed home files, listed in
/usr/share/kedra/home-files.toml. An OS update never changes them by itself: run
`sysroot home update` after rebooting into a new image. It merges the new defaults
into your edits, validates the result and restarts Noctalia. If both changed the
same lines it leaves your file alone and writes `<file>.kedra-new`: merge it into
your file, delete it, and run the command again. `sysroot home status` shows each
file's difference from the defaults. Noctalia stores GUI overrides in
`~/.local/state/noctalia/settings.toml`; they win over `config.toml`. Do not delete
that file to force new defaults.

## Agents and keys

`sysroot codex` runs the Codex CLI bundled with the image in your Kedra checkout
(`~/src/kedra` by default), with its own private profile; `sysroot claude --runtime
user` runs a Claude Code you installed yourself. `sysroot codex --print-plan` shows
what would run. Bitwarden Desktop (`bitwarden`) holds SSH keys: enable its SSH agent,
and SSH uses `~/.bitwarden-ssh-agent.sock` unless you set your own `SSH_AUTH_SOCK`.
Agent sessions never receive a `BW_SESSION`.

In a Kedra checkout, `sysroot source plan --host desktop` shows the committed build
inputs without changing anything. Do not run image-building scripts on this system
as a package installer.

## Recovery

Without a graphical session, press Ctrl+Alt+F2 and log in on the text console.
`bootc status` shows the running and staged images. Keep installation media and the
disk passphrase. Rolling back the OS does not restore personal data: home
directories and everything else under /var persist.

## Documentation

This file is /usr/share/doc/kedra/START-HERE.md. The full documentation lives in the
source repository: https://github.com/Reidond/kedra/tree/main/usr/src/kedra/docs
(installation: INSTALL.md, updates: UPDATES.md, home files: HOME.md, agents:
AGENT-LAUNCHERS.md).
