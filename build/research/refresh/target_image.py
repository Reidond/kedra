"""Native image-only filesystem/OCI observation; no normalization or release policy."""

import base64
import gzip
import hashlib
import json
from pathlib import PurePosixPath
import re
import subprocess
import time
import uuid


MAX_METADATA_BYTES = 128 * 1024**2


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8")


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def one_absolute_path(path):
    lines = path.read_text().splitlines()
    require(len(lines) == 1 and lines[0].startswith("/") and not any(character in lines[0] for character in ",\\\0\r\n"),
            "Native image path is missing, ambiguous or unsupported")
    result = PurePosixPath(lines[0])
    require(".." not in result.parts and str(result) == lines[0], "Native image path is not canonical")
    return result


def parse_xattrs(path, root, entries):
    """Map native getfattr records, retaining raw output and refusing ambiguity.

    Native escaped filenames/attribute names are deliberately unsupported in this
    first recorder: the raw dump remains evidence and the case fails rather than
    interpreting an escape incorrectly or assigning an xattr to the wrong path.
    """
    require(path.stat().st_size <= MAX_METADATA_BYTES, "Native xattr dump exceeds the complete metadata budget")
    prefix = str(root).encode("utf-8")
    result, current = {}, None
    for line in path.read_bytes().split(b"\n"):
        if not line:
            current = None
            continue
        if line.startswith(b"# file: "):
            name = line[len(b"# file: "):]
            require(b"\\" not in name and (name == prefix or name.startswith(prefix + b"/")),
                    f"Native xattr path is escaped or outside the validated image mount: {name[:300]!r}; raw dump retained")
            relative = name[len(prefix):].removeprefix(b"/")
            require(not relative.endswith(b"/") and all(part not in (b".", b"..") for part in relative.split(b"/")),
                    "Unsupported native xattr path notation; raw dump retained")
            current = base64.b64encode(relative).decode("ascii")
            require(current in entries and current not in result, "Native xattr path is missing or repeated in the image inventory")
            result[current] = {}
            continue
        require(current is not None and b"=" in line, "Unrecognized native xattr dump line; raw dump retained")
        name, value = line.split(b"=", 1)
        require(re.fullmatch(rb"[A-Za-z0-9_.:-]+", name) and value.startswith(b"0s"),
                f"Unsupported native xattr name/encoding: {name[:200]!r}; raw dump retained")
        decoded = base64.b64decode(value[2:], validate=True)
        require(base64.b64encode(decoded) == value[2:], "Noncanonical native xattr encoding")
        encoded_name = base64.b64encode(name).decode("ascii")
        require(encoded_name not in result[current], "Duplicate native xattr name")
        result[current][encoded_name] = {"name_display": name.decode("ascii"), "value_base64": value[2:].decode("ascii")}
    return {key: result.get(key, {}) for key in entries}


def parse_filesystem(path):
    require(path.stat().st_size <= MAX_METADATA_BYTES, "Filesystem dump exceeds the complete metadata budget")
    entries, header, summary = {}, None, None
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            kind = row.pop("record")
            if kind == "header":
                require(header is None and not entries and summary is None, "Misordered filesystem header")
                header = row
            elif kind == "entry":
                require(header is not None and summary is None, "Misordered filesystem entry")
                key = row["path_base64"]
                decoded = base64.b64decode(key, validate=True)
                require(base64.b64encode(decoded).decode("ascii") == key and key not in entries,
                        "Repeated or malformed filesystem path")
                entries[key] = row
            elif kind == "summary":
                require(header is not None and summary is None, "Misordered filesystem summary")
                summary = row
            else:
                raise RuntimeError("Unknown filesystem record")
    require(header is not None and summary is not None and summary["complete"] is True
            and summary["entry_count"] == len(entries) and "" in entries,
            "Mounted image filesystem observation is incomplete")
    return {"header": header, "entries": entries, "summary": summary}


