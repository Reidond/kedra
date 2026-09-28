#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Retained native QEMU instances; private state, QMP ownership and pinned guest SSH."""
import argparse
import fcntl
import hashlib
import json
import os
import re
import secrets
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


class GraphicsError(ValueError):
    """A ready compositor selected a renderer incompatible with its profile."""


def run(argv, *, timeout=30, **kwargs):
    return subprocess.run(list(map(str, argv)), check=True, timeout=timeout, **kwargs)


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write_json(path, value):
    pending = path.with_suffix('.pending')
    with pending.open('w') as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(pending, path)


def runtime_check(runtime):
    run(['uv', 'run', '--offline', '--script', HERE / 'check-runtime.py', '--runtime', runtime], stdout=subprocess.DEVNULL)


def qmp(directory, state, execute, arguments=None):
    with socket.socket(socket.AF_UNIX) as sock:
        sock.settimeout(5)
        sock.connect(str(directory / 'qmp.sock'))
        stream = sock.makefile('rwb')
        if 'QMP' not in json.loads(stream.readline(1048576)):
            raise ValueError('invalid QMP greeting')

        def command(name, params=None):
            stream.write(json.dumps({'execute': name, 'arguments': params or {}, 'id': name}).encode() + b'\n')
            stream.flush()
            for _ in range(100):
                response = json.loads(stream.readline(1048576))
                if response.get('id') == name:
                    if 'error' in response:
                        raise ValueError(str(response['error']))
                    return response.get('return')
            raise ValueError('QMP response missing')

        command('qmp_capabilities')
        if command('query-name').get('name') != state['identity']:
            raise ValueError('QMP socket belongs to another VM')
        return command(execute, arguments)


def process_identity(pid):
    value = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='], capture_output=True, text=True, check=False)
    return value.stdout.strip() if value.returncode == 0 else None


def owned_process(state, key):
    entry = state.get(key)
    return bool(entry and entry['identity'] and process_identity(entry['pid']) == entry['identity'])


def check_process_ownership(state):
    for key in ('qemu', 'swtpm'):
        entry = state.get(key)
        if not entry:
            continue
        if not isinstance(entry, dict) or type(entry.get('pid')) is not int or entry['pid'] <= 0:
            raise ValueError('invalid process record: ' + key)
        actual = process_identity(entry['pid'])
        if actual and actual != entry.get('identity'):
            raise ValueError(f'{key} PID belongs to another process; inspect the instance state before continuing')


def ssh(directory, state, argv, *, data=None, timeout=30, connect_timeout=10, session=True):
    # Controller updates should not require rebuilding the OS disk. This trusted
    # transport runs only as the fixture account; argv stays separately quoted.
    remote = ['sh', '-c', (HERE / 'session-exec').read_text(), 'kedra-lab-session', *argv] if session else argv
    remote = ['timeout', '--kill-after=2s', f'{max(0.1, timeout - 2):.3f}s', *remote]
    command = ['ssh', '-F', '/dev/null', '-o', 'BatchMode=yes', '-o', 'IdentitiesOnly=yes', '-o', 'IdentityAgent=none',
               '-o', 'StrictHostKeyChecking=yes', '-o', 'GlobalKnownHostsFile=/dev/null',
               '-o', 'UserKnownHostsFile=' + str(directory / 'known_hosts'), '-o', 'HostKeyAlias=' + state['identity'],
               '-o', f'ConnectTimeout={connect_timeout}', '-o', 'ConnectionAttempts=1', '-o', 'LogLevel=ERROR',
               '-i', directory / 'client-key', '-p', str(state['ssh_port']), 'kedra-test@127.0.0.1',
               shlex.join(remote)]
    try:
        return run(command, input=data, capture_output=True, timeout=timeout).stdout
    except subprocess.TimeoutExpired:
        raise ValueError(f'guest command {shlex.join(argv)} timed out after {timeout}s') from None


