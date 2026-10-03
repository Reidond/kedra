#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Closed execution context for the disposable ARM release workflow."""
import argparse
import hashlib
import json
import os
import platform
import re
import stat
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
PROGRAMS = ('sysroot', 'sysroot-helper', 'kedra-lab')
FIELDS = {'schema_version', 'kind', 'mode', 'source_revision', 'fixture_revision', 'repository', 'runner_temp',
          'evidence', 'binaries', 'binary_sha256', 'uid', 'retained_candidate', 'retained_candidate_sha256'}
MARKER = Path('/usr/share/kedra-release-fixture/controller')


def require(value, message):
    if not value:
        raise RuntimeError(message)


def path(value, *, existing=True):
    result = Path(value)
    require(result.is_absolute() and not any(c in str(result) for c in ':,\r\n'), 'Expected an absolute fixture path')
    require(not result.is_symlink(), 'Fixture path must not be a symlink')
    if existing:
        return result.resolve(strict=True)
    return result.parent.resolve(strict=True) / result.name


def binary_hashes(directory):
    result = {}
    for name in PROGRAMS:
        binary = directory / name
        require(binary.is_file() and not binary.is_symlink() and os.access(binary, os.X_OK),
                'Missing ordinary executable fixture input: ' + name)
        with binary.open('rb') as stream:
            header = stream.read(20)
            require(header[:6] == b'\x7fELF\x02\x01' and int.from_bytes(header[18:20], 'little') == 183,
                    'Fixture tools must be native Linux ARM64 ELF: ' + name)
            stream.seek(0)
            result[name] = hashlib.file_digest(stream, 'sha256').hexdigest()
    return result


def validate(value):
    require(isinstance(value, dict) and set(value) == FIELDS and type(value['schema_version']) is int
            and value['schema_version'] == 1 and value['kind'] == 'kedra-arm-release-fixture',
            'Unsupported fixture context')
    require(type(value['uid']) is int and sys.platform == 'linux' and platform.machine() == 'aarch64'
            and os.getuid() == os.geteuid() == value['uid'] and value['uid'] > 0,
            'The ARM fixture controller must run as an ordinary native Linux ARM user')
    require(value['mode'] in ('actions', 'local') and re.fullmatch('[a-f0-9]{40}', value['source_revision'])
            and re.fullmatch('[a-f0-9]{40}', value['fixture_revision']),
            'Invalid fixture mode or source revision')
    if value['retained_candidate'] is None:
        require(value['retained_candidate_sha256'] is None
                and value['fixture_revision'] == value['source_revision'], 'Fresh candidates require one exact revision')
    else:
        require(value['mode'] == 'local' and isinstance(value['retained_candidate_sha256'], str)
                and re.fullmatch('[a-f0-9]{64}', value['retained_candidate_sha256']),
                'Retained candidates require an independently selected hash in explicit local mode')
        retained = path(value['retained_candidate'])
        info = retained.stat()
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
                and 0 < info.st_size <= 65536, 'Retained candidate must be a bounded owned regular file')
        require(hashlib.sha256(retained.read_bytes()).hexdigest() == value['retained_candidate_sha256'],
                'Retained candidate differs from the selected hash')
    require(path(value['repository']) == ROOT, 'Fixture context belongs to another checkout')
    temporary = path(value['runner_temp'])
    evidence = path(value['evidence'], existing=False)
    binaries = path(value['binaries'])
    require(temporary.is_dir() and binaries.is_dir(), 'Fixture directories are missing')
    require(not evidence.is_relative_to(temporary / 'kedra-ghcr-private'), 'Evidence must be outside private inputs')
    if value['mode'] == 'actions':
        require(os.environ.get('GITHUB_ACTIONS') == 'true'
                and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra'
                and os.environ.get('GITHUB_SHA') == value['source_revision']
                and temporary == Path(os.environ['RUNNER_TEMP']).resolve(), 'Actions context differs from dispatch')
    else:
        require(os.environ.get('GITHUB_ACTIONS') != 'true', 'Local fixture mode is separate from Actions')
        info = MARKER.lstat()
        require(stat.S_ISREG(info.st_mode) and info.st_uid == 0 and info.st_nlink == 1
                and info.st_mode & 0o022 == 0
                and MARKER.read_bytes() == b'Kedra disposable ARM release controller v1\n'
                and (Path('/.dockerenv').is_file() or Path('/run/.containerenv').is_file()),
                'Local fixtures require the dedicated disposable controller container')
        info = temporary.stat()
        require(info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700,
                'Local runner temp must be owned and mode 0700')
    environment = {key: item for key, item in os.environ.items() if not key.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_OPTIONAL_LOCKS='0')
    head = subprocess.check_output(['/usr/bin/git', '--no-replace-objects', '-C', str(ROOT),
        'rev-parse', '--verify', 'HEAD^{commit}'], env=environment, timeout=30).decode().strip()
    require(head == value['fixture_revision'], 'Real committed HEAD differs from selected fixture tools')
    require(binary_hashes(binaries) == value['binary_sha256'], 'Selected fixture binaries changed')
    return value


