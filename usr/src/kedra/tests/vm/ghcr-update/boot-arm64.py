#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Boot only this workflow's generated ARM installer/installed disks under TCG."""
import argparse
import fcntl
import hashlib
import json
import os
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time
from pathlib import Path

from fixture import add_context_argument, load_context

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import secure_boot as firmware

TEMPLATE = Path('/usr/share/AAVMF/AAVMF_VARS.ms.fd')
CODE = Path('/usr/share/AAVMF/AAVMF_CODE.secboot.fd')
DESCRIPTOR = Path('/usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json')
LIMIT = 128 * 1024**2


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def require(value, message):
    if not value:
        raise RuntimeError(message)


def command(*argv, **kwargs):
    return subprocess.run(list(map(str, argv)), check=True, timeout=kwargs.pop('timeout', 120), **kwargs)


def qmp_key(path, password):
    # Fixed public QMP keyboard input into an observed LUKS prompt. Never log
    # request bodies or pass a password in process arguments/environment.
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(15)
        client.connect(str(path))
        stream = client.makefile('rwb')
        greeting = json.loads(stream.readline(65536))
        require('QMP' in greeting, 'Unexpected QMP greeting')

        def request(name, arguments=None):
            payload = {'execute': name, 'id': name}
            if arguments is not None:
                payload['arguments'] = arguments
            stream.write(json.dumps(payload).encode() + b'\n')
            stream.flush()
            while True:
                value = json.loads(stream.readline(65536))
                if value.get('id') == name:
                    require('return' in value, 'QMP keyboard command failed')
                    return
        request('qmp_capabilities')
        for character in password:
            request('send-key', {'keys': [{'type': 'qcode', 'data': character}], 'hold-time': 40})
            time.sleep(0.08)
        request('send-key', {'keys': [{'type': 'qcode', 'data': 'ret'}]})


def qmp_quit(path, process, phase_deadline):
    deadline = min(time.monotonic() + 15, phase_deadline)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'QMP quit deadline exceeded')
        client.settimeout(remaining)
        client.connect(str(path))
        with client.makefile('rwb') as stream:
            def message(allow_eof=False):
                remaining = deadline - time.monotonic()
                require(remaining > 0, 'QMP quit deadline exceeded')
                client.settimeout(remaining)
                line = stream.readline(65536)
                if not line and allow_eof:
                    return None
                require(line.endswith(b'\n'), 'QMP response absent or oversized')
                try:
                    result = json.loads(line)
                except (json.JSONDecodeError, UnicodeDecodeError):
                    raise RuntimeError('Malformed QMP response') from None
                require(isinstance(result, dict), 'Unexpected QMP response')
                return result

            def response(identifier, allow_eof=False):
                for _ in range(128):
                    result = message(allow_eof)
                    if result is None:
                        return
                    if result.get('id') == identifier:
                        require('return' in result and 'error' not in result, 'QMP command refused')
                        return
                    require('event' in result, 'Unexpected QMP response identifier')
                raise RuntimeError('QMP response bound exceeded')

            def send(payload):
                remaining = deadline - time.monotonic()
                require(remaining > 0, 'QMP quit deadline exceeded')
                client.settimeout(remaining)
                stream.write(payload)
                stream.flush()

            require('QMP' in message(), 'Unexpected QMP greeting')
            send(b'{"execute":"qmp_capabilities","id":"capabilities"}\n')
            response('capabilities')
            send(b'{"execute":"quit","id":"quit"}\n')
            # Closing first can discard a queued command in QEMU's monitor.
            # QAPI permits server EOF before the reply; only real exit0 qualifies it.
            response('quit', allow_eof=True)
            remaining = deadline - time.monotonic()
            require(remaining > 0, 'QMP quit deadline exceeded')
            require(process.wait(timeout=remaining) == 0, 'QEMU quit failed')


def stop(process):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=20)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=20)


