#!/usr/bin/env python3
"""Materialize the actual desktop twice in Actions and retain native evidence.

This is a release-input experiment, not a source/layout test runner or no-change
authority. It does not sign, publish, renew freshness, or compare a prior release.
"""

import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import time

import target_image


ROOT = Path(__file__).resolve().parents[2]
RESEARCH = ROOT / "tests/refresh"
OUTPUT = ROOT / "output/target-refresh-evidence"
CONTEXT = ROOT / "output/target-refresh-context"
TRUST = ROOT / "output/target-refresh-trust"
PODMAN = ["sudo", "podman"]
RPM_FORMAT = "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\t%{VENDOR}\t%{SOURCERPM}\n"
RPM_FIELDS = ("name", "epoch", "version", "release", "architecture", "header_sha256",
              "payload_sha256", "vendor", "source_rpm")
RECIPES = (
    "Containerfile", "build/assemble.sh", "build/release/Containerfile",
    "build/release/prepare-trust.py", "build/agents/prepare.sh", "build/agents/fetch.py",
    "build/agents/package.py", "build/bitwarden/prepare.py",
    "tests/refresh/Containerfile.target", "tests/refresh/target-native.sh",
    "tests/refresh/target-dnf.sh", "tests/refresh/target.py",
    "tests/refresh/target-filesystem.py", "tests/refresh/target_image.py",
)
BUILD_CONFIGURATION = (
    "Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "crates/sysroot/Cargo.toml",
    "crates/sysroot-core/Cargo.toml", "crates/sysroot-helper/Cargo.toml",
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8")


def digest(path):
    require(path.is_file() and not path.is_symlink(), f"Expected regular evidence/input file: {path.name}")
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(path):
    return {"sha256": digest(path), "size_bytes": path.stat().st_size,
            "mode": oct(stat.S_IMODE(path.stat().st_mode))}


def canonical_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def native(label, args, *, checked=True, timeout=None):
    """Retain actual argv, outputs and exit status; never turn an error into equality."""
    directory = OUTPUT / "commands" / label
    directory.mkdir()
    command = [str(arg) for arg in args]
    if timeout is not None:
        require(timeout > 0, "Native command deadline has expired")
        limit = ["timeout", "--signal=TERM", "--kill-after=10s", str(math.ceil(timeout)) + "s"]
        # Let the standard deadline tool run with the same privilege as its
        # command; an ordinary parent cannot reliably signal a sudo child tree.
        command = ["sudo", *limit, *command[1:]] if command[0] == "sudo" else [*limit, *command]
    write(directory / "argv.json", command)
    started = time.time_ns()
    print(f"{label}: starting", flush=True)
    with (directory / "stdout").open("wb") as stdout, (directory / "stderr").open("wb") as stderr:
        result = subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, check=False)
        code = result.returncode
    write(directory / "result.json", {"started_ns": started, "finished_ns": time.time_ns(),
                                       "exit_code": code, "native_timeout_seconds": timeout})
    print(f"{label}: exit {code}", flush=True)
    require(not checked or code == 0, f"Native {label} exited {code}; see retained command evidence")
    return directory, code


def image_identity(label, reference):
    directory, _ = native(label, [*PODMAN, "image", "inspect", reference])
    images = read(directory / "stdout")
    require(len(images) == 1, "Expected exactly one native image inspection")
    image_id = images[0]["Id"]
    require(re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}", image_id), "Missing immutable native image ID")
    return image_id


