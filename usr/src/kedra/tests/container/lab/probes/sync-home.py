#!/usr/bin/python3
"""Apply one validated desktop payload in a disposable guest account.

Runs with the guest's interpreter, as the lab user, never on the workstation.
Only image-declared niri/Noctalia files are managed. The lock/journal are lab
state, independent of the production sysroot home workflow.
"""
import base64
import fcntl
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

LAB_HOME = Path.home()
BASE = Path('/usr/share/sysroot/home/default')
STATE = LAB_HOME / '.local/state/kedra-lab/sync'
PREFIX = 'usr/share/sysroot/home/default/'


def supported(name):
    path = Path(name)
    return (not path.is_absolute() and '..' not in path.parts
            and name.startswith(('.config/niri/', '.config/noctalia/')))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def checked_path(root, name):
    path = root
    for component in Path(name).parts:
        path /= component
        if path.is_symlink():
            raise ValueError(f'symlink in managed path: {name}')
    if path.exists() and not path.is_file():
        raise ValueError(f'not a regular file: {name}')
    return path


def contents(name):
    path = checked_path(LAB_HOME, name)
    if not path.exists():
        return None
    return {'content': base64.b64encode(path.read_bytes()).decode(),
            'mode': stat.S_IMODE(path.stat().st_mode)}


def write_json(path, value):
    pending = path.with_suffix('.pending')
    with pending.open('w') as stream:
        json.dump(value, stream)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(pending, path)


def replace(name, value):
    path = checked_path(LAB_HOME, name)
    if value is None:
        path.unlink(missing_ok=True)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    checked_path(LAB_HOME, name)
    descriptor, temporary = tempfile.mkstemp(prefix='.kedra-sync-', dir=path.parent)
    try:
        with os.fdopen(descriptor, 'wb') as stream:
            stream.write(base64.b64decode(value['content'], validate=True))
            os.fchmod(stream.fileno(), value['mode'])
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def run(argv, **kwargs):
    result = subprocess.run(argv, capture_output=True, text=True, timeout=10, check=False, **kwargs)
    if result.returncode:
        raise ValueError(f'{argv[0]} failed: {(result.stderr or result.stdout)[-3000:]}')
    return result.stdout


def ready(changed):
    run(['niri', 'validate'])
    if any(name.startswith('.config/niri/') for name in changed):
        run(['niri', 'msg', 'action', 'load-config-file'])
    if any(name.startswith('.config/noctalia/') for name in changed):
        run(['systemctl', '--user', 'restart', 'kedra-noctalia.service'])
        run(['noctalia', 'config', 'validate'])
    deadline = time.monotonic() + 10
    while True:
        try:
            run(['niri', 'msg', 'version'])
            run(['noctalia', 'msg', 'log-level-status'])
            return
        except (ValueError, subprocess.TimeoutExpired):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.1)


def recover():
    path = STATE / 'transaction.json'
    if not path.exists():
        return
    transaction = json.loads(path.read_text())
    if transaction.get('schema_version') != 1:
        raise ValueError('unknown lab sync transaction; manual recovery required')
    conflicts = []
    for name, change in transaction['changes'].items():
        if not supported(name):
            raise ValueError('unsafe path in lab sync transaction')
        current = contents(name)
        if current == change['before']:
            continue
        if current != change['after']:
            conflicts.append(name)
            continue
        replace(name, change['before'])
    if conflicts:
        raise ValueError('recovery preserves newer guest edits: ' + ', '.join(conflicts))
    ready(transaction['changes'])
    if transaction['previous'] is None:
        (STATE / 'receipt.json').unlink(missing_ok=True)
    else:
        write_json(STATE / 'receipt.json', transaction['previous'])
    path.unlink()


def manifest_files(source, home):
    return {entry['destination']: (entry['sha256'], entry['mode'])
            for entry in source['files'] if bool(entry['home_baseline']) == home}


def leaves(value, prefix=''):
    result = {}
    for key, item in value.items():
        name = f'{prefix}.{key}' if prefix else key
        if isinstance(item, dict):
            result.update(leaves(item, name))
        else:
            result[name] = item
    return result


def validate(files, changed):
    masked = []
    with tempfile.TemporaryDirectory(prefix='validation-', dir=STATE) as temporary:
        staging = Path(temporary)
        for group in ('niri', 'noctalia'):
            directory = LAB_HOME / '.config' / group
            if directory.is_dir():
                for path in directory.rglob('*'):
                    if path.is_symlink():
                        raise ValueError(f'cannot validate a symlinked {group} config')
                shutil.copytree(directory, staging / '.config' / group)
        for name, value in files.items():
            target = staging / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(base64.b64decode(value['content'], validate=True))
        for name in changed:
            if name not in files:
                (staging / name).unlink(missing_ok=True)
        run(['niri', 'validate', '--config', str(staging / '.config/niri/config.kdl')])
        env = dict(os.environ, NOCTALIA_CONFIG_HOME=str(staging / '.config'),
                   NOCTALIA_STATE_HOME=str(staging / 'state'))
        run(['noctalia', 'config', 'validate', str(staging / '.config/noctalia')], env=env)
        override = LAB_HOME / '.local/state/noctalia/settings.toml'
        config = staging / '.config/noctalia/config.toml'
        if override.is_file() and config.is_file():
            proposed = leaves(tomllib.loads(config.read_text()))
            overridden = leaves(tomllib.loads(override.read_text()))
            masked = sorted(key for key in proposed if key in overridden and proposed[key] != overridden[key])
    return masked


