"""Actual user CLI across signed image A, image B, and retained-image rollback."""
import json
import ctypes
import fcntl
import os
import pathlib
import select
import signal
import sqlite3
import struct
import subprocess
import sys
import time

phase = sys.argv[1]
fixture = json.loads(pathlib.Path('/usr/share/kedra-research/fixture.json').read_text())
native = pathlib.Path.home() / '.config/niri/config.kdl'
repo = pathlib.Path.home() / 'kedra-r04-source'
home_state = pathlib.Path.home() / '.local/state/sysroot/home'


def run(args, success=True, environment=None):
    result = subprocess.run(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=90, check=False, env=environment)
    if (result.returncode == 0) != success:
        raise RuntimeError(f'generated native workflow failed: {args[:4]}: {result.stderr.decode(errors="replace")[:1200]}')
    return result.stdout


def cli(*args, success=True):
    result = run(['sysroot', 'home', 'file', *args], success)
    return json.loads(result) if success else None


def change(after):
    return next(row['change']['id'] for row in cli('status')['changes']
                if row['change']['after'].strip() == after)


def validate():
    run(['niri', 'validate'])


def assessment(*extra, environment=None):
    result = json.loads(run(['sysroot', 'update', 'status', '--home', *extra],
                            environment=environment))
    home = result['caller_home']
    if home['scope'] != 'invoking_user' or home['uid'] != os.getuid():
        raise RuntimeError('assessment has the wrong caller scope')
    if home['review_state_changed'] or home['live_files_changed'] or home['live_configuration_checked']:
        raise RuntimeError('assessment claims a capture or mutation')
    for group in home['groups'].values():
        if set(group) != {'status', 'accepted_baseline_revision', 'installed_baseline_revision', 'next_command'}:
            raise RuntimeError('assessment exposed unexpected private home fields')
    return result


def check_group(home, application, expected, accepted, installed):
    group = home['groups'][application]
    if (group['status'], group['accepted_baseline_revision'], group['installed_baseline_revision']) != (expected, accepted, installed):
        raise RuntimeError(f'caller {application} assessment has wrong baseline status')


def assessed(expected, accepted, installed, adopted=True,
             noctalia='accepted_baseline_matches_installed',
             overall='accepted_baseline_matches_installed'):
    before = cli('status') if adopted else None
    native_before = native.read_bytes()
    noctalia_before = run(['sysroot', 'home', 'status', '--last-capture']) \
        if home_state.exists() and noctalia != 'not_adopted' else None
    ordinary = json.loads(run(['sysroot', 'update', 'status']))
    if 'caller_home' in ordinary:
        raise RuntimeError('ordinary update status changed its output contract')
    combined = assessment()
    home = combined.pop('caller_home')
    if combined != ordinary:
        raise RuntimeError('caller assessment changed helper status fields')
    check_group(home, 'niri', expected, accepted, installed)
    check_group(home, 'noctalia', noctalia,
                None if noctalia == 'not_adopted' else fixture['a'], installed)
    if home['status'] != overall:
        raise RuntimeError('caller assessment has wrong overall status')
    if native.read_bytes() != native_before or (adopted and cli('status') != before):
        raise RuntimeError('assessment changed native contents or independent home decisions')
    if noctalia_before is not None and run(['sysroot', 'home', 'status', '--last-capture']) != noctalia_before:
        raise RuntimeError('assessment changed the last Noctalia capture or decisions')
    return home


