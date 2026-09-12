#!/usr/bin/env python3
"""Build small signed RPMs and immutable repository snapshots inside Fedora."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


PUBLIC = Path("/public")
PRIVATE = Path("/private")
GPG = PRIVATE / "gnupg"


def native(*args, accept=(0,)):
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    print(result.stdout.decode(errors="replace"), end="", flush=True)
    if result.returncode not in accept:
        raise RuntimeError(f"native {args[0]} exited {result.returncode}")
    return result


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def package(name, version, variant, fingerprint, requires=None):
    top = PRIVATE / "rpmbuild" / variant
    for directory in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
        (top / directory).mkdir(parents=True)
    spec = top / "SPECS" / f"{name}.spec"
    requirement = f"Requires: {requires}\n" if requires else ""
    spec.write_text(
        f"Name: {name}\nVersion: {version}\nRelease: 1\n"
        "Summary: Disposable Kedra native refresh fixture\n"
        "License: MIT\nBuildArch: noarch\nAutoReqProv: no\n"
        f"{requirement}\n%description\nSynthetic data only; no scriptlets.\n"
        "%install\nmkdir -p %{buildroot}/usr/share/kedra-refresh\n"
        f"printf '%s\\n' '{variant}' > %{{buildroot}}/usr/share/kedra-refresh/{name}\n"
        f"\n%files\n/usr/share/kedra-refresh/{name}\n",
        encoding="utf-8",
    )
    native("rpmbuild", "--define", f"_topdir {top}", "-bb", str(spec))
    paths = list((top / "RPMS").rglob("*.rpm"))
    if len(paths) != 1:
        raise RuntimeError("rpmbuild did not produce exactly one fixture RPM")
    native(
        "rpmsign", "--define", "_openpgp_sign gpg", "--define",
        f"_openpgp_sign_id {fingerprint}", "--define", f"_gpg_name {fingerprint}",
        "--define", f"_gpg_path {GPG}", "--addsign", str(paths[0]),
    )
    destination = PUBLIC / "packages" / variant / paths[0].name
    destination.parent.mkdir(parents=True)
    shutil.copyfile(paths[0], destination)
    shutil.copyfile(spec, PUBLIC / "specs" / f"{variant}.spec")
    native("rpmkeys", "--checksig", "--verbose", str(destination))
    return destination


def snapshot(name, packages, revision, fingerprint):
    directory = PUBLIC / "snapshots" / name
    repo = directory / "repo"
    repo.mkdir(parents=True)
    shutil.copyfile(PUBLIC / "key.asc", directory / "key.asc")
    for path in packages:
        shutil.copyfile(path, repo / path.name)
    native("createrepo_c", "--checksum", "sha256", "--revision", str(revision), str(repo))
    metadata = repo / "repodata" / "repomd.xml"
    native(
        "gpg", "--homedir", str(GPG), "--batch", "--yes", "--armor", "--detach-sign",
        "--local-user", fingerprint, "--output", str(metadata) + ".asc", str(metadata),
    )
    files = sorted(path for path in directory.rglob("*") if path.is_file())
    (directory / "snapshot.json").write_text(json.dumps({
        "schema_version": 1,
        "name": name,
        "repomd_sha256": digest(metadata),
        "files": [{"path": str(path.relative_to(directory)), "sha256": digest(path),
                   "size": path.stat().st_size} for path in files],
    }, indent=2) + "\n", encoding="utf-8")


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or not Path("/run/.containerenv").is_file():
        raise RuntimeError("fixture generation is restricted to its Actions container")
    os.umask(0o077)
    GPG.mkdir(mode=0o700)
    for directory in ("packages", "specs", "snapshots"):
        (PUBLIC / directory).mkdir()
    native(
        "gpg", "--homedir", str(GPG), "--batch", "--pinentry-mode", "loopback",
        "--passphrase", "", "--quick-generate-key", "Kedra disposable RPM refresh fixture",
        "rsa3072", "sign", "1d",
    )
    listing = native("gpg", "--homedir", str(GPG), "--batch", "--with-colons", "--list-keys")
    fingerprints = [line.split(":")[9] for line in listing.stdout.decode().splitlines()
                    if line.startswith("fpr:")]
    if len(fingerprints) != 1:
        raise RuntimeError("expected one disposable signing fingerprint")
    fingerprint = fingerprints[0]
    # Export is public-only. The private keyring never enters PUBLIC or an image.
    exported = subprocess.check_output(["gpg", "--homedir", str(GPG), "--armor", "--export", fingerprint])
    (PUBLIC / "key.asc").write_bytes(exported)
    (PUBLIC / "key-fingerprint.txt").write_text(fingerprint + "\n", encoding="utf-8")
    native("rpmkeys", "--import", str(PUBLIC / "key.asc"))
    inherited1 = package("kedra-refresh-inherited", "1", "inherited-1", fingerprint)
    inherited2 = package("kedra-refresh-inherited", "2", "inherited-2", fingerprint)
    dependency1 = package("kedra-refresh-dependency", "1", "dependency-1", fingerprint)
    dependency2 = package("kedra-refresh-dependency", "2", "dependency-2", fingerprint)
    requested1 = package("kedra-refresh-requested", "1", "requested-1", fingerprint,
                         "kedra-refresh-dependency >= 1")
    requested2 = package("kedra-refresh-requested", "2", "requested-2", fingerprint,
                         "kedra-refresh-dependency >= 1")
    republished = package("kedra-refresh-requested", "1", "requested-1-republished", fingerprint,
                          "kedra-refresh-dependency >= 1")
    impossible = package("kedra-refresh-requested", "3", "requested-3-unsatisfiable", fingerprint,
                         "kedra-refresh-dependency >= 99")
    baseline = [inherited1, dependency1, requested1]
    snapshot("baseline", baseline, 1, fingerprint)
    snapshot("requested-update", [inherited1, dependency1, requested2], 2, fingerprint)
    snapshot("transitive-update", [inherited1, dependency2, requested1], 3, fingerprint)
    snapshot("inherited-update", [inherited2, dependency1, requested1], 4, fingerprint)
    snapshot("metadata-only", baseline, 5, fingerprint)
    snapshot("same-nevra-new-bytes", [inherited1, dependency1, republished], 6, fingerprint)
    snapshot("unsatisfiable-newest", [*baseline, impossible], 7, fingerprint)
    corrupt = PUBLIC / "packages" / "requested-2-corrupt" / requested2.name
    corrupt.parent.mkdir()
    payload = bytearray(requested2.read_bytes())
    payload[-1] ^= 1
    corrupt.write_bytes(payload)
    result = native("rpmkeys", "--checksig", "--verbose", str(corrupt), accept=range(256))
    if result.returncode == 0:
        raise RuntimeError("native RPM accepted the intentionally damaged signed package")
    (PUBLIC / "corrupt-package-verification.json").write_text(json.dumps({
        "exit_code": result.returncode, "sha256": digest(corrupt),
        "mutation": "one payload byte changed after signing; repository checksums are regenerated",
    }, indent=2) + "\n", encoding="utf-8")
    snapshot("invalid-signature", [inherited1, dependency1, corrupt], 8, fingerprint)
    outage = PUBLIC / "snapshots" / "required-repo-unavailable"
    (outage / "repo").mkdir(parents=True)
    shutil.copyfile(PUBLIC / "key.asc", outage / "key.asc")
    (outage / "snapshot.json").write_text(json.dumps({
        "schema_version": 1, "name": "required-repo-unavailable",
        "repomd_sha256": None, "condition": "required local repository has no repodata",
    }, indent=2) + "\n", encoding="utf-8")
    native("gpgconf", "--homedir", str(GPG), "--kill", "all")


if __name__ == "__main__":
    main()