def capture_oci(name, image_id, evidence, native):
    reference = "containers-storage:" + image_id
    manifest_dir, _ = native("oci-manifest-" + name, ["sudo", "skopeo", "inspect", "--raw", reference], timeout=60)
    config_dir, _ = native("oci-config-" + name, ["sudo", "skopeo", "inspect", "--config", "--raw", reference], timeout=60)
    require((manifest_dir / "stdout").stat().st_size <= 16 * 1024**2
            and (config_dir / "stdout").stat().st_size <= 16 * 1024**2, "Raw OCI observation exceeds its metadata budget")
    manifest, config = read(manifest_dir / "stdout"), read(config_dir / "stdout")
    require(manifest.get("schemaVersion") == 2 and "manifests" not in manifest,
            "Expected the actual single-platform OCI/Docker image manifest")
    require(manifest["config"]["digest"] == "sha256:" + digest(config_dir / "stdout")
            and manifest["config"]["size"] == (config_dir / "stdout").stat().st_size,
            "Raw image configuration does not match its native manifest descriptor")
    require(image_id.removeprefix("sha256:") == digest(config_dir / "stdout"),
            "Raw image configuration differs from the exact built image ID")
    require(config.get("architecture") == "amd64" and config.get("os") == "linux" and isinstance(config.get("config"), dict),
            "Unexpected raw image platform/runtime configuration")
    platform_keys = ("architecture", "os", "os.version", "os.features", "variant")
    record = {"schema_version": 1, "native_image_id": image_id,
              "manifest_sha256": digest(manifest_dir / "stdout"), "config_sha256": digest(config_dir / "stdout"),
              "runtime_config": config["config"],
              "platform_config": {key: config[key] for key in platform_keys if key in config},
              "other_config_fields": {key: value for key, value in config.items() if key not in ("config", *platform_keys)},
              "manifest": manifest, "normalization_applied": False, "equivalence_authorized": False}
    write(evidence / "oci-observation.json", record)
    return record


def mounted_ids(path):
    value = read(path)
    require(isinstance(value, list), "Native mounted-image listing is not a JSON array")
    return {item["id"].removeprefix("sha256:") for item in value}


