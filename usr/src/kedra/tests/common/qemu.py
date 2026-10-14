#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Shared QEMU launcher for disposable Kedra VM tests.

Every VM driver builds its UEFI Secure Boot machine here: Ubuntu's OVMF/AAVMF
with a private Microsoft-enrolled variable store under KVM or TCG on a
disposable Linux host, or the private kedra-lab runtime under HVF on macOS
(usr/src/kedra/tests/container/qemu/vm.py). It owns the QEMU process, talks QMP
and refuses hosts that may be a workstation before a driver prepares host state.

  qemu.py guard --disposable-host        exit 0 only on an admitted disposable host
  qemu.py boot --disposable-host ...     boot generated disks until guest poweroff
"""
import argparse
import json
import os
import platform
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import secure_boot

# Path-only evidence of a bootc/ostree deployment or an installed Kedra system.
# The guard never executes bootc: inspecting these files is enough to refuse.
BOOTC_HOST_STATE = (
    Path('/run/ostree-booted'), Path('/sysroot/ostree'), Path('/ostree'),
    Path('/usr/share/sysroot'), Path('/usr/lib/sysroot'), Path('/usr/libexec/sysroot'),
)
# Kernel arguments of a booted ostree or composefs deployment. /proc/cmdline is
# the host kernel's even inside a container, so this also catches a privileged
# container started on a bootc workstation.
BOOTC_KERNEL_ARGUMENTS = ('ostree=', 'composefs=')
ARCHITECTURES = ('x86_64', 'aarch64')
ACCELERATORS = ('kvm', 'tcg', 'hvf')
FREEZE = 'systemd[1]: Freezing execution.'
# Fedora's shim fallback on aarch64 is test-signed; installed systems get this
# vendor-path entry from Anaconda/bootupd, generated disks from the launcher.
AARCH64_SHIM = '\\EFI\\fedora\\shimaa64.efi'
LOG_LIMIT = 128 * 1024**2


class VmError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise VmError(message)


def disposable_host_refusal(opted_in):
    """Why this host must not run a Linux VM driver, or None when admitted."""
    if not opted_in:
        return ('pass --disposable-host only on a throwaway Linux host or VM that nothing else uses; '
                'the drivers run privileged Podman, may edit /etc/hosts and container trust, and are '
                'never for a workstation')
    if sys.platform != 'linux':
        return 'Linux VM drivers need a disposable Linux host; on macOS use kedra-lab vm'
    present = [str(path) for path in BOOTC_HOST_STATE if path.exists() or path.is_symlink()]
    if present:
        return 'bootc, ostree or Kedra host state is present: ' + ', '.join(present)
    arguments = Path('/proc/cmdline').read_text().split()
    booted = [argument for argument in arguments if argument.startswith(BOOTC_KERNEL_ARGUMENTS)]
    if booted:
        return 'the running kernel booted a bootc/ostree deployment: ' + ' '.join(booted)
    return None


def require_disposable_host(opted_in):
    refusal = disposable_host_refusal(opted_in)
    if refusal is not None:
        raise SystemExit('Refusing this host: ' + refusal)


def option_path(path):
    """A path that cannot inject QEMU option syntax."""
    text = str(path)
    require(not any(character in text for character in ',\r\n'), 'QEMU paths must not contain commas or newlines: ' + text)
    return text


def default_accelerator(architecture):
    """KVM when the host can run this architecture natively, otherwise TCG."""
    native = platform.machine() == architecture
    return 'kvm' if native and os.access('/dev/kvm', os.R_OK | os.W_OK) else 'tcg'


def qemu_binary(architecture):
    return 'qemu-system-' + architecture


def machine_argv(architecture, accelerator, *, code, variables, qmp, serial, cpus, memory_mib, name=None):
    """The Secure Boot machine every driver shares; callers add disks and devices."""
    require(architecture in ARCHITECTURES and accelerator in ACCELERATORS, 'Unsupported machine')
    if architecture == 'x86_64':
        require(accelerator in ('kvm', 'tcg'), 'x86_64 VMs run under KVM or TCG')
        # OVMF's Secure Boot build requires SMM and an SMM-only writable varstore.
        argv = ['-machine', f'q35,smm=on,accel={accelerator}', '-cpu', 'host' if accelerator == 'kvm' else 'max',
                '-global', 'driver=cfi.pflash01,property=secure,value=on']
    elif accelerator == 'tcg':
        # pauth-impdef avoids TCG's costly architected pointer-authentication emulation.
        argv = ['-machine', 'virt', '-cpu', 'max,pauth-impdef=on', '-accel', 'tcg,thread=multi']
    elif accelerator == 'kvm':
        # Many arm64 KVM hosts implement only GICv3; the default GICv2 would refuse.
        argv = ['-machine', 'virt,gic-version=max', '-accel', 'kvm', '-cpu', 'host']
    else:
        argv = ['-machine', 'virt', '-accel', 'hvf', '-cpu', 'host']
    if name is not None:
        argv = ['-name', name, *argv]
    return argv + [
        '-smp', str(cpus), '-m', str(memory_mib),
        '-drive', f'if=pflash,format=raw,unit=0,readonly=on,file={option_path(code)}',
        '-drive', f'if=pflash,format=raw,unit=1,file={option_path(variables)}',
        '-device', 'virtio-rng-pci',
        '-qmp', f'unix:{option_path(qmp)},server=on,wait=off',
        '-serial', f'file:{option_path(serial)}', '-monitor', 'none',
    ]


def disk_argv(identifier, path, *, image_format='qcow2', readonly=False, snapshot=False, serial=None):
    drive = f'if=none,id={identifier},format={image_format},file={option_path(path)}'
    if readonly:
        drive += ',readonly=on'
    if snapshot:
        drive += ',snapshot=on'
    device = f'virtio-blk-pci,drive={identifier}' + (f',serial={serial}' if serial else '')
    return ['-drive', drive, '-device', device]


def events_argv(path):
    """A virtio-serial port the guest writes markers to as org.kedra.events."""
    return ['-device', 'virtio-serial-pci', '-chardev', f'file,id=events,path={option_path(path)}',
            '-device', 'virtserialport,chardev=events,name=org.kedra.events']


def network_argv(enabled, forward=None):
    if not enabled:
        return ['-nic', 'none']
    netdev = 'user,id=net0' + (f',hostfwd={forward}' if forward else '')
    # No iPXE option ROM: nothing network-boots.
    return ['-netdev', netdev, '-device', 'virtio-net-pci,netdev=net0,romfile=']


def create_variables(architecture, output, *, boot_entry=None, virt_fw_vars=('virt-fw-vars',)):
    """A new private Microsoft-enrolled store, optionally with one boot entry."""
    secure_boot.create('microsoft', output, architecture)
    if boot_entry is not None:
        subprocess.run([*virt_fw_vars, '--inplace', str(output), '--append-boot-filepath', boot_entry],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
        secure_boot.verify(output, architecture)


class Qmp:
    """One QMP session; events are skipped and every read is bounded."""

    def __init__(self, path, *, timeout=15, identity=None):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(timeout)
        try:
            self.socket.connect(str(path))
            self.stream = self.socket.makefile('rwb')
            greeting = json.loads(self.stream.readline(1048576) or b'{}')
            require('QMP' in greeting, 'Unexpected QMP greeting')
            self.execute('qmp_capabilities')
            if identity is not None:
                require(self.execute('query-name').get('name') == identity, 'QMP socket belongs to another VM')
        except BaseException:
            self.socket.close()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def close(self):
        self.socket.close()

    def _response(self, identifier, allow_eof=False):
        for _ in range(256):
            line = self.stream.readline(1048576)
            if not line and allow_eof:
                return None
            require(line.endswith(b'\n'), 'QMP response absent or oversized')
            message = json.loads(line)
            require(isinstance(message, dict), 'Unexpected QMP message')
            if message.get('id') == identifier:
                if 'error' in message:
                    raise VmError(f'QMP {identifier} failed: {message["error"]}')
                return message.get('return')
            require('event' in message, 'Unexpected QMP response identifier')
        raise VmError('QMP response bound exceeded')

    def execute(self, command, arguments=None, *, allow_eof=False):
        message = {'execute': command, 'id': command}
        if arguments is not None:
            message['arguments'] = arguments
        self.stream.write(json.dumps(message).encode() + b'\n')
        self.stream.flush()
        return self._response(command, allow_eof)

    def send_keys(self, keys, hold_time=100):
        self.execute('send-key', {'keys': [{'type': 'qcode', 'data': key} for key in keys], 'hold-time': hold_time})

    def type_text(self, text):
        """Typed one key at a time; never logged, so it may carry a generated secret."""
        names = {'-': 'minus', '/': 'slash', '.': 'dot', '\n': 'ret'}
        for character in text:
            self.send_keys([names.get(character, character)], hold_time=40)
            time.sleep(0.08)

    def screendump(self, path):
        self.execute('screendump', {'filename': str(path), 'format': 'png'})

    def quit(self, process, timeout=15):
        # QAPI permits server EOF before the reply; only a real exit 0 qualifies it.
        self.execute('quit', allow_eof=True)
        require(process.wait(timeout=timeout) == 0, 'QEMU quit failed')


def launch(argv, log, *, env=None):
    """Start QEMU in its own process group so stop() reaches every child."""
    with Path(log).open('ab') as output:
        return subprocess.Popen([str(item) for item in argv], stdin=subprocess.DEVNULL, stdout=output,
                                stderr=subprocess.STDOUT, env=env, start_new_session=True)


def stop(process, timeout=20):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=timeout)


def wait_for_socket(path, process, timeout=30):
    deadline = time.monotonic() + timeout
    while not Path(path).exists():
        require(process.poll() is None, 'QEMU exited before opening QMP')
        require(time.monotonic() < deadline, 'QEMU did not open QMP')
        time.sleep(0.1)


def read_log(path):
    path = Path(path)
    if not path.exists():
        return ''
    require(path.stat().st_size <= LOG_LIMIT, f'{path.name} exceeded its bound')
    return path.read_text(errors='replace')


def watch(process, qmp_path, logs, *, deadline, required=(), failures=(), until=False):
    """Poll guest logs until poweroff, a failure marker or (with until) every marker."""
    while process.poll() is None:
        require(time.monotonic() < deadline, 'VM did not finish within its bound')
        text = ''.join(read_log(path) for path in logs)
        failed = [marker for marker in (*failures, FREEZE) if marker in text]
        require(not failed, 'Guest reported ' + ', '.join(failed))
        if until and required and all(marker in text for marker in required):
            with Qmp(qmp_path) as qmp:
                qmp.quit(process, timeout=max(1, deadline - time.monotonic()))
            break
        time.sleep(2)
    text = ''.join(read_log(path) for path in logs)
    require(not [marker for marker in (*failures, FREEZE) if marker in text], 'Guest reported a failure marker')
    require(until or process.returncode == 0, f'QEMU exited with status {process.returncode}')
    missing = [marker for marker in required if marker not in text]
    require(not missing, 'Guest did not report ' + ', '.join(missing))


def boot_argv(args, work):
    code = secure_boot.firmware(args.architecture)['code']
    argv = [qemu_binary(args.architecture), *machine_argv(
        args.architecture, args.accelerator, code=code, variables=args.vars, qmp=work / 'qmp.sock',
        serial=args.serial, cpus=args.cpus, memory_mib=args.memory_mib)]
    argv += disk_argv('system', args.disk, snapshot=args.snapshot)
    for index, data in enumerate(args.data_disk):
        argv += disk_argv(f'data{index}', data, image_format='raw', readonly=True)
    if args.events is not None:
        argv += events_argv(args.events)
    argv += network_argv(args.network)
    if args.architecture == 'aarch64':
        argv += ['-device', 'virtio-gpu-pci']
    # No -no-reboot: shim's fallback may reset once after adding a boot entry.
    return argv + ['-display', 'none']


def boot(args):
    require_disposable_host(args.disposable_host)
    args.accelerator = args.accelerator or default_accelerator(args.architecture)
    require(args.disk.suffix == '.qcow2' and args.disk.is_file(), 'Only a generated QCOW2 disk can be booted')
    work = args.work.resolve()
    work.mkdir(mode=0o700, parents=True, exist_ok=True)
    require(not (work / 'qmp.sock').exists(), 'Another VM uses this work directory')
    if args.vars.exists():
        # Persisted state must still carry the Microsoft-enrolled trust.
        secure_boot.verify(args.vars, args.architecture)
    else:
        create_variables(args.architecture, args.vars,
                         boot_entry=AARCH64_SHIM if args.architecture == 'aarch64' else None)
    logs = [args.serial] + ([args.events] if args.events is not None else [])
    started = time.monotonic()
    process = launch(boot_argv(args, work), work / 'qemu.log')
    try:
        wait_for_socket(work / 'qmp.sock', process)
        watch(process, work / 'qmp.sock', logs, deadline=started + args.timeout,
              required=args.require, failures=args.fail_marker, until=args.until_markers)
    finally:
        stop(process)
        (work / 'qmp.sock').unlink(missing_ok=True)
        secure_boot.verify(args.vars, args.architecture)
    print(json.dumps({'schema_version': 1, 'architecture': args.architecture, 'accelerator': args.accelerator,
                      'elapsed_seconds': round(time.monotonic() - started, 1), 'markers': args.require,
                      'firmware_trust_unchanged': True}, sort_keys=True))


def parser():
    result = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = result.add_subparsers(dest='command', required=True)
    guard = commands.add_parser('guard', help='Admit or refuse this host for Linux VM drivers')
    guard.add_argument('--disposable-host', action='store_true')
    run = commands.add_parser('boot', help='Boot one generated disk under UEFI Secure Boot until guest poweroff')
    run.add_argument('--disposable-host', action='store_true')
    run.add_argument('--architecture', choices=ARCHITECTURES, default=platform.machine())
    run.add_argument('--accelerator', choices=('kvm', 'tcg'))
    run.add_argument('--disk', type=Path, required=True, help='Generated QCOW2 system disk')
    run.add_argument('--snapshot', action='store_true', help='Discard guest writes to the system disk')
    run.add_argument('--data-disk', type=Path, action='append', default=[], help='Read-only raw fixture disk')
    run.add_argument('--vars', type=Path, required=True, help='Variable store; created from the template if absent')
    run.add_argument('--work', type=Path, required=True, help='Private directory for QMP and the QEMU log')
    run.add_argument('--serial', type=Path, required=True)
    run.add_argument('--events', type=Path, help='Host file for the guest org.kedra.events port')
    run.add_argument('--network', action='store_true')
    run.add_argument('--cpus', type=int, default=2)
    run.add_argument('--memory-mib', type=int, default=4096)
    run.add_argument('--timeout', type=int, default=2400)
    run.add_argument('--require', action='append', default=[], help='Marker the guest must report')
    run.add_argument('--fail-marker', action='append', default=[])
    run.add_argument('--until-markers', action='store_true', help='Quit through QMP once every marker appeared')
    return result


def main():
    args = parser().parse_args()
    if args.command == 'guard':
        require_disposable_host(args.disposable_host)
        print('Admitted disposable host', platform.node(), platform.machine())
        return
    signal.signal(signal.SIGTERM, lambda _signal, _frame: sys.exit(130))
    try:
        boot(args)
    except (VmError, secure_boot.FirmwareError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit('kedra vm: ' + str(error)) from None


if __name__ == '__main__':
    main()