def check_renderer(directory, state, deadline=None):

    def query(argv):
        left = 30 if deadline is None else min(30, deadline - time.monotonic())
        if left <= 0:
            raise GraphicsError('graphics readiness deadline expired')
        return ssh(directory, state, argv, timeout=left).decode()

    journal = query(['journalctl', '--user', '-b', '-u', 'niri.service', '--no-pager', '-o', 'cat'])
    renderers = re.findall(r'GL Renderer: "([^"]+)"', journal)
    renderer_source = 'niri-journal'
    if not renderers:
        raise GraphicsError('niri did not report its actual renderer')
    renderer = renderers[-1]
    if 'virgl' not in renderer or 'ANGLE Metal Renderer:' not in renderer:
        raise GraphicsError('accelerated profile refused compositor renderer: ' + renderer)
    return renderer, renderer_source


def create(directory, args):
    runtime_check(args.runtime)
    if args.image is None:
        raise ValueError('new instance needs --image pointing to a prepared image.json')
    image = json.loads(args.image.read_text())
    disk = Path(image['disk'])
    if image.get('schema_version') != 1 or image.get('target') != 'qemu-arm64' or sha256(disk) != image.get('disk_sha256'):
        raise ValueError('prepared disk receipt/checksum mismatch')
    display = re.fullmatch(r'(\d+)x(\d+)@([\d.]+)', args.display)
    if display is None:
        raise ValueError('display must be WIDTHxHEIGHT@SCALE')
    width, height, scale = int(display[1]), int(display[2]), float(display[3])
    if not (640 <= width <= 7680 and 480 <= height <= 4320 and 0.5 <= scale <= 4):
        raise ValueError('display dimensions/scale out of range')
    state = {'schema_version': 1, 'mode': 'lab', 'identity': 'kedra-' + args.name + '-' + secrets.token_hex(12),
             'runtime': str(args.runtime), 'runtime_receipt': sha256(args.runtime / 'receipt.json'),
             'image': image, 'memory_mib': args.memory_mib, 'cpus': args.cpus,
             'display': {'width': width, 'height': height, 'scale': scale},
             'graphics': 'accelerated'}
    for name in ['client-key', 'host-key']:
        run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-C', 'disposable-kedra-lab', '-f', directory / name])
    host_public = (directory / 'host-key.pub').read_text().strip()
    (directory / 'known_hosts').write_text(state['identity'] + ' ' + host_public + '\n')
    write_json(directory / 'seed.json', {'schema_version': 1, 'host_key': (directory / 'host-key').read_text(),
                                      'authorized_key': (directory / 'client-key.pub').read_text(),
                                      'keyring_password': secrets.token_hex(32)})
    run([args.runtime / 'bin/qemu-img', 'create', '-f', 'qcow2', '-F', 'qcow2', '-b', disk, directory / 'disk.qcow2'])
    shutil.copy2(args.runtime / 'firmware/AAVMF_VARS.ms.fd', directory / 'vars.fd')
    run(['uv', 'run', '--project', HERE / 'build-tools', '--locked', 'virt-fw-vars', '--inplace', directory / 'vars.fd',
         '--append-boot-filepath', '\\EFI\\fedora\\shimaa64.efi'])
    (directory / 'tpm').mkdir(mode=0o700)
    write_json(directory / 'state.json', state)
    return state


def create_installer(directory, args):
    runtime_check(args.runtime)
    media = HERE.parents[2] / 'installer/macos/media.py'
    verified = run(['uv', 'run', '--script', media, 'verify', '--iso', args.iso], capture_output=True)
    image = json.loads(verified.stdout)
    state = {'schema_version': 1, 'mode': 'installer', 'identity': 'kedra-' + args.name + '-' + secrets.token_hex(12),
             'runtime': str(args.runtime), 'runtime_receipt': sha256(args.runtime / 'receipt.json'),
             'image': image, 'memory_mib': args.memory_mib, 'cpus': args.cpus,
             'installer': 'installer.iso', 'installer_probe': args.probe}
    # APFS copy-on-write clone; the reviewed source ISO remains untouched.
    run(['/bin/cp', '-c', args.iso, directory / 'installer.iso'], timeout=1800)
    if sha256(directory / 'installer.iso') != image['installer']['sha256']:
        raise ValueError('cloned installer checksum mismatch')
    run([args.runtime / 'bin/qemu-img', 'create', '-f', 'qcow2', directory / 'disk.qcow2', str(args.disk_gib) + 'G'])
    shutil.copy2(args.runtime / 'firmware/AAVMF_VARS.ms.fd', directory / 'vars.fd')
    (directory / 'tpm').mkdir(mode=0o700)
    write_json(directory / 'state.json', state)
    return state


