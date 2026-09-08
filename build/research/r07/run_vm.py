"""Disposable graphical integration test using QEMU's documented QMP interface."""
import argparse
import json
import pathlib
import re
import shutil
import socket
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--disk", type=pathlib.Path, required=True)
parser.add_argument("--work", type=pathlib.Path, required=True)
parser.add_argument("--password-file", type=pathlib.Path, required=True)
args = parser.parse_args()
disk = args.disk.resolve(strict=True)
if not disk.is_file() or disk.suffix != ".qcow2":
    raise SystemExit("Only a generated QCOW2 file can be tested")
password = args.password_file.read_text().strip()
if not re.fullmatch(r"[0-9a-f]{32}", password):
    raise SystemExit("Expected generated disposable test password")
work = args.work.resolve()
work.mkdir(parents=True, exist_ok=True)
shutil.copyfile("/usr/share/OVMF/OVMF_VARS_4M.fd", work / "OVMF_VARS.fd")
qmp_path = work / "qmp.sock"
log = work / "serial.log"
events = work / "events.log"

class Qmp:
    def __init__(self):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(10)
        self.socket.connect(str(qmp_path))
        self.file = self.socket.makefile("rwb", buffering=0)
        self.file.readline()  # QMP greeting
        self.call("qmp_capabilities")

    def call(self, command, arguments=None):
        message = {"execute": command}
        if arguments is not None:
            message["arguments"] = arguments
        self.file.write(json.dumps(message).encode() + b"\n")
        while True:
            line = self.file.readline()
            if not line:
                raise RuntimeError("QEMU closed QMP")
            response = json.loads(line)
            if "error" in response:
                raise RuntimeError(f"QMP {command} failed: {response['error']}")
            if "return" in response:
                return response["return"]

    def screenshot(self, name):
        try:
            self.call("screendump", {"filename": str(work / name), "format": "png"})
        except RuntimeError as error:
            if "no surface" not in str(error):
                raise
            # GL scanouts need not expose a software QMP surface. Capture the
            # actual isolated Xvfb display, retaining visual evidence either way.
            subprocess.run(["import", "-window", "root", str(work / name)], check=True, timeout=15)

    def type_text(self, value):
        for character in value:
            code = {"-": "minus", "\n": "ret"}.get(character, character)
            self.call("send-key", {"keys": [{"type": "qcode", "data": code}], "hold-time": 40})
            time.sleep(0.08)

command = [
    "qemu-system-x86_64", "-machine", "q35,accel=kvm", "-cpu", "host",
    "-smp", "4", "-m", "4096", "-device", "virtio-rng-pci",
    "-drive", "if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd",
    "-drive", f"if=pflash,format=raw,file={work / 'OVMF_VARS.fd'}",
    "-drive", f"file={disk},if=virtio,format=qcow2,snapshot=on",
    "-vga", "none", "-device", "virtio-vga-gl,xres=1280,yres=768", "-display", "gtk,gl=on", "-full-screen",
    "-audiodev", "none,id=audio0", "-device", "ich9-intel-hda", "-device", "hda-duplex,audiodev=audio0",
    "-serial", f"file:{log}", "-serial", f"file:{events}", "-monitor", "none", "-nic", "none",
    "-qmp", f"unix:{qmp_path},server=on,wait=off",
]
with (work / "qemu.log").open("w") as output:
    process = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT)
    qmp = None
    markers = set()
    try:
        # Xvfb has no window manager to honor GTK's fullscreen request. Resize
        # only this generated QEMU window so captures match the guest mode.
        windows = subprocess.check_output([
            "xdotool", "search", "--sync", "--all", "--onlyvisible", "--pid", str(process.pid), "--name", ".*",
        ], timeout=15).decode().splitlines()
        if len(windows) != 1 or not windows[0].isdigit():
            raise RuntimeError("Expected one owned QEMU display window")
        subprocess.run(["xdotool", "windowsize", windows[0], "1280", "768"], check=True, timeout=10)
        subprocess.run(["xdotool", "windowmove", windows[0], "0", "0"], check=True, timeout=10)
        (work / "window-geometry.log").write_bytes(subprocess.check_output([
            "xdotool", "getwindowgeometry", windows[0],
        ], timeout=10))
        deadline = time.monotonic() + 480
        while time.monotonic() < deadline and process.poll() is None:
            if qmp is None and qmp_path.exists():
                qmp = Qmp()
            text = events.read_text(errors="replace") if events.exists() else ""
            if "KEDRA_R07_FAIL" in text:
                if qmp:
                    qmp.screenshot("failure.png")
                raise RuntimeError("Guest desktop check failed; inspect serial.log")
            if qmp and "KEDRA_R07_LOGIN_READY" in text and "login" not in markers:
                time.sleep(1)
                qmp.screenshot("login.png")
                qmp.type_text("kedra-test\n")
                time.sleep(0.7)
                qmp.type_text(password + "\n")
                password = ""
                markers.add("login")
                print("Submitted disposable account login", flush=True)
            for marker, name in [("KEDRA_R07_SESSION_READY", "desktop.png"), ("KEDRA_R07_SETTINGS_READY", "settings.png")]:
                if qmp and marker in text and marker not in markers:
                    time.sleep(2)
                    qmp.screenshot(name)
                    markers.add(marker)
                    print(f"Captured {name}", flush=True)
            time.sleep(1)
        if process.poll() is None:
            if qmp:
                qmp.screenshot("timeout.png")
            raise RuntimeError("Desktop VM timed out")
        text = events.read_text(errors="replace") if events.exists() else ""
        if process.returncode != 0 or "KEDRA_R07_SESSION_PASS" not in text:
            raise RuntimeError("Desktop VM did not pass; inspect QEMU/serial evidence")
        print("PASS: graphical login, niri/Noctalia IPC, services and unlocked synthetic keyring", flush=True)
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
