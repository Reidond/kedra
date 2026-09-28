#!/usr/bin/python3
"""Guest-only fixture bootstrap: fresh keys arrive through private QEMU fw_cfg."""
import json
import os
import pwd
import re
import subprocess
import time
from pathlib import Path

seed = Path('/sys/firmware/qemu_fw_cfg/by_name/opt/kedra/seed/raw')
if os.geteuid() != 0 or not Path('/usr/share/kedra-lab/added-rpms.txt').is_file():
    raise SystemExit('seed runs only in the disposable native fixture')
subprocess.run(['modprobe', 'qemu_fw_cfg'], check=True)
for _ in range(100):
    if seed.is_file():
        break
    time.sleep(0.1)
with seed.open('rb') as stream:
    raw = stream.read(16385)
if len(raw) > 16384:
    raise SystemExit('oversized seed')
data = json.loads(raw)
if data.get('schema_version') != 1 or set(data) != {'schema_version', 'host_key', 'authorized_key', 'keyring_password'}:
    raise SystemExit('invalid seed schema')
if not data['authorized_key'].startswith('ssh-ed25519 ') or '\n' in data['authorized_key'].strip():
    raise SystemExit('invalid disposable client key')
account = pwd.getpwnam('kedra-test')
subprocess.run(['mkhomedir_helper', 'kedra-test'], check=True)
if not re.fullmatch('[a-f0-9]{64}', data['keyring_password']):
    raise SystemExit('invalid fixture keyring password')
Path('/run/kedra-lab').mkdir(mode=0o755, exist_ok=True)
ssh = Path(account.pw_dir) / '.ssh'
ssh.mkdir(mode=0o700, exist_ok=True)
ssh.chmod(0o700)
os.chown(ssh, account.pw_uid, account.pw_gid)
for path, content, uid, gid in [
    (Path('/etc/ssh/ssh_host_ed25519_key'), data['host_key'], 0, 0),
    (ssh / 'authorized_keys', data['authorized_key'].strip() + '\n', account.pw_uid, account.pw_gid),
    (Path('/run/kedra-lab/keyring-password'), data['keyring_password'], account.pw_uid, account.pw_gid),
]:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as stream:
        os.fchmod(stream.fileno(), 0o600)
        os.fchown(stream.fileno(), uid, gid)
        stream.write(content)
subprocess.run(['ssh-keygen', '-y', '-f', '/etc/ssh/ssh_host_ed25519_key'], check=True, stdout=subprocess.DEVNULL)
subprocess.run(['restorecon', '-RF', '/etc/ssh', str(ssh)], check=True)
