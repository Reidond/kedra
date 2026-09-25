"""Ubuntu OVMF UEFI Secure Boot firmware for disposable x86_64 VM tests.

Every CI VM boots Ubuntu's SMM-enforced Secure Boot build with a private copy
of its Microsoft-enrolled variable template. The snakeoil template trusts only
Ubuntu's test key; it exists solely for the negative case in which firmware
must refuse Fedora's Microsoft-signed shim. Nothing here writes outside the
explicit disposable output or evidence paths.
"""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import struct
import subprocess
import tempfile
import uuid

OVMF = pathlib.Path("/usr/share/OVMF")
CODE = OVMF / "OVMF_CODE_4M.secboot.fd"
TEMPLATES = {"microsoft": OVMF / "OVMF_VARS_4M.ms.fd", "snakeoil": OVMF / "OVMF_VARS_4M.snakeoil.fd"}
DESCRIPTORS = pathlib.Path("/usr/share/qemu/firmware")
PACKAGES = ("ovmf", "qemu-system-x86", "python3-virt-firmware")
GLOBAL = "8be4df61-93ca-11d2-aa0d-00e098032b8c"
SECURITY_DATABASE = "d719b2cb-3d3a-4596-a3bc-dad00e67656f"
# Variables that establish Secure Boot trust and enforcement, by vendor GUID.
# A persisted store must still equal the Microsoft template in every one.
TRUST = {
    "PK": GLOBAL, "KEK": GLOBAL, "db": SECURITY_DATABASE, "dbx": SECURITY_DATABASE,
    "SecureBootEnable": "f0a30bc7-af08-4556-99c4-001009c93a44",
    "CustomMode": "c076ec0c-7028-4399-a072-71ee5c448b9f",
}
X509 = uuid.UUID("a5c059a1-94e4-4aa7-87b5-ab155c2bf072")
# SHA-256 of the DER certificates; their SHA-1 thumbprints (46def63b... and
# b5eeb4a6...) are Microsoft's published values. Fedora 44 shim-x64 16.1-5 is
# signed only through the 2011 CA, so the Microsoft template must contain it.
MICROSOFT_2011 = "48e99b991f57fc52f76149599bff0a58c47154229b9f8d603ac40d3500248507"
MICROSOFT_2023 = "f6124e34125bee3fe6d79a574eaa7b91c0e7bd9d929c1a321178efd611dad901"


class FirmwareError(RuntimeError):
    pass


def variables(path):
    """Decode an edk2 variable store through virt-fw-vars' JSON dump."""
    with tempfile.TemporaryDirectory(prefix="kedra-uefi-vars-") as scratch:
        output = pathlib.Path(scratch) / "vars.json"
        result = subprocess.run(["virt-fw-vars", "--input", str(path), "--output-json", str(output)],
                                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=60)
        if result.returncode:
            detail = result.stderr.decode(errors="replace").strip().splitlines()
            raise FirmwareError(f"cannot decode UEFI variable store {path}: {detail[-1] if detail else result.returncode}")
        document = json.loads(output.read_bytes())
    if document.get("version") != 2 or not isinstance(document.get("variables"), list):
        raise FirmwareError("unexpected virt-fw-vars JSON document")
    return document["variables"]


def trust(path):
    found = {}
    for entry in variables(path):
        name = entry["name"]
        if TRUST.get(name) == entry["guid"].lower():
            if name in found:
                raise FirmwareError(f"duplicate {name} variable in {path}")
            found[name] = (entry["attr"], bytes.fromhex(entry["data"]))
    return found


def certificates(data):
    """SHA-256 of each X.509 entry in a sequence of EFI_SIGNATURE_LISTs."""
    found, offset = [], 0
    while offset < len(data):
        if len(data) - offset < 28:
            raise FirmwareError("truncated EFI signature list")
        kind = uuid.UUID(bytes_le=data[offset:offset + 16])
        list_size, header_size, entry_size = struct.unpack_from("<III", data, offset + 16)
        start, end = offset + 28 + header_size, offset + list_size
        if end > len(data) or start > end or entry_size <= 16 or (end - start) % entry_size:
            raise FirmwareError("malformed EFI signature list")
        if kind == X509:
            found += [hashlib.sha256(data[entry + 16:entry + entry_size]).hexdigest() for entry in range(start, end, entry_size)]
        offset = end
    return found


