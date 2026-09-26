"""Actual installed public-CLI flows across three disposable signed VM boots."""
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request

state = Path('/var/lib/kedra-ghcr-test')
state.mkdir(mode=0o700, exist_ok=True)
# The helper's persistent verified OCI layout (a transfer cache, never an authority).
layout = Path('/var/lib/sysroot/verified-oci')
EFI_GLOBAL = '8be4df61-93ca-11d2-aa0d-00e098032b8c'


def check_secure_boot():
    """UEFI Secure Boot as seen by shim, firmware variables, lockdown and kernel."""
    shim = subprocess.run(['mokutil', '--sb-state'], capture_output=True, text=True, timeout=60, check=True)
    assert shim.stdout.rstrip('\n') == 'SecureBoot enabled', shim.stdout
    for name, value in (('SecureBoot', 1), ('SetupMode', 0)):
        # efivarfs prefixes each value with 4 attribute bytes.
        with open(f'/sys/firmware/efi/efivars/{name}-{EFI_GLOBAL}', 'rb') as stream:
            data = stream.read(6)
        assert len(data) == 5 and data[4] == value, (name, data.hex())
    lockdown = Path('/sys/kernel/security/lockdown').read_text()
    print('LOCKDOWN', lockdown.strip(), flush=True)
    assert '[integrity]' in lockdown or '[confidentiality]' in lockdown, lockdown
    kernel = subprocess.run(['journalctl', '-k', '-b', '--no-pager', '-o', 'cat'],
                            capture_output=True, text=True, timeout=120, check=True).stdout
    assert 'secureboot: Secure boot enabled' in kernel.splitlines()
    print('KEDRA_SECUREBOOT_PASS', flush=True)


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


def transfers(label, name, before, reused=(), downloaded=()):
    """Layers in `reused` were not downloaded again since `before`; each in `downloaded` was."""
    after = layer_gets(name)
    again = sorted(digest for digest in reused if after[digest] != before[digest])
    missing = sorted(digest for digest in downloaded if after[digest] == before[digest])
    (state / ('transfers-' + label + '.json')).write_text(json.dumps({
        'image': name, 'layers': len(after), 'reused': len(reused), 'downloaded': len(downloaded),
        'reused_but_downloaded': again, 'expected_download_missing': missing}, indent=2) + '\n')
    assert not again, f'{label}: cached layers were downloaded again: {again}'
    assert not missing, f'{label}: layers were not downloaded: {missing}'
    return after


def cached_blobs():
    """Identity of each cached blob file; a downloaded blob is written as a new file."""
    return {path.name: (path.stat().st_ino, path.stat().st_mtime_ns) for path in (layout / 'blobs/sha256').iterdir()}


