"""End-to-end published material verification with independent OpenSSL signatures."""
import argparse
import base64
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument('--workdir', type=Path, required=True)
args = parser.parse_args()
work = args.workdir.resolve()
work.mkdir(parents=True, exist_ok=False)
cli = Path(__file__).resolve().parents[1] / 'build/release/material.py'
key, public = work / 'disposable.key', work / 'release.pub'


def native(*arguments):
    return subprocess.check_output(list(map(str, arguments)), stderr=subprocess.PIPE)


def write(name, value):
    (work / name).write_text(json.dumps(value, sort_keys=True) + '\n')


def check(current, decision=None, valid=True, release_hash=None):
    write('current.json', current)
    result = subprocess.run([sys.executable, str(cli), '--release-dir', str(work), '--public-key', str(public),
                             '--release-sha256', release_hash or expected_hash,
                             '--current-inputs', str(work / 'current.json')], capture_output=True)
    if valid:
        if result.returncode or json.loads(result.stdout)['decision'] != decision:
            raise RuntimeError('Material CLI workflow failed: ' + result.stderr.decode(errors='replace'))
    elif result.returncode == 0:
        raise RuntimeError('Material CLI accepted an unauthenticated release')


try:
    native('openssl', 'genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:P-256', '-out', key)
    key.chmod(0o600)
    native('openssl', 'pkey', '-in', key, '-pubout', '-out', public)
    release = {'project': 'Kedra', 'scope': {'target': 'desktop'}, 'source_revision': 'a' * 40,
               'build': {'run_id': 1}, 'image_digest': 'sha256:' + 'b' * 64,
               'home_manifest_sha256': 'c' * 64, 'installer': {'filename': 'fixture.iso'}}
    inputs = {'schema_version': 1, 'base': 'quay.io/fedora/fedora-bootc@sha256:' + 'd' * 64,
              'source': {'files': [{'destination': 'etc/example', 'sha256': 'e' * 64}]},
              'artifacts': {'sysroot': 'f' * 64}, 'recipes': {'Containerfile': '1' * 64},
              'packages': [['example', '0', '1', '1.fc44', 'x86_64', '2' * 64, '3' * 64]]}
    write('release.json', release)
    package_bytes = b'example-1-1.fc44.x86_64\n'
    (work / 'packages.txt').write_bytes(package_bytes)
    provenance = {**release, 'schema_version': 2, 'format': 'kedra-candidate-provenance',
                  'packages_sha256': hashlib.sha256(package_bytes).hexdigest(), 'resolved_inputs': inputs}
    write('provenance.json', provenance)
    expected_hash = hashlib.sha256((work / 'release.json').read_bytes()).hexdigest()
    checksum_bytes = ''.join(hashlib.sha256((work / name).read_bytes()).hexdigest() + '  ' + name + '\n'
                             for name in ('release.json', 'provenance.json', 'packages.txt')).encode()
    (work / 'SHA256SUMS').write_bytes(checksum_bytes)
    signature = native('openssl', 'dgst', '-sha256', '-sign', key, work / 'SHA256SUMS')
    (work / 'SHA256SUMS.sig').write_bytes(base64.b64encode(signature) + b'\n')
    check(inputs, 'no-change')
    for field in ('base', 'source', 'artifacts', 'recipes', 'packages'):
        changed = copy.deepcopy(inputs)
        if field == 'packages':
            changed[field][0][-1] = '4' * 64  # Same NEVRA, different payload.
        elif field == 'base':
            changed[field] = 'quay.io/fedora/fedora-bootc@sha256:' + '5' * 64
        else:
            changed[field]['new-content'] = '6' * 64
        check(changed, 'candidate')
    check(inputs, valid=False, release_hash='0' * 64)
    original = (work / 'provenance.json').read_bytes()
    (work / 'provenance.json').write_bytes(original + b' ')
    check(inputs, valid=False)
    (work / 'provenance.json').write_bytes(original)
    (work / 'packages.txt').write_bytes(b'example-2-1.fc44.x86_64\n')
    check(inputs, valid=False)
    (work / 'packages.txt').write_bytes(package_bytes)
    (work / 'SHA256SUMS.sig').write_bytes(base64.b64encode(bytes(len(signature))))
    check(inputs, valid=False)
    print('PASS: signed material no-change, five changed-input cases, four authentication refusals')
finally:
    if key.exists():
        key.unlink()
