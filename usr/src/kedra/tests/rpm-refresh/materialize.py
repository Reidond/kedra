#!/usr/bin/env python3
"""Exercise native DNF/RPM in a disposable container, retaining public evidence."""

import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import uuid


EVIDENCE = Path("/evidence")
FIXTURE = Path("/fixture")
# Fedora 44 uses RPM 6; PAYLOADSHA256 replaces RPM 4's PAYLOADDIGEST tag.
RPM_FORMAT = "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n"
COMMANDS = []


class NativeFailure(Exception):
    def __init__(self, step, code):
        super().__init__(f"{step} exited {code}")
        self.step = step
        self.code = code


def save(name, value):
    (EVIDENCE / name).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def native(step, args, accept=(0,)):
    started = time.time_ns()
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    log = f"{len(COMMANDS):02d}-{step}.log"
    (EVIDENCE / log).write_bytes(result.stdout)
    COMMANDS.append({"step": step, "argv": args, "exit_code": result.returncode,
                     "started_ns": started, "finished_ns": time.time_ns(), "log": log})
    save("commands.json", COMMANDS)
    print(f"{step}: exit {result.returncode}", flush=True)
    if result.returncode not in accept:
        raise NativeFailure(step, result.returncode)
    return result


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(label):
    result = native(label, ["rpm", "-qa", "--queryformat", RPM_FORMAT])
    value = "\n".join(sorted(result.stdout.decode().splitlines())) + "\n"
    (EVIDENCE / f"{label}.tsv").write_text(value, encoding="utf-8")
    return value


def retain_rejected_rpm():
    """Correlate DNF's named signature refusal with the exact native RPM bytes."""
    install = next(item for item in COMMANDS if item["step"] == "install")
    diagnostic = (EVIDENCE / install["log"]).read_text(encoding="utf-8", errors="replace")
    match = re.search(
        r'OpenPGP check for package "kedra-refresh-requested-(?:0:)?2-1\.noarch" '
        r'\(([^)\n]+)\) from repo "kedra-fixture" has failed:', diagnostic,
    )
    if "Signature verification failed." not in diagnostic or match is None:
        return {"recognized_dnf_signature_failure": False}
    path = Path(match[1]).resolve()
    allowed = (Path("/fixture/repo").resolve(), Path("/var/cache/libdnf5").resolve())
    if (not path.is_file() or path.name != "kedra-refresh-requested-2-1.noarch.rpm"
            or not any(path.is_relative_to(directory) for directory in allowed)):
        return {"recognized_dnf_signature_failure": True, "named_archive_retained": False}
    retained = EVIDENCE / "rejected-package.rpm"
    shutil.copyfile(path, retained)
    verification = native("rejected-rpm-checksig", ["rpmkeys", "--checksig", "--verbose", str(retained)],
                          accept=range(256))
    return {
        "recognized_dnf_signature_failure": True, "named_archive_retained": True,
        "dnf_archive_path": str(path), "archive_sha256": digest(retained),
        "matches_snapshot": digest(retained) == digest(FIXTURE / "repo" / path.name),
        "rpm_checksig_exit": verification.returncode,
        "native_bad_payload_digest": bool(re.search(r"Payload[^\n]*digest[^\n]*: BAD", verification.stdout.decode(errors="replace"))),
    }


