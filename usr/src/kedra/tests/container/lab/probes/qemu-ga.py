#!/usr/bin/env python3
# Runs inside the lab container as root with the image's own python3; uv applies to host-side scripts only.
"""qemu-arm64 guest agent: the target drop-in blocks exactly the root-granting RPCs.

Fedora 44 blocks no guest-agent RPC. The usr/src/kedra/image/targets/qemu-arm64
drop-in must add --block-rpcs on top of Fedora's otherwise unchanged command,
and the installed agent, on a private socket, must disable exactly that set.
"""
import json
import pathlib
import socket
import subprocess
import sys
import tempfile
import time

BLOCKED = sorted(
    ['guest-exec', 'guest-exec-status', 'guest-file-close', 'guest-file-flush', 'guest-file-open', 'guest-file-read', 'guest-file-seek', 'guest-file-write', 'guest-set-user-password', 'guest-ssh-add-authorized-keys', 'guest-ssh-get-authorized-keys', 'guest-ssh-remove-authorized-keys'])
FLAG = '  --block-rpcs=' + ','.join(BLOCKED) + ' \\'


def execstart(text):
    """The last ExecStart= with its continuation lines, as systemd resolves drop-ins."""
    command, more = None, False
    for line in text.splitlines():
        if line.startswith('ExecStart='):
            command, more = [line], line.endswith('\\')
        elif more:
            command.append(line)
            more = line.endswith('\\')
    return command or []


packaged = pathlib.Path('/usr/lib/systemd/system/qemu-guest-agent.service').read_text()
effective = subprocess.run(['systemctl', 'cat', 'qemu-guest-agent.service'],
                           check=True, capture_output=True, text=True).stdout
effective_command = execstart(effective)
if FLAG not in effective_command:
    sys.exit('The effective qemu-guest-agent ExecStart lacks the qemu-arm64 block list')
if [line for line in effective_command if line != FLAG] != execstart(packaged):
    sys.exit("The qemu-arm64 drop-in changes more than Fedora's qemu-ga command")
if subprocess.run(['rpm', '-V', '--configfiles', 'qemu-guest-agent'], capture_output=True, check=False).stdout:
    sys.exit('/etc/sysconfig/qemu-ga differs from the package')

with tempfile.TemporaryDirectory() as state:
    path = state + '/socket'
    agent_process = subprocess.Popen(
        ['qemu-ga', '--method=unix-listen', '--path=' + path, '--pidfile=' + state + '/pid',
         '--statedir=' + state, '--block-rpcs=' + ','.join(BLOCKED)], stdin=subprocess.DEVNULL)
    try:
        for _ in range(100):
            agent = socket.socket(socket.AF_UNIX)
            try:
                agent.connect(path)
                break
            except OSError:
                agent.close()
                time.sleep(0.1)
        else:
            sys.exit('qemu-ga did not listen on ' + path)
        agent.settimeout(30)
        stream = agent.makefile('rw')

        def call(command, **arguments):
            stream.write(json.dumps({'execute': command, 'arguments': arguments}) + '\n')
            stream.flush()
            return json.loads(stream.readline())

        info = call('guest-info')['return']
        disabled = sorted(c['name'] for c in info['supported_commands'] if not c['enabled'])
        print('qemu-ga', info['version'], 'disabled:', ','.join(disabled))
        if disabled != BLOCKED:
            sys.exit('qemu-ga disabled RPC set differs from the qemu-arm64 block list')
        refused = call('guest-exec', path='/usr/bin/true')
        print('guest-exec:', json.dumps(refused))
        if refused.get('error', {}).get('class') != 'CommandNotFound':
            sys.exit('qemu-ga did not refuse guest-exec')
    finally:
        agent_process.terminate()
        agent_process.wait(timeout=10)
print('KEDRA_QEMU_GA_BLOCK_PASS')