def up(directory, state):
    if owned_process(state, 'qemu'):
        if state['mode'] == 'lab':
            ssh(directory, state, ['niri', 'msg', '--json', 'outputs'])
            ssh(directory, state, ['noctalia', 'msg', 'log-level-status'])
            check_renderer(directory, state)
        print(json.dumps({'status': qmp(directory, state, 'query-status'), 'instance': str(directory)}, indent=2))
        return
    runtime = Path(state['runtime'])
    runtime_check(runtime)
    if sha256(runtime / 'receipt.json') != state['runtime_receipt']:
        raise ValueError('instance runtime has changed')
    if owned_process(state, 'swtpm'):
        raise ValueError('previous TPM process remains; use down --force before starting')
    for name in ['qmp.sock', 'tpm.sock', 'qga.sock']:
        (directory / name).unlink(missing_ok=True)
    if state['mode'] == 'lab':
        with socket.socket() as port:
            port.bind(('127.0.0.1', 0))
            state['ssh_port'] = port.getsockname()[1]
        network = f'user,id=net,hostfwd=tcp:127.0.0.1:{state["ssh_port"]}-:22'
    else:
        state.pop('ssh_port', None)
        network = 'user,id=net'
    env = dict(os.environ, DYLD_FALLBACK_LIBRARY_PATH=str(runtime / 'lib'))
    with (directory / 'swtpm.log').open('ab') as log:
        process = subprocess.Popen([runtime / 'bin/swtpm', 'socket', '--tpm2', '--tpmstate', 'dir=' + str(directory / 'tpm'),
                                    '--ctrl', 'type=unixio,path=' + str(directory / 'tpm.sock'), '--terminate'],
                                   stdin=subprocess.DEVNULL, stdout=log, stderr=log, env=env, start_new_session=True)
    state['swtpm'] = {'pid': process.pid, 'identity': process_identity(process.pid)}
    write_json(directory / 'state.json', state)
    # Agent shells may inherit Darwin background I/O policy. Restore normal
    # scheduling only for our new process; never change workstation-wide policy.
    run(['/usr/sbin/taskpolicy', '-B', '-t', '0', '-l', '0', '-p', str(process.pid)])
    for _ in range(50):
        if (directory / 'tpm.sock').exists():
            break
        if process.poll() is not None:
            raise ValueError('swtpm failed; see swtpm.log')
        time.sleep(0.1)
    display = state.get('display', {'width': 2560, 'height': 1600, 'scale': 2})
    command = [runtime / 'Kedra QEMU.app/Contents/MacOS/qemu-system-aarch64', '-name', state['identity'],
               '-machine', 'virt', '-accel', 'hvf', '-cpu', 'host', '-smp', str(state['cpus']), '-m', str(state['memory_mib']),
               '-nodefaults', '-display', 'cocoa,gl=es',
               '-device', f'virtio-gpu-gl-pci,xres={display["width"]},yres={display["height"]}',
               '-device', 'virtio-keyboard-pci', '-device', 'virtio-tablet-pci', '-device', 'virtio-rng-pci',
               '-drive', f'if=pflash,format=raw,unit=0,readonly=on,file={runtime}/firmware/AAVMF_CODE.secboot.fd',
               '-drive', f'if=pflash,format=raw,unit=1,file={directory}/vars.fd',
               '-drive', f'if=none,id=os,format=qcow2,file={directory}/disk.qcow2', '-device', 'virtio-blk-pci,drive=os,serial=KEDRALAB',
               '-netdev', network, '-device', 'virtio-net-pci,netdev=net,romfile=',
               '-chardev', f'socket,id=tpm,path={directory}/tpm.sock', '-tpmdev', 'emulator,id=tpm,chardev=tpm', '-device', 'tpm-tis-device,tpmdev=tpm',
               '-device', 'virtio-serial-pci', '-chardev', f'socket,id=qga,path={directory}/qga.sock,server=on,wait=off',
               '-device', 'virtserialport,chardev=qga,name=org.qemu.guest_agent.0',
               '-qmp', f'unix:{directory}/qmp.sock,server=on,wait=off', '-serial', f'file:{directory}/serial.log', '-monitor', 'none',
               '-audiodev', 'coreaudio,id=audio,out.fixed-settings=false', '-device', 'virtio-sound-pci,audiodev=audio,streams=1']
    if state['mode'] == 'lab':
        command += ['-fw_cfg', f'name=opt/kedra/seed,file={directory}/seed.json']
    if state.get('installer'):
        command += ['-device', 'qemu-xhci', '-drive', f'if=none,id=installer,media=cdrom,readonly=on,file={directory}/installer.iso',
                    '-device', 'usb-storage,drive=installer,bootindex=0']
        if state.get('installer_probe'):
            command += ['-smbios', 'type=11,value=io.systemd.credential:kedra.research=1',
                        '-chardev', f'file,id=events,path={directory}/installer-events.log',
                        '-device', 'virtserialport,chardev=events,name=org.kedra.events']
    with (directory / 'qemu.log').open('ab') as log:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log, stderr=log, env=env, start_new_session=True)
    state['qemu'] = {'pid': process.pid, 'identity': process_identity(process.pid)}
    write_json(directory / 'state.json', state)
    run(['/usr/sbin/taskpolicy', '-B', '-t', '0', '-l', '0', '-p', str(process.pid)])
    started = time.monotonic()
    deadline = started + 120
    last_error = ''

    def ready(argv):
        left = min(10, deadline - time.monotonic())
        if left <= 0:
            raise ValueError('desktop readiness deadline expired')
        return ssh(directory, state, argv, timeout=left, connect_timeout=3)

    while time.monotonic() < deadline:
        exit_code = process.poll()
        if exit_code is not None:
            raise ValueError(f'QEMU exited with status {exit_code}; inspect qemu.log and serial.log (no software fallback)')
        try:
            qmp(directory, state, 'query-status')
            if state['mode'] != 'lab':
                print('Native installer VM opened. Complete installation in its window; no lab access is added.')
                return
            outputs = json.loads(ready(['niri', 'msg', '--json', 'outputs']))
            for name in outputs:
                ready(['niri', 'msg', 'output', name, 'scale', str(display['scale'])])
            ready(['noctalia', 'msg', 'log-level-status'])
            renderer, renderer_source = check_renderer(directory, state, deadline)
            state['renderer'] = renderer
            state['renderer_source'] = renderer_source
            write_json(directory / 'state.json', state)
            print(json.dumps({'status': 'ready', 'instance': str(directory), 'boot_seconds': time.monotonic() - started,
                              'graphics': state.get('graphics', 'accelerated'), 'renderer': renderer,
                              'renderer_source': renderer_source,
                              'gpu_qualified': False}, indent=2))
            return
        except GraphicsError:
            raise
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            detail = getattr(error, 'stderr', None)
            last_error = detail.decode(errors='replace')[-1500:] if isinstance(detail, bytes) else str(error)
            time.sleep(0.5)
    raise ValueError('desktop readiness timed out; inspect retained logs: ' + last_error)