def negative_assessments():
    before, native_before = cli('status'), native.read_bytes()
    absent = pathlib.Path.home() / 'r04-never-adopted'
    if absent.exists() or assessment('--home-state', str(absent))['caller_home']['status'] != 'not_adopted' or absent.exists():
        raise RuntimeError('missing explicit store was initialized or misreported')
    unsafe = pathlib.Path.home() / 'r04-unsafe-state'
    unsafe.symlink_to(home_state, target_is_directory=True)
    try:
        if assessment('--home-state', str(unsafe))['caller_home']['status'] != 'unavailable':
            raise RuntimeError('symlink store was assessed as usable')
    finally:
        unsafe.unlink()
    inaccessible = pathlib.Path.home() / 'r04-inaccessible-state'
    inaccessible.mkdir(mode=0o700)
    inaccessible.chmod(0)
    try:
        if assessment('--home-state', str(inaccessible))['caller_home']['status'] != 'unavailable':
            raise RuntimeError('unreadable store was assessed as absent')
    finally:
        inaccessible.chmod(0o700)
        inaccessible.rmdir()
    damaged = pathlib.Path.home() / 'r04-damaged-state'
    damaged.mkdir(mode=0o700)
    database = damaged / 'state.sqlite'
    database.write_bytes(b'generated malformed database fixture\n')
    database.chmod(0o600)
    if assessment('--home-state', str(damaged))['caller_home']['status'] != 'unavailable':
        raise RuntimeError('damaged store was assessed as absent')
    database.unlink()
    # A copied real adopted store with a future schema exercises the installed
    # command's compatibility refusal. The original database remains untouched.
    with sqlite3.connect(home_state / 'state.sqlite') as source, sqlite3.connect(database) as copy:
        source.backup(copy)
        copy.execute('PRAGMA user_version=999')
    source.close()
    copy.close()
    database.chmod(0o600)
    if assessment('--home-state', str(damaged))['caller_home']['status'] != 'unavailable':
        raise RuntimeError('unknown store schema was assessed as usable')
    for path in damaged.iterdir():
        path.unlink()
    damaged.rmdir()
    with (home_state / 'operation.lock').open('r+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if assessment()['caller_home']['status'] != 'unavailable':
            raise RuntimeError('busy review store was assessed as usable')
    for name in ('XDG_CONFIG_HOME', 'NOCTALIA_CONFIG_HOME', 'XDG_STATE_HOME', 'NOCTALIA_STATE_HOME', 'NIRI_CONFIG'):
        environment = dict(os.environ, **{name: str(pathlib.Path.home() / 'different-profile')})
        if assessment(environment=environment)['caller_home']['status'] != 'unavailable':
            raise RuntimeError('unadopted application profile was assessed as usable')
    if cli('status') != before or native.read_bytes() != native_before:
        raise RuntimeError('negative assessment changed real home review state or live data')
    print('KEDRA_R04_CALLER_HOME_UNAVAILABLE_PASS', flush=True)


def interrupted_assessment():
    # Reuse the native inotify publication boundary from R07: kill the actual
    # user CLI, inspect its real reservation through update status, then abort.
    before, native_before = cli('status'), native.read_bytes()
    change_id = change('width 5')
    plan = cli('discard-plan', change_id)
    libc = ctypes.CDLL(None, use_errno=True)
    descriptor = libc.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
    if descriptor < 0:
        raise OSError(ctypes.get_errno(), 'inotify_init1 failed')
    child = None
    try:
        if libc.inotify_add_watch(descriptor, os.fsencode(native.parent), 0x100 | 0x80) < 0:
            raise OSError(ctypes.get_errno(), 'inotify_add_watch failed')
        child = subprocess.Popen(['sysroot', 'home', 'file', 'discard', change_id,
                                  '--plan', plan['plan_id'], '--activate-managed-file'],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, start_new_session=True)
        deadline, prepared, killed = time.monotonic() + 60, False, False
        while time.monotonic() < deadline and child.poll() is None:
            if not select.select([descriptor], [], [], 1)[0]:
                continue
            data, offset = os.read(descriptor, 65536), 0
            while offset < len(data):
                _, mask, _, length = struct.unpack_from('iIII', data, offset)
                name = data[offset + 16:offset + 16 + length].split(b'\0', 1)[0]
                offset += 16 + length
                if mask & 0x100 and name.startswith(b'.sysroot-activation-'):
                    prepared = True
                if prepared and mask & 0x80 and name == b'config.kdl':
                    os.killpg(child.pid, signal.SIGKILL)
                    killed = True
                    break
            if killed:
                break
        child.communicate(timeout=10)
        if not killed or child.returncode != -signal.SIGKILL:
            raise RuntimeError('actual CLI did not stop at native publication')
    finally:
        os.close(descriptor)
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.communicate(timeout=10)
    pending_before, live_pending = cli('recover'), native.read_bytes()
    if not pending_before['pending_activation']:
        raise RuntimeError('killed CLI did not leave real recovery state')
    home = assessment()['caller_home']
    check_group(home, 'niri', 'recovery_required', fixture['a'], fixture['a'])
    check_group(home, 'noctalia', 'accepted_baseline_matches_installed', fixture['a'], fixture['a'])
    if home['status'] != 'recovery_required' or home['groups']['niri']['next_command'] != 'sysroot home file recover':
        raise RuntimeError('real pending activation was not reported as recovery')
    if cli('recover') != pending_before or native.read_bytes() != live_pending:
        raise RuntimeError('assessment changed a pending journal or live file')
    cli('recover', 'abort', '--activate-managed-file')
    if native.read_bytes() != native_before or cli('status') != before:
        raise RuntimeError('native abort failed to restore prior review/live state')
    assessed('accepted_baseline_matches_installed', fixture['a'], fixture['a'])
    print('KEDRA_R04_CALLER_HOME_RECOVERY_PASS', flush=True)


if phase == 'prepare':
    fresh = assessed('not_adopted', None, fixture['a'], adopted=False,
                     noctalia='not_adopted', overall='not_adopted')
    if fresh['status'] != 'not_adopted' or home_state.exists():
        raise RuntimeError('fresh assessment adopted or initialized home state')
    cli('init', '--reviewed-safe')
    assessed('accepted_baseline_matches_installed', fixture['a'], fixture['a'], noctalia='not_adopted')
    niri_before, live_before = cli('status'), native.read_bytes()
    run(['sysroot', 'home', 'init'])
    if cli('status') != niri_before or native.read_bytes() != live_before:
        raise RuntimeError('later Noctalia adoption changed the independent niri state or live file')
    assessed('accepted_baseline_matches_installed', fixture['a'], fixture['a'])
    print('KEDRA_R04_INDEPENDENT_GROUP_ADOPTION_PASS', flush=True)
    run(['git', 'init', str(repo)])
    run(['git', '-C', str(repo), 'fetch', '--no-tags',
         '/usr/share/kedra-research/source.bundle', 'refs/heads/kedra-r04-b'])
    run(['git', '-C', str(repo), 'checkout', '--detach', fixture['a']])
    run(['git', '-C', str(repo), 'remote', 'add', 'origin', 'https://github.com/Reidond/kedra.git'])
    text = native.read_text()
    if text.count('gaps 12') != 1 or text.count('width 2') != 1:
        raise RuntimeError('unexpected generated A defaults')
    native.write_text(text.replace('gaps 12', 'gaps 14').replace('width 2', 'width 3'))
    validate()
    cli('keep-local', change('gaps 14'))
    cli('stage', change('width 3'))
    patch = pathlib.Path.home() / 'r04-selected.patch'
    cli('export', '--repo', str(repo), '--output', str(patch))
    run(['git', '-C', str(repo), 'apply', '--check', str(patch)])
    # The exact public fixture commit exists in the bundle and contains this
    # selection. Recording it is not a registry publication or a deployment.
    cli('record-source', '--repo', str(repo), '--commit', fixture['publication'])
    native.write_text(native.read_text().replace('width 3', 'width 4'))
    cli('stage', change('width 4'))
    native.write_text(native.read_text().replace('width 4', 'width 5'))
    validate()
    plan = cli('plan', '--repo', str(repo), '--commit', fixture['b'])
    if plan['retired_publications'] != 1 or plan['pending_publications'] != 0:
        raise RuntimeError('B preflight did not account for the actual source publication')
    state = cli('status')
    if state['accepted_baseline']['source_revision'] != fixture['a'] or state['selection'][0]['after'].strip() != 'width 4':
        raise RuntimeError('preflight advanced B or changed the pinned selection')
    assessed('accepted_baseline_matches_installed', fixture['a'], fixture['a'])
    negative_assessments()
    interrupted_assessment()
    print('KEDRA_R04_CALLER_HOME_A_MATCH_PASS', flush=True)
    print('KEDRA_R04_A_USER_STATE_PASS', flush=True)
elif phase == 'accept-b':
    assessed('reconciliation_required', fixture['a'], fixture['b'],
             noctalia='reconciliation_required', overall='reconciliation_required')
    state = cli('status')
    if state['accepted_baseline']['source_revision'] != fixture['a'] or state['publication_count'] != 1:
        raise RuntimeError('booting B silently changed the accepted home baseline')
    plan = cli('activate-plan', '--repo', str(repo))
    if plan['installed_baseline_revision'] != fixture['b'] or plan['pending_publications'] != 0:
        raise RuntimeError('native B plan has the wrong installed source/publication state')
    cli('apply', '--repo', str(repo), '--plan', '0' * 64, '--activate-managed-file', success=False)
    cli('apply', '--repo', str(repo), '--plan', plan['plan_id'], '--activate-managed-file')
    text, state = native.read_text(), cli('status')
    if not all(value in text for value in ['gaps 14', 'width 5', 'xcursor-size 28']):
        raise RuntimeError('B activation lost a live edit/local rule or failed to apply the incoming default')
    if state['accepted_baseline']['source_revision'] != fixture['b'] or state['publication_count'] != 0 or state['selection'][0]['after'].strip() != 'width 4':
        raise RuntimeError('B acceptance lost independent B/S/P state')
    if not any(rule['before'].strip() == 'gaps 18' and rule['after'].strip() == 'gaps 14' for rule in state['local_only']):
        raise RuntimeError('local gap rule was not reanchored to B')
    validate()
    assessed('accepted_baseline_matches_installed', fixture['b'], fixture['b'],
             noctalia='reconciliation_required', overall='reconciliation_required')
    print('KEDRA_R04_CALLER_HOME_B_RECONCILIATION_PASS', flush=True)
    print('KEDRA_R04_B_NATIVE_ACCEPTANCE_PASS', flush=True)
elif phase == 'accept-a':
    assessed('reconciliation_required', fixture['b'], fixture['a'],
             overall='reconciliation_required')
    before = native.read_text()
    state = cli('status')
    if state['accepted_baseline']['source_revision'] != fixture['b']:
        raise RuntimeError('OS rollback silently reset accepted home B')
    cli('activate-plan', '--repo', str(repo), success=False)
    if native.read_text() != before or cli('status')['accepted_baseline'] != state['accepted_baseline']:
        raise RuntimeError('conflicting rollback plan changed live data or B')
    assessed('reconciliation_required', fixture['b'], fixture['a'],
             overall='reconciliation_required')
    # Resolve the real pinned-selection conflict through the public UI.
    cli('unstage', state['selection'][0]['id'])
    cli('keep-local', change('width 5'))
    plan = cli('activate-plan', '--repo', str(repo))
    cli('apply', '--repo', str(repo), '--plan', plan['plan_id'], '--activate-managed-file')
    text, state = native.read_text(), cli('status')
    if 'gaps 14' not in text or 'width 5' not in text or 'xcursor-size 28' in text:
        raise RuntimeError('rollback acceptance lost local choices or retained the unmodified B-only default')
    if state['accepted_baseline']['source_revision'] != fixture['a'] or state['selection'] or len(state['local_only']) != 2:
        raise RuntimeError('rollback acceptance has the wrong B/S/I state')
    validate()
    assessed('accepted_baseline_matches_installed', fixture['a'], fixture['a'])
    print('KEDRA_R04_CALLER_HOME_ROLLBACK_RECONCILIATION_PASS', flush=True)
    print('KEDRA_R04_A_NATIVE_ROLLBACK_ACCEPTANCE_PASS', flush=True)
else:
    raise RuntimeError('unexpected generated workflow phase')