def actions_context():
    require(os.environ.get('GITHUB_ACTIONS') == 'true'
            and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra', 'Expected a real Actions dispatch')
    binaries = ROOT / 'target/release'
    output = ROOT / 'output'
    require(not output.is_symlink(), 'Actions output must not be a symlink')
    output.mkdir(mode=0o700, exist_ok=True)
    return {'schema_version': 1, 'kind': 'kedra-arm-release-fixture', 'mode': 'actions',
            'source_revision': os.environ['GITHUB_SHA'], 'fixture_revision': os.environ['GITHUB_SHA'],
            'retained_candidate': None, 'retained_candidate_sha256': None, 'repository': str(ROOT),
            'runner_temp': str(Path(os.environ['RUNNER_TEMP']).resolve()),
            'evidence': str(ROOT / 'output/ghcr-arm-evidence'), 'binaries': str(binaries),
            'binary_sha256': binary_hashes(binaries), 'uid': os.getuid()}


def load_context(filename=None):
    if filename is None:
        return validate(actions_context())
    filename = path(filename)
    info = filename.stat()
    require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
            and stat.S_IMODE(info.st_mode) == 0o600 and 0 < info.st_size <= 16384,
            'Fixture context must be a bounded owner-private regular file')
    return validate(json.loads(filename.read_bytes()))


def add_context_argument(parser):
    parser.add_argument('--fixture-context', type=Path, help='Previously selected closed local or Actions fixture context')


