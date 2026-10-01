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
FIELDS = {'schema_version', 'kind', 'mode', 'source_revision', 'repository', 'runner_temp',
          'evidence', 'binaries', 'binary_sha256', 'uid'}
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
    require(value['mode'] in ('actions', 'local') and re.fullmatch('[a-f0-9]{40}', value['source_revision']),
            'Invalid fixture mode or source revision')
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
    require(head == value['source_revision'], 'Real committed HEAD differs from selected fixture source')
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
            'source_revision': os.environ['GITHUB_SHA'], 'repository': str(ROOT),
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--local-fixture', action='store_true')
    parser.add_argument('--source-revision')
    parser.add_argument('--runner-temp', type=Path)
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--binaries', type=Path)
    args = parser.parse_args()
    selected = (args.source_revision, args.runner_temp, args.evidence, args.binaries)
    if args.local_fixture:
        if not all(item is not None for item in selected):
            parser.error('--local-fixture requires source revision, private runner temp, evidence and binaries')
        value = {'schema_version': 1, 'kind': 'kedra-arm-release-fixture', 'mode': 'local',
                 'source_revision': args.source_revision, 'repository': str(ROOT),
                 'runner_temp': str(path(args.runner_temp)), 'evidence': str(path(args.evidence, existing=False)),
                 'binaries': str(path(args.binaries)), 'binary_sha256': binary_hashes(path(args.binaries)),
                 'uid': os.getuid()}
    else:
        if any(item is not None for item in selected):
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
