#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Drive the disposable desktop VM's password login and session through QMP.

run.py starts this as root under xvfb-run: KVM, the GTK GL display and the
root-owned generated disk. The guest observer (check.sh) reports markers on its
second serial port; this host side types the generated password into tuigreet,
drives the Xwayland and Qt file choosers with real key events and captures
screenshots.
"""
import argparse
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import qemu
import secure_boot

CAPTURES = (('KEDRA_R06_LOGGED_OUT_READY', 'bitwarden.png'), ('KEDRA_R07_SESSION_READY', 'desktop.png'),
            ('KEDRA_R07_SETTINGS_READY', 'settings.png'))
TOOLKIT = re.compile(r'^KEDRA_TOOLKIT_(gtk3-wayland|gtk3-xwayland|libadwaita|qt5|qt6|qt6-override)_(ready|dialog|selected)$',
                     re.MULTILINE)
DEADLINE = 900


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--disk', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--password-file', type=Path, required=True)
    parser.add_argument('--success-marker', default='KEDRA_R07_SESSION_PASS')
    parser.add_argument('--login-marker', default='KEDRA_R07_LOGIN_READY')
    parser.add_argument('--failure-marker', default='KEDRA_R07_FAIL')
    parser.add_argument('--persistent-disk', action='store_true')
    parser.add_argument('--cases-disk', type=Path)
    parser.add_argument('--network', action='store_true')
    parser.add_argument('--firmware-vars', type=Path)
    parser.add_argument('--remembered-login', action='store_true')
    return parser.parse_args()


def firmware_variables(args, work):
    firmware = args.firmware_vars.resolve() if args.firmware_vars else work / 'OVMF_VARS.fd'
    try:
        if not firmware.exists():
            secure_boot.create('microsoft', firmware)
        elif not args.firmware_vars:
            raise SystemExit('Existing VM firmware state requires an explicit --firmware-vars path')
        else:
            # Persisted state must still carry the Microsoft-enrolled Secure Boot
            # trust; a store in setup mode or with other keys is never reused.
            secure_boot.verify(firmware)
    except secure_boot.FirmwareError as error:
        raise SystemExit(f'Refusing VM firmware variables: {error}') from None
    return firmware


def command(args, disk, work, firmware):
    argv = [qemu.qemu_binary('x86_64'), *qemu.machine_argv(
        'x86_64', 'kvm', code=secure_boot.CODE, variables=firmware, qmp=work / 'qmp.sock',
        serial=work / 'serial.log', cpus=4, memory_mib=4096)]
    # The second serial port is the guest observer's ttyS1 marker channel.
    argv += ['-serial', f'file:{qemu.option_path(work / "events.log")}']
    argv += qemu.disk_argv('system', disk, snapshot=not args.persistent_disk)
    if args.cases_disk:
        cases = args.cases_disk.resolve(strict=True)
        if not cases.is_file() or cases.suffix != '.raw':
            raise SystemExit('Expected a generated raw fixture disk')
        argv += qemu.disk_argv('cases', cases, image_format='raw', readonly=True)
    argv += qemu.network_argv(args.network)
    # No -no-reboot: shim's fallback may reset after adding a boot entry.
    return argv + ['-vga', 'none', '-device', 'virtio-vga-gl,xres=1280,yres=768', '-display', 'gtk,gl=on',
                   '-full-screen', '-audiodev', 'none,id=audio0', '-device', 'ich9-intel-hda',
                   '-device', 'hda-duplex,audiodev=audio0']


def fit_window(process, work):
    # Xvfb has no window manager to honor GTK's fullscreen request. Resize
    # only this generated QEMU window so captures match the guest mode.
    windows = subprocess.check_output([
        'xdotool', 'search', '--sync', '--all', '--onlyvisible', '--pid', str(process.pid), '--name', '.*',
    ], timeout=15).decode().splitlines()
    if len(windows) != 1 or not windows[0].isdigit():
        raise RuntimeError('Expected one owned QEMU display window')
    subprocess.run(['xdotool', 'windowsize', windows[0], '1280', '768'], check=True, timeout=10)
    subprocess.run(['xdotool', 'windowmove', windows[0], '0', '0'], check=True, timeout=10)
    (work / 'window-geometry.log').write_bytes(subprocess.check_output([
        'xdotool', 'getwindowgeometry', windows[0],
    ], timeout=10))


class Session:
    """Host half of the guest observer's marker protocol."""

    def __init__(self, args, work, password):
        self.args, self.work, self.password = args, work, password
        self.events = work / 'events.log'
        self.qmp = None
        self.markers = set()

    def text(self):
        return qemu.read_log(self.events)

    def screenshot(self, name):
        try:
            self.qmp.screendump(self.work / name)
        except qemu.VmError as error:
            if 'no surface' not in str(error):
                raise
            # GL scanouts need not expose a software QMP surface. Capture the
            # actual isolated Xvfb display, retaining visual evidence either way.
            subprocess.run(['import', '-window', 'root', str(self.work / name)], check=True, timeout=15)

    def login(self, text):
        if self.args.login_marker not in text or 'login' in self.markers:
            return
        time.sleep(1)
        self.screenshot('login.png')
        if not self.args.remembered_login:
            self.qmp.type_text('kedra-test\n')
            time.sleep(0.7)
        self.qmp.type_text(self.password + '\n')
        self.password = ''
        self.markers.add('login')
        print('Submitted disposable account login', flush=True)

    def captures(self, text):
        for marker, name in CAPTURES:
            if marker in text and marker not in self.markers:
                time.sleep(2)
                self.screenshot(name)
                self.markers.add(marker)
                print(f'Captured {name}', flush=True)

    def confirm_libadwaita(self):
        # Nautilus' portal navigates to the parent and selects the file on the
        # first Return. A second user confirmation opens that selection. Wait for
        # an already-completed response first so a portal that accepts
        # immediately is not activated again.
        selected = 'KEDRA_TOOLKIT_libadwaita_selected'
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            text = self.text()
            if selected in text or self.args.failure_marker in text:
                break
            time.sleep(0.25)
        text = self.text()
        if selected not in text and self.args.failure_marker not in text:
            self.qmp.type_text('\n')
            time.sleep(1)
            self.screenshot('toolkit-libadwaita-open-confirmed.png')

    def choose_file(self, case):
        # KDE's Ctrl+L edits the directory navigator. Its English &Name
        # mnemonic targets the file-entry field in both KF5 and KF6, whose
        # Return accepts the file.
        self.qmp.send_keys(['alt', 'n'] if case.startswith('qt') else ['ctrl', 'l'], hold_time=80)
        time.sleep(0.3)
        if case.startswith('qt'):
            self.qmp.send_keys(['ctrl', 'a'], hold_time=80)
            time.sleep(0.2)
        self.qmp.type_text('/home/kedra-test/toolkit-sample.txt')
        # GTK debounces location edits before enabling Open; Return immediately
        # after the last character is lost. Resolve the path before confirming it.
        time.sleep(1)
        self.screenshot(f'toolkit-{case}-submitted.png')
        self.qmp.type_text('\n')
        time.sleep(1)
        self.screenshot(f'toolkit-{case}-confirmed.png')
        if case == 'libadwaita':
            self.confirm_libadwaita()

    def toolkits(self, text):
        for case, stage in TOOLKIT.findall(text):
            marker = f'toolkit-{case}-{stage}'
            if marker in self.markers:
                continue
            time.sleep(1)
            self.screenshot(marker + '.png')
            if stage == 'ready':
                self.qmp.type_text('\n')
            elif stage == 'dialog':
                self.choose_file(case)
            self.markers.add(marker)
            print(f'Captured and drove {marker}', flush=True)

    def step(self, process):
        if self.qmp is None and (self.work / 'qmp.sock').exists():
            self.qmp = qemu.Qmp(self.work / 'qmp.sock', timeout=10)
        text = self.text()
        if self.args.failure_marker in text:
            if self.qmp:
                self.screenshot('failure.png')
            raise RuntimeError('Guest desktop check failed; inspect serial.log')
        if self.qmp:
            self.login(text)
            self.captures(text)
            self.toolkits(text)


