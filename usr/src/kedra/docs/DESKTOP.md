# Adwaita desktop defaults

Kedra's shared desktop configuration adapts GNOME's visual language to niri and
Noctalia. Native GTK applications keep their toolkit styling; the shell palette,
top bar and compositor defaults give a related appearance around them. This is a
configuration of niri and Noctalia: it does not implement GNOME Shell or turn
Noctalia's controls into libadwaita widgets.

## Design basis

The [GNOME HIG styling guidance](https://developer.gnome.org/hig/guidelines/ui-styling.html)
recommends light defaults for most applications, support for user style preferences
and restrained custom styling. Kedra follows it with native Adwaita application
defaults and a dark desktop shell; the dark shell is a GNOME desktop convention
adapted here, not an HIG requirement for application windows.

The [HIG typography guidance](https://developer.gnome.org/hig/guidelines/typography.html)
names Adwaita Sans as GNOME's interface font and asks for readable text, limited
variation and respect for accessibility settings. The
[GNOME palette](https://developer.gnome.org/hig/reference/palette.html) provides the
reference colors. Kedra's Noctalia palette maps them to Noctalia's own semantic color
roles; it cannot inherit libadwaita's adaptive CSS behavior.

Noctalia's custom palettes need terminal-color objects in both the light and dark
variants, even with terminal theme generation disabled (observed with Noctalia
5.0.1 and its tagged
[theme service](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/theme/theme_service.cpp)).
Without them Noctalia silently falls back to its built-in yellow and navy palette
while its settings still show Custom/Adwaita, so Kedra's palette includes them.

## Configuration and ownership

The desktop uses a solid, edge-to-edge top bar with workspaces on the left and
status controls on the right, with the clock immediately before Control Center.
The center is empty. Popovers are opaque with modest shadows. The launcher shows an
app grid; notifications appear at the top center and volume and brightness
indicators at the bottom center. Adwaita Sans and the custom Adwaita palette supply
the shell's typography and colors.

A quiet charcoal background (`color:#222226`), set through `[wallpaper.default]`,
replaces Noctalia's bundled illustrated wallpaper. It follows Noctalia's tagged
[wallpaper implementation](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/shell/wallpaper/wallpaper.cpp)
and [configuration example](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/example.toml).
Personal wallpaper overrides still win.

niri uses eight-pixel gaps, rounded window corners, a blue active focus ring and
subtle shadows while keeping its scrolling tiling layout. Common shortcuts:

| Shortcut | Action |
| --- | --- |
| Super+Tab or Super+O | Overview; type to search applications |
| Super+A or Super+D | Application launcher |
| Super+Return | Terminal |
| Super+L | Lock |
| Alt+F4 | Close the focused window |
| Print | Screenshot selection |
| Alt+Print / Shift+Print | Window / screen screenshot |

Image-owned GSettings defaults select native Adwaita styling, a blue accent, Adwaita
icons and cursor, and Adwaita Sans and Mono. A GTK 3 settings fallback covers clients
without an XSettings provider. The image compiles the schema defaults with strict
GLib validation. Explicit user settings still take precedence; no user dconf
database is rewritten. Noctalia's application theme generators are disabled to keep
native application styling. This follows GLib's
[vendor override mechanism](https://docs.gtk.org/gio/class.Settings.html#vendor-overrides)
and GTK 3's [settings lookup](https://docs.gtk.org/gtk3/class.Settings.html).

Shared defaults live under `etc/skel/.config/`; new accounts receive them as ordinary
writable files. Target-specific monitor, scaling and hardware settings belong in a
target overlay. Read the actual
[niri configuration](../../../../etc/skel/.config/niri/config.kdl) and
[Noctalia configuration](../../../../etc/skel/.config/noctalia/config.toml) for every
shortcut and appearance setting.

The niri configuration, the Noctalia configuration and the Adwaita palette are plain
[managed home files](HOME.md), declared in `usr/share/kedra/home-files.toml`.
`sysroot` has no niri- or Noctalia-specific code: after an OS update,
`sysroot home update` merges the new image's defaults into your edits, validates the
result with each application's own checker (`niri validate`,
`noctalia config validate`) and restarts Noctalia; niri reloads its file by itself.
Noctalia keeps GUI overrides separately in `~/.local/state/noctalia/settings.toml`.
That file is not managed and its values win over `config.toml`, so an existing
override can hide a new default. Do not erase it to force the new appearance.

## GTK and Qt applications

GTK 3 uses [adw-gtk3](https://github.com/lassekongo83/adw-gtk3) to match the modern
Adwaita appearance, from Fedora's official
[adw-gtk3-theme package](https://packages.fedoraproject.org/pkgs/adw-gtk3-theme/adw-gtk3-theme/fedora-44.html).
libadwaita keeps native Adwaita styling. The package's GTK 4 CSS stays intact, so
plain GTK 4 applications without libadwaita may also use it through the shared GNOME
`gtk-theme` setting. Legacy GTK 3 applications read `/etc/gtk-3.0/settings.ini` when
no XSettings provider runs; a personal `~/.config/gtk-3.0/settings.ini` overrides it.
Xwayland applications read that file at start, so restart them after editing it. No
GNOME XSettings daemon runs in this niri session, so GNOME setting changes do not
reach running X11 clients.

Applications that follow the GNOME color-scheme setting can be switched with
`gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'`; reset it with
`gsettings reset org.gnome.desktop.interface color-scheme`. For dark adw-gtk3, also
run `gsettings set org.gnome.desktop.interface gtk-theme 'adw-gtk3-dark'` and set
`gtk-theme-name=adw-gtk3-dark` under `[Settings]` in a personal
`~/.config/gtk-3.0/settings.ini` for Xwayland applications; color-scheme alone does
not select that GTK 3 variant. The default is light; fonts, icons and cursor stay
Adwaita, and KDE/Breeze styling is independent.

Qt 5 and Qt 6 applications use KDE's platform integration with the native Breeze
widget style, Breeze icons and KDE Qt Quick Controls desktop styles. The default
color scheme is Breeze Light. Adwaita Sans and Mono keep text consistent across the
desktop while Qt controls keep their KDE appearance. Image defaults live in
`/etc/xdg/kdeglobals`; personal `~/.config/kdeglobals` settings take precedence. The
`[KDE]` `widgetStyle`, `[General]` `ColorScheme` and `[Icons]` `Theme` entries control
those preferences. Restart applications after changing them.

The systemd user environment defaults `QT_QPA_PLATFORMTHEME` to `kde` unless you set
your own. The session stays niri with its existing portal routing. Noctalia theme
templates stay disabled, so changing the shell palette does not rewrite GTK or KDE
application settings. Native Qt integration applies to applications that use the
image's Qt libraries and plugins; sandboxed or separately bundled runtimes need their
own compatible plugins.

## Screen sharing and the X11 bridge

niri keeps its native Wayland portal screen sharing. The optional
`xwaylandvideobridge` does not start automatically under niri: Fedora 44's version
0.5.2 created a focused black tile in the desktop VM, because its
[capture-window implementation](https://raw.githubusercontent.com/KDE/xwaylandvideobridge/v0.5.2/src/contentswindow.cpp)
uses invisibility hints that were ineffective in niri's Xwayland session (see its
[README](https://raw.githubusercontent.com/KDE/xwaylandvideobridge/v0.5.2/README.md)
and [Fedora package](https://packages.fedoraproject.org/pkgs/xwaylandvideobridge/xwaylandvideobridge/fedora-44-updates.html)).
The image adds only `NotShowIn=niri;` to its packaged autostart entry; the package
and its manual launcher remain. Legacy X11 applications that need the bridge can
start `xwaylandvideobridge` explicitly, with the black-window behavior still
possible; that capture workflow is untested.

## Noctalia Greeter evaluation

Noctalia Greeter 1.5.0 is a candidate follow-up, not implemented: the login screen
stays greetd with tuigreet. The upstream
[overview](https://docs.noctalia.dev/greeter/),
[installation](https://docs.noctalia.dev/greeter/installation/),
[source build](https://docs.noctalia.dev/greeter/building-from-source/),
[configuration](https://docs.noctalia.dev/greeter/configuration/) and
[sync](https://docs.noctalia.dev/greeter/sync/) documents were reviewed.

Fedora 44's default repositories lack the package. The chosen approach is a pinned
source/archive build with official Fedora dependencies that keeps the existing
greetd account, PAM/keyring integration and niri session, with an
administrator-owned static Adwaita palette and no automatic sync with the user's
Noctalia profile. Upstream's Terra/Copr route and root setup scripts are out of
scope. Before adoption, review the 1.5.0
[packaging instructions](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/PACKAGING.md)
and [session wrapper](https://raw.githubusercontent.com/noctalia-dev/noctalia-greeter/v1.5.0/scripts/noctalia-greeter-session)
(predictable `/tmp` runtime directories, `-march=native`), then test wrong and correct
logins, session selection, the recovery TTY, 1× and 1.5× scaling and SELinux in a
disposable VM.

## Verification scope

Validate these defaults with the exact packaged niri and Noctalia versions; Noctalia
v4 JSON/Quickshell examples do not apply to this v5 TOML configuration. Parsing and
startup checks establish compatibility only: inspect the running desktop separately
for typography, focus, popovers, launcher behavior, light and dark applications,
scaling and high contrast. Noctalia's custom palette does not establish libadwaita
accessibility conformance. When changing visible desktop configuration, check it
with `kedra-lab up`, `kedra-lab sync` and `kedra-lab shot` and review the screenshots
([container lab](../tests/container/README.md)).

The container harness checks the shipped appearance (Noctalia renders the Adwaita
palette and wallpaper without fallback), session startup, the bridge's autostart
exclusion and the installed toolkit packages on every release candidate. The desktop
VM checks (greeter login, keyring unlock, the GTK, Xwayland and Qt file choosers,
reviewed screenshots) last passed on an earlier source and have not run on an image
built by the current pipeline. Physical displays, scaling, high contrast and a full
accessibility audit are untested; see [STATUS](STATUS.md). Source changes alone do
not update an installed image or an existing home.
