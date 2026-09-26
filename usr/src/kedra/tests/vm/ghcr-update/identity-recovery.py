"""Manual native recovery case in an already prepared, disposable signed A/B VM.

Run after signed B has booted and its v2 journal retains A as native rollback.
The guest must use a disposable key, the marker below, and a generated owner's
existing helper sudo authorization. This intentionally queues real rollback;
the surrounding VM workflow then boots A and checks retained state. It never
creates a registry, changes trust policy, or runs on the workstation.
"""
import argparse
import json
import os
from pathlib import Path
import pwd
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--owner', default='kedra-test')
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
marker = Path('/usr/share/sysroot/disposable-ghcr-test')
assert os.geteuid() == 0 and marker.is_file() and not marker.is_symlink()
assert marker.stat().st_uid == 0 and marker.read_text() == 'Kedra generated GHCR test VM\n'
assert subprocess.check_output(['systemd-detect-virt', '--vm'], text=True).strip() in ('qemu', 'kvm')
fingerprint = json.loads(subprocess.check_output([
    '/usr/bin/sysroot', 'release', 'key', '--public-key', '/usr/lib/sysroot/trust/release.pub', '--json']))
assert fingerprint['key_fingerprint_sha256'] != 'a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e'
owner = pwd.getpwnam(args.owner)
assert owner.pw_uid >= 1000
assert not args.output.exists()
args.output.mkdir(parents=True, mode=0o700)


def cli(*arguments, success=True):
    result = subprocess.run(['/usr/sbin/runuser', '-u', args.owner, '--', '/usr/bin/sysroot',
                             'update', *arguments], capture_output=True, timeout=2100)
    if success:
        if result.returncode:
            raise RuntimeError(result.stderr.decode(errors='replace'))
    elif result.returncode == 0:
        raise RuntimeError('Identity-mismatched forward operation unexpectedly succeeded')
    return result


before = json.loads(cli('status', '--json').stdout)
assert before['schema_version'] == 2 and before['enrolled']
assert before['journal']['identity_health_error'] is None
assert before['host']['rollback'] and not before['host']['rollback_queued'] and before['host']['staged'] is None
expected_rollback = before['host']['rollback']['digest']
high_water = before['journal']['high_water']
path = Path('/usr/share/sysroot/image-identity.json')
identity = json.loads(path.read_bytes())
identity['run_attempt'] += 1
mounted = False
try:
    with tempfile.TemporaryDirectory(prefix='kedra-native-identity-') as temporary:
        replacement = Path(temporary) / 'identity.json'
        replacement.write_text(json.dumps(identity, sort_keys=True, separators=(',', ':')))
        replacement.chmod(0o600)
        subprocess.run(['/usr/bin/mount', '--bind', str(replacement), str(path)], check=True)
        mounted = True
        damaged = json.loads(cli('status', '--json', success=False).stdout)
        assert damaged['state'] == 'identity_mismatch'
        assert damaged['journal']['high_water'] == high_water
        assert damaged['journal']['identity_health_error']
        for operation in ('check', 'enroll', 'stage'):
            cli(operation, success=False)
        rolled = json.loads(cli('rollback').stdout)
        assert rolled['host']['rollback_queued']
        assert rolled['journal']['operation']['target_digest'] == expected_rollback
        assert rolled['journal']['operation']['phase'] == 'awaiting_reboot'
        assert rolled['journal']['rollback_hold']
        assert rolled['journal']['high_water'] == high_water
        assert rolled['journal']['identity_health_error']
        for name, value in [('before', before), ('mismatched', damaged), ('rollback', rolled)]:
            (args.output / (name + '.json')).write_text(json.dumps(value, indent=2) + '\n')
        subprocess.run(['/usr/bin/umount', str(path)], check=True)
        mounted = False
finally:
    if mounted:
        subprocess.run(['/usr/bin/umount', str(path)], check=True)

after = json.loads(cli('status', '--json').stdout)
assert after['journal']['identity_health_error'] is None
assert after['journal']['high_water'] == high_water and after['journal']['rollback_hold']
assert after['host']['rollback_queued']
(args.output / 'after-unmount.json').write_text(json.dumps(after, indent=2) + '\n')
print('PASS: mismatched identity refuses forward work but queues authenticated retained rollback; reboot A next')