def finish(process, session, args):
    if process.poll() is None:
        if session.qmp:
            session.screenshot('timeout.png')
        raise RuntimeError('Desktop VM timed out')
    text = session.text()
    if process.returncode != 0 or args.success_marker not in text:
        raise RuntimeError('Desktop VM did not pass; inspect QEMU/serial evidence')
    if 'KEDRA_SECUREBOOT_PASS' not in text:
        raise RuntimeError('Guest did not prove UEFI Secure Boot; inspect serial evidence')
    print(f'PASS: generated graphical VM reported {args.success_marker} under UEFI Secure Boot', flush=True)


def main():
    args = arguments()
    disk = args.disk.resolve(strict=True)
    if not disk.is_file() or disk.suffix != '.qcow2':
        raise SystemExit('Only a generated QCOW2 file can be tested')
    password = args.password_file.read_text().strip()
    if not re.fullmatch(r'[0-9a-f]{32}', password):
        raise SystemExit('Expected generated disposable test password')
    work = args.work.resolve()
    work.mkdir(parents=True, exist_ok=True)
    firmware = firmware_variables(args, work)
    process = qemu.launch(command(args, disk, work, firmware), work / 'qemu.log')
    session = Session(args, work, password)
    try:
        fit_window(process, work)
        deadline = time.monotonic() + DEADLINE
        while time.monotonic() < deadline and process.poll() is None:
            session.step(process)
            time.sleep(1)
        finish(process, session, args)
    finally:
        if session.qmp:
            session.qmp.close()
        qemu.stop(process, timeout=10)


if __name__ == '__main__':
    main()