def resolve_base():
    pins = read(ROOT / "build/inputs.json")
    reference = pins["base"]
    require(re.fullmatch(r"quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}", reference), "Expected reviewed Fedora base pin")
    directory, _ = native("base-pinned-manifest", ["skopeo", "inspect", "--raw", f"docker://{reference}"])
    pinned_hash = digest(directory / "stdout")
    require(reference.endswith("@sha256:" + pinned_hash), "Registry manifest differs from the reviewed base pin")
    manifest = read(directory / "stdout")
    index_digest = None
    platform_reference = reference
    if "manifests" in manifest:
        choices = [item for item in manifest["manifests"]
                   if item.get("platform", {}).get("os") == "linux"
                   and item.get("platform", {}).get("architecture") == "amd64"
                   and item.get("platform", {}).get("variant", "") in ("", "v1")]
        require(len(choices) == 1 and re.fullmatch(r"sha256:[a-f0-9]{64}", choices[0]["digest"]),
                "Base index does not select exactly one Linux AMD64 manifest")
        index_digest = "sha256:" + pinned_hash
        platform_reference = reference.split("@")[0] + "@" + choices[0]["digest"]
    directory, _ = native("base-platform-manifest", ["skopeo", "inspect", "--raw", f"docker://{platform_reference}"])
    require(platform_reference.endswith("@sha256:" + digest(directory / "stdout")), "Base platform manifest digest changed")
    require("manifests" not in read(directory / "stdout"), "Selected platform is still an index")
    directory, _ = native("base-platform-config", ["skopeo", "inspect", "--config", f"docker://{platform_reference}"])
    config = read(directory / "stdout")
    require(config.get("architecture") == "amd64" and config.get("os") == "linux", "Base architecture differs from target")
    native("base-pull", [*PODMAN, "pull", "--arch=amd64", platform_reference])
    image_id = image_identity("base-image", platform_reference)
    directory, _ = native("base-inventory", [*PODMAN, "run", "--rm", "--network=none", platform_reference,
                                           "rpm", "-qa", "--queryformat", RPM_FORMAT])
    shutil.copyfile(directory / "stdout", OUTPUT / "base-inventory.tsv")
    record = {"pinned_reference": reference, "index_digest": index_digest,
              "platform_reference": platform_reference, "native_image_id": image_id,
              "moving_tag_discovery": "not-run", "upstream_signature_qualification": "not-run"}
    write(OUTPUT / "base.json", record)
    return record


def payload_projection(plan, path):
    """Validate actual CLI archive bytes; keep full provenance separate from intent."""
    require(plan.get("schema_version") == 1 and plan.get("input_scope") == "committed HEAD only",
            "Expected the actual committed source-plan CLI output")
    require(plan["target"]["id"] == "desktop" and plan["target"]["architecture"] == "x86_64"
            and plan["target"]["fedora_release"] == 44, "Source plan target differs")
    expected = {item["destination"]: item for item in plan["files"]}
    require(len(expected) == len(plan["files"]), "Source plan repeats a payload destination")
    observed, manifest_hash = set(), None
    with tarfile.open(path, mode="r|") as archive:
        for member in archive:
            require(member.isfile() and member.name not in observed, "Source archive contains a duplicate or non-file entry")
            require(member.uid == 0 and member.gid == 0 and member.mtime == 0,
                    "Source archive ownership/timestamp contract differs")
            observed.add(member.name)
            with archive.extractfile(member) as stream:
                if member.name == "usr/share/sysroot/source.json":
                    require(member.size <= 1_048_576 and member.mode == 0o644, "Source manifest exceeds its contract")
                    data = stream.read()
                    require(json.loads(data) == plan, "Source plan/archive came from different committed inputs")
                    manifest_hash = hashlib.sha256(data).hexdigest()
                else:
                    require(member.name in expected, "Source archive contains an unplanned file")
                    item = expected[member.name]
                    require(member.mode == {"100644": 0o644, "100755": 0o755}[item["mode"]]
                            and hashlib.file_digest(stream, "sha256").hexdigest() == item["sha256"],
                            "Actual archive file differs from selected source content/mode")
    require(observed == set(expected) | {"usr/share/sysroot/source.json"} and manifest_hash,
            "Source archive is incomplete")
    # source_path/replaces/home_baseline remain: installed home workflows use the
    # mapping. Only the checkout identity/read scope are separated from intent.
    projection = {key: value for key, value in plan.items() if key not in ("source_revision", "input_scope")}
    return projection, manifest_hash


