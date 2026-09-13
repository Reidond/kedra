"""Native GUI fixture: select a real file through each toolkit's own dialog.

QEMU sends the keyboard input. No widget methods simulate user activation.
Only the disposable VM account runs this program.
"""
import json
import pathlib
import sys

case = sys.argv[1]
result = pathlib.Path(sys.argv[2])
sample = pathlib.Path.home() / "toolkit-sample.txt"
sample.write_text("Kedra native file selection\n")
metadata = {"case": case}


def report(stage, **values):
    metadata.update(values)
    metadata["stage"] = stage
    result.write_text(json.dumps(metadata) + "\n")


def selected(filename):
    if pathlib.Path(filename) != sample or pathlib.Path(filename).read_text() != "Kedra native file selection\n":
        raise RuntimeError("Native dialog did not return the selected fixture file")
    report("selected", selected_file=pathlib.Path(filename).name)


if case.startswith("gtk") or case == "libadwaita":
    import gi

    gtk4 = case == "libadwaita"
    gi.require_version("Gtk", "4.0" if gtk4 else "3.0")
    if gtk4:
        gi.require_version("Adw", "1")
        from gi.repository import Adw
    from gi.repository import Gdk, GLib, Gtk

    app = Adw.Application(application_id="org.kedra.ToolkitFixture") if gtk4 else Gtk.Application(application_id="org.kedra.ToolkitFixture")

    def activate(application):
        window = Adw.ApplicationWindow(application=application) if gtk4 else Gtk.ApplicationWindow(application=application)
        window.set_title("Kedra " + case)
        window.set_default_size(640, 420)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        for edge in ("top", "bottom", "start", "end"):
            getattr(box, "set_margin_" + edge)(24)
        label = Gtk.Label(label="Native " + case + " appearance\nOpen a file with the system dialog")
        button = Gtk.Button(label="Open File")
        if gtk4:
            box.append(Adw.HeaderBar())
            box.append(label)
            box.append(button)
            window.set_content(box)
        else:
            box.pack_start(label, True, True, 0)
            box.pack_start(button, False, False, 0)
            window.add(box)
        settings = Gtk.Settings.get_default()
        display = Gdk.Display.get_default()
        backend = type(display).__name__
        expected = "X11" if case == "gtk3-xwayland" else "Wayland"
        if expected not in backend:
            raise RuntimeError(f"Expected {expected}, got {backend}")
        theme = settings.get_property("gtk-theme-name")
        font = settings.get_property("gtk-font-name")
        icons = settings.get_property("gtk-icon-theme-name")
        if not theme.startswith("Adwaita") or "Adwaita Sans" not in font or icons != "Adwaita":
            raise RuntimeError(f"Unexpected GTK appearance: {theme}, {font}, {icons}")
        icon_theme = Gtk.IconTheme.get_for_display(display) if gtk4 else Gtk.IconTheme.get_default()
        if not icon_theme.has_icon("document-open"):
            raise RuntimeError("Adwaita document-open icon unavailable")

        def open_file(_button):
            dialog = Gtk.FileChooserNative.new("Select toolkit-sample.txt", window, Gtk.FileChooserAction.OPEN, "Open", "Cancel")
            # Retain the real asynchronous native dialog until its response.
            window.fixture_dialog = dialog

            def response(chooser, response_id):
                if response_id != Gtk.ResponseType.ACCEPT:
                    report("failed", reason="file selection cancelled")
                    application.quit()
                    return
                selected(chooser.get_file().get_path())
                label.set_text("Opened toolkit-sample.txt successfully")
                chooser.destroy()
            dialog.connect("response", response)
            dialog.show()
            GLib.timeout_add(1000, lambda: (report("dialog"), False)[1])

        button.connect("clicked", open_file)
        if not gtk4:
            window.show_all()
        window.present()
        button.grab_focus()
        report("ready", backend=backend, theme=theme, font=font, icons=icons,
               gtk_version=f"{Gtk.get_major_version()}.{Gtk.get_minor_version()}.{Gtk.get_micro_version()}",
               libadwaita_version=f"{Adw.get_major_version()}.{Adw.get_minor_version()}.{Adw.get_micro_version()}" if gtk4 else None)

    app.connect("activate", activate)
    app.run([])
else:
    if case == "qt5":
        from PyQt5 import QtCore, QtGui, QtWidgets
    elif case in ("qt6", "qt6-override"):
        from PyQt6 import QtCore, QtGui, QtWidgets
    else:
        raise RuntimeError("Unknown toolkit case")
    app = QtWidgets.QApplication([])
    window = QtWidgets.QWidget()
    window.setWindowTitle("Kedra " + case)
    window.resize(640, 420)
    layout = QtWidgets.QVBoxLayout(window)
    label = QtWidgets.QLabel("Native KDE " + case + " appearance\nOpen a file with the KDE dialog")
    button = QtWidgets.QPushButton(QtGui.QIcon.fromTheme("document-open"), "Open File")
    layout.addWidget(label)
    layout.addWidget(button)
    style = app.style().objectName()
    icons = QtGui.QIcon.themeName()
    if style.lower() != "breeze" or icons != "breeze" or button.icon().isNull():
        raise RuntimeError(f"Expected native Breeze style/icons, got {style}/{icons}")
    if app.platformName() != "wayland":
        raise RuntimeError("Qt fixture is not using native Wayland")
    if case == "qt6-override" and (app.font().family() != "Adwaita Mono" or app.font().pointSize() != 12):
        raise RuntimeError("Explicit KDE user font preference was not honored")

    def open_file():
        def dialog_ready():
            # Runtime loaded libraries identify the real native KDE dialog;
            # a generic Qt fallback must not silently count as KDE integration.
            mappings = pathlib.Path("/proc/self/maps").read_text()
            if "libKF5KIOFileWidgets" not in mappings and "libKF6KIOFileWidgets" not in mappings:
                report("failed", reason="KDE native file dialog library not loaded")
                app.exit(1)
                return
            report("dialog", kde_file_dialog=True)
        QtCore.QTimer.singleShot(1000, dialog_ready)
        filename, _filter = QtWidgets.QFileDialog.getOpenFileName(window, "Select toolkit-sample.txt", str(pathlib.Path.home()))
        if not filename:
            report("failed", reason="file selection cancelled")
            app.exit(1)
            return
        selected(filename)
        label.setText("Opened toolkit-sample.txt successfully")

    button.clicked.connect(open_file)
    button.setDefault(True)
    window.show()
    button.setFocus()
    report("ready", backend=app.platformName(), theme=style, font=app.font().toString(),
           icons=icons, qt_version=QtCore.QT_VERSION_STR)
    sys.exit(app.exec())