def cached_refs():
    """Ref names of the images the verified OCI layout currently holds."""
    index = json.loads((layout / 'index.json').read_bytes())
    return {entry['annotations']['org.opencontainers.image.ref.name'] for entry in index.get('manifests', [])}


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
        first = layer_gets('A')
        enrolled = cli('enroll-a', 'enroll')
        assert enrolled['enrolled'] and enrolled['journal']['high_water']['digest'] == cases['digests']['A']
        # The first verification downloads the whole image; repeated ones reuse every cached layer.
        a_layers = transfers('first-a', 'A', first, downloaded=first)
        cli('repeat-enroll', 'enroll', success=False)
        current = cli('current', 'check', '--json')
        assert current['state'] == 'current'
        a_layers = transfers('repeat-a', 'A', a_layers, reused=a_layers)
        # Unexpected cache state is discarded without being followed, then downloaded again.
        canary = state / 'canary'
        canary.write_text('unchanged\n')
        (layout / 'index.json').unlink()
        (layout / 'index.json').symlink_to(canary)
        rechecked = cli('discarded-cache', 'check', '--json')
        assert rechecked['state'] == 'current' and rechecked['host'] == current['host']
        assert canary.read_text() == 'unchanged\n' and not (layout / 'index.json').is_symlink()
        assert 'discarding the verified image cache' in (state / 'discarded-cache.stderr').read_text()
        a_layers = transfers('discarded-cache', 'A', a_layers, downloaded=a_layers)
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
        b_layers = layer_gets('B')
        shared, new = sorted(set(b_layers) & set(a_layers)), sorted(set(b_layers) - set(a_layers))
        assert shared and new, 'fixture B must share base layers with A and add its own'
        available = cli('available-b', 'check', '--json')
        assert available['state'] == 'available' and available['available']['digest'] == cases['digests']['B']
        b_layers = transfers('available-b', 'B', b_layers, reused=shared, downloaded=new)
        # Staging re-verifies the checked image without downloading its cached layers
        # (bootc's own layer fetches go to ostree, so the cached files are compared).
        blobs = cached_blobs()
        assert all(digest.removeprefix('sha256:') in blobs for digest in b_layers)
        staged = cli('stage-b', 'stage', '--expected-digest', cases['digests']['B'])
        assert staged['host']['staged']['digest'] == cases['digests']['B']
        assert staged['journal']['operation']['phase'] == 'awaiting_reboot'
        restaged = cached_blobs()
        rewritten = sorted(d for d in b_layers if restaged.get(d.removeprefix('sha256:')) != blobs[d.removeprefix('sha256:')])
        (state / 'transfers-stage-b.json').write_text(json.dumps({'layers': len(b_layers), 'rewritten': rewritten}, indent=2) + '\n')
        assert not rewritten, f'stage downloaded cached layers again: {rewritten}'
        control('C')
        cli('preserve-pending', 'stage', success=False)
        pending = cli('pending-unchanged', 'status', '--json')
        assert pending['host'] == staged['host'] and pending['journal']['high_water'] == staged['journal']['high_water']
        # The refused stage verified and cached C. Once C's signature is withdrawn, its
        # cached layers must not authorize it: every run re-verifies the registry signature.
        c_ref = 'sha256-' + cases['digests']['C'].removeprefix('sha256:')
        assert c_ref in cached_refs(), 'the refused stage must leave verified C cached'
        c_layers = layer_gets('C')
        control('unsign/C')
        cli('cached-unsigned-c', 'check', '--json', success=False)
        assert 'Source image rejected' in (state / 'cached-unsigned-c.stderr').read_text()
        withdrawn = cli('after-unsigned-c', 'status', '--json')
        assert withdrawn['host'] == staged['host'] and withdrawn['journal']['high_water'] == staged['journal']['high_water']
        assert c_ref in cached_refs(), 'C must still be cached when its signature is refused'
        transfers('cached-unsigned-c', 'C', c_layers, reused=c_layers)
        control('B')
        b_layers = layer_gets('B')
        again = cli('idempotent-stage', 'stage')
        assert again['host'] == staged['host']
        transfers('idempotent-stage', 'B', b_layers, reused=b_layers)
        (state / 'personal-data').write_text('A data before B\n')
        phase_file.write_text('boot-b')
        print('KEDRA_GHCR_A_PASS', flush=True)
    elif (variant, phase) == ('B', 'boot-b'):
        booted = cli('booted-b', 'status', '--json')
        assert booted['host']['booted']['digest'] == cases['digests']['B']
        assert booted['journal']['operation']['phase'] == 'booted'
        assert (state / 'personal-data').read_text() == 'A data before B\n'
        control('A')
        a_layers = layer_gets('A')
        cli('lower-replay', 'check', success=False)
        # The cache persists across reboot for the retained rollback image.
        transfers('lower-replay', 'A', a_layers, reused=a_layers)
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
        b_layers = layer_gets('B')
        assert cli('held-check', 'check')['state'] == 'held'
        transfers('held-check', 'B', b_layers, reused=b_layers)
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
