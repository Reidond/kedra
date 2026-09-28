#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Build and verify signed ARM64 installer media on an Apple Silicon Mac."""
import argparse
import hashlib
import json
import os
import platform
import re
import secrets
import shlex
import shutil
import signal
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
KEDRA = 'usr/src/kedra/'
TABLE = json.loads((ROOT / KEDRA / 'image/release/targets.json').read_text())
if TABLE.get('schema_version') != 1:
    raise SystemExit('unsupported release target table')
TARGET = 'qemu-arm64'
SPEC = TABLE['targets'][TARGET]
IMAGE_PATTERN = re.escape(SPEC['repository']) + r'@sha256:([a-f0-9]{64})'
BASE_PATTERN = r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}'
TOOLS = {'shasum': '/usr/bin/shasum'}
STORAGE_VOLUME = 'kedra-macos-media-storage'
LABEL = 'dev.kedra.macos-media'
GIB = 1024**3
MIB = 1024**2
UNSAFE_PATH = set(':,"\n\r')


class ToolError(Exception):
    """An operator-facing refusal or failure; existing files were not overwritten."""


def require(condition, message):
    if not condition:
        raise ToolError(message)


def run(arguments, *, timeout, capture=False, check=True, cwd=None):
    arguments = [str(item) for item in arguments]
    try:
        result = subprocess.run(arguments, stdin=subprocess.DEVNULL, cwd=cwd, timeout=timeout, check=False,
                                stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.PIPE if capture else None)
    except FileNotFoundError:
        raise ToolError('Required program not found: ' + arguments[0]) from None
    except subprocess.TimeoutExpired:
        raise ToolError(f"Timed out after {timeout} s: {' '.join(arguments[:3])}") from None
    if check and result.returncode != 0:
        detail = result.stderr.decode('utf-8', 'replace').strip()[-1500:] if capture else ''
        suffix = '\n' + detail if detail else ''
        raise ToolError(f"Command failed with status {result.returncode}: {' '.join(arguments[:3])}{suffix}")
    return result


def regular(path):
    return path.is_file() and not path.is_symlink()


def read_bounded(path, limit=MIB):
    require(regular(path) and path.stat().st_size <= limit, 'Missing, unsafe or oversized input: ' + str(path))
    return path.read_bytes()


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(4 * MIB), b''):
            digest.update(block)
    return digest.hexdigest()


def free_bytes(path):
    while not path.exists():
        path = path.parent
    return shutil.disk_usage(str(path)).free


def require_mac():
    require(sys.platform == 'darwin' and platform.machine() == 'arm64',
            'Run this on macOS on Apple Silicon (arm64); the qemu-arm64 target is aarch64')
    require(os.geteuid() != 0, 'Run as your ordinary macOS user; this tool never needs sudo')
    require(SPEC['architecture'] == 'aarch64' and SPEC['oci_architecture'] == 'arm64',
            'The shared target table no longer describes an aarch64 qemu-arm64 target')


def docker_cli():
    found = shutil.which('docker')
    candidate = Path(found) if found else Path('/usr/local/bin/docker')
    require(candidate.is_absolute() and candidate.is_file() and os.access(str(candidate), os.X_OK),
            'Docker CLI not found; install and start an arm64 Docker engine (OrbStack was qualified)')
    return str(candidate)


def docker_engine(docker):
    endpoint = os.environ.get('DOCKER_HOST', 'unix:///var/run/docker.sock')
    require(endpoint.startswith('unix://'), 'macOS media builds require a local Unix Docker socket')
    os.environ['DOCKER_HOST'] = endpoint
    os.environ.pop('DOCKER_CONTEXT', None)
    result = run([docker, 'version', '--format', '{{.Server.Os}}/{{.Server.Arch}} {{.Server.Version}}'],
                 timeout=60, capture=True, check=False)
    require(result.returncode == 0, 'The Docker engine is not reachable; start OrbStack or another arm64 engine')
    engine, _, version = result.stdout.decode('utf-8', 'replace').strip().partition(' ')
    require(engine == 'linux/' + SPEC['oci_architecture'],
            f'The Docker engine runs {engine}; a native linux/arm64 engine is required')
    name = run([docker, 'info', '--format', '{{.OperatingSystem}}'], timeout=60, capture=True).stdout
    return {'cli': docker, 'version': version, 'name': name.decode('utf-8', 'replace').strip()}


