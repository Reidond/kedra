"""End-to-end image-input CLI checks using real files/processes and OpenSSL.

The disposable config signature seals fixture artifacts independently. Native
Skopeo/bootc signature enforcement is qualified by the separate VM workflow.
Both release targets are exercised with generated identities, including
cross-target, wrong-architecture and wrong-repository refusals.
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
workdir = args.workdir.resolve()
workdir.mkdir(parents=True, exist_ok=False)
cli = Path(__file__).resolve().parents[1] / 'build/release/material.py'
qualified_bootc = json.loads(cli.with_name('compatibility.json').read_text())['bootc_version']
private, public = workdir / 'disposable.key', workdir / 'public.pem'
# Independent expectations for the closed target table.
TARGETS = {'desktop': {'architecture': 'x86_64', 'oci': 'amd64', 'repository': 'ghcr.io/reidond/kedra-desktop'},
           'utm': {'architecture': 'aarch64', 'oci': 'arm64', 'repository': 'ghcr.io/reidond/kedra-utm'}}


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def native(*arguments):
    return subprocess.check_output(list(map(str, arguments)), stderr=subprocess.PIPE)


def call(*arguments, expected=True, reason=None):
    result = subprocess.run([sys.executable, str(cli), *map(str, arguments)], capture_output=True)
    if expected and result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    if not expected and result.returncode == 0:
        raise RuntimeError('Unsafe artifact was accepted by the image-input CLI')
    if not expected and reason is not None and reason not in result.stderr.decode(errors='replace'):
        raise RuntimeError('Refused for an unexpected reason: ' + result.stderr.decode(errors='replace')[-600:])
    return json.loads(result.stdout) if expected else None


def exercise(target):
    spec = TARGETS[target]
    other = next(name for name in TARGETS if name != target)
    root = workdir / target
    root.mkdir()
    signature = root / 'config.sig'

    def put(name, value):
        (root / name).write_bytes(encode(value))

    def seal(value, oci=spec['oci']):
        put('identity.json', value)
        put('config.json', {'os': 'linux', 'architecture': oci,
                           'config': {'Labels': {'org.kedra.image.identity': encode(value).decode()}}})
        native('openssl', 'dgst', '-sha256', '-sign', private, '-out', signature, root / 'config.json')
        native('openssl', 'dgst', '-sha256', '-verify', public, '-signature', signature, root / 'config.json')

    def consistent(source, inputs, **changes):
        # Seal a self-consistent record so only the named scope property differs.
        put('source.json', source)
        put('inputs.json', inputs)
        value = dict(identity, source_manifest_sha256=hashlib.sha256(encode(source)).hexdigest(),
                     resolved_inputs_sha256=hashlib.sha256(encode(inputs)).hexdigest(), **changes)
        seal(value)
        return value

    source = {'schema_version': 1, 'source_revision': 'a' * 40, 'input_scope': 'committed HEAD only',
              'target': {'id': target, 'image': spec['repository'],
                         'architecture': spec['architecture'], 'fedora_release': 44}, 'files': []}
    inputs = {'schema_version': 1, 'base': 'quay.io/fedora/fedora-bootc@sha256:' + 'b' * 64,
              'source': {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')},
              'artifacts': {'sysroot': 'c' * 64}, 'recipes': {'Containerfile': 'd' * 64},
              'packages': [['example', '0', '1', '1.fc44', spec['architecture'], 'e' * 64, 'f' * 64]]}
    identity = {'schema_version': 2, 'project': 'Kedra', 'target': target, 'architecture': spec['architecture'],
                'fedora_release': 44, 'repository': spec['repository'], 'channel': 'stable',
                'source_revision': source['source_revision'],
                'source_manifest_sha256': hashlib.sha256(encode(source)).hexdigest(),
                'resolved_inputs_sha256': hashlib.sha256(encode(inputs)).hexdigest(),
                'workflow': '.github/workflows/release.yml', 'epoch': 1, 'run_number': 10, 'run_attempt': 1,
                'resolved_at': int(time.time()), 'minimum_protocol': 2}
    verify = ['verify', '--target', target, '--config', root / 'config.json', '--identity', root / 'identity.json',
              '--inputs', root / 'inputs.json', '--source', root / 'source.json']
    put('source.json', source)
    put('inputs.json', inputs)
    seal(identity)
    result = call(*verify)
    assert result['material_verified'] and result['identity']['target'] == target
    assert result['identity']['architecture'] == spec['architecture'] and result['identity']['repository'] == spec['repository']
    # The same sealed image is not valid for the other target's channel.
    call(*[other if value == target else value for value in verify], expected=False, reason='Image identity scope differs')

    bootc_inputs = copy.deepcopy(inputs)
    bootc_inputs['packages'].append(['bootc', '0', qualified_bootc, '1.fc44', spec['architecture'], '1' * 64, '2' * 64])
    put('bootc-inputs.json', bootc_inputs)
    assert call('bootc', '--target', target, '--inputs', root / 'bootc-inputs.json')['compatible']
    call('bootc', '--target', other, '--inputs', root / 'bootc-inputs.json', expected=False, reason='Unqualified bootc package')
    bootc_inputs['packages'][-1][4] = TARGETS[other]['architecture']
    put('bootc-inputs.json', bootc_inputs)
    call('bootc', '--target', target, '--inputs', root / 'bootc-inputs.json', expected=False, reason='Unqualified bootc package')
    bootc_inputs['packages'][-1][4] = spec['architecture']
    bootc_inputs['packages'][-1][2] = '1.17.0'
    put('bootc-inputs.json', bootc_inputs)
    call('bootc', '--target', target, '--inputs', root / 'bootc-inputs.json', expected=False)
    put('bootc-inputs.json', inputs)
    call('bootc', '--target', target, '--inputs', root / 'bootc-inputs.json', expected=False)

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
    # A retained identity from the other target's repository is never an ordering floor here.
    put('previous-identity.json', dict(identity, run_number=9, target=other, architecture=TARGETS[other]['architecture'],
                                       repository=TARGETS[other]['repository']))
    call(*verify, *ordering, expected=False, reason='Image identity scope differs')
    put('previous-identity.json', identity)
    seal(dict(identity, run_number=11, resolved_at=identity['resolved_at'] - 1))
    call(*verify, *ordering, expected=False)
    seal(identity)

    for field, value in [('target', 'xps'), ('target', other), ('architecture', TARGETS[other]['architecture']),
                         ('repository', TARGETS[other]['repository']), ('repository', spec['repository'] + '-builds'),
                         ('channel', 'other'), ('minimum_protocol', 3), ('resolved_at', int(time.time()) + 600)]:
        seal(dict(identity, **{field: value}))
        call(*verify, expected=False)
    # Other target's complete identity, correctly sealed, is still refused for this channel.
    seal(dict(identity, target=other, architecture=TARGETS[other]['architecture'], repository=TARGETS[other]['repository']))
    call(*verify, expected=False, reason='Image identity scope differs')
    seal(identity, oci=TARGETS[other]['oci'])
    call(*verify, expected=False, reason='Wrong OCI platform')
    seal(dict(identity, resolved_at=1))
    assert call(*verify)['material_verified']  # No fabricated checkpoint expiry.

    # Wrong-architecture and wrong-repository material, each sealed consistently.
    foreign = copy.deepcopy(inputs)
    foreign['packages'].append(['foreign', '0', '1', '1.fc44', TARGETS[other]['architecture'], '6' * 64, '7' * 64])
    foreign['packages'].sort()
    consistent(source, foreign)
    call(*verify, expected=False, reason='RPM architecture is not allowed')
    multilib = copy.deepcopy(inputs)
    multilib['packages'].append(['multilib', '0', '1', '1.fc44', 'i686', '6' * 64, '7' * 64])
    multilib['packages'].sort()
    consistent(source, multilib)
    if spec['architecture'] == 'x86_64':
        assert call(*verify)['material_verified']  # i686 multilib stays x86_64-only.
    else:
        call(*verify, expected=False, reason='RPM architecture is not allowed')
    noarch = copy.deepcopy(inputs)
    noarch['packages'].append(['shared', '0', '1', '1.fc44', 'noarch', '6' * 64, '7' * 64])
    noarch['packages'].sort()
    consistent(source, noarch)
    assert call(*verify)['material_verified']
    for key, value in [('image', TARGETS[other]['repository']), ('architecture', TARGETS[other]['architecture']),
                       ('id', other)]:
        moved = copy.deepcopy(source)
        moved['target'][key] = value
        moved_inputs = dict(copy.deepcopy(inputs), source={k: v for k, v in moved.items()
                                                           if k not in ('source_revision', 'input_scope')})
        consistent(moved, moved_inputs)
        call(*verify, expected=False, reason='Source provenance differs')

    put('source.json', source)
    put('inputs.json', inputs)
    seal(identity)
    (root / 'config.json').write_bytes((root / 'config.json').read_bytes() + b' ')
    rejected = subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(public), '-signature',
                               str(signature), str(root / 'config.json')], capture_output=True)
    assert rejected.returncode != 0
    put('identity.json', dict(identity, run_attempt=2))
    call(*verify, expected=False)
    print(f'PASS: {target} sealed image-input CLI, no-change, content changes, rank/replay and tamper/scope refusals')
    print(f'PASS: {target} cross-target, wrong-architecture/OCI-platform/repository and RPM-architecture refusals')


try:
    native('openssl', 'genpkey', '-algorithm', 'EC', '-pkeyopt', 'ec_paramgen_curve:P-256', '-out', private)
    private.chmod(0o600)
    native('openssl', 'pkey', '-in', private, '-pubout', '-out', public)
    for name in TARGETS:
        exercise(name)
    unknown = subprocess.run([sys.executable, str(cli), 'bootc', '--target', 'xps', '--inputs', str(public)],
                             capture_output=True)
    assert unknown.returncode != 0 and not unknown.stdout, 'Unknown target was accepted'
    print('PASS: unknown release target refused by the image-input CLI')
finally:
    if private.exists():
        private.unlink()
