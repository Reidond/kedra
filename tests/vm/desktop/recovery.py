#!/usr/bin/env python3
"""Kill the installed CLI at a real file publication, then use its recovery UI.

Runs only as the generated graphical VM user. No implementation imports, altered
journals, substitute programs or fault-injection hooks in the product.
"""
import ctypes
import json
import os
import pathlib
import select
import signal
import struct
import subprocess
import sys
import time


state = sys.argv[1]
native = pathlib.Path.home() / ".local/state/noctalia"
service = ["systemctl", "--user"]
home = ["sysroot", "home", "--state", state]


def run(args, *, success=True):
    result = subprocess.run(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=90, check=False)
    if (result.returncode == 0) != success:
        raise RuntimeError(f"unexpected exit for {args[:2]}: {result.returncode}")
    return result.stdout


def response(*args):
    return json.loads(run(home + list(args)))


def theme(value):
    run(["noctalia", "msg", "theme-mode-set", value])


def interrupt_publication(plan):
    libc = ctypes.CDLL(None, use_errno=True)
    descriptor = libc.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
    if descriptor < 0:
        raise OSError(ctypes.get_errno(), "inotify_init1 failed")
    child = None
    try:
        # Observe creation of the private candidate, followed by publication of
        # the native file. Noctalia's earlier stop-time writes do not trigger it.
        if libc.inotify_add_watch(descriptor, os.fsencode(native), 0x100 | 0x80) < 0:
            raise OSError(ctypes.get_errno(), "inotify_add_watch failed")
        child = subprocess.Popen(home + ["discard", "theme.mode", "--plan", plan],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, start_new_session=True)
        deadline = time.monotonic() + 60
        prepared = False
        killed = False
        while time.monotonic() < deadline and child.poll() is None:
            if not select.select([descriptor], [], [], 1)[0]:
                continue
            data = os.read(descriptor, 65536)
            offset = 0
            while offset < len(data):
                _, mask, _, length = struct.unpack_from("iIII", data, offset)
                name = data[offset + 16:offset + 16 + length].split(b"\0", 1)[0]
                offset += 16 + length
                if mask & 0x100 and name.startswith(b".sysroot-activation-"):
                    prepared = True
                if prepared and mask & 0x80 and name == b"settings.toml":
                    os.killpg(child.pid, signal.SIGKILL)
                    killed = True
                    break
            if killed:
                break
        child.communicate(timeout=10)
        if not killed or child.returncode != -signal.SIGKILL:
            raise RuntimeError("CLI was not interrupted at native publication")
    finally:
        os.close(descriptor)
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.communicate(timeout=10)
    pending = response("recover")
    if not pending["pending"] or pending["journal"]["phase"] not in ("prepared", "published"):
        raise RuntimeError("interrupted publication did not retain recoverable pending state")
    run(home + ["stage", "theme.mode"], success=False)


for action in ("abort", "resume", "keep-current"):
    theme("light")
    response("stage", "theme.mode")
    theme("auto")
    plan = response("plan", "--discard", "theme.mode")["plan_id"]
    metadata = run(["stat", "-c", "%u:%g:%a:%C", str(native / "settings.toml")])
    interrupt_publication(plan)
    if action == "keep-current":
        # A real application edits the file after the interrupted operation.
        # Exact rollback must refuse that edit; explicit keep-current retains it.
        run(service + ["start", "kedra-noctalia.service"])
        for attempt in range(30):
            ping = subprocess.run(["noctalia", "msg", "log-level-status"],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                  timeout=12, check=False)
            if ping.returncode == 0:
                break
            time.sleep(0.2)
        theme("dark")
        run(service + ["stop", "kedra-noctalia.service"])
        run(home + ["recover", "abort"], success=False)
        if not response("recover")["pending"]:
            raise RuntimeError("conflicting abort cleared the pending operation")
    result = response("recover", action)
    expected = {"abort": "auto", "resume": "light", "keep-current": "dark"}[action]
    if not (result["operation_completed"] and result["pending"] is None
            and result["fields"][0]["live"]["value"] == expected
            and result["fields"][0]["selected"]["value"] == "light"):
        raise RuntimeError(f"{action} did not preserve live/selected state")
    run(service + ["is-active", "kedra-noctalia.service"])
    if run(["stat", "-c", "%u:%g:%a:%C", str(native / "settings.toml")]) != metadata:
        raise RuntimeError("native settings metadata changed during recovery")
    expected_phase = {"abort": "aborted", "resume": "completed", "keep-current": "kept_current"}[action]
    journal = response("recover")
    if journal["journal"]["phase"] != expected_phase:
        raise RuntimeError("recovery did not record its completed outcome")
    checkpoints = list(native.glob(".sysroot-activation-*"))
    if (action == "keep-current") != bool(checkpoints):
        raise RuntimeError("unexpected retained checkpoint state")
    print(f"KEDRA_R04_KILLED_CLI_{action.upper().replace('-', '_')}_PASS", flush=True)

response("unstage", "theme.mode")