def down(directory, state, force):
    deadline = time.monotonic() + (10 if force else 30)
    if owned_process(state, 'qemu'):
        if not force and state['mode'] == 'lab':
            # A desktop shell may inhibit the ACPI power key to display its menu.
            # The fixture permits exactly this shutdown command through sudo.
            try:
                ssh(directory, state, ['sudo', '-n', '/usr/bin/systemctl', 'poweroff', '--no-block'], timeout=10, session=False)
            except subprocess.CalledProcessError as error:
                # sshd can stop before sending its successful exit status. Only
                # accept that disconnect if the owned QEMU process really exits.
                if error.returncode != 255:
                    raise
        else:
            try:
                qmp(directory, state, 'quit' if force else 'system_powerdown')
            except (OSError, ValueError):
                if not force:
                    raise
        graceful_deadline = deadline - 2 if force else deadline
        while owned_process(state, 'qemu') and time.monotonic() < graceful_deadline:
            time.sleep(0.2)
        if owned_process(state, 'qemu'):
            if not force:
                raise ValueError('guest did not stop; use down --force explicitly')
            os.kill(state['qemu']['pid'], signal.SIGTERM)
            while owned_process(state, 'qemu') and time.monotonic() < deadline:
                time.sleep(0.1)
            if owned_process(state, 'qemu'):
                os.kill(state['qemu']['pid'], signal.SIGKILL)
    if owned_process(state, 'swtpm'):
        os.kill(state['swtpm']['pid'], signal.SIGTERM)
        until = time.monotonic() + 5
        while owned_process(state, 'swtpm') and time.monotonic() < until:
            time.sleep(0.1)
        if owned_process(state, 'swtpm'):
            raise ValueError('owned TPM process did not exit; state retained for recovery')
    state.pop('qemu', None)
    state.pop('swtpm', None)
    write_json(directory / 'state.json', state)
    print('Stopped; disk, firmware variables and TPM state retained.')


