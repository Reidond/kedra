#!/usr/bin/env python3
"""Actions-only end-to-end experiment using real Podman, DNF, RPM and repos."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parents[2]
CONTEXT = ROOT / "tests/refresh"
OUTPUT = ROOT / "output/refresh-evidence"
PODMAN = ["sudo", "podman"]
HOST_COMMANDS = []
CASES = [
    ("baseline", "baseline", "baseline"),
    ("requested-update", "requested-update", "changed"),
    ("transitive-update", "transitive-update", "changed"),
    ("inherited-update", "inherited-update", "changed"),
    ("metadata-only", "metadata-only", "equivalent"),
    ("same-nevra-new-bytes", "same-nevra-new-bytes", "changed"),
    ("repeat-fixed-inputs", "baseline", "equivalent"),
    ("required-repo-unavailable", "required-repo-unavailable", "refused"),
    ("invalid-signature", "invalid-signature", "refused"),
    ("unsatisfiable-newest", "unsatisfiable-newest", "refused"),
]


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def native(label, args, *, checked=True):
    started = time.time_ns()
    print(f"{label}: starting", flush=True)
    with (OUTPUT / f"{label}.log").open("wb") as log:
        result = subprocess.run(args, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=False)
    HOST_COMMANDS.append({"step": label, "argv": args, "exit_code": result.returncode,
                          "started_ns": started, "finished_ns": time.time_ns()})
    write(OUTPUT / "host-commands.json", HOST_COMMANDS)
    print(f"{label}: exit {result.returncode}", flush=True)
    if checked and result.returncode != 0:
        raise RuntimeError(f"{label} exited {result.returncode}; see {label}.log")
    return result.returncode


def image_identity(label, reference):
    native(label, [*PODMAN, "image", "inspect", reference])
    value = read(OUTPUT / f"{label}.log")
    image_id = value[0]["Id"] if len(value) == 1 else ""
    image_hash = image_id.removeprefix("sha256:")
    if len(image_hash) != 64 or any(character not in "0123456789abcdef" for character in image_hash):
        raise RuntimeError("Podman did not provide one immutable image ID")
    return image_id


def build(label, containerfile, argument, snapshot, evidence, tag):
    evidence.mkdir()
    return native(label, [
        *PODMAN, "build", "--layers", "--no-cache", "--pull=never", "--network=none",
        "--volume", f"{snapshot}:/fixture:ro", "--volume", f"{evidence}:/evidence:rw",
        "--build-arg", argument, "--file", str(CONTEXT / containerfile),
        "--tag", tag, str(CONTEXT),
    ], checked=False)


def evaluate(case, expected, exit_code, evidence, seed_id, intent_sha256, baseline, seen):
    result = {"case": case, "expected": expected, "status": "fail",
              "container_build_exit": exit_code, "scope": "RPM fixture only",
              "freshness_written": False, "evidence": str(evidence.relative_to(OUTPUT))}
    receipt_path = evidence / "outcome.json"
    if not receipt_path.is_file():
        result["reason"] = "no new native RUN receipt; build/cache/infrastructure failure"
        return result
    receipt = read(receipt_path)
    result["materialization"] = receipt
    execution = receipt["execution_id"]
    if execution in seen:
        result["reason"] = "reused RUN execution identity"
        return result
    seen.add(execution)
    commands = read(evidence / "commands.json")
    if expected == "refused":
        required_step = "check-upgrade" if case == "required-repo-unavailable" else "install"
        before = evidence / "before.tsv"
        after = evidence / "after-failure.tsv"
        unchanged = before.is_file() and after.is_file() and before.read_bytes() == after.read_bytes()
        failure = next((item for item in commands if item["step"] == required_step), None)
        diagnostic = (evidence / failure["log"]).read_text(encoding="utf-8", errors="replace") if failure else ""
        if case == "required-repo-unavailable":
            cause_matches = ("Curl error (37)" in diagnostic
                             and "file:///fixture/repo/repodata/repomd.xml" in diagnostic)
        elif case == "unsatisfiable-newest":
            cause_matches = ("nothing provides kedra-refresh-dependency >= 99 needed by kedra-refresh-requested-3-1.noarch" in diagnostic)
        else:
            rejected = receipt.get("rejected_rpm", {})
            cause_matches = (rejected.get("recognized_dnf_signature_failure", False)
                             and rejected.get("named_archive_retained", False)
                             and rejected.get("matches_snapshot", False)
                             and rejected.get("rpm_checksig_exit", 0) != 0
                             and rejected.get("native_bad_payload_digest", False))
        refused = (exit_code != 0 and receipt["state"] == "failed"
                   and receipt.get("failure_step") == required_step
                   and receipt.get("native_exit_code", 0) != 0
                   and not receipt["rpm_fixture_equivalence_evaluable"] and unchanged and cause_matches)
        result.update(status="pass" if refused else "fail", classification="refused" if refused else "unexpected",
                      installed_inventory_unchanged=unchanged, native_failure_cause_matches=cause_matches)
        if not refused:
            result["reason"] = "native refusal lacks the specific expected cause, boundary or unchanged inventory"
        return result
    if exit_code != 0 or receipt["state"] != "succeeded" or not receipt["rpm_fixture_equivalence_evaluable"]:
        result["reason"] = "required native materialization/evidence did not complete"
        return result
    successful = {item["step"] for item in commands if item["exit_code"] == 0}
    if not {"upgrade", "install", "dnf-check", "download-selected", "clean-cache"}.issubset(successful):
        result["reason"] = "fresh DNF transaction and evidence are incomplete"
        return result
    expected_probe = 100 if case == "inherited-update" else 0
    if receipt.get("check_upgrade_exit") != expected_probe:
        result["reason"] = "native check-upgrade did not return the expected 0/100 distinction"
        return result
    selected = read(evidence / "selected-packages.json")
    # Source/base and complete installed inventory matter. Retained RPM bytes
    # distinguish republishing even when every name/epoch/version/arch matches.
    # Only this generated fixture is compared; this is not an OS equivalence key.
    comparison = {
        "schema_version": 1, "scope": "RPM fixture only", "seed_image_id": seed_id,
        "source_intent_sha256": intent_sha256,
        "native_inventory_sha256": digest(evidence / "after.tsv"),
        "selected_packages": sorted(selected, key=lambda item: item["name"]),
    }
    write(evidence / "rpm-fixture-comparison.json", comparison)
    encoded = json.dumps(comparison, sort_keys=True, separators=(",", ":")).encode()
    key = hashlib.sha256(encoded).hexdigest()
    metadata = read(evidence / "snapshot.json")["repomd_sha256"]
    result.update(comparison_sha256=key, repomd_sha256=metadata)
    versions = {item["name"]: item["version"] for item in selected}
    expected_versions = {"kedra-refresh-inherited": "1", "kedra-refresh-dependency": "1",
                         "kedra-refresh-requested": "1"}
    changed_package = {"requested-update": "kedra-refresh-requested",
                       "transitive-update": "kedra-refresh-dependency",
                       "inherited-update": "kedra-refresh-inherited"}.get(case)
    if changed_package:
        expected_versions[changed_package] = "2"
    if versions != expected_versions:
        result["reason"] = "actual installed RPM versions differ from the intended native workflow"
        return result
    classification = "baseline" if baseline is None else ("equivalent" if key == baseline["comparison_sha256"] else "changed")
    result["classification"] = classification
    result["status"] = "pass" if classification == expected else "fail"
    if case == "metadata-only" and metadata == baseline["repomd_sha256"]:
        result.update(status="fail", reason="metadata identity did not change")
    if case == "repeat-fixed-inputs" and metadata != baseline["repomd_sha256"]:
        result.update(status="fail", reason="repeated snapshot metadata identity changed")
    if case == "same-nevra-new-bytes":
        baseline_packages = read(OUTPUT / "cases/baseline/selected-packages.json")
        fields = ("name", "epoch", "version", "release", "architecture")
        old_nevra = sorted(tuple(item[field] for field in fields) for item in baseline_packages)
        new_nevra = sorted(tuple(item[field] for field in fields) for item in selected)
        result["nevra_unchanged"] = old_nevra == new_nevra
        old_requested = next(item for item in baseline_packages if item["name"] == "kedra-refresh-requested")
        new_requested = next(item for item in selected if item["name"] == "kedra-refresh-requested")
        result["rpm_bytes_changed"] = old_requested["rpm_sha256"] != new_requested["rpm_sha256"]
        result["installed_payload_changed"] = old_requested["installed_file_sha256"] != new_requested["installed_file_sha256"]
        if (old_nevra != new_nevra or classification != "changed"
                or not result["rpm_bytes_changed"] or not result["installed_payload_changed"]):
            result.update(status="fail", reason="same-NEVRA byte republish was not detected")
    return result


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("GITHUB_REF") != "refs/heads/main":
        raise RuntimeError("OS/container builds are restricted to the development-branch Actions experiment")
    OUTPUT.mkdir(parents=True, exist_ok=False)
    cases = [{"case": name, "status": "not-run", "expected": expected} for name, _, expected in CASES]
    result_document = {"schema_version": 1, "scope": "RPM fixture only", "status": "not-run", "cases": cases}
    write(OUTPUT / "results.json", result_document)
    runner_temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    private = Path(tempfile.mkdtemp(prefix="kedra-refresh-private-", dir=runner_temp)).resolve()
    public = OUTPUT / "repositories"
    public.mkdir()
    try:
        resolved_base = subprocess.check_output(
            [sys.executable, str(ROOT / "tests/resolve-fedora-base.py"),
             "--output", str(OUTPUT / "base-resolution.json")], text=True).strip()
        source_files = ["Containerfile.tools", "Containerfile.seed", "Containerfile.case", "prepare.py", "materialize.py", "run.py"]
        source_hashes = {name: digest(CONTEXT / name) for name in source_files}
        source_intent = {"source_files": source_hashes, "requested": ["kedra-refresh-requested"],
                         "inherited": ["kedra-refresh-inherited"], "remove": []}
        intent_sha256 = hashlib.sha256(json.dumps(source_intent, sort_keys=True).encode()).hexdigest()
        write(OUTPUT / "environment.json", {
            "schema_version": 1, "source_revision": os.environ["GITHUB_SHA"],
            "run_id": os.environ["GITHUB_RUN_ID"], "run_attempt": os.environ["GITHUB_RUN_ATTEMPT"],
            "run_url": f"https://github.com/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}",
            "fedora_base": resolved_base, "source_intent": source_intent,
            "source_intent_sha256": intent_sha256,
            "no_production_signing_or_publication": True,
        })
        native("runner-kernel", ["uname", "-a"])
        native("podman-version", [*PODMAN, "version"])
        native("runner-storage", ["df", "-h"])
        native("tools-build", [*PODMAN, "build", "--no-cache", "--pull=always", "--build-arg",
                               f"BASE_IMAGE={resolved_base}", "--file", str(CONTEXT / "Containerfile.tools"),
                               "--tag", "localhost/kedra-refresh-tools:research", str(CONTEXT)])
        tool_id = image_identity("tools-image", "localhost/kedra-refresh-tools:research")
        image_identity("fedora-base-image", resolved_base)
        native("native-tool-packages", [*PODMAN, "run", "--rm", "--network=none", tool_id,
                                        "rpm", "-qa", "--queryformat", "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\n"])
        native("prepare-snapshots", [*PODMAN, "run", "--rm", "--network=none", "--env", "GITHUB_ACTIONS=true",
                                     "--volume", f"{private}:/private:rw", "--volume", f"{public}:/public:rw",
                                     tool_id, "python3", "/opt/kedra-refresh/prepare.py"])
        native("public-fixture-ownership", ["sudo", "chown", "-R", f"{os.getuid()}:{os.getgid()}", str(public)])
        seed_evidence = OUTPUT / "seed"
        baseline_snapshot = public / "snapshots/baseline"
        if build("seed-build", "Containerfile.seed", f"TOOL_IMAGE={tool_id}", baseline_snapshot,
                 seed_evidence, "localhost/kedra-refresh-seed:research") != 0:
            raise RuntimeError("native inherited-RPM seed build failed")
        seed_id = image_identity("seed-image", "localhost/kedra-refresh-seed:research")
        environment = read(OUTPUT / "environment.json")
        environment.update(tool_image_id=tool_id, fixture_base_image_id=seed_id)
        write(OUTPUT / "environment.json", environment)
        (OUTPUT / "cases").mkdir()
        baseline = None
        seen = {read(seed_evidence / "outcome.json")["execution_id"]}
        for index, (case, snapshot, expected) in enumerate(CASES):
            evidence = OUTPUT / "cases" / case
            tag = f"localhost/kedra-refresh-case:{case}"
            code = build(f"case-{case}", "Containerfile.case", f"FIXTURE_BASE={seed_id}",
                         public / "snapshots" / snapshot, evidence, tag)
            result = evaluate(case, expected, code, evidence, seed_id, intent_sha256, baseline, seen)
            if code == 0:
                result["image_id"] = image_identity(f"image-{case}", tag)
            if case == "baseline":
                if result["status"] != "pass":
                    cases[index] = result
                    raise RuntimeError("baseline did not qualify; dependent cases remain not-run")
                baseline = result
            cases[index] = result
            write(OUTPUT / "results.json", result_document)
        result_document["status"] = "pass" if all(case["status"] == "pass" for case in cases) else "fail"
    except (OSError, ValueError, RuntimeError, KeyError) as error:
        result_document.update(status="fail", infrastructure_error=str(error))
        print(str(error), file=sys.stderr, flush=True)
    finally:
        write(OUTPUT / "results.json", result_document)
        # This exact private directory was created above, is never mounted in a
        # build and is outside the sole uploaded artifact path.
        if private.parent != runner_temp or not private.name.startswith("kedra-refresh-private-"):
            raise RuntimeError("refusing private cleanup outside the generated runner directory")
        native("private-cleanup", ["sudo", "rm", "-rf", "--", str(private)])
        native("evidence-ownership", ["sudo", "chown", "-R", f"{os.getuid()}:{os.getgid()}", str(OUTPUT)])
    print(json.dumps(result_document, indent=2), flush=True)
    return 0 if result_document["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