def publish_phase_failure(fixture, root, phase, error, progress):
    """Retain bounded public metadata before private fixture cleanup."""
    try:
        reasons = {
            'ARM fixture exceeded its phase deadline': 'phase_deadline',
            'ARM serial output exceeded its bound': 'serial_limit',
            'Guest PID 1 froze': 'pid1_frozen',
            'QEMU failed': 'qemu_failed',
            'Expected phase result is absent or failed': 'phase_result_absent',
            'Secure-Boot-disabled refusal or untouched target evidence is absent': 'refusal_evidence_absent',
        }
        markers = {
            'firmware_boot_started': b'BdsDxe: starting Boot',
            'grub_version': b'GRUB version ',
            'installer_menu': b'Install Kedra (Fedora 44)',
            'grub_autoboot': b'executed automatically',
            'grub_prompt': b'grub>',
            'kernel_started': b'Linux version ',
            'initramfs_started': b'Running in initial RAM disk',
            'systemd_started': b'systemd[1]:',
            'secureboot_refusal': b'installation requires UEFI Secure Boot and must not start',
            'pid1_frozen': b'systemd[1]: Freezing execution.',
            'install_complete': b'KEDRA_FIXTURE_INSTALL_COMPLETE',
            'guest_failed': b'KEDRA_GHCR_FAIL',
            'no_space': b'No space left on device',
        }
        if phase in ('A', 'B', 'ROLLBACK'):
            markers['phase_pass'] = ('KEDRA_GHCR_' + phase + '_PASS').encode()
        private = root.parent / 'kedra-ghcr-private'
        serial_root = private if phase in ('refuse-insecure', 'install') else root
        logs = []
        for kind, path in (('serial', serial_root / (phase + '.serial.log')),
                           ('qemu', private / (phase + '.qemu.log'))):
            item = {'stream': kind, 'available': False}
            try:
                flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
                with os.fdopen(os.open(path, flags), 'rb') as stream:
                    before = os.fstat(stream.fileno())
                    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1,
                            'Diagnostic input is not an ordinary log')
                    data = stream.read(16 * 1024**2)
                    after = os.fstat(stream.fileno())
                item.update(available=True, size_bytes=before.st_size, hashed_bytes=len(data),
                            sha256=hashlib.sha256(data).hexdigest(),
                            complete=len(data) == before.st_size == after.st_size
                            and before.st_mtime_ns == after.st_mtime_ns,
                            observed_markers=[name for name, value in markers.items() if value in data])
            except (OSError, RuntimeError, ValueError):
                pass
            logs.append(item)
        process = progress.get('process')
        exit_code = process.poll() if process is not None else None
        started = progress.get('started')
        report = {
            'schema_version': 1, 'phase': phase,
            'reason': reasons.get(str(error), 'phase_failed'),
            'source_revision': fixture['source_revision'],
            'fixture_revision': fixture['fixture_revision'],
            'elapsed_seconds': time.monotonic() - started if started is not None else None,
            'deadline_seconds': progress.get('deadline_seconds'),
            'refusal_seen_elapsed_seconds': progress.get('refusal_seen_elapsed_seconds'),
            'quit_attempted': progress.get('quit_attempted', False),
            'quit_completed': progress.get('quit_completed', False),
            'quit_completed_elapsed_seconds': progress.get('quit_completed_elapsed_seconds'),
            'qemu_started': process is not None, 'qemu_exit_code': exit_code,
            'qemu_reaped': process is not None and exit_code is not None,
            'logs': logs, 'raw_private_output_exported': False,
        }
        with (Path(fixture['evidence']) / (phase + '-failure.json')).open('x') as stream:
            json.dump(report, stream, sort_keys=True)
            stream.write('\n')
    except (OSError, RuntimeError, ValueError, TypeError, KeyError, MemoryError):
        # No diagnostic exception or private text may replace the first failure.
        print('ARM public phase diagnostic unavailable', file=sys.stderr)