def installer_record(directory, image=None, iso_name=None):
    """Check build-local.py's records; ISO bytes are hashed by the caller."""
    record = json.loads(read_bounded(directory / 'installer.json'))
    require(isinstance(record, dict), 'installer.json is not an object')
    require(record.get('schema_version') == 1 and record.get('architecture') == SPEC['architecture'],
            'installer.json has an unsupported schema or architecture')
    fingerprint = read_bounded(ROOT / SPEC['key_sha256'], 65).decode('ascii').strip()
    require(record.get('public_key_fingerprint') == fingerprint,
            'installer.json names a different release authority')
    match = re.fullmatch(IMAGE_PATTERN, str(record.get('image', '')))
    require(match, 'installer.json does not describe {} media; this tool installs only the {} target'.format(SPEC['repository'], TARGET))
    require(image is None or record['image'] == image, 'installer.json names a different image than requested')
    require(record.get('target', TARGET) == TARGET and record.get('uploaded') is False,
            'installer.json records an unexpected target or publication state')
    name = f'kedra-{TARGET}-44-{match.group(1)[:16]}.iso'
    installer = record.get('installer')
    require(isinstance(installer, dict) and installer.get('filename') == name
            and isinstance(installer.get('size_bytes'), int)
            and re.fullmatch('[a-f0-9]{64}', str(installer.get('sha256', ''))),
            'installer.json has an unexpected installer identity')
    require(iso_name is None or iso_name == name, 'The ISO name does not match installer.json; expected ' + name)
    sums = read_bounded(directory / 'SHA256SUMS', 4096).decode('ascii', 'replace')
    require(sums == installer['sha256'] + '  ' + name + '\n', 'SHA256SUMS does not match installer.json')
    iso = directory / name
    require(regular(iso) and iso.stat().st_size == installer['size_bytes'],
            'The installer ISO is missing or has the wrong size: ' + str(iso))
    return record, iso