def template(kind):
    """Trust variables of an installed template, after checking its purpose."""
    values = trust(TEMPLATES[kind])
    missing = [name for name in ("PK", "KEK", "db", "SecureBootEnable") if name not in values]
    if missing or values["SecureBootEnable"][1] != b"\x01":
        raise FirmwareError(f"{TEMPLATES[kind]} does not enable Secure Boot with enrolled keys")
    db = set(certificates(values["db"][1]))
    if kind == "microsoft" and MICROSOFT_2011 not in db:
        raise FirmwareError(f"{TEMPLATES[kind]} db lacks Microsoft Corporation UEFI CA 2011")
    if kind == "snakeoil" and db & {MICROSOFT_2011, MICROSOFT_2023}:
        raise FirmwareError(f"{TEMPLATES[kind]} db unexpectedly trusts a Microsoft UEFI CA")
    return values


def verify(path):
    """Refuse a store whose Secure Boot trust differs from the Microsoft template."""
    path = pathlib.Path(path)
    expected = template("microsoft")
    if not path.is_file() or path.stat().st_size != TEMPLATES["microsoft"].stat().st_size:
        raise FirmwareError(f"{path} is not a raw OVMF variable store of the template's size")
    actual = trust(path)
    changed = [name for name in TRUST if actual.get(name) != expected.get(name)]
    if changed:
        raise FirmwareError(f"{path} is not derived from {TEMPLATES['microsoft']}; changed: {', '.join(changed)}")


def create(kind, output):
    """Copy a checked template to a new private store; never overwrite."""
    template(kind)
    try:
        with TEMPLATES[kind].open("rb") as source, pathlib.Path(output).open("xb") as target:
            shutil.copyfileobj(source, target)
    except FileExistsError:
        raise FirmwareError(f"{output} already exists; VM firmware state is never overwritten") from None
    if kind == "microsoft":
        verify(output)


def enrolled_descriptor():
    """Ubuntu's own QEMU descriptor declaring this code/template pairing."""
    matches = []
    for path in sorted(DESCRIPTORS.glob("*.json")):
        document = json.loads(path.read_bytes())
        features = set(document.get("features", []))
        mapping = document.get("mapping", {})
        executable = mapping.get("executable", {}).get("filename")
        variables_template = mapping.get("nvram-template", {}).get("filename")
        if (executable and variables_template and {"secure-boot", "requires-smm", "enrolled-keys"} <= features
                and os.path.realpath(executable) == os.path.realpath(CODE)
                and os.path.realpath(variables_template) == os.path.realpath(TEMPLATES["microsoft"])):
            matches.append(path)
    if len(matches) != 1:
        raise FirmwareError(f"expected one enrolled Secure Boot descriptor for {CODE}, found {len(matches)}")
    return matches[0]


def sha256(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def provenance(evidence):
    evidence.mkdir(parents=True, exist_ok=True)
    packages = subprocess.run(["dpkg-query", "--show", "--showformat=${Package}\t${Version}\t${Architecture}\n", *PACKAGES],
                              check=True, capture_output=True, text=True, timeout=60).stdout
    descriptor = enrolled_descriptor()
    record = {
        "schema_version": 1,
        "packages": [dict(zip(("package", "version", "architecture"), line.split("\t"))) for line in packages.splitlines()],
        "descriptor": {"path": str(descriptor), "sha256": sha256(descriptor)},
        "code": {"path": str(CODE), "resolved": os.path.realpath(CODE), "sha256": sha256(CODE)},
        "templates": {},
    }
    for kind, path in TEMPLATES.items():
        values = template(kind)
        listing = subprocess.run(["virt-fw-vars", "--input", str(path), "--print", "--verbose"],
                                 check=True, capture_output=True, text=True, timeout=60)
        (evidence / f"secure-boot-vars-{kind}.txt").write_text(listing.stdout + listing.stderr)
        record["templates"][kind] = {
            "path": str(path), "resolved": os.path.realpath(path), "sha256": sha256(path),
            "x509_sha256": {name: certificates(values[name][1]) for name in ("PK", "KEK", "db")},
        }
    (evidence / "secure-boot-firmware.json").write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("provenance").add_argument("--evidence", type=pathlib.Path, required=True)
    copy = commands.add_parser("vars")
    copy.add_argument("--template", choices=sorted(TEMPLATES), required=True)
    copy.add_argument("--output", type=pathlib.Path, required=True)
    commands.add_parser("verify").add_argument("--vars", type=pathlib.Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "provenance":
            provenance(args.evidence)
        elif args.command == "vars":
            create(args.template, args.output)
        else:
            verify(args.vars)
    except FirmwareError as error:
        raise SystemExit(f"Secure Boot firmware refused: {error}") from None


if __name__ == "__main__":
    main()
