"""Exercise line review against the generated account's actual niri file."""
import json
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
    cli("unstage", selected)
    cli("clear-local", local)
finally:
    native.write_text(original)
    subprocess.run(["niri", "validate"], check=True, timeout=20)
print("KEDRA_R03_NATIVE_NIRI_LINES_PASS", flush=True)
