"""Exercise line review against the generated account's actual niri file."""
import json
import os
import ctypes
import pathlib
import select
import signal
import struct
import subprocess
import time


def cli(*args, success=True):
    result = subprocess.run(["sysroot", "home", "file", *args], stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=60, check=False)
    if (result.returncode == 0) != success:
        # This runs only against generated public fixture content in the VM.
        detail = result.stderr.decode(errors="replace")[:1200]
        raise RuntimeError(f"niri review command {args[0]} returned {result.returncode}: {detail}")
    return json.loads(result.stdout) if success else None


def interrupt_discard(change, plan_id):
    libc = ctypes.CDLL(None, use_errno=True)
    descriptor = libc.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
    if descriptor < 0:
        raise OSError(ctypes.get_errno(), "inotify_init1 failed")
    child = None
    try:
        if libc.inotify_add_watch(descriptor, os.fsencode(native.parent), 0x100 | 0x80) < 0:
            raise OSError(ctypes.get_errno(), "inotify_add_watch failed")
        child = subprocess.Popen(["sysroot", "home", "file", "discard", change,
                                  "--plan", plan_id, "--activate-managed-file"],
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
                if prepared and mask & 0x80 and name == b"config.kdl":
                    os.killpg(child.pid, signal.SIGKILL)
                    killed = True
                    break
            if killed:
                break
        child.communicate(timeout=10)
        if not killed or child.returncode != -signal.SIGKILL:
            raise RuntimeError("niri CLI was not interrupted at file publication")
    finally:
        os.close(descriptor)
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.communicate(timeout=10)
    pending = cli("recover")
    if not pending["pending_activation"] or pending["journal"]["phase"] not in ("prepared", "published"):
        raise RuntimeError("niri interruption did not retain its reservation")


native = pathlib.Path.home() / ".config/niri/config.kdl"
original = native.read_text()
if original.count("gaps 12") != 1 or original.count("width 2") != 1:
    raise RuntimeError("native fixture does not have the expected niri defaults")
cli("init", "--reviewed-safe")
try:
    native.write_text(original.replace("gaps 12", "gaps 14").replace("width 2", "width 3"))
    subprocess.run(["niri", "validate"], check=True, timeout=20)
    rows = cli("status")["changes"]
    selected = next(row["change"]["id"] for row in rows if row["change"]["after"].strip() == "width 3")
    local = next(row["change"]["id"] for row in rows if row["change"]["after"].strip() == "gaps 14")
    cli("stage", selected)
    cli("keep-local", local)
    cli("stage", local, success=False)
    native.write_text(original.replace("gaps 12", "gaps 16").replace("width 2", "width 4"))
    subprocess.run(["niri", "validate"], check=True, timeout=20)
    state = cli("status")
    if state["selection"][0]["after"].strip() != "width 3":
        raise RuntimeError("later native edit replaced the pinned line")
    if not any(row["change"]["after"].strip() == "gaps 16" and row["visible_change"]
               and not row["local_only"] for row in state["changes"]):
        raise RuntimeError("changed exact-local value did not return to review")
    cli("stage", selected, success=False)
    current_width = next(row["change"]["id"] for row in state["changes"]
                         if row["change"]["after"].strip() == "width 4")
    planned = cli("discard-plan", current_width)
    before = native.stat()
    before_label = os.getxattr(native, "security.selinux")
    cli("discard", current_width, "--plan", "0" * 64, "--activate-managed-file", success=False)
    if "width 4" not in native.read_text():
        raise RuntimeError("stale discard plan changed the file")
    cli("discard", current_width, "--plan", planned["plan_id"], "--activate-managed-file")
    after = native.stat()
    if "width 3" not in native.read_text() or "gaps 16" not in native.read_text():
        raise RuntimeError("discard did not restore pinned width while preserving the later gap edit")
    if (before.st_uid, before.st_gid, before.st_mode) != (after.st_uid, after.st_gid, after.st_mode) or os.getxattr(native, "security.selinux") != before_label:
        raise RuntimeError("discard changed niri file metadata")
    recovered = cli("recover")
    if recovered["pending_activation"] is not None or recovered["journal"]["phase"] != "completed":
        raise RuntimeError("discard did not finish its journal")
    if cli("selection")["selection"][0]["after"].strip() != "width 3":
        raise RuntimeError("discard lost the pinned selection")
    # A real relative include must resolve from the native configuration directory.
    included = native.parent / "discard-include.kdl"
    included.write_text('// generated relative include\n')
    native.write_text(native.read_text() + '\ninclude "discard-include.kdl"\n')
    gap = next(row["change"]["id"] for row in cli("status")["changes"]
               if row["change"]["after"].strip() == "gaps 16")
    planned = cli("discard-plan", gap)
    cli("discard", gap, "--plan", planned["plan_id"], "--activate-managed-file")
    if "gaps 12" not in native.read_text() or "include" not in native.read_text():
        raise RuntimeError("relative-include discard changed unrelated content")
    print("KEDRA_R04_NATIVE_NIRI_DISCARD_PASS", flush=True)
    accepted = cli("status")["accepted_baseline"]
    for action in ("abort", "resume", "keep-current"):
        native.write_text(native.read_text().replace("width 3", "width 4"))
        change = next(row["change"]["id"] for row in cli("status")["changes"]
                      if row["change"]["after"].strip() == "width 4")
        planned = cli("discard-plan", change)
        interrupt_discard(change, planned["plan_id"])
        cli("unstage", selected, success=False)
        if action == "keep-current":
            native.write_text(native.read_text().replace("width 3", "width 8"))
            cli("recover", "abort", "--activate-managed-file", success=False)
            if not cli("recover")["pending_activation"]:
                raise RuntimeError("conflicting niri abort cleared recovery")
        cli("recover", action, "--activate-managed-file")
        expected = {"abort": "width 4", "resume": "width 3", "keep-current": "width 8"}[action]
        state = cli("status")
        if expected not in native.read_text() or state["accepted_baseline"] != accepted or state["selection"][0]["after"].strip() != "width 3":
            raise RuntimeError("niri recovery lost the live/selected/baseline distinction")
        recovered = cli("recover")
        phase = {"abort": "aborted", "resume": "completed", "keep-current": "kept_current"}[action]
        if recovered["pending_activation"] is not None or recovered["journal"]["phase"] != phase:
            raise RuntimeError("niri recovery did not record completion")
        if (action == "keep-current") != bool(list(native.parent.glob(".sysroot-activation-*"))):
            raise RuntimeError("unexpected niri recovery checkpoint retention")
        print(f"KEDRA_R04_NIRI_KILLED_CLI_{action.upper().replace('-', '_')}_PASS", flush=True)
    print("KEDRA_R04_NATIVE_NIRI_RECOVERY_PASS", flush=True)
    cli("unstage", selected)
    cli("clear-local", local)
    # Reconcile against the actual installed image and its exact public Git history.
    # This bundle is confined to the generated research derivative.
    repo = pathlib.Path.home() / "kedra-source"
    subprocess.run(["git", "init", str(repo)], check=True, timeout=20)
    subprocess.run(["git", "-C", str(repo), "fetch", "--no-tags",
                    "/usr/share/kedra-research/source.bundle", "HEAD"], check=True, timeout=60)
    subprocess.run(["git", "-C", str(repo), "checkout", "--detach", "FETCH_HEAD"], check=True, timeout=20)
    subprocess.run(["git", "-C", str(repo), "remote", "add", "origin",
                    "https://github.com/Reidond/kedra.git"], check=True, timeout=20)
    before_accept = native.read_text()
    plan = cli("activate-plan", "--repo", str(repo))
    if not plan["installed_image_checked"] or plan["installed_baseline_revision"] != accepted["source_revision"]:
        raise RuntimeError("activation plan did not bind the actual installed baseline")
    cli("apply", "--repo", str(repo), "--plan", "0" * 64, "--activate-managed-file", success=False)
    cli("apply", "--repo", str(repo), "--plan", plan["plan_id"], "--activate-managed-file")
    if native.read_text() != before_accept or cli("status")["accepted_baseline"] != accepted:
        raise RuntimeError("accepting the current installed baseline lost a later native edit")
    print("KEDRA_R04_NIRI_INSTALLED_BASELINE_PASS", flush=True)
finally:
    native.write_text(original)
    subprocess.run(["niri", "validate"], check=True, timeout=20)
print("KEDRA_R03_NATIVE_NIRI_LINES_PASS", flush=True)
