"""Actual user CLI across signed image A, image B, and retained-image rollback."""
import json
import pathlib
import subprocess
import sys

phase = sys.argv[1]
fixture = json.loads(pathlib.Path('/usr/share/kedra-research/fixture.json').read_text())
native = pathlib.Path.home() / '.config/niri/config.kdl'
repo = pathlib.Path.home() / 'kedra-r04-source'


def run(args, success=True):
    result = subprocess.run(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=90, check=False)
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


if phase == 'prepare':
    run(['sysroot', 'home', 'init'])
    cli('init', '--reviewed-safe')
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
    print('KEDRA_R04_A_USER_STATE_PASS', flush=True)
elif phase == 'accept-b':
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
    print('KEDRA_R04_B_NATIVE_ACCEPTANCE_PASS', flush=True)
elif phase == 'accept-a':
    before = native.read_text()
    state = cli('status')
    if state['accepted_baseline']['source_revision'] != fixture['b']:
        raise RuntimeError('OS rollback silently reset accepted home B')
    cli('activate-plan', '--repo', str(repo), success=False)
    if native.read_text() != before or cli('status')['accepted_baseline'] != state['accepted_baseline']:
        raise RuntimeError('conflicting rollback plan changed live data or B')
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
    print('KEDRA_R04_A_NATIVE_ROLLBACK_ACCEPTANCE_PASS', flush=True)
else:
    raise RuntimeError('unexpected generated workflow phase')
