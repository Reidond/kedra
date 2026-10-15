# Runs inside the disposable VM guest with its own python3; uv applies to host-side scripts only.
"""Actual installed public-CLI flows across three disposable signed VM boots."""
import hashlib
import importlib.util
import json
import os
import pwd
import stat
import subprocess
import time
import urllib.request
from pathlib import Path

state = Path('/var/lib/kedra-ghcr-test')
state.mkdir(mode=0o700, exist_ok=True)
# The one shared guest check, installed from usr/src/kedra/tests/common.
SECURE_BOOT = Path('/usr/libexec/kedra-secure-boot-check.py')


def check_secure_boot():
    """UEFI Secure Boot through the shared guest check, then SELinux enforcement."""
    specification = importlib.util.spec_from_file_location('kedra_secure_boot_check', SECURE_BOOT)
    shared = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(shared)
    print(json.dumps(shared.check(), sort_keys=True), flush=True)
    assert subprocess.check_output(['/usr/sbin/getenforce'], text=True).strip() == 'Enforcing'
    print(shared.PASS, flush=True)


def native_material(cases):
    expected = cases.get('native_receipt_sha256')
    if expected is None:
        return
    receipt_path = Path('/usr/share/sysroot/native-receipt.json')
    assert hashlib.sha256(receipt_path.read_bytes()).hexdigest() == expected
    receipt = json.loads(receipt_path.read_bytes())
    assert receipt['schema'] == 1 and receipt['artifacts'] and receipt['kernels']
    observer_path = Path('/usr/libexec/kedra-ghcr-native.py')
    assert hashlib.sha256(observer_path.read_bytes()).hexdigest() == cases['native_observer_sha256']
    specification = importlib.util.spec_from_file_location('kedra_native_observer', observer_path)
    observer = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(observer)
    value = {'material': receipt}
    observer.verify_material(value, installed=True)
    selected = observer.boot_artifacts(value)
    (state / 'native-material.json').write_text(json.dumps({
        'schema_version': 1, 'receipt_sha256': expected, 'running_kernel': os.uname().release,
        'artifacts_verified': len(receipt['artifacts']), 'boot_artifacts': selected,
    }) + '\n')


def home_data(expected=None, replacement=None):
    owner = pwd.getpwnam('kedra-test')
    home = Path(owner.pw_dir)
    path = home / '.config/kedra-fixture/preference'
    if expected is not None:
        info = path.stat()
        assert path.read_text() == expected and info.st_uid == owner.pw_uid
        assert stat.S_IMODE(info.st_mode) == 0o600
    if replacement is not None:
        subprocess.run(['/usr/sbin/runuser', '-u', 'kedra-test', '--', '/usr/bin/python3', '-c',
            ('from pathlib import Path; import os,sys; p=Path.home()/".config/kedra-fixture/preference"; '
             'p.parent.mkdir(parents=True,exist_ok=True); p.write_text(sys.argv[1]); os.chmod(p,0o600)'),
            replacement], check=True, timeout=30)


def control(name):
    request = urllib.request.Request('http://10.0.2.2:18080/' + name, data=b'', method='POST')
    with urllib.request.urlopen(request, timeout=45) as response:
        assert response.status == 200


def layer_gets(name):
    """Completed registry downloads per layer of signed test image `name`, counted on the registry host."""
    request = urllib.request.Request('http://10.0.2.2:18080/layer-gets/' + name, data=b'', method='POST')
    with urllib.request.urlopen(request, timeout=150) as response:
        assert response.status == 200
        counts = json.loads(response.read(1048577))
    assert counts and all(isinstance(value, int) for value in counts.values())
    return counts


def downloads(label, name, before, once=()):
    """Layers in `once` were downloaded exactly once since `before`; every other layer not at all."""
    after = layer_gets(name)
    changed = {digest: after[digest] - before[digest] for digest in after if after[digest] != before[digest]}
    expected = dict.fromkeys(once, 1)
    (state / ('downloads-' + label + '.json')).write_text(json.dumps({
        'image': name, 'layers': len(after), 'downloaded': changed, 'expected': expected}, indent=2) + '\n')
    assert changed == expected, f'{label}: layer downloads {changed}, expected {expected}'
    return after


