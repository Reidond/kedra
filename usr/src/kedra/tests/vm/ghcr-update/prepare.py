#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Generate public image identities for the disposable fixed-domain native test."""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--root', type=Path, required=True, help='The fixture root that run.sh created')
parser.add_argument('--source-revision', help='Committed checkout revision recorded in the fixture source')
parser.add_argument('--target', choices=('desktop', 'qemu-arm64'), default='desktop')
parser.add_argument('--binaries', type=Path, default=Path('target/release'),
                    help='Directory holding the Linux sysroot and sysroot-helper the fixture installs')
parser.add_argument('stage', nargs='?', choices=('finalize',))
args = parser.parse_args()
root = args.root.resolve()
assert root.name == 'kedra-ghcr' and root.is_dir()
context = root / 'context'
private = root.parent / 'kedra-ghcr-private'
ARCHITECTURES = {'desktop': 'x86_64', 'qemu-arm64': 'aarch64'}
scope = {'target': args.target, 'architecture': ARCHITECTURES[args.target], 'fedora_release': 44,
         'repository': 'ghcr.io/reidond/kedra-' + args.target}


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def write(name, value, pretty=False):
    data = (json.dumps(value, sort_keys=True, indent=2) + '\n').encode() if pretty else canonical(value)
    (context / name).write_bytes(data)
    return data


if args.stage == 'finalize':
    material = json.loads((context / 'resolved-inputs.json').read_bytes())
    rows = [line.split('\t') for line in (root / 'package-material.txt').read_text().splitlines()]
    material['packages'] = sorted(row for row in rows if row[0] != 'gpg-pubkey')
    assert material['packages'] and all(len(row) == 7 for row in material['packages'])
    material_bytes = write('resolved-inputs.json', material)
    for path in context.glob('*/image-identity.json'):
        value = json.loads(path.read_bytes())
        value['resolved_inputs_sha256'] = hashlib.sha256(material_bytes).hexdigest()
        path.write_bytes(canonical(value))
    raise SystemExit(0)


der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(private / 'allowed.pub'), '-outform', 'DER'])
shutil.copyfile(private / 'allowed.pub', context / 'release.pub')
write('release-policy.json', {'schema_version': 1, 'scope': scope, 'key_fingerprint': hashlib.sha256(der).hexdigest()}, True)
requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
               'signedIdentity': {'type': 'exactRepository', 'dockerRepository': scope['repository']}}
write('policy.json', {'default': [{'type': 'reject'}], 'transports': {
    'docker': {scope['repository']: [requirement]}, 'containers-storage': {'': [requirement]}}}, True)
(context / 'registries.yaml').write_text('docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n')
(context / 'hosts').write_text('127.0.0.1 localhost\n::1 localhost\n10.0.2.2 ghcr.io\n')
(context / 'install.toml').write_text('[install]\nenforce-container-sigpolicy = true\n')
assert args.source_revision is not None and re.fullmatch('[a-f0-9]{40}', args.source_revision)
source = {'schema_version': 1, 'source_revision': args.source_revision, 'input_scope': 'generated-native-ghcr-fixture',
          'target': {'id': scope['target'], 'architecture': scope['architecture'], 'image': scope['repository'],
                     'fedora_release': 44, 'candidate_target': True, 'hardware_status': 'disposable VM'},
          'files': [], 'packages': [], 'remove_packages': []}
source_bytes = write('source.json', source, True)
material = {'schema_version': 1, 'base': (root / 'base.txt').read_text().strip(),
            'source': {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')},
            'artifacts': {'sysroot': hashlib.sha256((args.binaries / 'sysroot').read_bytes()).hexdigest(),
                          'helper': hashlib.sha256((args.binaries / 'sysroot-helper').read_bytes()).hexdigest()},
            'recipes': {'fixture': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}, 'packages': []}
material_bytes = write('resolved-inputs.json', material)
identity = {'schema_version': 2, 'project': 'Kedra', **scope, 'channel': 'stable',
            'source_revision': source['source_revision'], 'source_manifest_sha256': hashlib.sha256(source_bytes).hexdigest(),
            'resolved_inputs_sha256': hashlib.sha256(material_bytes).hexdigest(), 'workflow': '.github/workflows/release.yml',
            'epoch': 1, 'run_number': 10, 'run_attempt': 1, 'resolved_at': int(time.time()), 'minimum_protocol': 2}
# D is a correctly signed image older than A: the helper's downgrade refusal, not bootc's.
other_architecture = 'aarch64' if scope['architecture'] == 'x86_64' else 'x86_64'
variants = {'A': {}, 'B': {'run_number': 20}, 'C': {'run_number': 30}, 'D': {'run_number': 5}, 'E': {},
            'U': {'run_number': 40}, 'W': {'run_number': 40}, 'R': {'run_number': 40}, 'N': {'run_number': 40},
            'T': {'target': 'xps'}, 'X': {'architecture': other_architecture}, 'H': {'channel': 'other'},
            'M': {'unexpected': True}}
for name, changes in variants.items():
    (context / name).mkdir()
    write(name + '/image-identity.json', {**identity, **changes})