def expected_files(plan, manifest_hash):
    """Inventory the actual declared non-RPM artifacts and reviewed assembly edits."""
    files = {item["destination"]: {"sha256": item["sha256"],
             "mode": {"100644": 0o644, "100755": 0o755}[item["mode"]]} for item in plan["files"]}
    for binary, destination in (("sysroot", "usr/bin/sysroot"), ("sysroot-helper", "usr/libexec/sysroot/helper")):
        require(destination not in files, "Source payload shadows a compiled sysroot program")
        files[destination] = {"sha256": digest(CONTEXT / binary), "mode": 0o755}
    for filename in ("agents.tar", "bitwarden.tar"):
        with tarfile.open(CONTEXT / filename, mode="r|") as archive:
            for member in archive:
                require(member.isfile() and member.name not in files and not member.name.startswith("/")
                        and all(part not in ("", ".", "..") for part in member.name.split("/"))
                        and not any(ord(character) < 32 for character in member.name),
                        "Generated external payload has an unsupported path/type/collision")
                require(member.uid == 0 and member.gid == 0 and member.mode in (0o644, 0o755),
                        "Generated external payload ownership/mode differs")
                with archive.extractfile(member) as stream:
                    files[member.name] = {"sha256": hashlib.file_digest(stream, "sha256").hexdigest(), "mode": member.mode}
    for item in plan["files"]:
        if item["home_baseline"]:
            prefix = "usr/share/sysroot/home/default/"
            require(item["destination"].startswith(prefix), "Unknown home baseline destination")
            files["etc/skel/" + item["destination"].removeprefix(prefix)] = dict(files[item["destination"]])
    files["usr/share/sysroot/source.json"] = {"sha256": manifest_hash, "mode": 0o644}
    # These exact chmod and trust-copy effects are part of the existing assembly
    # recipes, whose hashes are retained. No inferred general normalization.
    files["usr/libexec/kedra-session"]["mode"] = 0o755
    destinations = {"release.pub": "usr/lib/sysroot/trust/release.pub",
                    "release-policy.json": "usr/lib/sysroot/trust/release-policy.json",
                    "policy.json": "etc/containers/policy.json",
                    "registries.yaml": "etc/containers/registries.d/kedra.yaml",
                    "install.toml": "usr/lib/bootc/install/10-kedra.toml"}
    for name, destination in destinations.items():
        files[destination] = {"sha256": digest(TRUST / "trust" / name), "mode": 0o644}
    return files


