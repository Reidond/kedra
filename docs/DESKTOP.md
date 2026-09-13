# Adwaita desktop defaults

Kedra's shared desktop configuration adapts GNOME's visual language to niri and
Noctalia. Native GTK applications retain their toolkit styling. The shell palette,
top bar and compositor defaults provide a related appearance around those apps.
This is a configuration of niri and Noctalia; it does not implement GNOME Shell
or turn Noctalia's controls into libadwaita widgets.

## Design basis

The [GNOME HIG styling guidance](https://developer.gnome.org/hig/guidelines/ui-styling.html)
recommends light defaults for most applications, support for user style preferences
and restrained custom styling. Kedra follows that direction with native Adwaita
application defaults and a dark desktop shell. The shell treatment is a GNOME
desktop convention adapted here, rather than an HIG requirement for application
windows.

The [HIG typography guidance](https://developer.gnome.org/hig/guidelines/typography.html)
identifies Adwaita Sans as GNOME's interface font and emphasizes readable text,
limited variation and respect for accessibility settings. The
[GNOME palette](https://developer.gnome.org/hig/reference/palette.html) provides
the reference colors. The Noctalia palette maps those references to its own
semantic color roles; it cannot inherit libadwaita's adaptive CSS behavior.

## Configuration and ownership

The desktop uses a solid, edge-to-edge top bar with workspaces on the left, a
centered clock and status controls on the right. Popovers are opaque with modest
shadows. The launcher displays an app grid; notifications appear at the top
center and volume/brightness indicators at the bottom center. Adwaita Sans and
the custom Adwaita palette supply the shell's typography and colors.

Niri uses eight-pixel gaps, rounded window corners, a blue active focus ring and
subtle shadows while retaining its scrolling tiling layout. Common shortcuts are:

| Shortcut | Action |
| --- | --- |
| Super+Tab or Super+O | Overview; type to search applications |
| Super+A or Super+D | Application launcher |
| Super+Return | Terminal |
| Super+L | Lock |
| Alt+F4 | Close the focused window |
| Print | Screenshot selection |
| Alt+Print / Shift+Print | Window / screen screenshot |

Image-owned GSettings defaults select native Adwaita styling, blue accent, Adwaita
icons/cursor and Adwaita Sans/Mono. A GTK 3 settings fallback covers clients without
an XSettings provider. The image compiles the schema defaults with strict GLib
validation. Explicit user settings still take precedence; no user dconf database
is rewritten. Noctalia's application theme generators are disabled to preserve
native application styling. This follows GLib's
[vendor override mechanism](https://docs.gtk.org/gio/class.Settings.html#vendor-overrides)
and GTK 3's [settings lookup](https://docs.gtk.org/gtk3/class.Settings.html).

Shared defaults live under `home/.config/`; new accounts receive ordinary writable
files. Host-specific monitor, scaling and hardware settings remain separately
scoped. Review the actual [niri configuration](../home/.config/niri/config.kdl)
and [Noctalia configuration](../home/.config/noctalia/config.toml) for the full
shortcut and appearance settings.

Noctalia 5.0.1 stores GUI overrides separately from curated `config.toml`. An
existing override can continue to win over a new source default. Do not erase
the override file to force the appearance. The [home review workflow](HOME-REVIEW.md)
still manages only `theme.mode`, `shell.button_borders` and `shell.input_borders`;
the appearance work does not expand that safe projection to palette, typography,
bar layout or GTK settings. Niri's adopted main file has its own
[line review and activation workflow](TEXT-REVIEW.md).

## Verification scope

Use the exact packaged niri and Noctalia versions to validate these defaults.
Noctalia v4 JSON/Quickshell examples are incompatible with this v5 TOML profile.
Native parsing and startup checks establish configuration compatibility; inspect
the running desktop separately for typography, focus, popovers, launcher behavior,
light/dark applications, scaling and high contrast. Noctalia's custom palette does
not establish libadwaita accessibility conformance.

See [verified status](STATUS.md) and the [worklog](../worklog.md) for actual results.
Source changes alone do not update an existing installed image or writable home.