def check_home_only(source, installed):
    if source['target'] != installed['target']:
        raise ValueError('sync target differs from the running image')
    for key in ('packages', 'remove_packages'):
        if sorted(source[key]) != sorted(installed[key]):
            raise ValueError('package changes need an image build, not home sync')
    if manifest_files(source, False) != manifest_files(installed, False):
        raise ValueError('rootfs changes need an image build, not home sync')


def desktop_entries(source):
    return {entry['destination'][len(PREFIX):]: entry for entry in source['files']
            if entry['destination'].startswith(PREFIX) and supported(entry['destination'][len(PREFIX):])}


def check_payload(files, expected):
    if set(files) != set(expected):
        raise ValueError('desktop payload does not match its source manifest')
    for name, value in files.items():
        if (set(value) != {'content', 'mode'} or not supported(name)
                or value['mode'] not in (0o644, 0o755)):
            raise ValueError(f'unsafe lab sync file: {name}')
        if digest(base64.b64decode(value['content'], validate=True)) != expected[name]['sha256']:
            raise ValueError(f'lab sync content hash differs: {name}')
        if value['mode'] != int(expected[name]['mode'], 8) & 0o777:
            raise ValueError(f'lab sync file mode differs: {name}')


def load_receipt(path):
    previous = json.loads(path.read_text()) if path.exists() else None
    if previous is not None and previous.get('schema_version') != 1:
        raise ValueError('unknown lab sync receipt')
    return previous


def image_baseline(installed):
    baseline = {}
    for entry in installed['files']:
        name = entry['destination'].removeprefix(PREFIX)
        if entry['destination'].startswith(PREFIX) and supported(name):
            path = checked_path(BASE, name)
            baseline[name] = {'content': base64.b64encode(path.read_bytes()).decode(),
                              'mode': stat.S_IMODE(path.stat().st_mode)}
    return baseline


def plan_changes(files, baseline):
    changes = {}
    for name in sorted(set(files) | set(baseline)):
        if not supported(name):
            raise ValueError('unsafe path in lab sync receipt')
        current, wanted, before = contents(name), files.get(name), baseline.get(name)
        if current != before:
            raise ValueError(f'guest edit preserved; sync conflict: {name}')
        if current != wanted:
            changes[name] = {'before': current, 'after': wanted}
    return changes


def check_guest_unchanged(changes):
    for name, change in changes.items():
        if contents(name) != change['before']:
            raise ValueError(f'guest config changed during validation: {name}')


def commit(changes, previous, receipt_path, receipt):
    write_json(STATE / 'transaction.json', {'schema_version': 1, 'previous': previous, 'changes': changes})
    try:
        # Install includes before entrypoints; the journal makes an interruption recoverable.
        for name in sorted(changes, key=lambda name: name.endswith(('config.kdl', 'config.toml'))):
            replace(name, changes[name]['after'])
        ready(changes)
        write_json(receipt_path, receipt)
        (STATE / 'transaction.json').unlink()
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        try:
            recover()
        except (ValueError, OSError, subprocess.TimeoutExpired) as recovery:
            raise ValueError(f'{error}; recovery also failed: {recovery}') from error
        raise


def apply(request):
    if request.get('schema_version') != 1 or set(request) != {'schema_version', 'source', 'files'}:
        raise ValueError('unsupported lab sync request')
    source, files = request['source'], request['files']
    installed = json.loads(Path('/usr/share/sysroot/source.json').read_text())
    check_home_only(source, installed)
    check_payload(files, desktop_entries(source))
    receipt_path = STATE / 'receipt.json'
    previous = load_receipt(receipt_path)
    baseline = image_baseline(installed) if previous is None else previous['files']
    changes = plan_changes(files, baseline)
    masked = validate(files, changes)
    if not changes:
        return {'changed': [], 'masked_settings': masked, 'source': source['source_revision']}
    check_guest_unchanged(changes)
    commit(changes, previous, receipt_path, {'schema_version': 1, 'source': source, 'files': files})
    return {'changed': list(changes), 'masked_settings': masked, 'source': source['source_revision']}


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
        recover()
        started = time.monotonic()
        raw = sys.stdin.buffer.read(32 * 1024 * 1024 + 1)
        if len(raw) > 32 * 1024 * 1024:
            raise ValueError('lab sync request is too large')
        outcome = apply(json.loads(raw))
        outcome['duration_ms'] = round((time.monotonic() - started) * 1000)
        print(json.dumps(outcome))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired) as error:
        print(f'lab sync refused: {error}', file=sys.stderr)
        raise SystemExit(1) from None