def cli(label, *arguments, success=True):
    result = subprocess.run(['/usr/sbin/runuser', '-u', 'kedra-test', '--', '/usr/bin/sysroot', 'update', *arguments],
                            capture_output=True, timeout=2100, check=False)
    with (state / (label + '.exit.json')).open('w') as record:
        json.dump({'schema_version': 1, 'label': label, 'returncode': result.returncode}, record, sort_keys=True)
        record.write('\n')
        record.flush()
        os.fsync(record.fileno())
    directory = os.open(state, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)
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
    check_secure_boot()
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
    native_material(cases)
    if cases.get('require_fresh_installation'):
        installed = json.loads((state / 'fresh-install.json').read_bytes())
        assert installed == {'schema_version': 1, 'installer': 'anaconda',
                             'target_serial': 'KEDRA_INSTALL_ONLY', 'sentinel_serial': 'KEDRA_KEEP_DATA'}
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
    phase = phase_file.read_text() if phase_file.exists() else 'update-a'
    print('NATIVE PHASE', variant, phase, flush=True)
    if (variant, phase) == ('A', 'update-a'):
        marker = update_a(cases)
        phase_file.write_text('boot-b')
    elif (variant, phase) == ('B', 'boot-b'):
        marker = boot_b(cases)
        phase_file.write_text('rollback-a')
    elif (variant, phase) == ('A', 'rollback-a'):
        marker = rollback_a(cases)
        phase_file.write_text('complete')
    else:
        raise RuntimeError('Unexpected image/phase')
    return marker


def unchanged(label, before):
    """A refused operation left native deployments and the rollback hold as they were."""
    after = cli(label, 'status', '--json')
    assert after['host'] == before['host'] and after['rollback_hold'] == before['rollback_hold']
    return after


# Each refused signature names the reason bootc's policy check gave, before any layer download.
SIGNATURE_REFUSALS = {
    'U': 'A signature was required, but no signature exists',
    'W': 'cryptographic signature verification failed',
    'R': 'is not accepted',
    'N': 'A signature was required, but no signature exists',
}


def update_a(cases):
    home_data(replacement='A preference before B\n')
    digests = cases['digests']
    control('A')
    initial = cli('initial', 'status', '--json')
    assert initial['schema_version'] == 3 and initial['state'] == 'booted' and not initial['rollback_hold']
    assert initial['host']['booted']['digest'] == digests['A'] and initial['installed_identity_error'] is None
    current = cli('current', 'check', '--json')
    assert current['state'] == 'current' and current['available']['digest'] == digests['A']
    assert current['signature_verified'] and current['registry_checked']
    # Identity refusals: the helper reads only the digest-bound label, before bootc runs.
    for name in ('T', 'X', 'H', 'M', 'E', 'D'):
        control(name)
        before = layer_gets(name)
        cli('check-' + name, 'check', '--json', success=False)
        cli('stage-' + name, 'stage', success=False)
        downloads('refused-' + name, name, before)
        unchanged('after-' + name, initial)
    assert 'below the booted image' in (state / 'check-D.stderr').read_text()
    # Signature refusals: the label is in scope and newer, so check reports it;
    # bootc's policy refuses it at staging before fetching any layer.
    for name, reason in SIGNATURE_REFUSALS.items():
        control(name)
        before = layer_gets(name)
        reported = cli('check-' + name, 'check', '--json')
        assert reported['state'] == 'available' and not reported['signature_verified']
        cli('stage-' + name, 'stage', success=False)
        assert reason in (state / ('stage-' + name + '.stderr')).read_text(), name
        downloads('refused-' + name, name, before)
        unchanged('after-' + name, initial)
    control('offline')
    cli('offline', 'check', '--json', success=False)
    control('online')
    control('B')
    a_layers, b_layers = layer_gets('A'), layer_gets('B')
    new = sorted(set(b_layers) - set(a_layers))
    assert new and set(b_layers) & set(a_layers), 'fixture B must share base layers with A and add its own'
    available = cli('available-b', 'check', '--json')
    assert available['state'] == 'available' and available['available']['digest'] == digests['B']
    assert not available['signature_verified']
    cli('expected-other', 'stage', '--expected-digest', digests['A'], success=False)
    unchanged('after-expected-other', initial)
    # bootc verifies B and fetches only its new layers, once; no separate verification copy.
    staged = cli('stage-b', 'stage', '--expected-digest', digests['B'])
    assert staged['state'] == 'staged' and staged['host']['staged']['digest'] == digests['B']
    assert staged['signature_verified'] and staged['host']['booted']['digest'] == digests['A']
    b_layers = downloads('stage-b', 'B', b_layers, once=new)
    control('C')
    pending = cli('pending-c', 'check', '--json')
    assert pending['state'] == 'pending_other_deployment' and pending['host'] == staged['host']
    cli('preserve-pending', 'stage', success=False)
    unchanged('pending-unchanged', staged)
    control('B')
    again = cli('idempotent-stage', 'stage')
    assert again['host'] == staged['host'] and again['state'] == 'staged'
    downloads('idempotent-stage', 'B', b_layers)
    (state / 'personal-data').write_text('A data before B\n')
    return 'KEDRA_GHCR_A_PASS'


