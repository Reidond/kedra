#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Write generated authority/material for a non-promotable full ARM fixture."""
import argparse
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

from fixture import add_context_argument, load_context

ROOT = Path(__file__).resolve().parents[6]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'usr/src/kedra/image/release'))
import material as m


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    add_context_argument(parser)
    args = parser.parse_args()
    fixture = load_context(args.fixture_context)
    root, context = args.root.resolve(), args.root.resolve() / 'context'
    m.require(root == Path(fixture['runner_temp']) / 'kedra-ghcr', 'Wrong fixture directory')
    private = Path(fixture['runner_temp']) / 'kedra-ghcr-private'
    selected = m.document(m.read(root / 'candidate/candidate.json'))
    m.require(selected['target'] == 'qemu-arm64' and selected['source_revision'] == fixture['source_revision']
              and selected['signed'] is False and selected['production_publication'] is False,
              'Expected the full independently selected unsigned ARM candidate')
    source = m.document(m.read(root / 'source-plan.json'))
    source['input_scope'] = 'generated-full-arm-release-fixture'
    material = m.document(m.read(root / 'candidate/resolved-inputs.json'))
    material['source'] = {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')}
    scope = {'target': 'qemu-arm64', 'architecture': 'aarch64', 'fedora_release': 44,
             'repository': 'ghcr.io/reidond/kedra-qemu-arm64'}
    key = private / 'allowed.pub'
    der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(key), '-outform', 'DER'], timeout=30)
    fingerprint = m.sha(der)
    production = (ROOT / 'usr/src/kedra/image/release/authority/qemu-arm64.sha256').read_text().strip()
    m.require(fingerprint != production, 'Fixture must not use the production authority')
    shutil.copyfile(key, context / 'release.pub')
    requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
                   'signedIdentity': {'type': 'exactRepository', 'dockerRepository': scope['repository']}}

    def write(name, value, pretty=False):
        data = (json.dumps(value, sort_keys=True, indent=2) + '\n').encode() if pretty else m.canonical(value)
        (context / name).write_bytes(data)
        return data

    write('release-policy.json', {'schema_version': 1, 'scope': scope, 'key_fingerprint': fingerprint}, pretty=True)
    write('policy.json', {'default': [{'type': 'reject'}], 'transports': {
        'docker': {scope['repository']: [requirement]}, 'containers-storage': {'': [requirement]}}}, pretty=True)
    (context / 'registries.yaml').write_text('docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n')
    (context / 'install.toml').write_text('[install]\nenforce-container-sigpolicy = true\n')
    source_bytes = write('source.json', source, pretty=True)
    material['artifacts']['disposable-public-authority'] = m.sha(key.read_bytes())
    for name in ('prepare-arm64.py', 'arm64.Containerfile', 'arm64-variant.Containerfile',
                 'check.py', 'identity-recovery.py', 'arm64-check.service'):
        material['recipes']['fixture/' + name] = m.sha(Path(__file__).with_name(name).read_bytes())
    observer_hash = m.sha((ROOT / 'usr/src/kedra/tests/container/qemu/boot-check.py').read_bytes())
    material['recipes']['fixture/native-observer.py'] = observer_hash
    material_bytes = write('resolved-inputs.json', material)
    identity = {'schema_version': 2, 'project': 'Kedra', **scope, 'channel': 'stable',
                'source_revision': source['source_revision'], 'source_manifest_sha256': m.sha(source_bytes),
                'resolved_inputs_sha256': m.sha(material_bytes), 'workflow': '.github/workflows/release.yml',
                'epoch': 1, 'run_number': 10, 'run_attempt': 1, 'resolved_at': int(time.time()), 'minimum_protocol': 2}
    variants = {'A': {}, 'B': {'run_number': 20}, 'C': {'run_number': 30}, 'E': {},
                'U': {'run_number': 40}, 'W': {'run_number': 40}, 'R': {'run_number': 40},
                'N': {'run_number': 40}, 'T': {'target': 'xps'}, 'X': {'architecture': 'x86_64'},
                'H': {'channel': 'other'}, 'M': {'unexpected': True}}
    for name, changes in variants.items():
        (context / name).mkdir()
        write(name + '/image-identity.json', {**identity, **changes})
    (root / 'fixture-authority.json').write_text(json.dumps({
        'schema_version': 1, 'scope': scope, 'fingerprint': fingerprint,
        'production_authority_used': False, 'production_publication': False,
        'native_receipt_sha256': selected['candidate']['native_receipt_sha256'],
        'native_observer_sha256': observer_hash,
        'source_revision': source['source_revision'],
        'variant_scope': 'shared verified native output; distinct signed fixture marker and rank',
    }, sort_keys=True, indent=2) + '\n')


if __name__ == '__main__':
    main()