def prepare_inputs():
    CONTEXT.mkdir(parents=True, exist_ok=False)
    TRUST.mkdir(parents=True, exist_ok=False)
    sysroot = ROOT / "target/release/sysroot"
    directory, _ = native("source-plan", [sysroot, "source", "plan", "--host", "desktop", "--json"])
    plan = read(directory / "stdout")
    require(plan["source_revision"] == os.environ["GITHUB_SHA"], "Source CLI did not describe the dispatched commit")
    shutil.copyfile(directory / "stdout", OUTPUT / "source-plan.json")
    native("source-archive", [sysroot, "source", "archive", "--host", "desktop", "--output", CONTEXT / "payload.tar"])
    for source, name in ((sysroot, "sysroot"), (ROOT / "target/release/sysroot-helper", "sysroot-helper"),
                         (ROOT / "Containerfile", "Containerfile"), (ROOT / "build/assemble.sh", "assemble.sh"),
                         (RESEARCH / "target-native.sh", "target-native.sh"), (RESEARCH / "target-dnf.sh", "target-dnf.sh")):
        shutil.copyfile(source, CONTEXT / name)
    # The real public preparation code verifies pins. No owner runtime/profile or
    # production signing material is mounted or forwarded to this experiment.
    downloads = Path(os.environ["RUNNER_TEMP"]) / "kedra-target-refresh-agent-inputs"
    native("prepare-agents", ["bash", "build/agents/prepare.sh", CONTEXT, downloads, OUTPUT / "agent-inputs.json"])
    native("prepare-bitwarden", ["python3", "build/bitwarden/prepare.py", "--context", CONTEXT,
                                 "--evidence", OUTPUT / "bitwarden-inputs.json"])
    native("prepare-public-trust", ["python3", "build/release/prepare-trust.py", "--source", OUTPUT / "source-plan.json",
                                    "--public-key", "build/release/authority/desktop.pub", "--expected-fingerprint",
                                    (ROOT / "build/release/authority/desktop.sha256").read_text().strip(),
                                    "--sysroot", sysroot, "--output", TRUST / "trust"])
    projection, manifest_hash = payload_projection(plan, CONTEXT / "payload.tar")
    context_identity = {name: identity(CONTEXT / name) for name in (
        "sysroot", "sysroot-helper", "payload.tar", "agents.tar", "bitwarden.tar", "Containerfile",
        "assemble.sh", "target-native.sh", "target-dnf.sh")}
    # COPY modes are set to the same executable contract by assemble.sh. Source
    # context owner/mode is provenance; actual output bytes are always compared.
    binaries = {name: {"sha256": context_identity[name]["sha256"], "mode": "0o755"}
                for name in ("sysroot", "sysroot-helper")}
    trust = {path.name: {"sha256": digest(path), "mode": "0o644"} for path in sorted((TRUST / "trust").iterdir())}
    material = {"schema_version": 1, "source_intent": projection, "compiled_binaries": binaries,
                "external_payloads": {name: context_identity[name]["sha256"] for name in ("agents.tar", "bitwarden.tar")},
                "public_trust": trust, "recipes": {name: digest(ROOT / name) for name in RECIPES},
                "build_configuration": {name: digest(ROOT / name) for name in BUILD_CONFIGURATION},
                "builder_tool_records": {name: digest(OUTPUT / "commands" / name / "stdout") for name in
                                         ("podman-version", "skopeo-version", "rustc-version", "cargo-version")}}
    write(OUTPUT / "material-intent.json", material)
    write(OUTPUT / "input-provenance.json", {"schema_version": 1, "source_revision": plan["source_revision"],
          "run_id": os.environ["GITHUB_RUN_ID"], "run_attempt": os.environ["GITHUB_RUN_ATTEMPT"],
          "source_plan_sha256": digest(OUTPUT / "source-plan.json"), "installed_manifest_sha256": manifest_hash,
          "generated_context": context_identity, "material_intent_sha256": canonical_hash(material),
          "agent_input_record_sha256": digest(OUTPUT / "agent-inputs.json"),
          "bitwarden_input_record_sha256": digest(OUTPUT / "bitwarden-inputs.json"),
          "comparison_to_promoted_release": "not-run; no earlier evidence is backfilled"})
    return plan, material, manifest_hash, expected_files(plan, manifest_hash)


def rpm_rows(path):
    packages, keys = {}, []
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split("\t")
        require(len(fields) == len(RPM_FIELDS), "Native RPM inventory has an unsupported row")
        row = dict(zip(RPM_FIELDS, fields))
        if row["name"] == "gpg-pubkey":
            keys.append(row)
            continue
        require(row["epoch"].isdigit() and row["architecture"] in ("x86_64", "noarch", "i686"),
                "Native RPM scope/epoch differs from desktop")
        require(all(re.fullmatch(r"[a-f0-9]{64}", row[field]) for field in ("header_sha256", "payload_sha256")),
                "Native RPM row lacks complete header/payload identity")
        nevra = f'{row["name"]}-{row["epoch"]}:{row["version"]}-{row["release"]}.{row["architecture"]}'
        require(nevra not in packages, "Native RPM inventory repeats a full NEVRA")
        packages[nevra] = row
    require(packages, "Native RPM inventory is empty")
    return packages, sorted(keys, key=canonical_hash)


def command_output(evidence, label):
    directory = evidence / "commands" / label
    require(read(directory / "result.json")["exit_code"] == 0, f"Native evidence command failed: {label}")
    return directory / "stdout"


