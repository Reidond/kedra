# Welcome to Kedra

Kedra uses Fedora 44, the niri window manager and Noctalia desktop controls.
At the login screen, enter the account and password you created during installation.
The Super key is usually the Windows key.

| Shortcut | Action |
|---|---|
| Super+Return | Open the Foot terminal |
| Super+Shift+/ | Show the shortcut overlay |
| Super+D | Open the application launcher |
| Super+Shift+D | Open the independent Fuzzel launcher |
| Super+Comma | Open Noctalia settings |
| Super+S | Open desktop controls |
| Super+Q | Close the focused window |
| Super+arrows | Move focus |
| Super+Ctrl+arrows | Move the window/column |
| Super+PageUp/PageDown | Change workspace |
| Super+F / Super+Shift+F | Maximize column / fullscreen |
| Super+V | Toggle floating window |
| Super+O | Overview |
| Super+Shift+E | Log out, with confirmation |

Your configuration files are normal writable files under `~/.config/`. Noctalia
stores GUI overrides in `~/.local/state/noctalia/settings.toml`; these override
the source defaults. Do not delete that file to force an update over local edits.

The source checkout is separate from the installed OS. In a Kedra checkout:

    sysroot source plan --host desktop
    sysroot source plan --host desktop --json

Those commands inspect committed build inputs and do not modify your machine.
`sysroot status` reports which management capabilities this build implements.
`sysroot doctor` checks the installed desktop session without changing it or
requesting administrator access. Use `sysroot doctor --json` for structured output.
Managed updates require configured release trust and enrollment. Home review and
reconciliation support three Noctalia fields and explicit niri line/hunk review,
discard and installed-baseline acceptance. Use `sysroot home file --help` for the
niri workflow and review the file for secrets before adopting it. Wider file
groups and a promoted owner installer are still being prepared.
Do not run image-building scripts as a workstation package installer.

For recovery without a graphical session, use Ctrl+Alt+F2 and log in on the text
console. `bootc status` distinguishes the current image from a staged one.
Keep installation/recovery media available. Rolling back the OS does not restore
all personal data; files under /var, including home directories, persist.