def unmount_exact(name, image_id, native):
    try:
        _, code = native("image-unmount-" + name, ["sudo", "podman", "image", "unmount", image_id], checked=False, timeout=30)
    except OSError as error:
        # A failed evidence write must not prevent cleanup. Query native state
        # first because the logged unmount may already have executed successfully.
        listing = subprocess.run(["sudo", "timeout", "--kill-after=10s", "30s", "podman", "image", "mount", "--format=json"],
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        require(listing.returncode == 0, "Cannot inspect exact image cleanup after an evidence I/O failure")
        mounted = {item["id"].removeprefix("sha256:") for item in json.loads(listing.stdout)}
        if image_id.removeprefix("sha256:") in mounted:
            cleanup = subprocess.run(["sudo", "timeout", "--kill-after=10s", "30s", "podman", "image", "unmount", image_id],
                                     stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
            require(cleanup.returncode == 0, "Exact image cleanup failed after an evidence I/O failure")
        raise RuntimeError("Exact image cleanup attempted, but evidence I/O failed; observation incomplete") from error
    require(code == 0, "Exact image unmount failed; inspect its retained native result")


def stop_reader(name, container, native):
    io_errors, codes = [], []
    operations = [
        ("reader-stop-", ["sudo", "podman", "stop", "--ignore", "--time=10", container]),
        ("reader-remove-", ["sudo", "podman", "rm", "--ignore", container]),
        ("reader-absent-", ["sudo", "podman", "container", "exists", container]),
    ]
    for label, command in operations:
        try:
            _, code = native(label + name, command, checked=False, timeout=30)
        except OSError as error:
            # Continue the exact owned-container cleanup even if recording it
            # fails. The overall observation remains incomplete in that case.
            result = subprocess.run(["sudo", "timeout", "--kill-after=10s", "30s", *command[1:]],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
            code = result.returncode
            io_errors.append(type(error).__name__)
        codes.append(code)
    require(codes == [0, 0, 1], "Exact reader container was not stopped/reaped; inspect native cleanup evidence")
    require(not io_errors, "Reader cleanup ran but evidence I/O failed; observation incomplete")


def capture_filesystem(name, image_id, evidence, recorder, native, overall_deadline):
    require(re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}", image_id), "Expected an exact native built image ID")
    deadline = min(overall_deadline, time.monotonic() + 15 * 60)

    def observe(label, args, *, checked=True):
        remaining = deadline - time.monotonic()
        require(remaining > 0, "Complete image observation deadline exceeded")
        return native(label, args, checked=checked, timeout=remaining)

    container = "kedra-image-reader-" + uuid.uuid4().hex
    _, exists = observe("reader-name-unused-" + name, ["sudo", "podman", "container", "exists", container], checked=False)
    require(exists == 1, "Reader container name is not unused or storage is unavailable")
    write(evidence / "image-observation-request.json", {"schema_version": 1, "native_image_id": image_id,
          "reader_container": container, "mounted_phase_limit_seconds": 15 * 60,
          "metadata_limit_bytes": MAX_METADATA_BYTES, "reader_memory_bytes": 2 * 1024**3})
    store_dir, _ = observe("image-store-" + name, ["sudo", "podman", "info", "--format=json"])
    graph_root = read(store_dir / "stdout")["store"]["graphRoot"]
    require(isinstance(graph_root, str) and graph_root.startswith("/"), "Native Podman GraphRoot is missing")
    graph_dir, _ = observe("image-graph-root-" + name, ["sudo", "realpath", "--canonicalize-existing", "--", graph_root])
    graph_root = one_absolute_path(graph_dir / "stdout")
    require(graph_root != PurePosixPath("/"), "Podman GraphRoot cannot be the host filesystem root")
    mounted_dir, _ = observe("image-mounts-before-" + name, ["sudo", "podman", "image", "mount", "--format=json"])
    require(image_id.removeprefix("sha256:") not in mounted_ids(mounted_dir / "stdout"),
            "The target image was already mounted; ownership is ambiguous")
    root, filesystem_dir, before_dir, after_dir = None, None, None, None
    try:
        mount_dir, _ = observe("image-mount-" + name, ["sudo", "podman", "image", "mount", image_id])
        mount_path = one_absolute_path(mount_dir / "stdout")
        real_dir, _ = observe("image-mount-root-" + name, ["sudo", "realpath", "--canonicalize-existing", "--", str(mount_path)])
        root = one_absolute_path(real_dir / "stdout")
        require(root != graph_root and root.is_relative_to(graph_root), "Mounted image escaped the actual Podman GraphRoot")
        xattrs = ["sudo", "getfattr", "--dump", "--match=-", "--encoding=base64", "--recursive",
                  "--physical", "--no-dereference", "--absolute-names", "--", str(root)]
        before_dir, _ = observe("image-xattrs-before-" + name, xattrs)
        require((before_dir / "stderr").stat().st_size == 0, "Native xattr observation emitted a diagnostic; snapshot incomplete")
        require((before_dir / "stdout").stat().st_size <= MAX_METADATA_BYTES, "Native xattrs exceed the complete metadata budget")
        filesystem_dir, _ = observe("image-filesystem-" + name, [
            "sudo", "podman", "run", "--rm", "--pull=never", "--network=none", "--read-only", "--read-only-tmpfs=false",
            "--name=" + container, "--memory=2g", "--memory-swap=2g",
            "--user=0:0", "--workdir=/", "--env=PYTHONDONTWRITEBYTECODE=1", "--security-opt=no-new-privileges",
            "--mount", f"type=bind,source={root},destination=/image,ro,bind-nonrecursive",
            "--mount", f"type=bind,source={recorder},destination=/recorder.py,ro",
            "--entrypoint=/usr/bin/python3", image_id, "/recorder.py",
        ])
        require((filesystem_dir / "stderr").stat().st_size == 0, "Image filesystem reader emitted a diagnostic; snapshot incomplete")
        require((filesystem_dir / "stdout").stat().st_size <= MAX_METADATA_BYTES, "Filesystem observation exceeds metadata budget")
        after_dir, _ = observe("image-xattrs-after-" + name, xattrs)
        require((after_dir / "stderr").stat().st_size == 0, "Native final xattr observation emitted a diagnostic; snapshot incomplete")
    finally:
        # Only our one exact image mount is released. No --all/--force cleanup and
        # no checkout program executes as host root.
        try:
            stop_reader(name, container, native)
        finally:
            unmount_exact(name, image_id, native)
    mounted_dir, _ = native("image-mounts-after-" + name, ["sudo", "podman", "image", "mount", "--format=json"], timeout=30)
    require(image_id.removeprefix("sha256:") not in mounted_ids(mounted_dir / "stdout"), "Exact image remains mounted after cleanup")
    observed = parse_filesystem(filesystem_dir / "stdout")
    before = parse_xattrs(before_dir / "stdout", root, observed["entries"])
    after = parse_xattrs(after_dir / "stdout", root, observed["entries"])
    require(before == after, "Image xattrs changed during filesystem observation; snapshot incomplete")
    for group in observed["summary"]["hardlink_groups"]:
        values = [before[key] for key in group["paths_base64"]]
        require(all(value == values[0] for value in values), "Native xattr coverage differs across hardlinks; snapshot incomplete")
    for key, row in observed["entries"].items():
        row["xattrs"] = before[key]
    observed.update(schema_version=1, native_image_id=image_id,
                    native_xattr_before_sha256=digest(before_dir / "stdout"),
                    native_xattr_after_sha256=digest(after_dir / "stdout"),
                    native_filesystem_sha256=digest(filesystem_dir / "stdout"),
                    image_unmounted=True, complete=True, normalization_applied=False, equivalence_authorized=False)
    # Compression is only an evidence container; no observed image fact changes.
    path = evidence / "filesystem-observation.json.gz"
    with path.open("xb") as destination, gzip.GzipFile(filename="", mode="wb", fileobj=destination, mtime=0) as compressed:
        compressed.write((json.dumps(observed, sort_keys=True, separators=(",", ":")) + "\n").encode())
    write(evidence / "filesystem-observation.json", {"schema_version": 1, "native_image_id": image_id,
          "complete": True, "entry_count": len(observed["entries"]), "summary": observed["summary"],
          "compressed_filename": path.name, "compressed_sha256": digest(path),
          "image_unmounted": True, "normalization_applied": False, "equivalence_authorized": False})
    return observed


def differences(before, after, path=""):
    """All observed JSON fields participate; no ignore list or timestamp masking."""
    if type(before) is not type(after):
        return [{"path": path, "before": before, "after": after}]
    if isinstance(before, dict):
        result = []
        for key in sorted(before.keys() | after.keys()):
            pointer = path + "/" + key.replace("~", "~0").replace("/", "~1")
            if key not in before or key not in after:
                result.append({"path": pointer, "before_present": key in before, "after_present": key in after,
                               "before": before.get(key), "after": after.get(key)})
            else:
                result.extend(differences(before[key], after[key], pointer))
        return result
    if isinstance(before, list):
        result = []
        for index in range(max(len(before), len(after))):
            pointer = path + "/" + str(index)
            if index >= len(before) or index >= len(after):
                result.append({"path": pointer, "before_present": index < len(before), "after_present": index < len(after),
                               "before": before[index] if index < len(before) else None,
                               "after": after[index] if index < len(after) else None})
            else:
                result.extend(differences(before[index], after[index], pointer))
        return result
    return [] if before == after else [{"path": path, "before": before, "after": after}]


def compare_pair(output, images):
    before, after = images
    left, right = before["filesystem"]["entries"], after["filesystem"]["entries"]
    changes, counts = [], {}
    for key in sorted(left.keys() | right.keys()):
        if key not in left or key not in right:
            changes.append({"path_base64": key, "path_display": (left.get(key) or right[key])["path_display"],
                            "change": "removed" if key not in right else "added"})
        else:
            fields = differences(left[key], right[key])
            if fields:
                changes.append({"path_base64": key, "path_display": left[key]["path_display"], "change": "changed", "fields": fields})
                for field in fields:
                    counts[field["path"]] = counts.get(field["path"], 0) + 1
    hardlinks = differences(before["filesystem"]["summary"]["hardlink_groups"], after["filesystem"]["summary"]["hardlink_groups"])
    oci = {category: differences(before["oci"][category], after["oci"][category]) for category in
           ("runtime_config", "platform_config", "other_config_fields", "manifest")}
    any_difference = bool(changes or hardlinks or any(oci.values()))
    record = {"schema_version": 1, "scope": "complete-final-image-observations",
              "status": "differences-observed" if any_difference else "observed-identical",
              "complete": True, "baseline_image_id": before["oci"]["native_image_id"],
              "repeat_image_id": after["oci"]["native_image_id"], "filesystem_changes": changes,
              "filesystem_changed_field_counts": counts, "hardlink_relationship_changes": hardlinks,
              "oci_changes": oci, "normalization_applied": False, "equivalence_authorized": False,
              "freshness_written": False}
    path = output / "final-image-differences.json.gz"
    with path.open("xb") as destination, gzip.GzipFile(filename="", mode="wb", fileobj=destination, mtime=0) as compressed:
        compressed.write((json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode())
    summary = {key: value for key, value in record.items() if key not in ("filesystem_changes", "hardlink_relationship_changes", "oci_changes")}
    summary.update(filesystem_changed_paths=len(changes), hardlink_difference_count=len(hardlinks),
                   oci_changed_field_counts={category: len(changes) for category, changes in oci.items()},
                   compressed_filename=path.name, compressed_sha256=digest(path))
    write(output / "final-image-differences.json", summary)
    return summary
