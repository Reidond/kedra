#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Build and cache a disposable ARM64 QCOW2 through the existing pinned bootc builder."""
import argparse
import base64
import fcntl
import hashlib
import json
import os
import re
import secrets
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
OWNER = 'dev.kedra.lab.owner=kedra-container-tests'


def run(argv, **kwargs):
    return subprocess.run(argv, check=True, timeout=kwargs.pop('timeout', 120), **kwargs)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def capture_public(container, output):
    names = ['signing.json', 'fixture-policy.json', 'production-policy.json', 'fixture-signing.pub',
             'signing-versions.txt', 'target-trust-before.json', 'target-trust-after.json',
             'imported-native.json', 'imported-image.json', 'imported-manifest-digest',
             'boot-reference', 'buildroot-image.json', 'buildroot.Containerfile', 'buildroot-build.log',
             'wrong.digest', 'allowed.digest', 'sign-wrong.log', 'sign-allowed.log',
             'verify-allowed-ordinary.manifest.json', 'verify-allowed-bib.manifest.json']
    for phase in ('unsigned', 'wrong', 'allowed'):
        names += [phase + '-image.json', phase + '-native.json']
        for scope in ('ordinary', 'bib'):
            names += ['verify-' + phase + '-' + scope + suffix for suffix in ('.exit', '.stdout', '.stderr')]
    for name in names:
        path = '/output/' + name
        observed = subprocess.run(['docker', 'exec', container, 'stat', '-c', '%F\n%h\n%s', path],
                                  check=False, capture_output=True, text=True, timeout=30)
        fields = observed.stdout.splitlines()
        if observed.returncode or len(fields) != 3 or fields[0] != 'regular file' or fields[1] != '1':
            continue
        if int(fields[2]) > 8 * 1024 * 1024:
            continue
        run(['docker', 'cp', container + ':' + path, str(output / name)])