def shot(directory, state, label, root):
    if not re.fullmatch('[a-zA-Z0-9_-]{1,64}', label):
        raise ValueError('screenshot label must contain 1–64 letters, digits, _ or -')
    started = time.monotonic()
    png = ssh(directory, state, ['grim', '-t', 'png', '-'])
    elapsed = time.monotonic() - started
    if png[:8] != b'\x89PNG\r\n\x1a\n' or len(png) < 24 or png[12:16] != b'IHDR':
        raise ValueError('guest capture is not a PNG')
    width, height = struct.unpack('>II', png[16:24])
    if not (0 < width <= 16384 and 0 < height <= 16384):
        raise ValueError('guest capture has invalid dimensions')
    path = root / 'shots' / f'{label}-{time.time_ns()}.png'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)
    # Runs through the guest's interpreter; export only the revision, not stored file contents.
    metadata = json.loads(ssh(directory, state, ['python3', '-c', '''
import json, re, subprocess
from pathlib import Path
p = Path.home() / '.local/state/kedra-lab/sync/receipt.json'
if any(part.is_symlink() for part in [p, *list(p.parents)[:4]]):
    raise SystemExit('unsafe home sync receipt')
if not p.exists():
    revision = None
else:
    with p.open() as stream:
        data = stream.read(32 * 1024 * 1024 + 1)
    if len(data) > 32 * 1024 * 1024:
        raise SystemExit('oversized home sync receipt')
    value = json.loads(data)
    revision = value['source']['source_revision']
    if value.get('schema_version') != 1 or not re.fullmatch('[a-f0-9]{40}', revision):
        raise SystemExit('invalid home sync receipt')
print(json.dumps({
    'home_source': revision,
    'source': json.loads(Path('/usr/share/sysroot/source.json').read_text()),
    'outputs': json.loads(subprocess.check_output(['niri', 'msg', '--json', 'outputs'], timeout=5)),
}))
''']))
    source, home_source, outputs = metadata['source'], metadata['home_source'], metadata['outputs']
    active = [output['logical'] for output in outputs.values() if output.get('logical')]
    if len(active) != 1:
        raise ValueError('native capture expects exactly one active display')
    receipt = {'schema_version': 1, 'capture': 'guest-grim',
               'mode': 'native-qemu',
               'renderer_at_startup': state.get('renderer'), 'gpu_qualified': False,
               'renderer_source': state.get('renderer_source'),
               'width': width, 'height': height, 'sha256': sha256(path), 'source': source,
               'installed_source': source['source_revision'], 'home_source': home_source,
               'display': {'width': width, 'height': height, 'scale': active[0]['scale']},
               'capture_ms': round(elapsed * 1000), 'captured_at_unix_ms': time.time_ns() // 1000000,
               'timing_scope': 'compositor capture and PNG transfer; excludes metadata collection',
               'runtime_receipt': state['runtime_receipt'], 'image': state['image']['inputs'],
               'controller_sha256': sha256(Path(__file__)), 'session_transport_sha256': sha256(HERE / 'session-exec'),
               'outputs': outputs}
    write_json(path.with_suffix('.json'), receipt)
    print(path)


