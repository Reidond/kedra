"""Actual installed public-CLI flows across three disposable signed VM boots."""
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request

state = Path('/var/lib/kedra-ghcr-test')
state.mkdir(mode=0o700, exist_ok=True)


def control(name):
    request = urllib.request.Request('http://10.0.2.2:18080/' + name, data=b'', method='POST')
    with urllib.request.urlopen(request, timeout=45) as response:
        assert response.status == 200


def cli(label, *arguments, success=True):
    result = subprocess.run(['/usr/sbin/runuser', '-u', 'kedra-test', '--', '/usr/bin/sysroot', 'update', *arguments],
                            capture_output=True, timeout=2100)
    (state / (label + '.stdout')).write_bytes(result.stdout)
    (state / (label + '.stderr')).write_bytes(result.stderr)
    print('CLI', label, 'exit', result.returncode, flush=True)
    if success:
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors='replace'))
        value=json.loads(result.stdout)
        (state / (label + '.json')).write_text(json.dumps(value,indent=2)+'\n')
        return value
    assert result.returncode != 0, 'Refusal expected: ' + label
    return None


def main():
    assert os.geteuid() == 0
    assert Path('/usr/share/sysroot/disposable-ghcr-test').read_text() == 'Kedra generated GHCR test VM\n'
    hosts=Path('/etc/hosts')
    if '10.0.2.2 ghcr.io' not in hosts.read_text():
        with hosts.open('a') as stream:
            stream.write('\n10.0.2.2 ghcr.io\n')
    assert {line.split()[0] for line in subprocess.check_output(['getent','ahostsv4','ghcr.io'],text=True).splitlines()} == {'10.0.2.2'}
    Path('/var/lib/sysroot').mkdir(mode=0o700, exist_ok=True)
    os.chmod('/var/lib/sysroot', 0o700)
    mount = Path('/run/kedra-ghcr-cases')
    mount.mkdir(exist_ok=True)
    subprocess.run(['mount', '-o', 'ro', '/dev/disk/by-label/KEDRA_GHCR_CASES', str(mount)], check=True)
    cases = json.loads((mount / 'cases.json').read_bytes())
    subprocess.run(['umount', str(mount)], check=True)
    for attempt in range(30):
        try:
            control('online')
            break
        except Exception:
            if attempt == 29:
                raise
            time.sleep(1)
    variant = Path('/usr/share/sysroot/test-variant').read_text().strip()
    phase_file = state / 'phase'
    phase = phase_file.read_text() if phase_file.exists() else 'enroll-a'
    print('NATIVE PHASE', variant, phase, flush=True)
    if (variant, phase) == ('A', 'enroll-a'):
        control('A')
        initial = cli('initial', 'status', '--json')
        assert not initial['enrolled'] and initial['host']['booted']['digest'] == cases['digests']['A']
        enrolled = cli('enroll-a', 'enroll')
        assert enrolled['enrolled'] and enrolled['journal']['high_water']['digest'] == cases['digests']['A']
        cli('repeat-enroll', 'enroll', success=False)
        current = cli('current', 'check', '--json')
        assert current['state'] == 'current'
        for name in ('U', 'W', 'R', 'N', 'T', 'X', 'H', 'M', 'E'):
            control(name)
            cli('reject-' + name, 'check', '--json', success=False)
            unchanged = cli('after-' + name, 'status', '--json')
            assert unchanged['journal']['high_water'] == current['journal']['high_water']
            assert unchanged['host'] == current['host']
        control('offline')
        cli('offline', 'check', '--json', success=False)
        control('online')
        control('B')
        available = cli('available-b', 'check', '--json')
        assert available['state'] == 'available' and available['available']['digest'] == cases['digests']['B']
        staged = cli('stage-b', 'stage', '--expected-digest', cases['digests']['B'])
        assert staged['host']['staged']['digest'] == cases['digests']['B']
        assert staged['journal']['operation']['phase'] == 'awaiting_reboot'
        control('C')
        cli('preserve-pending', 'stage', success=False)
        pending = cli('pending-unchanged', 'status', '--json')
        assert pending['host'] == staged['host'] and pending['journal']['high_water'] == staged['journal']['high_water']
        control('B')
        again = cli('idempotent-stage', 'stage')
        assert again['host'] == staged['host']
        (state / 'personal-data').write_text('A data before B\n')
        phase_file.write_text('boot-b')
        print('KEDRA_GHCR_A_PASS', flush=True)
    elif (variant, phase) == ('B', 'boot-b'):
        booted = cli('booted-b', 'status', '--json')
        assert booted['host']['booted']['digest'] == cases['digests']['B']
        assert booted['journal']['operation']['phase'] == 'booted'
        assert (state / 'personal-data').read_text() == 'A data before B\n'
        control('A')
        cli('lower-replay', 'check', success=False)
        control('B')
        (state / 'personal-data').write_text('B data survives rollback\n')
        subprocess.run(['/usr/bin/python3', '/usr/libexec/kedra-ghcr-identity-recovery.py',
                        '--output', str(state / 'identity-recovery')], check=True, timeout=2400)
        held = cli('rollback-queued', 'status', '--json')
        assert held['journal']['rollback_hold'] and held['host']['rollback_queued']
        assert held['journal']['high_water'] == booted['journal']['high_water']
        phase_file.write_text('rollback-a')
        print('KEDRA_GHCR_B_PASS', flush=True)
    elif (variant, phase) == ('A', 'rollback-a'):
        rolled = cli('rolled-a', 'status', '--json')
        assert rolled['host']['booted']['digest'] == cases['digests']['A']
        assert rolled['journal']['high_water']['digest'] == cases['digests']['B']
        assert rolled['journal']['rollback_hold'] and rolled['journal']['operation']['phase'] == 'booted'
        assert (state / 'personal-data').read_text() == 'B data survives rollback\n'
        control('B')
        assert cli('held-check', 'check')['state'] == 'held'
        cli('held-stage', 'stage', success=False)
        resumed = cli('resume', 'stage', '--resume')
        assert not resumed['journal']['rollback_hold'] and resumed['host']['staged']['digest'] == cases['digests']['B']
        phase_file.write_text('complete')
        print('KEDRA_GHCR_ROLLBACK_PASS', flush=True)
    else:
        raise RuntimeError('Unexpected image/phase')


try:
    main()
except Exception as error:
    print('KEDRA_GHCR_FAIL', type(error).__name__, str(error), flush=True)
finally:
    # Selected generated results only; no account, private-key or unrelated system logs.
    for path in sorted(state.rglob('*.json')):
        print('EVIDENCE', path.relative_to(state), path.read_text(errors='replace'), flush=True)
    for path in sorted(state.glob('*.stderr')):
        print('STDERR', path.name, path.read_text(errors='replace')[:4096], flush=True)
    subprocess.run(['systemctl', 'poweroff', '--no-block'], check=True)