def registry_volume(args):
    fixture = load_context(args.fixture_context)
    require(fixture['mode'] == 'local' and fixture['retained_candidate'] is not None,
            'Existing registry volumes require an explicit local retained candidate')
    name = args.registry_volume
    require(re.fullmatch('[a-f0-9]{64}', name)
            and re.fullmatch(r'\d{4}-\d{2}-\d{2}T[\d:.]+Z', args.registry_volume_created_at)
            and len(args.registry_volume_created_at) <= 40, 'Invalid selected volume identity')
    require((args.registry_container is None) == (args.registry_volume_before is None),
            'Stopped registry validation requires the prior volume observation')
    if args.registry_container is not None:
        require(re.fullmatch('[a-f0-9]{64}', args.registry_container), 'Invalid registry container identity')
    podman = ['/usr/bin/sudo', '--non-interactive', '/usr/bin/podman']

    def run(arguments):
        result = subprocess.run(arguments, stdin=subprocess.DEVNULL, capture_output=True,
                                timeout=30, check=True)
        require(len(result.stdout) <= 1024**2, 'Oversized registry volume inspection')
        return result.stdout

    records = json.loads(run([*podman, 'volume', 'inspect', name]))
    require(isinstance(records, list) and len(records) == 1, 'Expected one existing registry volume')
    volume = records[0]
    require(isinstance(volume, dict), 'Invalid registry volume observation')
    mountpoint = '/var/lib/containers/storage/volumes/' + name + '/_data'
    require(volume.get('Name') == name and volume.get('Driver') == 'local' and volume.get('Scope') == 'local'
            and volume.get('Options') in (None, {}) and volume.get('Mountpoint') == mountpoint
            and volume.get('CreatedAt') == args.registry_volume_created_at
            and type(volume.get('MountCount')) is int and volume['MountCount'] == 0,
            'Registry volume identity, local storage or unused state differs')
    real = run(['/usr/bin/sudo', '--non-interactive', '/usr/bin/readlink', '-e', '--', mountpoint]).decode().strip()
    require(real == mountpoint, 'Registry volume backing path has an alias')
    fields = run(['/usr/bin/sudo', '--non-interactive', '/usr/bin/stat',
                  '--format=%d:%i:%u:%g:%f', '--', mountpoint]).decode().strip().split(':')
    require(len(fields) == 5, 'Invalid registry backing directory observation')
    identity = [int(value) for value in fields[:4]] + [int(fields[4], 16)]
    require(stat.S_ISDIR(identity[4]), 'Registry volume backing must be an ordinary directory')
    result = {'schema_version': 1, 'name': name, 'created_at': volume['CreatedAt'],
              'mountpoint': mountpoint, 'backing_identity': identity}
    consumers = run([*podman, 'ps', '--all', '--no-trunc', '--filter', 'volume=' + name,
                     '--format', '{{.ID}}']).decode().split()
    require(consumers == ([args.registry_container] if args.registry_container else []),
            'Registry volume has an unexpected consumer')
    if args.registry_container is not None:
        before = path(args.registry_volume_before)
        require(before == Path(fixture['runner_temp']) / 'kedra-ghcr/registry-volume-before.json',
                'Registry observation belongs to another fixture root')
        with os.fdopen(os.open(before, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
            info = os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1
                    and stat.S_IMODE(info.st_mode) == 0o600 and 0 < info.st_size <= 16384,
                    'Registry observation must be bounded and owner-private')
            prior = stream.read(16385)
        require(json.loads(prior) == result, 'Registry volume changed during stopped container creation')
        records = json.loads(run([*podman, 'inspect', args.registry_container]))
        require(isinstance(records, list) and len(records) == 1, 'Expected one stopped registry')
        container = records[0]
        mounts = [item for item in container.get('Mounts', []) if item.get('Destination') == '/var/lib/registry']
        require(container.get('Id') == args.registry_container
                and container.get('Name') in ('kedra-ghcr-registry', '/kedra-ghcr-registry')
                and container.get('State', {}).get('Running') is False
                and (container.get('Config', {}).get('Labels') or {}).get('dev.kedra.lab.owner') == 'kedra-release-fixture'
                and len(mounts) == 1 and mounts[0].get('Type') == 'volume'
                and mounts[0].get('Name') == name and mounts[0].get('Source') == mountpoint
                and mounts[0].get('RW') is True, 'Stopped registry does not bind the exact selected volume')
    print(json.dumps(result, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--local-fixture', action='store_true')
    parser.add_argument('--source-revision')
    parser.add_argument('--fixture-revision', help='Exact current tools commit when continuing a retained local candidate')
    parser.add_argument('--retained-candidate', type=Path)
    parser.add_argument('--retained-candidate-sha256')
    parser.add_argument('--runner-temp', type=Path)
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--binaries', type=Path)
    add_context_argument(parser)
    parser.add_argument('--registry-volume', help='Read-only validation of an explicitly selected existing working volume')
    parser.add_argument('--registry-volume-created-at', help='Independently selected exact volume creation time')
    parser.add_argument('--registry-container', help='Exact stopped registry container, after creation with nocopy')
    parser.add_argument('--registry-volume-before', type=Path, help='Private pre-creation volume observation')
    args = parser.parse_args()
    selected = (args.source_revision, args.runner_temp, args.evidence, args.binaries)
    if args.registry_volume is not None:
        if (args.fixture_context is None or args.registry_volume_created_at is None or args.local_fixture
                or any(item is not None for item in (*selected, args.fixture_revision,
                                                    args.retained_candidate, args.retained_candidate_sha256))):
            parser.error('registry inspection requires only an existing fixture context and explicit volume identity')
        registry_volume(args)
        return
    if any(item is not None for item in (args.fixture_context, args.registry_volume_created_at,
                                        args.registry_container, args.registry_volume_before)):
        parser.error('registry inspection options require --registry-volume')
    if args.local_fixture:
        if not all(item is not None for item in selected):
            parser.error('--local-fixture requires source revision, private runner temp, evidence and binaries')
        retained = (args.retained_candidate, args.retained_candidate_sha256)
        if any(item is not None for item in retained) and not all(item is not None for item in retained):
            parser.error('retained candidate path and independently selected SHA-256 must be supplied together')
        if args.retained_candidate is not None and args.fixture_revision is None:
            parser.error('retained candidates require an explicit --fixture-revision')
        value = {'schema_version': 1, 'kind': 'kedra-arm-release-fixture', 'mode': 'local',
                 'source_revision': args.source_revision, 'fixture_revision': args.fixture_revision or args.source_revision,
                 'retained_candidate': str(path(args.retained_candidate)) if args.retained_candidate is not None else None,
                 'retained_candidate_sha256': args.retained_candidate_sha256, 'repository': str(ROOT),
                 'runner_temp': str(path(args.runner_temp)), 'evidence': str(path(args.evidence, existing=False)),
                 'binaries': str(path(args.binaries)), 'binary_sha256': binary_hashes(path(args.binaries)),
                 'uid': os.getuid()}
    else:
        if any(item is not None for item in (*selected, args.fixture_revision, args.retained_candidate,
                                            args.retained_candidate_sha256)):
            parser.error('Actions defaults cannot be overridden; select explicit local fixture mode')
        value = actions_context()
    validate(value)
    filename = Path(value['runner_temp']) / 'kedra-release-context.json'
    descriptor = os.open(filename, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    print(filename)


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit('release fixture: ' + str(error)) from error