def main():
    if not Path("/run/.containerenv").is_file() or len(sys.argv) != 2 or sys.argv[1] not in ("seed", "reconcile"):
        raise RuntimeError("use only the mounted Actions seed/reconciliation container")
    mode = sys.argv[1]
    # A new receipt is written by the RUN process, outside the image layer. A
    # cached RUN cannot produce one, even when the mounted snapshot has changed.
    receipt = {"schema_version": 1, "mode": mode, "execution_id": str(uuid.uuid4()),
               "started_ns": time.time_ns(), "state": "running",
               "rpm_fixture_equivalence_evaluable": False, "freshness_written": False}
    save("outcome.json", receipt)
    try:
        native("dnf-version", ["dnf", "--version"])
        native("rpm-version", ["rpm", "--version"])
        native("rpm-querytags", ["rpm", "--querytags"])
        inventory("before")
        config = Path("/tmp/kedra-refresh-config")
        config.mkdir(exist_ok=True)
        (config / "dnf.conf").write_text(
            "[main]\ngpgcheck=True\nlocalpkg_gpgcheck=True\n", encoding="utf-8")
        (config / "fixture.repo").write_text(
            "[kedra-fixture]\nname=Disposable signed RPM snapshot\n"
            "baseurl=file:///fixture/repo\nenabled=True\n"
            "gpgcheck=True\nrepo_gpgcheck=True\ngpgkey=file:///fixture/key.asc\n"
            "skip_if_unavailable=False\nmetadata_expire=0\nretries=1\ntimeout=10\n",
            encoding="utf-8",
        )
        shutil.copytree(config, EVIDENCE / "repo-config")
        shutil.copyfile(FIXTURE / "snapshot.json", EVIDENCE / "snapshot.json")
        shutil.copyfile(FIXTURE / "key.asc", EVIDENCE / "key.asc")
        if (FIXTURE / "repo/repodata").is_dir():
            shutil.copytree(FIXTURE / "repo/repodata", EVIDENCE / "repodata")
        common = [
            "dnf", "-y", "--best", "--refresh", "--repo=kedra-fixture",
            "--setopt=*.skip_if_unavailable=False", "--setopt=*.gpgcheck=True",
            f"--config={config / 'dnf.conf'}", f"--setopt=reposdir={config}",
        ]
        save("intent.json", {"mode": mode, "requested": ["kedra-refresh-requested"],
                              "inherited": ["kedra-refresh-inherited"],
                              "dnf_options": common[1:],
                              "scope": "signed synthetic RPM fixture only"})
        # Match the production --best/--refresh upgrade then install semantics.
        # check-upgrade is advisory: 100 is not an error or freshness success.
        probe = native("check-upgrade", [*common, "check-upgrade"], accept=(0, 100))
        receipt["check_upgrade_exit"] = probe.returncode
        if mode == "reconcile":
            native("upgrade", [*common, "upgrade"])
        names = ["kedra-refresh-inherited"] if mode == "seed" else ["kedra-refresh-requested"]
        native("install", [*common, "install", *names])
        native("dnf-check", ["dnf", "check"])
        installed = inventory("after")
        fixture_rows = [line.split("\t") for line in installed.splitlines()
                        if line.startswith("kedra-refresh-")]
        expected = {"kedra-refresh-inherited"}
        if mode == "reconcile":
            expected |= {"kedra-refresh-requested", "kedra-refresh-dependency"}
        if {row[0] for row in fixture_rows} != expected or len(fixture_rows) != len(expected):
            raise RuntimeError("native installed fixture closure lost or duplicated a required package")
        selected = EVIDENCE / "selected-rpms"
        selected.mkdir()
        specs = [f"{row[0]}-{row[1]}:{row[2]}-{row[3]}.{row[4]}" for row in fixture_rows]
        # Download the exact installed closure from the same immutable snapshot;
        # native RPM, not a Python resolver, checks identity/content/signatures.
        native("download-selected", [*common, "download", f"--destdir={selected}", *specs])
        retained = sorted(selected.glob("*.rpm"))
        if len(retained) != len(expected):
            raise RuntimeError("native download did not retain the complete fixture closure")
        packages = []
        installed_by_name = {row[0]: "\t".join(row) + "\n" for row in fixture_rows}
        for index, path in enumerate(retained):
            native(f"signature-{index}", ["rpmkeys", "--checksig", "--verbose", str(path)])
            query = native(f"archive-header-{index}", ["rpm", "-qp", "--queryformat", RPM_FORMAT, str(path)])
            row = query.stdout.decode().strip().split("\t")
            if len(row) != 7 or row[0] not in installed_by_name:
                raise RuntimeError("retained RPM identity is outside the installed fixture closure")
            # A same-NEVRA inherited republish that DNF did not reinstall must
            # fail here; matching package names/version text alone is insufficient.
            if query.stdout.decode() != installed_by_name[row[0]]:
                raise RuntimeError("retained RPM header/payload differs from installed package")
            native(f"verify-installed-{index}", ["rpm", "--verify", row[0]])
            native(f"file-inventory-{index}", ["rpm", "-q", "--dump", row[0]])
            payload = Path("/usr/share/kedra-refresh") / row[0]
            packages.append({"name": row[0], "epoch": row[1], "version": row[2],
                             "release": row[3], "architecture": row[4],
                             "header_sha256": row[5], "payload_digest": row[6],
                             "rpm_file": path.name, "rpm_sha256": digest(path),
                             "rpm_size": path.stat().st_size,
                             "installed_file_sha256": digest(payload),
                             "installed_file_mode": oct(payload.stat().st_mode & 0o7777)})
        save("selected-packages.json", packages)
        native("installed-reasons", [*common, "repoquery", "--installed", "--queryformat",
                                     "%{name}\t%{reason}\n", "kedra-refresh-*"])
        native("clean-cache", ["dnf", "clean", "all"])
        receipt["state"] = "succeeded"
        receipt["rpm_fixture_equivalence_evaluable"] = True
    except NativeFailure as error:
        receipt.update(state="failed", failure_step=error.step, native_exit_code=error.code)
        if error.step == "install" and json.loads((FIXTURE / "snapshot.json").read_text())["name"] == "invalid-signature":
            try:
                receipt["rejected_rpm"] = retain_rejected_rpm()
            except (OSError, RuntimeError, ValueError, StopIteration) as evidence_error:
                receipt["rejected_rpm"] = {"evidence_error": str(evidence_error)}
    except (OSError, ValueError, RuntimeError) as error:
        receipt.update(state="failed", failure_step="evidence-validation", error=str(error))
    finally:
        if receipt["state"] == "failed":
            try:
                inventory("after-failure")
            except (NativeFailure, OSError) as error:
                receipt["failure_inventory_error"] = str(error)
        receipt["finished_ns"] = time.time_ns()
        save("outcome.json", receipt)
    return 0 if receipt["state"] == "succeeded" else 1


if __name__ == "__main__":
    sys.exit(main())
