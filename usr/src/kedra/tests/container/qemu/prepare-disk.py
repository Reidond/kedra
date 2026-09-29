#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Build and cache a disposable ARM64 QCOW2 through the existing pinned bootc builder."""
import argparse
import fcntl
import hashlib
import json
import os
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


def main():
    signal.signal(signal.SIGTERM, lambda _signal, _frame: sys.exit(130))
    signal.signal(signal.SIGINT, lambda _signal, _frame: sys.exit(130))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--image-id', required=True)
    parser.add_argument('--engine-id', required=True)
    parser.add_argument('--cache', required=True, type=Path)
    args = parser.parse_args()
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
    builder = json.loads((ROOT / 'usr/src/kedra/image/inputs.json').read_text())['platforms']['arm64']['builder']
    metadata = {'Architecture': identity['Architecture'], 'Os': identity['Os'], 'RootFS': identity['RootFS']['Layers'],
                'Config': {name: identity['Config'].get(name) for name in ['Labels', 'Env', 'Cmd', 'Entrypoint', 'User', 'WorkingDir']}}
    metadata_sha = hashlib.sha256(json.dumps(metadata, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
    inputs = {'image_id': identity['Id'], 'builder': builder, 'recipe': digest(Path(__file__)),
              'metadata_sha256': metadata_sha,
              'containerfile': digest(HERE / 'disk-builder.Containerfile'),
              'script': digest(HERE / 'build-disk.sh')}
    key = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()
    output = args.cache / 'images' / key
    output.mkdir(parents=True, exist_ok=True)
    # Nested Podman uses VM-wide shared memory; serialize these builds in this checkout.
    with (args.cache / 'image-build.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        disk, receipt = output / 'base.qcow2', output / 'image.json'
        if disk.is_file() and receipt.is_file():
            saved = json.loads(receipt.read_text())
            if saved.get('inputs') == inputs and saved.get('disk_sha256') == digest(disk):
                print(receipt)
                return
            raise ValueError(f'cached image changed; inspect {output} before removing it')
        tag = 'kedra-qemu-builder:' + inputs['containerfile'][:16] + inputs['script'][:16]
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
                 '--mount', 'type=volume,source=kedra-qemu-podman-cache,target=/var/lib/containers',
                 '--mount', f'type=volume,source={volume},target=/output', tag], stdout=subprocess.DEVNULL)
            # Stream the exact local fixture into the private builder; no registry publication.
            with (output / 'build.log').open('wb') as log:
                producer = subprocess.Popen(['docker', 'save', identity['Id']], stdout=subprocess.PIPE, stderr=log)
                try:
                    reference = 'localhost/kedra-qemu-fixture:' + identity['Id'].removeprefix('sha256:')
                    result = subprocess.run(['docker', 'exec', '-i', container, 'kedra-build-disk', reference, builder, metadata_sha],
                                            stdin=producer.stdout, stdout=log, stderr=log, timeout=7200, check=False)
                    producer.stdout.close()
                    if result.returncode or producer.wait(timeout=30):
                        raise ValueError(f'disk build failed; see {output / "build.log"}')
                finally:
                    if producer.poll() is None:
                        producer.terminate()
                        producer.wait(timeout=30)
            pending = output / 'base.pending.qcow2'
            run(['docker', 'cp', container + ':/output/base.qcow2', str(pending)], timeout=1800)
            run(['docker', 'cp', container + ':/output/imported-image.json', str(output / 'imported-image.json')])
            saved = {'schema_version': 1, 'target': 'qemu-arm64', 'inputs': inputs, 'disk': str(disk),
                     'disk_sha256': digest(pending), 'preparation_seconds': time.monotonic() - started,
                     'imported_image': json.loads((output / 'imported-image.json').read_text())}
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
