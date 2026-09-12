"""End-to-end image-input CLI checks using real files/processes and OpenSSL.

The disposable config signature seals fixture artifacts independently. Native
Skopeo/bootc signature enforcement is qualified by the separate VM workflow.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

parser = argparse.ArgumentParser()
parser.add_argument('--workdir', type=Path, required=True)
args = parser.parse_args()
root = args.workdir.resolve()
root.mkdir(parents=True, exist_ok=False)
cli = Path(__file__).resolve().parents[1] / 'build/release/material.py'
private, public = root / 'disposable.key', root / 'public.pem'
signature = root / 'config.sig'


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def put(name, value):
    (root / name).write_bytes(encode(value))


def native(*arguments):
    return subprocess.check_output(list(map(str, arguments)), stderr=subprocess.PIPE)


def call(*arguments, expected=True):
    result = subprocess.run([sys.executable, str(cli), *map(str, arguments)], capture_output=True)
    if expected and result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    if not expected and result.returncode == 0:
        raise RuntimeError('Unsafe artifact was accepted by the image-input CLI')
    return json.loads(result.stdout) if expected else None


source = {'schema_version': 1, 'source_revision': 'a' * 40, 'input_scope': 'committed HEAD only',
          'target': {'id': 'desktop', 'image': 'ghcr.io/reidond/kedra-desktop',
                     'architecture': 'x86_64', 'fedora_release': 44}, 'files': []}
inputs = {'schema_version': 1, 'base': 'quay.io/fedora/fedora-bootc@sha256:' + 'b' * 64,
          'source': {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')},
          'artifacts': {'sysroot': 'c' * 64}, 'recipes': {'Containerfile': 'd' * 64},
          'packages': [['example', '0', '1', '1.fc44', 'x86_64', 'e' * 64, 'f' * 64]]}
identity = {'schema_version': 2, 'project': 'Kedra', 'target': 'desktop', 'architecture': 'x86_64',
            'fedora_release': 44, 'repository': 'ghcr.io/reidond/kedra-desktop', 'channel': 'stable',
            'source_revision': source['source_revision'], 'source_manifest_sha256': hashlib.sha256(encode(source)).hexdigest(),
            'resolved_inputs_sha256': hashlib.sha256(encode(inputs)).hexdigest(),
            'workflow': '.github/workflows/release.yml', 'epoch': 1, 'run_number': 10, 'run_attempt': 1,
            'resolved_at': int(time.time()), 'minimum_protocol': 2}
verify = ['verify', '--config', root / 'config.json', '--identity', root / 'identity.json',
          '--inputs', root / 'inputs.json', '--source', root / 'source.json']


def seal(value):
    put('identity.json', value)
    put('config.json', {'os': 'linux', 'architecture': 'amd64',
                       'config': {'Labels': {'org.kedra.image.identity': encode(value).decode()}}})
    native('openssl', 'dgst', '-sha256', '-sign', private, '-out', signature, root / 'config.json')
    native('openssl', 'dgst', '-sha256', '-verify', public, '-signature', signature, root / 'config.json')


try:
    native('openssl', 'genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:P-256', '-out', private)
    private.chmod(0o600)
    native('openssl', 'pkey', '-in', private, '-pubout', '-out', public)
    put('source.json', source)
    put('inputs.json', inputs)
    seal(identity)
    assert call(*verify)['material_verified']
    bootc_inputs = copy.deepcopy(inputs)
    bootc_inputs['packages'].append(['bootc', '0', '1.16.10', '1.fc44', 'x86_64', '1' * 64, '2' * 64])
    put('bootc-inputs.json', bootc_inputs)
    assert call('bootc', '--inputs', root / 'bootc-inputs.json')['compatible']
    bootc_inputs['packages'][-1][2] = '1.17.0'
    put('bootc-inputs.json', bootc_inputs)
    call('bootc', '--inputs', root / 'bootc-inputs.json', expected=False)
    put('bootc-inputs.json', inputs)
    call('bootc', '--inputs', root / 'bootc-inputs.json', expected=False)
    put('previous-inputs.json', inputs)
    assert call('compare', '--current', root / 'inputs.json', '--previous', root / 'previous-inputs.json')['decision'] == 'unchanged'
    for field in ('base', 'source', 'artifacts', 'recipes', 'packages'):
        changed = copy.deepcopy(inputs)
        if field == 'base':
            changed[field] = 'quay.io/fedora/fedora-bootc@sha256:' + '1' * 64
        elif field == 'packages':
            changed[field][0][-1] = '2' * 64  # Equal NEVRA does not hide changed payload.
        else:
            changed[field]['changed'] = '3' * 64
        put('inputs.json', changed)
        assert call('compare', '--current', root / 'inputs.json', '--previous', root / 'previous-inputs.json')['decision'] == 'changed'
        call(*verify, expected=False)
    put('inputs.json', inputs)
    previous = dict(identity, run_number=9)
    put('previous-identity.json', previous)
    ordering = ['--previous-identity', root / 'previous-identity.json', '--digest', 'sha256:' + '4' * 64,
                '--previous-digest', 'sha256:' + '5' * 64]
    assert call(*verify, *ordering)['ordering'] == 'advance'
    put('previous-identity.json', identity)
    call(*verify, *ordering, expected=False)
    ordering[-1] = 'sha256:' + '4' * 64
    assert call(*verify, *ordering)['ordering'] == 'already-current'
    put('previous-identity.json', dict(identity, run_number=11))
    call(*verify, *ordering, expected=False)
    put('previous-identity.json', identity)
    seal(dict(identity, run_number=11, resolved_at=identity['resolved_at'] - 1))
    call(*verify, *ordering, expected=False)
    seal(identity)
    for field, value in [('target', 'xps'), ('channel', 'other'), ('minimum_protocol', 3), ('resolved_at', int(time.time()) + 600)]:
        seal(dict(identity, **{field: value}))
        call(*verify, expected=False)
    seal(dict(identity, resolved_at=1))
    assert call(*verify)['material_verified']  # No fabricated checkpoint expiry.
    seal(identity)
    (root / 'config.json').write_bytes((root / 'config.json').read_bytes() + b' ')
    rejected = subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(public), '-signature',
                               str(signature), str(root / 'config.json')], capture_output=True)
    assert rejected.returncode != 0
    put('identity.json', dict(identity, run_attempt=2))
    call(*verify, expected=False)
    print('PASS: sealed image-input CLI, no-change, content changes, rank/replay and tamper/scope refusals')
finally:
    if private.exists():
        private.unlink()