def main():
    os.umask(0o077)
    signal.signal(signal.SIGTERM, lambda _signal, _frame: sys.exit(130))
    signal.signal(signal.SIGINT, lambda _signal, _frame: sys.exit(130))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--runtime', required=True, type=Path)
    parser.add_argument('--name', default='default')
    sub = parser.add_subparsers(dest='operation', required=True)
    start = sub.add_parser('up')
    start.add_argument('--image', type=Path)
    start.add_argument('--memory-mib', type=int, default=4096)
    start.add_argument('--cpus', type=int, default=6)
    start.add_argument('--display', default='2560x1600@2')
    installer = sub.add_parser('installer')
    installer.add_argument('--iso', type=Path, required=True)
    installer.add_argument('--disk-gib', type=int, default=96)
    installer.add_argument('--memory-mib', type=int, default=8192)
    installer.add_argument('--cpus', type=int, default=6)
    installer.add_argument('--probe', action='store_true')
    sub.add_parser('detach-installer')
    sub.add_parser('status')
    stop = sub.add_parser('down')
    stop.add_argument('--force', action='store_true')
    sub.add_parser('remove')
    capture = sub.add_parser('shot')
    capture.add_argument('label', default='desktop', nargs='?')
    execute = sub.add_parser('exec')
    execute.add_argument('argv', nargs=argparse.REMAINDER)
    sub.add_parser('logs')
    sub.add_parser('sync')
    keyboard = sub.add_parser('key')
    keyboard.add_argument('keys', nargs='+', help='QEMU qcodes, for example meta_l ret')
    args = parser.parse_args()
    if not re.fullmatch('[a-zA-Z0-9_-]{1,32}', args.name):
        parser.error('invalid instance name')
    args.root, args.runtime = args.root.resolve(), args.runtime.resolve()
    directory = args.root / 'instances' / args.name
    if any(char in str(directory) + str(args.runtime) for char in ',\n\r') or len(str(directory / 'qmp.sock').encode()) >= 104:
        parser.error('QEMU paths must fit Unix socket limits and contain no commas/newlines')
    if directory.is_symlink() or directory.parent.is_symlink():
        parser.error('instance directory or its parent is a symlink')
    if not directory.exists() and args.operation not in ('up', 'installer'):
        parser.error('instance does not exist')
    directory.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if args.operation == 'status':
        state = json.loads((directory / 'state.json').read_text())
        if state.get('schema_version') != 1:
            raise ValueError('unsupported instance state')
        check_process_ownership(state)
        print(json.dumps(qmp(directory, state, 'query-status') if owned_process(state, 'qemu') else {'status': 'stopped'}))
        return
    with (directory.parent / (args.name + '.lock')).open('w') as lock:
        mode = fcntl.LOCK_SH if args.operation in ('exec', 'shot', 'logs', 'key') else fcntl.LOCK_EX
        try:
            fcntl.flock(lock, mode | fcntl.LOCK_NB)
        except BlockingIOError:
            raise ValueError('instance is busy with another lifecycle or sync operation; retry when it finishes') from None
        record = directory / 'state.json'
        for child in ('state.json', 'disk.qcow2', 'vars.fd', 'tpm', 'seed.json', 'client-key', 'known_hosts'):
            if (directory / child).is_symlink():
                raise ValueError('unsafe symlink in instance: ' + child)
        if record.exists():
            if args.operation == 'installer':
                raise ValueError('instance already exists; choose a new --name')
            state = json.loads(record.read_text())
            if state.get('schema_version') != 1:
                raise ValueError('unsupported instance state')
            check_process_ownership(state)
            if args.operation == 'up' and state.get('graphics', 'accelerated') != 'accelerated':
                raise ValueError('unsupported retired graphics profile; create a new GPU instance')
        else:
            if directory.exists():
                raise ValueError('instance directory exists without a receipt; inspect it before removing it')
            if args.operation not in ('up', 'installer') or not 1024 <= args.memory_mib <= 32768 or not 1 <= args.cpus <= 16:
                parser.error('invalid new instance options')
            if args.operation == 'installer' and not 32 <= args.disk_gib <= 1024:
                parser.error('--disk-gib must be between 32 and 1024')
            pending = directory.parent / ('.' + args.name + '.create-' + secrets.token_hex(6))
            pending.mkdir(mode=0o700)
            try:
                state = create_installer(pending, args) if args.operation == 'installer' else create(pending, args)
                os.rename(pending, directory)
            finally:
                if pending.exists():
                    shutil.rmtree(pending)
        if args.operation in ('exec', 'shot', 'logs', 'sync') and state['mode'] != 'lab':
            raise ValueError('guest automation is available only in a disposable lab fixture')
        if args.operation in ('up', 'installer'):
            was_running = owned_process(state, 'qemu')
            try:
                up(directory, state)
            except BaseException:
                # Retain disk/logs for diagnosis, but do not leak a newly started
                # TPM or QEMU after failed or interrupted readiness.
                if not was_running:
                    try:
                        down(directory, state, True)
                    except (OSError, ValueError, subprocess.SubprocessError) as cleanup_error:
                        print(f'owned startup cleanup failed: {cleanup_error}', file=sys.stderr)
                raise
        elif args.operation == 'detach-installer':
            if owned_process(state, 'qemu') or owned_process(state, 'swtpm'):
                raise ValueError('stop the VM before detaching media')
            if state.get('installer') != 'installer.iso':
                raise ValueError('this instance has no attached installer')
            state.pop('installer')
            write_json(record, state)
            (directory / 'installer.iso').unlink()
            print('Installer detached; original ISO retained.')
        elif args.operation == 'down':
            down(directory, state, args.force)
        elif args.operation == 'remove':
            if owned_process(state, 'qemu') or owned_process(state, 'swtpm'):
                raise ValueError('stop the instance before removing it')
            shutil.rmtree(directory)
        elif args.operation == 'exec':
            argv = args.argv[1:] if args.argv[:1] == ['--'] else args.argv
            if not argv:
                parser.error('exec needs a guest command')
            os.write(1, ssh(directory, state, argv, timeout=120))
        elif args.operation == 'shot':
            shot(directory, state, args.label, args.root)
        elif args.operation == 'key':
            qmp(directory, state, 'send-key', {'keys': [{'type': 'qcode', 'data': key} for key in args.keys], 'hold-time': 100})
        elif args.operation == 'sync':
            data = sys.stdin.buffer.read(32 * 1024 * 1024 + 1)
            if len(data) > 32 * 1024 * 1024:
                raise ValueError('sync payload too large')
            os.write(1, ssh(directory, state, ['/usr/libexec/kedra-lab/probes/sync-home.py'], data=data, timeout=60))
        elif args.operation == 'logs':
            output = args.root / 'logs' / f'{args.name}-{time.time_ns()}'
            output.mkdir(parents=True)
            for name in ['qemu.log', 'serial.log', 'swtpm.log']:
                if (directory / name).exists():
                    shutil.copy2(directory / name, output / name)
            for name, argv in [('user-journal.log', ['journalctl', '--user', '-b', '--no-pager']),
                               ('outputs.json', ['niri', 'msg', '--json', 'outputs']), ('bootc.json', ['bootc', 'status', '--json'])]:
                (output / name).write_bytes(ssh(directory, state, argv))
            print(output)


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        detail = error.stderr or b''
        if isinstance(detail, bytes):
            detail = detail.decode(errors='replace')
        raise SystemExit(f'kedra-lab vm: command exited {error.returncode}: {detail[-2000:]}') from None
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        raise SystemExit('kedra-lab vm: ' + str(error)) from None