def resolution_record(evidence, base, plan, material, manifest_hash):
    outcome = read(evidence / "outcome.json")
    require(outcome["state"] == "succeeded" and outcome["process_exit_code"] == 0, "Native assembly did not complete")
    require(digest(evidence / "source.before.json") == manifest_hash
            and digest(evidence / "source.after.json") == manifest_hash, "Assembler changed immutable source provenance")
    initial, _ = rpm_rows(evidence / "initial.tsv")
    pinned, _ = rpm_rows(OUTPUT / "base-inventory.tsv")
    require(initial == pinned, "Pre-transaction inventory differs from the selected pinned base")
    final, key_rows = rpm_rows(evidence / "final.tsv")
    requested, removed = set(plan["packages"]), set(plan["remove_packages"])
    names = {row["name"] for row in final.values()}
    require(requested <= names and not (removed & names), "Final native inventory does not satisfy direct install/removal intent")
    for operation in ("upgrade", "install", "clean", "check"):
        command_output(evidence, operation)
    if removed:
        command_output(evidence, "remove")
    repositories = read(command_output(evidence, "final-repos"))
    require(len(repositories) == 2 and {repo["id"] for repo in repositories} == {"fedora", "updates"},
            "Native enabled repository set differs from the reviewed allowlist")
    require(all(repo["is_enabled"] and not repo["skip_if_unavailable"] and repo["pkg_gpgcheck"]
                and repo["gpg_key"] for repo in repositories), "Required repository/package-verification evidence differs")
    reasons = {}
    for line in command_output(evidence, "final-reasons").read_text().splitlines():
        fields = line.split("\t")
        require(len(fields) == 3 and fields[0] not in reasons, f"Unsupported native installed reason row: {line[:300]!r}")
        reasons[fields[0]] = {"from_repo": fields[1], "reason": fields[2]}
    archive_rows = {}
    origins = {}
    origin_file = evidence / "archive-origins.jsonl"
    for line in origin_file.read_text().splitlines() if origin_file.exists() else ():
        item = json.loads(line)
        origins.setdefault(item["sha256"], []).append({"step": item["step"], "cache_path": item["cache_path"]})
    for archive_hash, locations in sorted(origins.items()):
        archive = evidence / "rpms" / f"{archive_hash}.rpm"
        require(digest(archive) == archive_hash, "Retained native RPM bytes changed")
        command_output(evidence, "signature-" + archive_hash)
        rows, _ = rpm_rows(command_output(evidence, "header-" + archive_hash))
        require(len(rows) == 1, "An RPM archive must describe one package")
        nevra, row = next(iter(rows.items()))
        require(all(re.match(r"(?:fedora|updates)-[^/]+/", location["cache_path"]) for location in locations),
                "Transaction archive cache is outside the native repository allowlist")
        archive_rows.setdefault(nevra, []).append({"identity": row, "sha256": archive_hash,
                                                 "size_bytes": archive.stat().st_size, "origins": locations})
    accounted = []
    for nevra, row in sorted(final.items()):
        native_reason = reasons.get(nevra)
        require(native_reason is not None, "Final package lacks native repository/reason evidence")
        matches = [archive for archive in archive_rows.get(nevra, []) if archive["identity"] == row]
        if matches:
            require(native_reason["from_repo"] in ("fedora", "updates"), "Changed package origin is not an allowed Fedora repo")
            source = {"kind": "verified-transaction-archive",
                      "rpm_sha256": sorted(archive["sha256"] for archive in matches)}
        else:
            require(pinned.get(nevra) == row, "Final package has neither matching verified archive nor exact base inheritance")
            source = {"kind": "unchanged-pinned-base", "platform_reference": base["platform_reference"],
                      "current_repo_same_nevra_republication": "not-qualified"}
        accounted.append({"nevra": nevra, **row, **native_reason, "content_source": source})
    metadata = {}
    for step in ("upgrade", "install", "remove", "before-clean"):
        directory = evidence / "metadata" / step
        if not directory.is_dir():
            continue
        files = {path.relative_to(directory).as_posix(): digest(path) for path in sorted(directory.rglob("*")) if path.is_file()}
        repomd = [name for name in files if name.endswith("/repodata/repomd.xml")]
        require(len(repomd) == 2 and all(any(name.startswith(repo + "-") for name in repomd) for repo in ("fedora", "updates")),
                "Each materialized transaction must retain both required repository metadata identities")
        metadata[step] = files
    require({"upgrade", "install", "before-clean"} <= metadata.keys(), "Fresh transaction metadata evidence is incomplete")
    # Repository timestamps and full checkout provenance remain in this record,
    # but they do not alone decide material-input identity.
    comparable = {"schema_version": 1, "material_intent_sha256": canonical_hash(material),
                  "base_platform_reference": base["platform_reference"], "installed_packages": accounted,
                  "rpm_public_keys_sha256": digest(command_output(evidence, "final-public-keys"))}
    record = {"schema_version": 1, "scope": "desktop-resolution-research", "state": "recorded",
              "source_revision": plan["source_revision"], "execution": outcome,
              "base": base, "installed_packages": accounted, "rpmdb_public_key_rows": key_rows,
              "transaction_archives": archive_rows, "repositories": repositories, "metadata": metadata,
              "removed_base_nevras": sorted(pinned.keys() - final.keys()),
              "native_operations": {"upgrade": "pass", "install": "pass", "check": "pass",
                                    "remove": "pass" if removed else "not-run: empty explicit removal intent"},
              "material_comparison": comparable, "material_comparison_sha256": canonical_hash(comparable),
              "whole_image_equivalence": "unproven", "freshness_written": False,
              "limitations": ["No previous promoted-release comparison or backfill",
                              "Unchanged base RPM archives are not re-resolved from current mirrors",
                              "Final filesystem/config/xattr normalization is not qualified",
                              "Lower EVR/vendor/hold/obsolete policy qualification remains open",
                              "Base signature qualification and moving-tag discovery remain open",
                              "No boot, ISO, signing or checkpoint operation"]}
    write(evidence / "resolution.json", record)
    return record