def build_iso(args):
    require_mac()
    require(re.fullmatch(IMAGE_PATTERN, args.image),
            'An exact reviewed {}@sha256:... digest is required, not a mutable tag'.format(SPEC['repository']))
    require(args.base_image is None or re.fullmatch(BASE_PATTERN, args.base_image),
            'Invalid reviewed Fedora base digest')
    pins = load_pins()
    engine = docker_engine(docker_cli())
    docker = engine['cli']
    if engine['name'] != 'OrbStack':
        print('kedra-media: warning: only OrbStack was qualified for this build; continuing with ' + engine['name'],
              file=sys.stderr)
    require(regular(ROOT / KEDRA / 'installer/build-local.py') and regular(HERE / 'build-in-container.sh'),
            'Run this from a complete trusted Kedra checkout')
    public, recorded = ROOT / SPEC['public_key'], ROOT / SPEC['key_sha256']
    require(regular(public) and regular(recorded), 'The checkout lacks the qemu-arm64 release authority files')
    fingerprint = read_bounded(recorded, 65).decode('ascii', 'replace').strip()
    require(re.fullmatch('[a-f0-9]{64}', fingerprint), 'Malformed ' + SPEC['key_sha256'])
    output = args.output.expanduser().absolute()
    require(not os.path.lexists(str(output)) and output.parent.is_dir(),
            '--output must name a new directory under an existing parent; nothing is overwritten')
    require(not any(char in UNSAFE_PATH for char in str(ROOT) + str(output.parent.resolve()) + output.name),
            'The checkout and output paths must not contain : , " or newlines (container mount syntax)')
    require(free_bytes(output.parent) >= 8 * GIB, 'Less than 8 GiB free for the ISO under ' + str(output.parent))
    running = run([docker, 'ps', '--filter', f'label={LABEL}=iso', '--format', '{{.Names}}'],
                  timeout=60, capture=True).stdout.decode('utf-8', 'replace').split()
    require(not running, 'Another kedra-qemu-arm64 ISO build is running: ' + ', '.join(running))
    print(f'kedra-media: qemu-arm64 public key SPKI SHA-256 {fingerprint}; confirm it independently', file=sys.stderr)
    token = secrets.token_hex(6)
    container, volume = 'kedra-media-iso-' + token, 'kedra-media-iso-out-' + token
    run([docker, 'volume', 'create', '--label', LABEL + '=iso-output', volume], timeout=120, capture=True)
    created = verified = False
    try:
        output.mkdir(mode=0o755)
        created = True
        # A fresh privileged container per run (no stale Podman locks); rootful
        # Podman state persists only in the named storage volume.
        run([docker, 'run', '--rm', '--name', container, '--label', LABEL + '=iso', '--platform', 'linux/arm64',
             '--pull', 'missing', '--privileged', '--cgroupns', 'private',
             '--mount', f'type=volume,source={STORAGE_VOLUME},target=/var/lib/containers',
             '--mount', f'type=volume,source={volume},target=/var/tmp/kedra-out',
             '--mount', f'type=bind,source={ROOT},target=/kedra,readonly',
             '--mount', f'type=bind,source={output.resolve()},target=/export',
             pins['build_container'], '/bin/bash', '/kedra/usr/src/kedra/installer/macos/build-in-container.sh',
             args.image, args.base_image or ''], timeout=4 * 3600)
        record, iso = installer_record(output, image=args.image)
        names = sorted(entry.name for entry in output.iterdir())
        require(names == sorted([iso.name, 'installer.json', 'SHA256SUMS']),
                'Unexpected output files: ' + ', '.join(names))
        run([TOOLS['shasum'], '-a', '256', '-c', 'SHA256SUMS'], timeout=1800, cwd=str(output))
        verified = True
    finally:
        run([docker, 'rm', '--force', container], timeout=300, capture=True, check=False)
        if verified:
            run([docker, 'volume', 'rm', volume], timeout=300, capture=True, check=False)
        else:
            print(f'kedra-media: build scratch retained in Docker volume {volume} (remove: docker volume rm {volume})', file=sys.stderr)
            if created and not any(output.iterdir()):
                output.rmdir()
            elif created:
                print('kedra-media: unverified output retained for inspection in ' + str(output), file=sys.stderr)
    print(f"Verified {iso} ({record['installer']['size_bytes']} bytes) built from {record['image']}")
    print(f'Next: kedra-lab vm installer --iso {shlex.quote(str(iso))}')
    return 0


def load_pins():
    pins = json.loads(read_bounded(HERE / 'inputs.json'))
    require(pins.get('schema_version') == 1 and pins.get('architecture') == SPEC['oci_architecture']
            and re.fullmatch(r'quay\.io/fedora/fedora@sha256:[a-f0-9]{64}', str(pins.get('build_container', ''))),
            'Unexpected macOS media builder pins')
    return pins


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='operation', required=True)
    build = sub.add_parser('build')
    build.add_argument('--image', required=True)
    build.add_argument('--output', required=True, type=Path)
    build.add_argument('--base-image')
    verify = sub.add_parser('verify')
    verify.add_argument('--iso', required=True, type=Path)
    args = parser.parse_args()
    if args.operation == 'build':
        return build_iso(args)
    record, iso = installer_record(args.iso.resolve().parent, iso_name=args.iso.name)
    require(sha256(iso) == record['installer']['sha256'], 'ISO checksum mismatch')
    print(json.dumps(record))
    return 0


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, lambda _signum, _frame: sys.exit(130))
    signal.signal(signal.SIGINT, lambda _signum, _frame: sys.exit(130))
    try:
        raise SystemExit(main())
    except (ToolError, OSError, ValueError) as error:
        raise SystemExit('kedra-media: ' + str(error)) from None
