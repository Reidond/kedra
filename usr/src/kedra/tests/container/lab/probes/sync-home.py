#!/usr/bin/python3
"""Merge working-tree home baselines into a disposable guest account.

Runs with the guest's interpreter, as the lab user, never on the workstation.
The request's home files become the baseline of a private image root; the
production `sysroot home update --root` then merges, checks and reloads the
files that the installed usr/share/kedra/home-files.toml declares, exactly as
after an image update. Local guest edits are merged, never overwritten.
"""
import base64
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

LAB_HOME = Path.home()
STATE = LAB_HOME / '.local/state/kedra-lab/sync'
PREFIX = 'usr/share/sysroot/home/default/'
DATA_FILE = 'usr/share/kedra/home-files.toml'


def safe(name):
    path = Path(name)
    return not path.is_absolute() and all(part not in ('', '.', '..') for part in name.split('/'))


def manifest_files(source, home):
    return {entry['destination']: (entry['sha256'], entry['mode'])
            for entry in source['files'] if bool(entry['home_baseline']) == home}


def check_home_only(source, installed):
    if source['target'] != installed['target']:
        raise ValueError('sync target differs from the running image')
    for key in ('packages', 'remove_packages'):
        if sorted(source[key]) != sorted(installed[key]):
            raise ValueError('package changes need an image build, not home sync')
    if manifest_files(source, False) != manifest_files(installed, False):
        raise ValueError('rootfs changes, including home-files.toml, need an image build, not home sync')


def check_payload(files, source):
    expected = {entry['destination'][len(PREFIX):]: entry for entry in source['files']
                if entry['destination'].startswith(PREFIX)}
    if set(files) != set(expected):
        raise ValueError('home payload does not match its source manifest')
    for name, value in files.items():
        if set(value) != {'content', 'mode'} or not safe(name) or value['mode'] not in (0o644, 0o755):
            raise ValueError(f'unsafe lab sync file: {name}')
        content = base64.b64decode(value['content'], validate=True)
        if hashlib.sha256(content).hexdigest() != expected[name]['sha256']:
            raise ValueError(f'lab sync content hash differs: {name}')
        if value['mode'] != int(expected[name]['mode'], 8) & 0o777:
            raise ValueError(f'lab sync file mode differs: {name}')


def stage(files):
    """A private image root: the installed file list plus the request's baselines."""
    root = Path(tempfile.mkdtemp(prefix='root-', dir=STATE))
    target = root / DATA_FILE
    target.parent.mkdir(parents=True, mode=0o755)
    shutil.copyfile(Path('/') / DATA_FILE, target)
    target.chmod(0o644)
    for name, value in files.items():
        path = root / PREFIX / name
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
        path.write_bytes(base64.b64decode(value['content'], validate=True))
        path.chmod(value['mode'])
    return root


def apply(request):
    if request.get('schema_version') != 1 or set(request) != {'schema_version', 'source', 'files'}:
        raise ValueError('unsupported lab sync request')
    source, files = request['source'], request['files']
    installed = json.loads(Path('/usr/share/sysroot/source.json').read_text())
    check_home_only(source, installed)
    check_payload(files, source)
    root = stage(files)
    try:
        result = subprocess.run(['sysroot', 'home', '--root', str(root), 'update', '--json'],
                                capture_output=True, text=True, timeout=120, check=False)
    finally:
        shutil.rmtree(root)
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError:
        raise ValueError(f'sysroot home update failed: {result.stderr.strip()[-3000:]}') from None
    report['source'] = source['source_revision']
    report['exit'] = result.returncode
    return report


def main():
    if (sys.platform != 'linux' or os.geteuid() == 0 or LAB_HOME.name != 'kedra-test'
            or not Path('/usr/share/kedra-lab/added-rpms.txt').is_file()):
        raise ValueError('lab sync requires the disposable kedra-test account in a lab image')
    for parent in (LAB_HOME / '.local', LAB_HOME / '.local/state', STATE.parent, STATE):
        if parent.is_symlink():
            raise ValueError('lab state directory must not be a symlink')
        parent.mkdir(exist_ok=True, mode=0o700)
    with (STATE / 'lock').open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError('another lab sync holds the guest lock; retry after it finishes') from None
        started = time.monotonic()
        raw = sys.stdin.buffer.read(32 * 1024 * 1024 + 1)
        if len(raw) > 32 * 1024 * 1024:
            raise ValueError('lab sync request is too large')
        outcome = apply(json.loads(raw))
        outcome['duration_ms'] = round((time.monotonic() - started) * 1000)
        print(json.dumps(outcome))
        if outcome['exit']:
            raise SystemExit(1)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired) as error:
        print(f'lab sync refused: {error}', file=sys.stderr)
        raise SystemExit(1) from None