def phase_main(progress=None):
    if progress is None:
        progress = {}
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--phase', choices=('prepare-hvf', 'refuse-insecure', 'install', 'A', 'B', 'ROLLBACK'), required=True)
    parser.add_argument('--iso', type=Path)
    add_context_argument(parser)
    args = parser.parse_args()
    fixture = load_context(args.fixture_context)
    root = args.root.resolve()
    require(root == Path(fixture['runner_temp']) / 'kedra-ghcr',
            'Unexpected fixture directory')
    private = root.parent / 'kedra-ghcr-private'
    require(root.is_dir() and private.is_dir(), 'Fixture preparation is absent')
    preparing = args.phase == 'prepare-hvf'
    if preparing:
        require(fixture['mode'] == 'local' and args.fixture_context is not None,
                'Fresh HVF preparation requires an explicit local fixture')
        os.umask(0o077)
        require(not any(path.exists() or path.is_symlink() for path in (
            root / 'insecure-installer',
            *(root / (phase + suffix) for phase in ('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK')
              for suffix in ('-result.json', '-timeout.json', '.serial.log', '.qmp.sock')),
            *(private / (phase + suffix) for phase in ('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK')
              for suffix in ('.serial.log', '.qemu.log')),
        )), 'Fresh HVF preparation refuses previous boot evidence')
    insecure = args.phase == 'refuse-insecure'
    media_boot = preparing or args.phase in ('refuse-insecure', 'install')
    original_trust = firmware.trust(TEMPLATE)
    require({'PK', 'KEK', 'db', 'SecureBootEnable'} <= original_trust.keys()
            and original_trust['SecureBootEnable'][1] == b'\x01'
            and firmware.MICROSOFT_2023 in firmware.certificates(original_trust['db'][1])
            and original_trust['PK'][1] and original_trust['KEK'][1],
            'ARM template does not enforce the required Microsoft trust')
    descriptor = json.loads(DESCRIPTOR.read_text())
    require(descriptor['mapping']['nvram-template']['filename'] == str(TEMPLATE)
            and Path(descriptor['mapping']['executable']['filename']).resolve() == CODE.resolve()
            and {'enrolled-keys', 'secure-boot'} <= set(descriptor['features'])
            and any(target.get('architecture') == 'aarch64' for target in descriptor['targets']),
            'Unexpected ARM firmware descriptor')
    provenance = {'schema_version': 1, 'code_sha256': sha(CODE), 'vars_sha256': sha(TEMPLATE),
                  'descriptor_sha256': sha(DESCRIPTOR),
                  'db_certificate_sha256': sorted(firmware.certificates(original_trust['db'][1]))}
    package_record = Path('/usr/share/kedra-release-fixture/firmware/packages.txt')
    if package_record.is_file():
        provenance['firmware_packages'] = package_record.read_text()
    else:
        provenance['firmware_packages'] = command('dpkg-query', '-W', '-f=${Package} ${Version}\n',
            'qemu-efi-aarch64', 'python3-virt-firmware', capture_output=True, text=True).stdout
    (root / 'firmware-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
    state_root = root / 'insecure-installer' if insecure else root
    if insecure:
        state_root.mkdir(mode=0o700)
    variables = state_root / 'AAVMF_VARS.fd'
    disk = state_root / 'installed.qcow2'
    sentinel = state_root / 'sentinel.raw'
    manifest = state_root / 'disks.json'
    if media_boot:
        require(args.iso is not None and args.iso.is_file() and not args.iso.is_symlink(), 'Expected private fixture ISO')
        require(not any(path.exists() or path.is_symlink() for path in (variables, disk, sentinel, manifest)),
                'Install disks must be new')
        shutil.copyfile(TEMPLATE, variables)
        if insecure:
            command('virt-fw-vars', '--inplace', variables, '--set-false', 'SecureBootEnable',
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        command('qemu-img', 'create', '-f', 'qcow2', disk, '96G', stdout=subprocess.DEVNULL)
        with sentinel.open('xb') as stream:
            stream.write(os.urandom(16 * 1024**2))
        manifest.write_text(json.dumps({'schema_version': 1, 'blank_target_sha256': sha(disk),
            'sentinel_sha256': sha(sentinel), 'iso_sha256': sha(args.iso),
            'iso': str(args.iso.resolve())}, sort_keys=True) + '\n')
    else:
        require(args.iso is None and manifest.is_file() and disk.is_file(), 'Installed phase must not attach an ISO')
    selected = json.loads(manifest.read_text())
    require(sha(sentinel) == selected['sentinel_sha256'], 'Sentinel changed before boot')
    expected_trust = dict(original_trust)
    if insecure:
        expected_trust['SecureBootEnable'] = (original_trust['SecureBootEnable'][0], b'\x00')
    require(firmware.trust(variables) == expected_trust, 'ARM firmware trust changed')
    if preparing:
        command('qemu-img', 'check', disk, stdout=subprocess.DEVNULL)
        files = {
            'disks.json': manifest, 'installed.qcow2': disk, 'sentinel.raw': sentinel,
            'AAVMF_VARS.fd': variables, 'cases.raw': root / 'cases.raw',
            'installation-plan.json': root / 'installation-plan.json',
            'firmware-provenance.json': root / 'firmware-provenance.json',
            'installer.iso': args.iso, 'code.fd': CODE, 'template.fd': TEMPLATE,
            'firmware.json': DESCRIPTOR,
        }
        receipt = {'schema_version': 1, 'kind': 'kedra-fresh-hvf-preparation',
                   'source_revision': fixture['source_revision'],
                   'fixture_revision': fixture['fixture_revision'],
                   'context_sha256': sha(args.fixture_context), 'qemu_started': False,
                   'files': {name: {'sha256': sha(path), 'bytes': path.stat().st_size}
                             for name, path in files.items()}}
        output = root / 'hvf-preparation.json'
        with output.open('x') as stream:
            json.dump(receipt, stream, sort_keys=True)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        print(sha(output))
        return
    log = private / (args.phase + '.serial.log') if media_boot else root / (args.phase + '.serial.log')
    qmp = root / (args.phase + '.qmp.sock')
    require(not qmp.exists() and not log.exists(), 'Phase output already exists')
    argv = ['qemu-system-aarch64', '-machine', 'virt', '-cpu', 'max,pauth-impdef=on',
            '-accel', 'tcg,thread=multi', '-smp', '4', '-m', '8192',
            '-drive', f'if=pflash,format=raw,unit=0,readonly=on,file={CODE}',
            '-drive', f'if=pflash,format=raw,unit=1,file={variables}',
            '-drive', f'if=none,id=target,format=qcow2,file={disk}',
            '-device', 'virtio-blk-pci,drive=target,serial=KEDRA_INSTALL_ONLY',
            '-drive', f'if=none,id=sentinel,format=raw,file={sentinel}',
            '-device', 'virtio-blk-pci,drive=sentinel,serial=KEDRA_KEEP_DATA',
            '-drive', f'if=none,id=cases,format=raw,readonly=on,file={root / "cases.raw"}',
            '-device', 'virtio-blk-pci,drive=cases,serial=KEDRA_CASES',
            '-device', 'virtio-rng-pci', '-device', 'virtio-gpu-pci', '-device', 'virtio-keyboard-pci',
            '-netdev', 'user,id=net0', '-device', 'virtio-net-pci,netdev=net0,romfile=',
            '-display', 'none', '-monitor', 'none', '-no-reboot',
            '-qmp', f'unix:{qmp},server=on,wait=off', '-serial', f'file:{log}']
    if media_boot:
        argv.extend(['-device', 'virtio-scsi-pci,id=scsi0', '-drive',
                     f'if=none,id=cdrom,format=raw,media=cdrom,readonly=on,file={args.iso.resolve()}',
                     '-device', 'scsi-cd,drive=cdrom,bus=scsi0.0', '-boot', 'order=d'])
    password = (private / 'disk-passphrase').read_text().strip()
    require(len(password) == 48 and all(c in '0123456789abcdef' for c in password), 'Invalid private fixture passphrase')
    entered = False
    refused = False
    started = time.monotonic()
    deadline = started + (1800 if insecure else 7200 if media_boot else 5400)
    progress.update(started=started, deadline_seconds=deadline - started)
    with (private / (args.phase + '.qemu.log')).open('xb') as errors:
        process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=errors, stderr=errors, start_new_session=True)
        progress['process'] = process
        try:
            while process.poll() is None:
                require(time.monotonic() < deadline, 'ARM fixture exceeded its phase deadline')
                if log.exists():
                    require(log.stat().st_size <= LIMIT, 'ARM serial output exceeded its bound')
                    text = log.read_text(errors='replace')
                    require('systemd[1]: Freezing execution.' not in text, 'Guest PID 1 froze')
                    if insecure and not refused and 'installation requires UEFI Secure Boot and must not start' in text:
                        require('KEDRA_FIXTURE_INSTALL_COMPLETE' not in text, 'Insecure installation unexpectedly completed')
                        progress['refusal_seen_elapsed_seconds'] = time.monotonic() - started
                        progress['quit_attempted'] = True
                        qmp_quit(qmp, process, deadline)
                        progress.update(quit_completed=True, quit_completed_elapsed_seconds=time.monotonic() - started)
                        refused = True
                    if not media_boot and not entered and 'Please enter passphrase for disk' in text:
                        qmp_key(qmp, password)
                        entered = True
                time.sleep(1)
            require(process.returncode == 0, 'QEMU failed')
        finally:
            stop(process)
    text = log.read_text(errors='replace')
    marker = 'KEDRA_FIXTURE_INSTALL_COMPLETE' if media_boot else 'KEDRA_GHCR_' + args.phase + '_PASS'
    if insecure:
        require(refused and marker not in text and sha(disk) == selected['blank_target_sha256'],
                'Secure-Boot-disabled refusal or untouched target evidence is absent')
    else:
        require(marker in text and 'KEDRA_GHCR_FAIL' not in text, 'Expected phase result is absent or failed')
    if not media_boot:
        require('KEDRA_SECUREBOOT_PASS' in text and entered, 'Installed boot lacks security/unlock evidence')
    require(sha(sentinel) == selected['sentinel_sha256'], 'Unselected sentinel disk changed')
    require(sha(Path(selected['iso'])) == selected['iso_sha256'], 'Original installer media changed')
    require(firmware.trust(variables) == expected_trust, 'Firmware authority changed during boot')
    command('qemu-img', 'check', disk, stdout=subprocess.DEVNULL)
    report = {'schema_version': 1, 'phase': args.phase, 'outcome': 'refused' if insecure else 'pass',
              'elapsed_seconds': time.monotonic() - started, 'sentinel_unchanged': True,
              'iso_unchanged': True, 'firmware_trust_unchanged': True,
              'iso_attached': media_boot, 'luks_unlock_observed': entered,
              'stop': 'qmp-quit-after-verifier-refusal' if insecure else 'guest-poweroff'}
    (root / (args.phase + '-result.json')).write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, sort_keys=True))