def verify_final_payload(name, tag, evidence, expected, record):
    write(evidence / "expected-files.json", expected)
    directory, _ = native("payload-readback-" + name, [*PODMAN, "run", "--rm", "--network=none",
                          "--volume", f"{evidence}:/evidence:ro", "--volume", f"{RESEARCH / 'target-native.sh'}:/observer.sh:ro",
                          tag, "/bin/bash", "/observer.sh", "inspect"])
    files, links = {}, {}
    for line in (directory / "stdout").read_text().splitlines():
        item = json.loads(line)
        path = item.pop("path")
        require(path not in files and path not in links, "Repeated final payload path")
        if item["kind"] == "file":
            require(path in expected and item["sha256"] == expected[path]["sha256"]
                    and int(item["mode"], 8) == expected[path]["mode"] and item["uid"] == 0 and item["gid"] == 0,
                    "Final image file differs from declared material content/mode/ownership")
            files[path] = item
        else:
            require(item["kind"] == "symlink", "Unknown final payload type")
            links[path] = item
    require(files.keys() == expected.keys(), "Final declared non-RPM payload readback is incomplete")
    for unit_type in ("timer", "service"):
        require(links[f"etc/systemd/system/bootc-fetch-apply-updates.{unit_type}"]["target"] == "/dev/null",
                "Inherited automatic update unit is not masked in the final image")
    directory, _ = native("final-rpm-readback-" + name, [*PODMAN, "run", "--rm", "--network=none", tag,
                                                      "rpm", "-qa", "--queryformat", RPM_FORMAT])
    require(rpm_rows(directory / "stdout") == rpm_rows(evidence / "final.tsv"),
            "Public trust derivative changed the recorded final RPM inventory")
    readback = {"files": files, "generated_links": links}
    write(evidence / "declared-payload-readback.json", readback)
    # The manifest remains fully verified and retained above; separate its source
    # identity from material comparison without rewriting installed provenance.
    compared = {path: item for path, item in files.items() if path != "usr/share/sysroot/source.json"}
    record["material_comparison"]["declared_payload_readback"] = {"files": compared, "generated_links": links}
    record["material_comparison_sha256"] = canonical_hash(record["material_comparison"])
    record["declared_payload_readback_sha256"] = digest(evidence / "declared-payload-readback.json")
    write(evidence / "resolution.json", record)