def validate_signing(output, reference, native_observation, imported_digest):
    receipt = json.loads((output / 'signing.json').read_text())
    imported = json.loads((output / 'imported-image.json').read_text())[0]
    target_id = imported['Id'].removeprefix('sha256:')
    scopes = ['[overlay@/var/lib/containers/storage]@' + target_id,
              '[overlay@/run/osbuild/containers/storage2]@' + target_id]
    signed_digest = receipt.get('signed_manifest_digest', '')
    if (receipt.get('schema_version') != 1 or receipt.get('signed_reference') != reference
            or receipt.get('target_storage_id') != target_id or receipt.get('scopes') != scopes
            or receipt.get('imported_manifest_digest') != imported_digest
            or not re.fullmatch(r'sha256:[a-f0-9]{64}', signed_digest)
            or receipt.get('boot_reference') != reference.removesuffix(':boot') + '@' + signed_digest
            or receipt.get('consumer_policy') != 'default'
            or receipt.get('private_keys_removed') is not True
            or receipt.get('target_production_trust') != 'unchanged'
            or receipt.get('admission') != {'unsigned': 'refused', 'wrong_key_only': 'refused', 'allowed_key_only': 'pass'}):
        raise ValueError('signed fixture admission binding differs')
    key = (output / 'fixture-signing.pub').read_text()
    if len(key) > 16384 or not key.startswith('-----BEGIN PUBLIC KEY-----\n') or not key.rstrip().endswith('-----END PUBLIC KEY-----'):
        raise ValueError('invalid fixture public key')
    der = base64.b64decode(''.join(key.splitlines()[1:-1]), validate=True)
    hashes = {'policy_sha256': digest(output / 'fixture-policy.json'),
              'buildroot_recipe_sha256': digest(output / 'buildroot.Containerfile'),
              'public_key_sha256': digest(output / 'fixture-signing.pub'),
              'key_fingerprint_sha256_spki': hashlib.sha256(der).hexdigest(),
              'target_trust_sha256': digest(output / 'target-trust-before.json')}
    if any(receipt.get(name) != value for name, value in hashes.items()):
        raise ValueError('fixture signing public material changed')
    baseline = json.loads((output / 'production-policy.json').read_text())
    if digest(output / 'production-policy.json') != native_observation['production_trust']['/etc/containers/policy.json']['sha256']:
        raise ValueError('signing buildroot used a different production policy')
    policy = json.loads((output / 'fixture-policy.json').read_text())
    requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/share/kedra-lab/fixture-signing.pub',
                   'signedIdentity': {'type': 'exactReference', 'dockerReference': reference}}
    for scope in scopes:
        if policy['transports']['containers-storage'].pop(scope, None) != [requirement]:
            raise ValueError('fixture verification scope differs')
    if policy != baseline:
        raise ValueError('fixture buildroot changed a production policy entry')
    for phase in ('before', 'after'):
        if json.loads((output / ('target-trust-' + phase + '.json')).read_text()) != native_observation['production_trust']:
            raise ValueError('target production trust changed while signing')
    for phase in ('unsigned', 'wrong', 'allowed'):
        if json.loads((output / (phase + '-native.json')).read_text()) != native_observation:
            raise ValueError('target native material changed while signing')
        for scope in ('ordinary', 'bib'):
            code = int((output / ('verify-' + phase + '-' + scope + '.exit')).read_text())
            if (code == 0) != (phase == 'allowed'):
                raise ValueError('fixture signature admission outcome differs')
    for scope in ('ordinary', 'bib'):
        if 'sha256:' + digest(output / ('verify-allowed-' + scope + '.manifest.json')) != signed_digest:
            raise ValueError('admitted manifest bytes differ from the signed fixture')
    final = json.loads((output / 'allowed-image.json').read_text())[0]
    if final['Id'].removeprefix('sha256:') != target_id or final['Digest'] != signed_digest:
        raise ValueError('signed target image differs')
    root = json.loads((output / 'buildroot-image.json').read_text())[0]
    if (root['Id'].removeprefix('sha256:') != receipt.get('buildroot_id', '').removeprefix('sha256:')
            or root['Digest'] != receipt.get('buildroot_manifest_digest')
            or receipt.get('buildroot_reference') != 'localhost/kedra-qemu-buildroot@' + root['Digest']
            or len(root['RootFS']['Layers']) <= len(final['RootFS']['Layers'])
            or root['RootFS']['Layers'][:len(final['RootFS']['Layers'])] != final['RootFS']['Layers']
            or root['Config']['Labels'].get('dev.kedra.lab.kind') != 'qemu-signing-buildroot'):
        raise ValueError('signing buildroot lineage differs')
    return receipt


