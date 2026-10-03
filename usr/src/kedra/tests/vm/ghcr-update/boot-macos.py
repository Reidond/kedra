#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Explicit private HVF fixture copy; no production media/trust bypass in vm.py.

Preparation draft. No compile, execution, port or throughput qualification claimed.
"""
import argparse
import contextlib
import fcntl
import hashlib
import importlib.util
import json
import os
import platform
import re
import shlex
import shutil
import signal
import socket
import socketserver
import stat
import subprocess
import sys
import threading
import time
from pathlib import Path

GUEST_FORWARD = len(sys.argv) == 4 and sys.argv[1] == '--guest-forward'
if GUEST_FORWARD:
    # libslirp connects child stderr to the same TCP stream. Diagnostic bytes
    # must never be injected into TLS, including errors during module imports.
    null = os.open(os.devnull, os.O_WRONLY)
    os.dup2(null, 2)
    os.close(null)

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
arm = None
marker_input = None
if not GUEST_FORWARD:
    spec = importlib.util.spec_from_file_location('arm_fixture', HERE / 'boot-arm64.py')
    arm = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(arm)
    marker_spec = importlib.util.spec_from_file_location('fixture_marker', HERE / 'marker_input.py')
    marker_input = importlib.util.module_from_spec(marker_spec)
    marker_spec.loader.exec_module(marker_input)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def installed_phase_marker(lines, marker):
    prefix = r'(?:\[[ ]*[0-9]+\.[0-9]{6}\] )?kedra-ghcr-fixture\[[0-9]+\]: '
    tagged = re.compile(prefix + re.escape(marker))
    return any(line == marker or tagged.fullmatch(line) is not None for line in lines)

# Runs inside the selected disposable controller using its installed interpreter.
# It can only forward one of two fixed loopback ports. No TLS termination or files.
ADAPTER = '''import os,socket,sys,threading
port=int(sys.argv[1]); assert port in (443,18080)
watchdog=threading.Timer(2100, lambda: os._exit(124)); watchdog.daemon=True; watchdog.start()
peer=socket.create_connection(("127.0.0.1",port),timeout=15)
peer.settimeout(2100)
def upload():
    try:
        while True:
            block=os.read(0,65536)
            if not block: break
            peer.sendall(block)
        peer.shutdown(socket.SHUT_WR)
    except OSError: pass
threading.Thread(target=upload,daemon=True).start()
try:
    while True:
        block=peer.recv(65536)
        if not block: break
        sys.stdout.buffer.write(block); sys.stdout.buffer.flush()
finally:
    peer.close()
    os._exit(0)
'''


def private(path, directory=False):
    info = path.lstat()
    require((stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode))
            and info.st_uid == os.getuid() and info.st_mode & 0o077 == 0
            and (directory or info.st_nlink == 1), 'Unsafe private fixture input')


def run(argv, **kwargs):
    return subprocess.run(list(map(str, argv)), check=True, timeout=kwargs.pop('timeout', 120), **kwargs)


def write_new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def stop_relay_child(child):
    try:
        arm.stop(child)
    except (PermissionError, ProcessLookupError):
        # A concurrent handler may be reaping this child. Darwin can return
        # EPERM for its exited process group; require actual exit before accepting it.
        child.wait(timeout=5)


class Relay(socketserver.ThreadingUnixStreamServer):
    allow_reuse_address = False
    daemon_threads = True

    def __init__(self, path, execute):
        self.execute = execute
        self.children = set()
        self.connections = set()
        self.guard = threading.Lock()
        self.closing = False
        self.capacity = threading.BoundedSemaphore(32)
        require(len(str(path).encode()) < 104 and not path.exists() and not path.is_symlink(),
                'Relay socket exists or exceeds Unix socket bounds')
        private(path.parent, directory=True)
        self.path = path
        super().__init__(str(path), Forward)
        info = path.lstat()
        require(stat.S_ISSOCK(info.st_mode) and info.st_uid == os.getuid() and info.st_mode & 0o077 == 0,
                'Unsafe relay socket')
        self.identity = (info.st_dev, info.st_ino)

    def stop(self):
        with self.guard:
            self.closing = True
        self.shutdown()
        self.server_close()
        with self.guard:
            for connection in self.connections:
                try:
                    connection.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
            for child in self.children:
                stop_relay_child(child)
        info = self.path.lstat()
        require(stat.S_ISSOCK(info.st_mode) and (info.st_dev, info.st_ino) == self.identity,
                'Relay socket identity changed; refusing cleanup')
        self.path.unlink()


class Forward(socketserver.BaseRequestHandler):
    def handle(self):
        if not self.server.capacity.acquire(blocking=False):
            return
        child = None
        try:
            with self.server.guard:
                if self.server.closing:
                    return
                self.server.connections.add(self.request)
                child = subprocess.Popen(self.server.execute, stdin=self.request, stdout=self.request,
                                         stderr=subprocess.DEVNULL, start_new_session=True)
                self.server.children.add(child)
            try:
                child.wait(timeout=2100)
            except subprocess.TimeoutExpired:
                stop_relay_child(child)
        finally:
            if child is not None:
                stop_relay_child(child)
                with self.server.guard:
                    self.server.children.discard(child)
            with self.server.guard:
                self.server.connections.discard(self.request)
            self.server.capacity.release()


def connector(expected):
    path = Path('/usr/bin/nc')
    info = path.lstat()
    require(re.fullmatch('[a-f0-9]{64}', expected) and stat.S_ISREG(info.st_mode)
            and info.st_uid == 0 and info.st_mode & 0o022 == 0
            and os.access(path, os.X_OK) and sha(path) == expected,
            'Reviewed system nc connector is absent or changed; no installation/fallback')
    return path


def guest_forward(path, expected):
    require(path.is_absolute() and path.resolve(strict=True) == path
            and path.name in ('guest-443.sock', 'guest-18080.sock')
            and path.parent.name.startswith('d2-hvf-'), 'Unsupported private relay endpoint')
    private(path.parent, directory=True)
    private(path.parent / 'owner.json')
    info = path.lstat()
    require(stat.S_ISSOCK(info.st_mode) and info.st_uid == os.getuid() and info.st_mode & 0o077 == 0,
            'Private relay endpoint changed')
    program = connector(expected)
    # Fixed executable/argv only. No caller-supplied command or shell interpreter.
    os.execv(program, [str(program), '-U', '-w', '2100', str(path)])


def guest_network(state, expected):
    connector(expected)
    network = 'user,id=net0,net=10.0.2.0/24,host=10.0.2.254'
    for port in (443, 18080):
        command = shlex.join([sys.executable, '-I', str(Path(__file__).resolve()),
                              '--guest-forward', str(state / f'guest-{port}.sock'), expected])
        require(len(command.encode()) < 900 and not any(char in command for char in ',\r\n'),
                'Guest forwarding command is not safely representable')
        network += f',guestfwd=tcp:10.0.2.2:{port}-cmd:{command}'
    return network


@contextlib.contextmanager
def guest_relays(state, execute, transfer_hash):
    marker = state / 'active-transport.json'
    write_new(marker, {'schema': 1, 'transport': 'guestfwd-unix-v1', 'manifest_sha256': transfer_hash})
    relays = []
    try:
        for port in (443, 18080):
            relay = Relay(state / f'guest-{port}.sock',
                          [*execute, '/usr/bin/python3', '-I', '-c', ADAPTER, str(port)])
            thread = threading.Thread(target=relay.serve_forever, daemon=True)
            thread.start()
            relays.append(relay)
        yield
    finally:
        errors = []
        for relay in reversed(relays):
            try:
                relay.stop()
            except (OSError, RuntimeError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        require(not errors, 'Owned relay cleanup uncertain; transport marker retained')
        marker.unlink()


def controller(manifest):
    endpoint = os.environ.get('DOCKER_HOST', 'unix:///var/run/docker.sock')
    require(endpoint.startswith('unix://'), 'Only an explicit local Unix Docker endpoint is supported')
    os.environ['DOCKER_HOST'] = endpoint
    os.environ.pop('DOCKER_CONTEXT', None)
    require(re.fullmatch('[a-f0-9]{64}', manifest['controller']), 'Expected exact controller ID')
    records = json.loads(run(['docker', 'inspect', manifest['controller']], capture_output=True).stdout)
    require(len(records) == 1 and records[0]['Id'] == manifest['controller']
            and records[0]['State']['Running']
            and records[0]['Config']['Labels'].get('dev.kedra.lab.owner') == 'kedra-release-fixture'
            and records[0]['Config']['Labels'].get('dev.kedra.lab.kind') == 'release-controller',
            'Selected controller identity differs')
    execute = ['docker', 'exec', '--user', str(manifest['uid']), '-i', manifest['controller']]
    observed = run([*execute, '/usr/bin/sha256sum', manifest['helper']], capture_output=True).stdout.decode().split()[0]
    require(observed == manifest['helper_sha256'], 'Controller transfer helper changed')
    return execute


def verify_vars(execute, manifest, variables, insecure):
    arguments = [*execute, 'uv', 'run', '--offline', '--script', manifest['helper'],
                 '--fixture-context', manifest['context'], '--operation', 'verify-vars']
    if insecure:
        arguments.append('--insecure')
    with variables.open('rb') as stream:
        result = json.loads(run(arguments, stdin=stream, capture_output=True).stdout)
    require(result['trust_unchanged'] is True and result['code_sha256'] == manifest['code_sha256']
            and result['template_sha256'] == manifest['template_sha256'], 'Firmware check differs from transfer')


def phase(args, manifest, state, execute):
    initial = manifest['start']
    insecure = args.phase == 'refuse-insecure'
    media = args.phase in ('refuse-insecure', 'install')
    instrumentation = args.instrumentation
    cases = args.marker_input.parent / 'cases-marker.raw' if instrumentation and media else args.transfer / 'cases.raw'
    require(initial == 'install' or not media, 'Installed transfer cannot restart installation')
    previous = {'install': 'refuse-insecure', 'A': 'install', 'B': 'A', 'ROLLBACK': 'B'}.get(args.phase)
    if previous and not (initial == 'A' and args.phase == 'A'):
        report = json.loads((state / (previous + '.result.json')).read_text())
        require(report['phase'] == previous and report['outcome'] in ('pass', 'refused'), 'Prior phase absent')
    working = state / 'insecure' if insecure else state
    if insecure:
        working.mkdir(mode=0o700)
        for name in ('disk.qcow2', 'sentinel.raw'):
            shutil.copyfile(args.transfer / name, working / name)
        shutil.copyfile(args.transfer / 'insecure-vars.fd', working / 'vars.fd')
    for name in ('disk.qcow2', 'vars.fd', 'sentinel.raw'):
        private(working / name)
    variables, disk, sentinel = (working / name for name in ('vars.fd', 'disk.qcow2', 'sentinel.raw'))
    require(sha(sentinel) == manifest['files']['sentinel.raw']['sha256'], 'Sentinel differs before phase')
    verify_vars(execute, manifest, variables, insecure)
    run([args.runtime / 'bin/qemu-img', 'check', disk], stdout=subprocess.DEVNULL, timeout=300)
    description = json.loads(run([args.runtime / 'bin/qemu-img', 'info', '--output=json', disk], capture_output=True).stdout)
    require(description['format'] == 'qcow2' and description['virtual-size'] == 96 * 1024**3
            and not description.get('backing-filename'), 'Unexpected fixture disk or backing chain')
    qmp = state / (args.phase + '.qmp.sock')
    log = state / (args.phase + '.serial.log')
    require(len(str(qmp).encode()) < 104 and not qmp.exists() and not log.exists(), 'Phase path exists or is too long')
    command = [args.runtime / 'bin/qemu-system-aarch64', '-name', 'kedra-d2-hvf-' + args.phase,
               '-machine', 'virt', '-cpu', 'host', '-accel', 'hvf', '-smp', '4', '-m', '8192',
               '-drive', f'if=pflash,format=raw,unit=0,readonly=on,file={args.transfer}/code.fd',
               '-drive', f'if=pflash,format=raw,unit=1,file={variables}',
               '-drive', f'if=none,id=target,format=qcow2,file={disk}',
               '-device', 'virtio-blk-pci,drive=target,serial=KEDRA_INSTALL_ONLY',
               '-drive', f'if=none,id=sentinel,format=raw,file={sentinel}',
               '-device', 'virtio-blk-pci,drive=sentinel,serial=KEDRA_KEEP_DATA',
               '-drive', f'if=none,id=cases,format=raw,readonly=on,file={cases}',
               '-device', 'virtio-blk-pci,drive=cases,serial=KEDRA_CASES',
               '-device', 'virtio-rng-pci', '-device', 'virtio-gpu-pci', '-device', 'virtio-keyboard-pci',
               '-netdev', guest_network(state, args.nc_sha256), '-device', 'virtio-net-pci,netdev=net0,romfile=',
               '-display', 'none', '-monitor', 'none', '-no-reboot',
               '-qmp', f'unix:{qmp},server=on,wait=off', '-serial', f'file:{log}']
    if media:
        command += ['-device', 'virtio-scsi-pci,id=scsi0', '-drive',
                    f'if=none,id=cdrom,format=raw,media=cdrom,readonly=on,file={args.transfer}/installer.iso',
                    '-device', 'scsi-cd,drive=cdrom,bus=scsi0.0', '-boot', 'order=d']
        if instrumentation:
            command += ['-device', 'qemu-xhci,id=fixture-usb', '-device', 'usb-kbd,bus=fixture-usb.0']
    password = (args.transfer / 'disk-passphrase').read_text().strip()
    require(re.fullmatch('[0-9a-f]{48}', password), 'Invalid private fixture passphrase')
    entered = refused = False
    started = time.monotonic()
    deadline = started + (1800 if insecure else 7200 if media else 5400)
    selection = marker_input.GrubSelection(qmp, 'kedra-d2-hvf-' + args.phase) if instrumentation and media else None
    environment = dict(os.environ, DYLD_FALLBACK_LIBRARY_PATH=str(args.runtime / 'lib'))
    active = state / 'active-phase.json'
    write_new(active, {'phase': args.phase, 'manifest_sha256': args.manifest_sha256})
    with (state / (args.phase + '.qemu.log')).open('xb') as errors:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=errors, stderr=errors,
                                   env=environment, start_new_session=True)
        try:
            while process.poll() is None:
                require(time.monotonic() < deadline, 'HVF fixture exceeded unchanged phase deadline')
                if log.exists():
                    require(log.stat().st_size <= arm.LIMIT, 'Serial output exceeded unchanged bound')
                    output = log.read_text(errors='replace')
                    if selection:
                        selection.observe(output)
                    require('systemd[1]: Freezing execution.' not in output, 'Guest PID 1 froze')
                    if insecure and not refused and 'installation requires UEFI Secure Boot and must not start' in output:
                        require('KEDRA_FIXTURE_INSTALL_COMPLETE' not in output, 'Insecure installation completed')
                        arm.qmp_quit(qmp)
                        refused = True
                    if not media and not entered and 'Please enter passphrase for disk' in output:
                        arm.qmp_key(qmp, password)
                        entered = True
                time.sleep(1)
            require(process.returncode == 0, 'QEMU failed; private logs retained')
        finally:
            arm.stop(process)
            require(process.poll() is not None, 'Owned QEMU did not retire')
            active.unlink()
    output = log.read_text(errors='replace')
    output_lines = marker_input.ANSI.sub('', output).splitlines() if instrumentation or not media else output.splitlines()
    marker = 'KEDRA_FIXTURE_INSTALL_COMPLETE' if media else 'KEDRA_GHCR_' + args.phase + '_PASS'
    if insecure:
        require(refused and marker not in output_lines and sha(disk) == manifest['files']['disk.qcow2']['sha256'],
                'Secure Boot refusal or unchanged target evidence absent')
    else:
        observed = marker in output_lines if media else installed_phase_marker(output_lines, marker)
        require(observed and 'KEDRA_GHCR_FAIL' not in output, 'Guest phase result absent or failed')
    if instrumentation and media:
        require(selection.commands_sent, 'External GRUB configuration was not selected')
        if not insecure:
            require('KEDRA_EXTERNAL_KICKSTART_SELECTED_' + instrumentation['token'] in output_lines,
                    'New external Kickstart was not observed; old/default automation cannot pass')
    if not media:
        require(entered and 'KEDRA_SECUREBOOT_PASS' in output, 'Installed unlock/security evidence absent')
    require(sha(sentinel) == manifest['files']['sentinel.raw']['sha256'], 'Sentinel changed')
    require(sha(args.transfer / 'installer.iso') == manifest['files']['installer.iso']['sha256'], 'ISO changed')
    verify_vars(execute, manifest, variables, insecure)
    run([args.runtime / 'bin/qemu-img', 'check', disk], timeout=300, stdout=subprocess.DEVNULL)
    report = {'schema_version': 1, 'phase': args.phase, 'outcome': 'refused' if insecure else 'pass',
              'elapsed_seconds': time.monotonic() - started, 'sentinel_unchanged': True, 'iso_unchanged': True,
              'firmware_trust_unchanged': True, 'iso_attached': media, 'luks_unlock_observed': entered,
              'stop': 'qmp-quit-after-verifier-refusal' if insecure else 'guest-poweroff',
              'accelerator': 'hvf', 'transfer_sha256': args.manifest_sha256}
    report.update(transport='guestfwd-unix-v1', connector_sha256=args.nc_sha256)
    if instrumentation:
        report['external_instrumentation'] = {
            'manifest_sha256': args.marker_input_sha256, 'host_revision': instrumentation['host_revision'],
            'media_cases_used': media, 'cases_sha256': sha(cases),
            'kickstart_selected': bool(media and not insecure), 'selection': 'fixed-grub-configfile',
        }
    write_new(state / (args.phase + '.result.json'), report)
    print(json.dumps(report, sort_keys=True))


def finish_handoff(args, manifest, state, execute):
    require(not (state / 'active-phase.json').exists(), 'An active or uncertain QEMU phase needs inspection')
    require(not (state / 'active-transport.json').exists(), 'An active or uncertain relay needs inspection')
    request = state / 'finish-request.json'
    if request.exists():
        private(request)
        payload = request.read_bytes()
        require(json.loads(payload)['outcome'] == ('pass' if args.phase == 'finish' else 'abort'),
                'Conflicting finish retry')
    else:
        reports, files = {}, {}
        if args.phase == 'finish':
            phases = ('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK') if manifest['start'] == 'install' else ('A', 'B', 'ROLLBACK')
            for name in phases:
                filename = state / (name + '.result.json')
                private(filename)
                reports[name] = json.loads(filename.read_bytes())
                if args.instrumentation:
                    extra = reports[name].get('external_instrumentation', {})
                    require(extra.get('manifest_sha256') == args.marker_input_sha256
                            and extra.get('host_revision') == args.instrumentation['host_revision'],
                            'Cannot finish mixed original/instrumented phases')
            for name in ('disk.qcow2', 'vars.fd', 'sentinel.raw'):
                private(state / name)
                files[name] = sha(state / name)
            files['installer.iso'] = sha(args.transfer / 'installer.iso')
            verify_vars(execute, manifest, state / 'vars.fd', False)
        receipt = {'schema': 1, 'manifest_sha256': args.manifest_sha256,
                   'source_revision': manifest['source_revision'], 'fixture_revision': manifest['fixture_revision'],
                   'controller': manifest['controller'], 'registry': manifest['registry'],
                   'outcome': 'pass' if args.phase == 'finish' else 'abort', 'reports': reports,
                   'final_files': files, 'runtime_receipt_sha256': args.runtime_receipt_sha256,
                   'context_admission': manifest['context_admission'],
                   'transport': 'guestfwd-unix-v1', 'connector_sha256': args.nc_sha256}
        write_new(request, receipt)
        payload = request.read_bytes()
    result = run([*execute, 'uv', 'run', '--offline', '--script', manifest['helper'],
                  '--fixture-context', manifest['context'], '--operation', 'finish',
                  '--manifest-sha256', args.manifest_sha256], input=payload, capture_output=True, timeout=900)
    report = json.loads(result.stdout)
    require(report['manifest_sha256'] == args.manifest_sha256, 'Finish acknowledgement belongs elsewhere')
    acknowledged = state / 'finish-acknowledged.json'
    if acknowledged.exists():
        private(acknowledged)
        require(json.loads(acknowledged.read_bytes()) == report, 'Finish acknowledgement changed')
    else:
        write_new(acknowledged, report)
    print(json.dumps(report, sort_keys=True))


def main():
    os.umask(0o077)
    if GUEST_FORWARD:
        guest_forward(Path(sys.argv[2]), sys.argv[3])
        return
    require(platform.system() == 'Darwin' and platform.machine() == 'arm64' and os.geteuid() != 0,
            'An ordinary Apple Silicon owner is required')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--transfer', type=Path, required=True)
    parser.add_argument('--manifest-sha256', required=True)
    parser.add_argument('--state', type=Path, required=True)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--runtime-receipt-sha256', required=True)
    parser.add_argument('--nc-sha256', required=True, help='Independently reviewed /usr/bin/nc SHA-256')
    parser.add_argument('--marker-input', type=Path, help='Separately bound private external Kickstart/cases manifest')
    parser.add_argument('--marker-input-sha256')
    parser.add_argument('--phase', choices=('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK', 'finish', 'abort'), required=True)
    args = parser.parse_args()
    for path in (args.transfer, args.state, args.runtime):
        require(path.is_absolute() and not path.is_symlink() and not any(c in str(path) for c in ',\r\n'),
                'Explicit non-symlink absolute paths are required')
    private(args.transfer, directory=True)
    private(args.transfer / 'transfer.json')
    require(sha(args.transfer / 'transfer.json') == args.manifest_sha256, 'Transfer manifest hash differs')
    manifest = json.loads((args.transfer / 'transfer.json').read_text())
    require(manifest['schema'] == 2 and manifest['kind'] == 'kedra-private-hvf-transfer'
            and manifest['start'] in ('install', 'A') and type(manifest['uid']) is int and manifest['uid'] > 0,
            'Unknown transfer protocol')
    require((args.marker_input is None) == (args.marker_input_sha256 is None), 'Marker input and hash must be selected together')
    args.instrumentation = None
    if args.marker_input is not None:
        require(manifest['start'] == 'install', 'Marker recovery requires a fresh blank target')
        args.instrumentation = marker_input.load_input(args.marker_input, args.marker_input_sha256,
                                                      args.manifest_sha256, manifest,
                                                      Path(__file__).resolve().parents[6])
    expected_files = {'installer.iso', 'cases.raw', 'sentinel.raw', 'code.fd', 'template.fd', 'firmware.json',
                      'disk-passphrase', 'disk.qcow2', 'vars.fd', 'insecure-vars.fd'}
    require(set(manifest['files']) == expected_files, 'Unknown transferred artifact')
    for name, record in manifest['files'].items():
        path = args.transfer / name
        private(path)
        require(record['mode'] == '0600' and stat.S_IMODE(path.stat().st_mode) == 0o600
                and path.stat().st_size == record['bytes']
                and sha(path) == record['sha256'], 'Transferred bytes/modes differ')
    require(sha(args.runtime / 'receipt.json') == args.runtime_receipt_sha256, 'Selected runtime changed')
    check = HERE.parents[1] / 'container/qemu/check-runtime.py'
    run(['uv', 'run', '--offline', '--script', check, '--runtime', args.runtime], stdout=subprocess.DEVNULL)
    execute = controller(manifest)
    require(args.state.name.startswith('d2-hvf-') and args.state != args.transfer,
            'State must be a distinct explicit d2-hvf-* directory')
    owner = {'manifest_sha256': args.manifest_sha256}
    if args.instrumentation:
        owner.update(marker_input_sha256=args.marker_input_sha256,
                     host_revision=args.instrumentation['host_revision'])
    if not args.state.exists():
        require(args.phase == ('refuse-insecure' if manifest['start'] == 'install' else 'A'), 'Wrong initial phase')
        args.state.mkdir(mode=0o700)
        for name in ('disk.qcow2', 'vars.fd', 'sentinel.raw'):
            shutil.copyfile(args.transfer / name, args.state / name)
        write_new(args.state / 'owner.json', owner)
    private(args.state, directory=True)
    private(args.state / 'owner.json')
    require(json.loads((args.state / 'owner.json').read_text()) == owner,
            'State belongs to a different transfer')
    descriptor = os.open(args.state / 'launch.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'r+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if args.phase in ('finish', 'abort'):
            finish_handoff(args, manifest, args.state, execute)
            return
        with guest_relays(args.state, execute, args.manifest_sha256):
            phase(args, manifest, args.state, execute)


if __name__ == '__main__':
    signal.signal(signal.SIGINT, lambda _signal, _frame: sys.exit(130))
    signal.signal(signal.SIGTERM, lambda _signal, _frame: sys.exit(130))
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit('HVF fixture refused: ' + str(error)) from error
