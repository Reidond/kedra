#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Cross-check release-key identity and public trust preparation with OpenSSL.

`sysroot release key` must report the SHA-256 of the key's DER encoding that
OpenSSL computes independently, and `prepare-trust.py` must accept exactly that
fingerprint for each enabled target and refuse every other authority or scope.
All keys here are disposable synthetic fixtures; this cannot produce a
production signing key or promotion.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--workdir", type=pathlib.Path, required=True)
parser.add_argument("--sysroot", type=pathlib.Path, required=True)
parser.add_argument("--openssl", default="openssl")
args = parser.parse_args()
root = args.workdir.resolve()
root.mkdir(parents=True, exist_ok=False)
binary = args.sysroot.resolve()
private = root / "disposable.pem"
public = root / "release.pub"
repository = pathlib.Path(__file__).resolve().parents[5]


def openssl(*arguments):
    return subprocess.run([args.openssl, *map(str, arguments)], check=True, capture_output=True).stdout


def prepare_trust(name, plan_value, fingerprint):
    plan = root / ('public-source-plan-' + name + '.json')
    plan.write_text(json.dumps(plan_value), encoding='utf-8')
    destination = root / ('public-trust-' + name)
    result = subprocess.run(['uv', 'run', str(repository / 'usr/src/kedra/image/release/prepare-trust.py'),
        '--source', str(plan), '--public-key', str(public), '--expected-fingerprint', fingerprint,
        '--sysroot', str(binary), '--output', str(destination)], capture_output=True, check=False)
    return result, destination


def check_accepted(result, destination, scope):
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    assert (destination / 'release.pub').read_bytes() == public.read_bytes()
    policy = json.loads((destination / 'policy.json').read_bytes())
    assert policy['default'] == [{'type': 'reject'}]
    assert list(policy['transports']['docker']) == [scope['image']]
    assert policy['transports']['docker'][scope['image']][0]['signedIdentity'] == {
        'type': 'exactRepository', 'dockerRepository': scope['image']}
    release_policy = json.loads((destination / 'release-policy.json').read_bytes())
    assert release_policy['scope'] == {'target': scope['id'], 'architecture': scope['architecture'],
                                       'fedora_release': 44, 'repository': scope['image']}


try:
    print(openssl("version").decode().strip(), flush=True)
    openssl("genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", private)
    private.chmod(0o600)
    openssl("pkey", "-in", private, "-pubout", "-out", public)
    identity = subprocess.run([str(binary), 'release', 'key', '--public-key', str(public), '--json'],
                              capture_output=True, check=True)
    identity = json.loads(identity.stdout)
    expected = hashlib.sha256(openssl('pkey', '-pubin', '-in', public, '-outform', 'DER')).hexdigest()
    assert identity['key_fingerprint_sha256'] == expected and not identity['trust_established']
    invalid_key = subprocess.run([str(binary), 'release', 'key', '--public-key', str(private)],
                                 capture_output=True, check=False)
    assert invalid_key.returncode != 0 and not invalid_key.stdout, 'A private key was identified as a public key'
    print('PASS: release key matches the OpenSSL DER fingerprint and refuses a private key', flush=True)
    # The public producer validates the checked-in/explicit fingerprint;
    # signing independently binds that value to protected environment authority.
    desktop_plan = json.loads(subprocess.check_output([str(binary), 'source', 'plan', '--host', 'desktop', '--json'],
                                                      cwd=repository))
    # The qemu-arm64 plan is generated from the actual committed desktop plan so this
    # exercises trust preparation independently of which hosts are committed.
    qemu_plan = dict(desktop_plan, target=dict(desktop_plan['target'], id='qemu-arm64', architecture='aarch64',
                                              image='ghcr.io/reidond/kedra-qemu-arm64', hardware_status='virtual'))
    for plan_value in (desktop_plan, qemu_plan):
        scope = plan_value['target']
        for name, fingerprint, accepted in [('accepted', expected, True), ('wrong', '0' * 64, False), ('missing', '', False)]:
            result, destination = prepare_trust(scope['id'] + '-' + name, plan_value, fingerprint)
            if accepted:
                check_accepted(result, destination, scope)
            else:
                assert result.returncode != 0 and not destination.exists(), 'Untrusted fingerprint produced public trust'
    print('PASS: desktop and qemu-arm64 public trust accept the exact key fingerprint and refuse missing/wrong authority', flush=True)
    for name, target_changes in [
        ('qemu-arm64-x86_64', {'id': 'qemu-arm64', 'architecture': 'x86_64', 'image': 'ghcr.io/reidond/kedra-qemu-arm64'}),
        ('qemu-arm64-desktop-repository', {'id': 'qemu-arm64', 'architecture': 'aarch64', 'image': 'ghcr.io/reidond/kedra-desktop'}),
        ('desktop-aarch64', {'architecture': 'aarch64'}),
        ('desktop-qemu-arm64-repository', {'image': 'ghcr.io/reidond/kedra-qemu-arm64'}),
        ('disabled-xps', {'id': 'xps', 'image': 'ghcr.io/reidond/kedra-xps'}),
        ('qemu-arm64-not-candidate', {'id': 'qemu-arm64', 'architecture': 'aarch64', 'image': 'ghcr.io/reidond/kedra-qemu-arm64',
                                      'candidate_target': False}),
    ]:
        refused = dict(desktop_plan, target=dict(desktop_plan['target'], **target_changes))
        result, destination = prepare_trust(name, refused, expected)
        assert result.returncode != 0 and not destination.exists(), 'Unsupported target scope produced public trust: ' + name
    print('PASS: public trust refuses wrong-architecture, wrong-repository, disabled and unknown target scopes', flush=True)
finally:
    private.unlink(missing_ok=True)