def boot_b(cases):
    home_data(expected='A preference before B\n', replacement='B preference survives rollback\n')
    digests = cases['digests']
    booted = cli('booted-b', 'status', '--json')
    assert booted['state'] == 'booted' and booted['host']['booted']['digest'] == digests['B']
    assert booted['host']['rollback']['digest'] == digests['A'] and not booted['rollback_hold']
    assert (state / 'personal-data').read_text() == 'A data before B\n'
    control('A')
    cli('lower-replay', 'check', success=False)
    assert 'below the booted image' in (state / 'lower-replay.stderr').read_text()
    cli('lower-replay-stage', 'stage', success=False)
    unchanged('after-lower-replay', booted)
    control('B')
    assert cli('current-b', 'check')['state'] == 'current'
    (state / 'personal-data').write_text('B data survives rollback\n')
    subprocess.run(['/usr/bin/python3', '/usr/libexec/kedra-ghcr-identity-recovery.py',
                    '--output', str(state / 'identity-recovery')], check=True, timeout=2400)
    held = cli('rollback-queued', 'status', '--json')
    assert held['state'] == 'held' and held['rollback_hold'] and held['host']['rollback_queued']
    cli('queued-stage', 'stage', '--resume', success=False)
    unchanged('after-queued-stage', held)
    return 'KEDRA_GHCR_B_PASS'


def rollback_a(cases):
    home_data(expected='B preference survives rollback\n')
    digests = cases['digests']
    rolled = cli('rolled-a', 'status', '--json')
    assert rolled['host']['booted']['digest'] == digests['A'] and rolled['host']['rollback']['digest'] == digests['B']
    assert rolled['state'] == 'held' and rolled['rollback_hold'] and not rolled['host']['rollback_queued']
    assert (state / 'personal-data').read_text() == 'B data survives rollback\n'
    control('B')
    b_layers = layer_gets('B')
    held = cli('held-check', 'check')
    assert held['state'] == 'held' and held['available']['digest'] == digests['B']
    cli('held-stage', 'stage', success=False)
    unchanged('after-held-stage', rolled)
    resumed = cli('resume', 'stage', '--resume')
    assert not resumed['rollback_hold'] and resumed['host']['staged']['digest'] == digests['B']
    assert resumed['state'] == 'staged' and resumed['signature_verified']
    # The retained B deployment already holds every layer.
    downloads('resume', 'B', b_layers)
    assert not cli('after-resume', 'status', '--json')['rollback_hold']
    return 'KEDRA_GHCR_ROLLBACK_PASS'


phase_marker = None
try:
    phase_marker = main()
except Exception as error:  # noqa: BLE001 - any failure must reach the host as a marker
    print('KEDRA_GHCR_FAIL', type(error).__name__, str(error), flush=True)
finally:
    try:
        # Selected generated results only; no account, private-key or unrelated system logs.
        for path in sorted(state.rglob('*.json')):
            print('EVIDENCE', path.relative_to(state), path.read_text(errors='replace'), flush=True)
        for path in sorted(state.glob('*.stderr')):
            print('STDERR', path.name, path.read_text(errors='replace')[:4096], flush=True)
        if phase_marker is not None:
            print(phase_marker, flush=True)
    finally:
        subprocess.run(['systemctl', 'poweroff', '--no-block'], check=True)
