#!/usr/bin/env python3
"""Read a mounted *image* at /image; never traverse this container's live root.

Only stat facts, names, symlink targets, hardlink relationships and content hashes
are emitted. A separate native host-root getfattr observation supplies all xattr
namespaces; this container receives no extra capabilities or writable mounts.
"""

import base64
import hashlib
import json
import os
import stat
import sys
import time


ROOT = b"/image"
LIMITS = {"entries": 1_000_000, "path_bytes": 64 * 1024**2, "content_bytes": 64 * 1024**3,
          "output_bytes": 128 * 1024**2, "seconds": 14 * 60, "depth": 256}
STAT_FIELDS = ("st_mode", "st_uid", "st_gid", "st_size", "st_nlink", "st_ino", "st_dev",
               "st_rdev", "st_atime_ns", "st_mtime_ns", "st_ctime_ns", "st_blocks", "st_blksize")
KINDS = {stat.S_IFREG: "regular", stat.S_IFDIR: "directory", stat.S_IFLNK: "symlink",
         stat.S_IFCHR: "character-device", stat.S_IFBLK: "block-device", stat.S_IFIFO: "fifo",
         stat.S_IFSOCK: "socket"}


def encoded(value):
    return base64.b64encode(value).decode("ascii")


def facts(observed):
    return {field.removeprefix("st_"): getattr(observed, field) for field in STAT_FIELDS}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    require(len(sys.argv) == 1 and sys.platform == "linux", "Use the explicit Linux image-recorder entrypoint")
    require(os.statvfs(ROOT).f_flag & os.ST_RDONLY, "/image must be a read-only mount")
    root_fd = os.open(ROOT, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    rows, groups, directory_entries = {}, {}, {}
    started = time.time_ns()
    deadline = time.monotonic() + LIMITS["seconds"]
    root_device = os.fstat(root_fd).st_dev
    total_bytes, path_bytes, output_bytes = 0, 0, 0
    active_path = b""

    def budget():
        require(time.monotonic() < deadline, "Mounted-image reader deadline exceeded")

    def emit(value):
        nonlocal output_bytes
        budget()
        encoded_value = json.dumps(value, sort_keys=True, ensure_ascii=True, separators=(",", ":"))
        output_bytes += len(encoded_value) + 1
        require(output_bytes <= LIMITS["output_bytes"], "Complete filesystem metadata exceeds the output budget")
        print(encoded_value, flush=True)

    def directory_names(descriptor):
        names, size = [], 0
        with os.scandir(descriptor) as iterator:
            for entry in iterator:
                budget()
                name = os.fsencode(entry.name)
                names.append(name)
                size += len(name)
                require(len(names) <= LIMITS["entries"] and size <= LIMITS["path_bytes"],
                        "Native directory exceeds the complete observation budget")
        return sorted(names)

    def visit(path, parent_fd, name):
        nonlocal total_bytes, path_bytes, active_path
        active_path = path
        budget()
        path_bytes += len(path)
        require(len(rows) < LIMITS["entries"] and path_bytes <= LIMITS["path_bytes"]
                and path.count(b"/") < LIMITS["depth"], "Complete filesystem traversal exceeds its resource budget")
        before = os.fstat(root_fd) if not path else os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        require(before.st_dev == root_device, "A nested filesystem is outside the complete image snapshot")
        kind = KINDS.get(stat.S_IFMT(before.st_mode))
        require(kind is not None, "Unsupported image filesystem object type")
        require(path not in rows, "Repeated image filesystem path")
        row = {"path_base64": encoded(path), "path_display": path.decode("utf-8", errors="backslashreplace") or ".",
               "kind": kind, "stat": facts(before)}
        if kind == "regular":
            require(total_bytes + before.st_size <= LIMITS["content_bytes"], "Image content exceeds the complete hash budget")
            descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=parent_fd)
            try:
                require(facts(os.fstat(descriptor)) == facts(before), "Regular image file changed while opening it")
                hashed, count = hashlib.sha256(), 0
                while data := os.read(descriptor, 1024 * 1024):
                    budget()
                    hashed.update(data)
                    count += len(data)
                require(count == before.st_size and facts(os.fstat(descriptor)) == facts(before),
                        "Regular image file changed while hashing it")
                row["content_sha256"] = hashed.hexdigest()
                total_bytes += count
            finally:
                os.close(descriptor)
        elif kind == "symlink":
            target = os.readlink(name, dir_fd=parent_fd)
            require(isinstance(target, bytes), "Symlink target was not read as filesystem bytes")
            row["symlink_target_base64"] = encoded(target)
            row["symlink_target_display"] = target.decode("utf-8", errors="backslashreplace")
        rows[path] = row
        if kind != "directory":
            groups.setdefault((before.st_dev, before.st_ino), []).append(path)
        if kind == "directory":
            directory_fd = os.dup(root_fd) if not path else os.open(
                name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent_fd)
            try:
                require(facts(os.fstat(directory_fd)) == facts(before), "Image directory changed while opening it")
                names = directory_names(directory_fd)
                directory_entries[path] = names
                for child in names:
                    require(child not in (b"", b".", b"..") and b"/" not in child and b"\0" not in child,
                            "Unexpected native directory entry")
                    visit(path + b"/" + child if path else child, directory_fd, child)
                active_path = path
                require(names == directory_names(directory_fd)
                        and facts(os.fstat(directory_fd)) == facts(before),
                        "Image directory changed during its complete traversal")
            finally:
                os.close(directory_fd)
        after = os.fstat(root_fd) if not path else os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        require(facts(after) == facts(before), "Image filesystem entry changed during observation")

    try:
        emit({"record": "header", "schema_version": 1, "scope": "mounted-image-filesystem",
              "started_ns": started, "python_version": sys.version,
              "root_read_only": True, "limits": LIMITS,
              "xattrs_source": "separate native getfattr before/after snapshots"})
        visit(b"", root_fd, b".")
        # Re-list every directory and re-stat every entry after all content reads.
        # No timestamps, inode facts or paths are ignored to obtain a clean result.
        for path, row in rows.items():
            active_path = path
            budget()
            absolute = ROOT + (b"/" + path if path else b"")
            if row["kind"] == "directory":
                directory_fd = os.open(absolute, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
                try:
                    require(directory_entries[path] == directory_names(directory_fd),
                            "Image directory entries changed before snapshot completion")
                finally:
                    os.close(directory_fd)
            require(facts(os.stat(absolute, follow_symlinks=False)) == row["stat"],
                    "Image metadata changed before snapshot completion")
        hardlinks = []
        for paths in groups.values():
            active_path = paths[0]
            budget()
            count = rows[paths[0]]["stat"]["nlink"]
            require(all(rows[path]["stat"]["nlink"] == count for path in paths), "Inconsistent native hardlink counts")
            require(len(paths) == count, "Native hardlink count is not fully accounted inside the image")
            if count > 1:
                hardlinks.append({"paths_base64": [encoded(path) for path in sorted(paths)], "link_count": count})
        for path in sorted(rows):
            active_path = path
            emit({"record": "entry", **rows[path]})
        emit({"record": "summary", "schema_version": 1, "complete": True, "entry_count": len(rows),
              "regular_file_bytes_hashed": total_bytes, "hardlink_groups": sorted(hardlinks, key=lambda item: item["paths_base64"]),
              "finished_ns": time.time_ns(), "normalization_applied": False, "equivalence_authorized": False})
        return 0
    except (OSError, ValueError, RuntimeError, RecursionError) as error:
        raise RuntimeError(f"{error}; image path {active_path[:300]!r}") from error
    finally:
        os.close(root_fd)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, RecursionError) as error:
        print(f"Incomplete mounted-image filesystem observation: {error}", file=sys.stderr)
        sys.exit(1)