def main():
    # The same controller-only gate protects the optional Mac transfer boundary.
    if '--help' in sys.argv:
        return phase_main()
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--phase', choices=('prepare-hvf', 'refuse-insecure', 'install', 'A', 'B', 'ROLLBACK'), required=True)
    add_context_argument(parser)
    args, _ = parser.parse_known_args()
    fixture = load_context(args.fixture_context)
    root = args.root.resolve()
    require(root == Path(fixture['runner_temp']) / 'kedra-ghcr' and root.is_dir(),
            'Unexpected fixture directory')
    descriptor = os.open(root / 'host-launch.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'r+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require(not (root / 'hvf-owner.json').exists(), 'Fixture was transferred; Linux boot is retired')
        require(not ((root / 'hvf-preparation.json').exists() or (root / 'hvf-preparation.json').is_symlink()),
                'Fixture was prepared for HVF; Linux boot is retired')
        progress = {}
        try:
            phase_main(progress)
        except Exception as error:
            publish_phase_failure(fixture, root, args.phase, error, progress)
            # phase_main has already reaped its owned QEMU in finally. Only the
            # unchanged phase deadline licenses a fresh-target HVF retry.
            if isinstance(error, RuntimeError) and str(error) == 'ARM fixture exceeded its phase deadline':
                try:
                    receipt = {'schema_version': 1, 'phase': args.phase, 'reason': 'phase_deadline',
                               'source_revision': fixture['source_revision'],
                               'fixture_revision': fixture['fixture_revision'], 'qemu_reaped': True}
                    with (root / (args.phase + '-timeout.json')).open('x') as stream:
                        json.dump(receipt, stream, sort_keys=True)
                        stream.write('\n')
                        stream.flush()
                        os.fsync(stream.fileno())
                except (OSError, ValueError, TypeError):
                    print('ARM timeout receipt unavailable', file=sys.stderr)
            raise


def interrupt(number, _frame):
    raise InterruptedError('Interrupted fixture phase: ' + str(number))


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, interrupt)
    signal.signal(signal.SIGINT, interrupt)
    main()