def main():
    require(os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("GITHUB_REPOSITORY") == "Reidond/kedra"
            and os.environ.get("GITHUB_REF") == "refs/heads/main",
            "Only the development-branch Actions desktop-resolution experiment may build images")
    OUTPUT.mkdir(parents=True, exist_ok=False)
    (OUTPUT / "commands").mkdir()
    results = {"schema_version": 1, "status": "not-run", "scope": "desktop-resolution-research",
               "cases": [], "whole_image_equivalence": "unproven", "freshness_written": False}
    write(OUTPUT / "results.json", results)
    observation_deadline = time.monotonic() + 60 * 60
    try:
        for label, command in (("podman-version", [*PODMAN, "version"]), ("skopeo-version", ["skopeo", "--version"]),
                               ("rustc-version", ["rustc", "-Vv"]), ("cargo-version", ["cargo", "-V"]),
                               ("getfattr-version", ["getfattr", "--version"]),
                               ("timeout-version", ["timeout", "--version"]),
                               ("runner-kernel", ["uname", "-a"]), ("runner-disk", ["df", "-h"])):
            native(label, command)
        base = resolve_base()
        plan, material, manifest_hash, expected = prepare_inputs()
        records, images, executions = [], [], set()
        for name in ("baseline", "repeat-fresh"):
            evidence = OUTPUT / name
            evidence.mkdir()
            write(evidence / "request.json", {"schema_version": 1, "scope": "desktop-resolution-research",
                  "source_revision": plan["source_revision"], "base_platform_reference": base["platform_reference"],
                  "material_intent_sha256": canonical_hash(material), "freshness_written": False})
            tag = "localhost/kedra-target-refresh:" + name
            _, code = native("build-" + name, [*PODMAN, "build", "--layers", "--no-cache", "--pull=never",
                "--volume", f"{evidence}:/evidence:rw", "--build-arg", "BASE_IMAGE=" + base["platform_reference"],
                "--file", RESEARCH / "Containerfile.target", "--tag", tag, CONTEXT], checked=False)
            native("ownership-" + name, ["sudo", "chown", "-R", f"{os.getuid()}:{os.getgid()}", evidence])
            case = {"case": name, "container_build_exit": code, "status": "fail", "freshness_written": False}
            results["cases"].append(case)
            write(OUTPUT / "results.json", results)
            require(code == 0, f"Actual desktop build {name} failed; native evidence is retained")
            record = resolution_record(evidence, base, plan, material, manifest_hash)
            execution = record["execution"]["execution_id"]
            require(execution not in executions, "A repeated execution reused an earlier native receipt")
            executions.add(execution)
            # Same public trust derivative as release.yml; no production signer.
            native("trust-" + name, [*PODMAN, "build", "--no-cache", "--pull=never", "--build-arg", "CANDIDATE_IMAGE=" + tag,
                                    "--file", ROOT / "build/release/Containerfile", "--tag", tag + "-trust", TRUST])
            verify_final_payload(name, tag + "-trust", evidence, expected, record)
            image_id = image_identity("image-" + name, tag + "-trust")
            case.update(native_assembly_status="pass", native_execution_id=execution, native_image_id=image_id,
                        material_comparison_sha256=record["material_comparison_sha256"],
                        whole_image_equivalence="unproven")
            image = {"oci": target_image.capture_oci(name, image_id, evidence, native),
                     "filesystem": target_image.capture_filesystem(name, image_id, evidence, RESEARCH / "target-filesystem.py",
                                                                   native, observation_deadline)}
            case.update(status="recorded", complete_final_image_observation=True, image_unmounted=True)
            records.append(record)
            images.append(image)
            write(OUTPUT / "results.json", results)
        equal = records[0]["material_comparison"] == records[1]["material_comparison"]
        results.update(status="recorded", classification="material-inputs-equal/equivalence-unproven" if equal else "material-inputs-changed",
                       repeated_native_execution=True,
                       repository_metadata_equal=records[0]["metadata"] == records[1]["metadata"],
                       final_image_observation=target_image.compare_pair(OUTPUT, images),
                       no_change_release_decision=False)
    except (OSError, ValueError, RuntimeError, KeyError, tarfile.TarError) as error:
        results.update(status="fail", error=str(error), classification="resolution-or-evidence-failed",
                       no_change_release_decision=False)
    finally:
        write(OUTPUT / "results.json", results)
    print(json.dumps(results, indent=2), flush=True)
    return 0 if results["status"] == "recorded" else 1


if __name__ == "__main__":
    sys.exit(main())
