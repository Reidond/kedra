"""Exercise line review against the generated account's actual niri file."""
import json
import os
import pathlib
import subprocess


def cli(*args, success=True):
    result = subprocess.run(["sysroot", "home", "file", *args], stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=60, check=False)
    if (result.returncode == 0) != success:
        raise RuntimeError(f"niri review command {args[0]} returned {result.returncode}")
    return json.loads(result.stdout) if success else None


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
    cli("unstage", selected)
    cli("clear-local", local)
finally:
    native.write_text(original)
    subprocess.run(["niri", "validate"], check=True, timeout=20)
print("KEDRA_R03_NATIVE_NIRI_LINES_PASS", flush=True)