def main():
    signal.signal(signal.SIGTERM, lambda _signal, _frame: sys.exit(130))
    signal.signal(signal.SIGINT, lambda _signal, _frame: sys.exit(130))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--image-id', required=True)
    parser.add_argument('--engine-id', required=True)
    parser.add_argument('--cache', required=True, type=Path)
    parser.add_argument('--native-provenance', action='store_true', help='Read verified native provenance from stdin')
    parser.add_argument('--fixture-reference')
    args = parser.parse_args()
    native = None
    if args.native_provenance:
        data = sys.stdin.buffer.read(8 * 1024 * 1024 + 1)
        if len(data) > 8 * 1024 * 1024:
            parser.error('native provenance exceeds 8 MiB')
        native = json.loads(data)
        if not isinstance(native, dict) or set(native) != {'image', 'engine', 'parent_image', 'material'}:
            parser.error('invalid native provenance tuple')
        if not args.fixture_reference or not re.fullmatch(r'localhost/kedra-qemu-fixture/[a-f0-9]{32}:boot', args.fixture_reference):
            parser.error('native disk requires an independently selected fixture reference')
    elif args.fixture_reference:
        parser.error('fixture reference requires native provenance')
    endpoint = os.environ.get('DOCKER_HOST', 'unix:///var/run/docker.sock')
    if not endpoint.startswith('unix://'):
        parser.error('native disk preparation requires a local Unix Docker socket')
    os.environ['DOCKER_HOST'] = endpoint
    os.environ.pop('DOCKER_CONTEXT', None)
    engine_id = run(['docker', 'info', '--format', '{{.ID}}'], capture_output=True, text=True).stdout.strip()
    if engine_id != args.engine_id:
        parser.error('Docker CLI context differs from the Testcontainers engine; select the same local engine')
    args.cache.mkdir(parents=True, exist_ok=True)
    identity = json.loads(run(['docker', 'image', 'inspect', args.image], capture_output=True).stdout)[0]
    if identity['Id'] != args.image_id or (identity['Os'], identity['Architecture']) != ('linux', 'arm64'):
        parser.error('native VM needs a linux/arm64 fixture image')
    if identity['Config'].get('Labels', {}).get('dev.kedra.lab.owner') != 'kedra-container-tests':
        parser.error('image must be prepared by kedra-lab')
    if identity['Config'].get('Labels', {}).get('dev.kedra.lab.kind') != 'qemu':
        parser.error('image must be the native QEMU fixture, without container adaptations')
    native_observation = None
    observer = (HERE / 'boot-check.py').read_text()
    observer_sha = hashlib.sha256(observer.encode()).hexdigest()
    if native is not None:
        if identity['Config']['Labels'].get('dev.kedra.lab.fixture-reference') != args.fixture_reference:
            parser.error('fixture reference differs from immutable image label')
        if native['engine'] != engine_id or not re.fullmatch(r'sha256:[a-f0-9]{64}', native['image']):
            parser.error('native provenance names a different engine or invalid image')
        parent = json.loads(run(['docker', 'image', 'inspect', native['image']], capture_output=True).stdout)[0]
        layers = parent['RootFS']['Layers']
        if (parent['Id'] != native['image'] or len(identity['RootFS']['Layers']) <= len(layers)
                or identity['RootFS']['Layers'][:len(layers)] != layers):
            parser.error('fixture does not extend the selected native image')
        native_observation = json.loads(run([
            'docker', 'run', '--rm', '--network', 'none', '--read-only', '--cap-drop=ALL',
            '--security-opt', 'no-new-privileges', '--tmpfs', '/tmp:rw,nosuid,nodev,noexec,size=64m',
            '--entrypoint', '/usr/bin/python3', args.image_id, '-I', '-c', observer, 'image',
        ], capture_output=True).stdout)
        if native_observation.get('provenance') != native or native_observation.get('passed') is not True:
            parser.error('fixture native-material readback differs from independent selection')
    builder = json.loads((ROOT / 'usr/src/kedra/image/inputs.json').read_text())['platforms']['arm64']['builder']
    metadata = {'Architecture': identity['Architecture'], 'Os': identity['Os'], 'RootFS': identity['RootFS']['Layers'],
                'Config': {name: identity['Config'].get(name) for name in ['Labels', 'Env', 'Cmd', 'Entrypoint', 'User', 'WorkingDir']}}
    metadata_sha = hashlib.sha256(json.dumps(metadata, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
    inputs = {'image_id': identity['Id'], 'builder': builder, 'recipe': digest(Path(__file__)),
              'metadata_sha256': metadata_sha,
              'containerfile': digest(HERE / 'disk-builder.Containerfile'),
              'script': digest(HERE / 'build-disk.sh'), 'signing_script': digest(HERE / 'sign-fixture.sh')}
    if native is not None:
        inputs['native'] = native
        inputs['native_receipt_sha256'] = native_observation['native_receipt_sha256']
        inputs['boot_observer_sha256'] = observer_sha
        inputs['fixture_reference'] = args.fixture_reference
    key = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()
    output = args.cache / 'images' / key
    output.mkdir(parents=True, exist_ok=True)
    # Nested Podman uses VM-wide shared memory; serialize these builds in this checkout.
    with (args.cache / 'image-build.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        disk, receipt = output / 'base.qcow2', output / 'image.json'
        if disk.is_file() and receipt.is_file():
            saved = json.loads(receipt.read_text())
            signing = None if native is None else validate_signing(
                output, args.fixture_reference, native_observation, saved.get('imported_manifest_digest', ''))
            if (saved.get('inputs') == inputs and saved.get('disk_sha256') == digest(disk)
                    and saved.get('native_observation') == native_observation
                    and saved.get('signing') == signing
                    and re.fullmatch(r'sha256:[a-f0-9]{64}', saved.get('imported_manifest_digest', ''))):
                print(receipt)
                return
            raise ValueError(f'cached image changed; inspect {output} before removing it')
        tag = 'kedra-qemu-builder:' + hashlib.sha256(''.join(
            inputs[name] for name in ('containerfile', 'script', 'signing_script')).encode()).hexdigest()[:32]
        run(['docker', 'build', '--platform', 'linux/arm64', '-f', str(HERE / 'disk-builder.Containerfile'),
             '-t', tag, str(HERE)], timeout=1800)
        token = secrets.token_hex(8)
        container = 'kedra-qemu-builder-' + token
        volume = 'kedra-qemu-output-' + token
        run(['docker', 'volume', 'create', '--label', OWNER, volume], stdout=subprocess.DEVNULL)
        started = time.monotonic()
        try:
            run(['docker', 'run', '-d', '--name', container, '--label', OWNER, '--privileged',
                 '--cgroupns', 'private', '--platform', 'linux/arm64',
                 '--tmpfs', '/run/kedra-signing:rw,nosuid,nodev,noexec,size=16m,mode=0700',
                 '--mount', 'type=volume,source=kedra-qemu-podman-cache,target=/var/lib/containers',
                 '--mount', f'type=volume,source={volume},target=/output', tag], stdout=subprocess.DEVNULL)
            if native is not None:
                run(['docker', 'cp', str(HERE / 'boot-check.py'), container + ':/tmp/kedra-native-boot-check.py'])
            # Stream the exact local fixture into the private builder; no registry publication.
            with (output / 'build.log').open('wb') as log:
                producer = subprocess.Popen(['docker', 'save', identity['Id']], stdout=subprocess.PIPE, stderr=log)
                try:
                    reference = args.fixture_reference or 'localhost/kedra-qemu-fixture:' + identity['Id'].removeprefix('sha256:')
                    result = subprocess.run(['docker', 'exec', '-i', container, 'kedra-build-disk', reference, builder, metadata_sha,
                                             'native' if native is not None else 'ordinary', observer_sha],
                                            stdin=producer.stdout, stdout=log, stderr=log, timeout=7200, check=False)
                    producer.stdout.close()
                    if result.returncode or producer.wait(timeout=30):
                        if native is not None:
                            capture_public(container, output)
                        raise ValueError(f'disk build failed; see {output / "build.log"}')
                finally:
                    if producer.poll() is None:
                        producer.terminate()
                        producer.wait(timeout=30)
            pending = output / 'base.pending.qcow2'
            run(['docker', 'cp', container + ':/output/base.qcow2', str(pending)], timeout=1800)
            run(['docker', 'cp', container + ':/output/imported-image.json', str(output / 'imported-image.json')])
            run(['docker', 'cp', container + ':/output/imported-manifest-digest', str(output / 'imported-manifest-digest')])
            imported_digest = (output / 'imported-manifest-digest').read_text().strip()
            if not re.fullmatch(r'sha256:[a-f0-9]{64}', imported_digest):
                raise ValueError('builder did not retain the imported manifest digest')
            if native is not None:
                capture_public(container, output)
                if json.loads((output / 'imported-native.json').read_text()) != native_observation:
                    raise ValueError('imported image changed verified native material')
            signing = None if native is None else validate_signing(
                output, args.fixture_reference, native_observation, imported_digest)
            saved = {'schema_version': 1, 'target': 'qemu-arm64', 'inputs': inputs, 'disk': str(disk),
                     'disk_sha256': digest(pending), 'preparation_seconds': time.monotonic() - started,
                     'imported_image': json.loads((output / 'imported-image.json').read_text()),
                     'imported_manifest_digest': imported_digest,
                     'fixture_reference': reference if signing is None else signing['boot_reference'],
                     'deployment_manifest_digest': imported_digest if signing is None else signing['signed_manifest_digest'],
                     'native_observation': native_observation, 'signing': signing}
            os.replace(pending, disk)
            temporary = output / 'image.pending.json'
            temporary.write_text(json.dumps(saved, indent=2) + '\n')
            os.replace(temporary, receipt)
            print(receipt)
        finally:
            subprocess.run(['docker', 'rm', '-f', container], timeout=60, check=False, stdout=subprocess.DEVNULL)
            subprocess.run(['docker', 'volume', 'rm', volume], timeout=60, check=False, stdout=subprocess.DEVNULL)


if __name__ == '__main__':
    main()
