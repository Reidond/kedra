"""Cross-check the Rust release verifier with independent OpenSSL signatures.

All identities, bytes and keys here are disposable synthetic fixtures. This is
not an OS release and cannot produce a production signing key or promotion.
"""
import argparse
import base64
import hashlib
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--workdir", type=pathlib.Path, required=True)
parser.add_argument("--sysroot", type=pathlib.Path)
args = parser.parse_args()
root = args.workdir.resolve()
root.mkdir(parents=True, exist_ok=False)
binary = args.sysroot.resolve() if args.sysroot else None
private = root / "disposable.pem"
wrong_private = root / "wrong-disposable.pem"
public = root / "release.pub"
wrong_public = root / "wrong.pub"
payload = root / "release.json"
artifact = root / "kedra-desktop-44-42.iso"
artifact.write_bytes(b"abc")
release = {
    "schema_version": 1,
    "project": "Kedra",
    "scope": {"target": "desktop", "architecture": "x86_64", "fedora_release": 44, "repository": "ghcr.io/reidond/kedra-desktop"},
    "sequence": 42,
    "source_revision": "a" * 40,
    "build": {"repository": "Reidond/kedra", "workflow": ".github/workflows/release.yml", "run_id": 123, "run_attempt": 1},
    "image_digest": "sha256:" + "b" * 64,
    "home_manifest_sha256": "c" * 64,
    "installer": {"filename": artifact.name, "size_bytes": 3, "sha256": hashlib.sha256(b"abc").hexdigest()},
    "approval": "promoted",
    "minimum_protocol": 1,
}
payload.write_text(json.dumps(release, separators=(",", ":")), encoding="utf-8")
def openssl(*arguments):
    return subprocess.run(["openssl", *map(str, arguments)], check=True, capture_output=True).stdout

def verify(signature, key=public, manifest=payload, image=artifact, expected=True):
    result = subprocess.run([str(binary), "release", "verify", "--manifest", str(manifest),
        "--signature", str(signature), "--public-key", str(key), "--artifact", str(image),
        "--target", "desktop", "--json"], capture_output=True)
    if expected:
        if result.returncode != 0:
            raise RuntimeError("Rust rejected an OpenSSL signature: " + result.stderr.decode())
        report = json.loads(result.stdout)
        assert report["signature_valid"] and report["artifact_verified"]
        assert not report["deployment_authorized"] and not report["channel_freshness_verified"]
    else:
        assert result.returncode != 0 and not result.stdout, "Invalid input received success output"

try:
    print(openssl("version").decode().strip(), flush=True)
    for secret, pub in [(private, public), (wrong_private, wrong_public)]:
        openssl("genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", secret)
        secret.chmod(0o600)
        openssl("pkey", "-in", secret, "-pubout", "-out", pub)
    for number in range(16):
        # OpenSSL chooses independent nonces, independently of RustCrypto.
        der = openssl("dgst", "-sha256", "-sign", private, payload)
        signature = root / f"signature-{number}.sig"
        signature.write_bytes(base64.b64encode(der) + b"\n")
        if binary:
            verify(signature)
    if binary:
        identity = subprocess.run([str(binary), 'release', 'key', '--public-key', str(public), '--json'], capture_output=True, check=True)
        identity = json.loads(identity.stdout)
        expected = hashlib.sha256(openssl('pkey', '-pubin', '-in', public, '-outform', 'DER')).hexdigest()
        assert identity['key_fingerprint_sha256'] == expected and not identity['trust_established']
        invalid_key = subprocess.run([str(binary), 'release', 'key', '--public-key', str(private)], capture_output=True)
        assert invalid_key.returncode != 0 and not invalid_key.stdout
        verify(signature, key=wrong_public, expected=False)
        altered = root / "altered.json"
        altered.write_bytes(payload.read_bytes() + b" ")
        verify(signature, manifest=altered, expected=False)
        corrupted = root / "corrupted.iso"
        corrupted.write_bytes(b"abd")
        verify(signature, image=corrupted, expected=False)
        # Exercise the distributed-download workflow through the actual CLI.
        # OpenSSL, not sysroot, supplies its signed release fixture.
        parts = []
        for number, content in enumerate([b"a", b"b", b"c"]):
            path = root / f"installer.part{number}"
            path.write_bytes(content)
            parts.append(path)
        destination = root / "assembled"
        destination.mkdir()
        assemble = [str(binary), "release", "assemble", "--manifest", str(payload),
            "--signature", str(signature), "--public-key", str(public), "--target", "desktop",
            "--output-dir", str(destination), "--json"]
        for invalid in [parts[:-1], list(reversed(parts)), parts + parts]:
            result = subprocess.run(assemble + list(map(str, invalid)), capture_output=True)
            assert result.returncode != 0 and not list(destination.iterdir()), "Bad parts published output"
        result = subprocess.run(assemble + list(map(str, parts)), capture_output=True)
        if result.returncode != 0:
            raise RuntimeError("Installer assembly failed: " + result.stderr.decode())
        assert json.loads(result.stdout)["artifact_verified"]
        assembled = destination / artifact.name
        assert assembled.read_bytes() == artifact.read_bytes()
        verify(signature, image=assembled)
        result = subprocess.run(assemble + list(map(str, parts)), capture_output=True)
        assert result.returncode != 0 and assembled.read_bytes() == b"abc", "Existing output was replaced"
        print("PASS: 16 OpenSSL signatures, wrong-key/tamper/artifact rejection, advisory output", flush=True)
        print("PASS: CLI installer assembly, missing/reordered/extra parts and existing-output refusal", flush=True)
    else:
        print("Generated 16 public signature fixtures for separate-platform CLI verification", flush=True)
finally:
    private.unlink(missing_ok=True)
    wrong_private.unlink(missing_ok=True)
