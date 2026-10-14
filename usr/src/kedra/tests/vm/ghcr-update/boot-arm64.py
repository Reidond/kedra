#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Boot only this workflow's generated ARM installer/installed disks.

The shared launcher (usr/src/kedra/tests/common/qemu.py) builds the Secure Boot
machine: KVM when the disposable host offers it, TCG otherwise.
"""
import argparse
import fcntl
import hashlib
import json
import os
import re
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path
from typing import NamedTuple

import installer_stages
from fixture import add_context_argument, load_context

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import qemu
import secure_boot as firmware

TEMPLATE = firmware.FIRMWARE['aarch64']['templates']['microsoft']
CODE = firmware.FIRMWARE['aarch64']['code']
LIMIT = qemu.LOG_LIMIT


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
    with qemu.Qmp(path) as session:
        session.type_text(password + '\n')


def qmp_quit(path, process, phase_deadline):
    remaining = min(15, phase_deadline - time.monotonic())
    require(remaining > 0, 'QMP quit deadline exceeded')
    with qemu.Qmp(path, timeout=remaining) as session:
        session.quit(process, timeout=max(0.1, phase_deadline - time.monotonic()))


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
            'payload_verification_failed': b'embedded payload signature verification failed; installation must not start',
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
            'installer_stages_first_seen_seconds': progress.get('installer_stages', {}),
            'guest_health': progress.get('guest_health', {}),
            'host_qemu_health': progress.get('host_qemu_health', {}),
            'installer_stage_observer_sha256': sha(Path(installer_stages.__file__)),
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


def parse_phase_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--phase', choices=('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK'), required=True)
    parser.add_argument('--iso', type=Path)
    add_context_argument(parser)
    return parser.parse_args()


def verified_firmware_trust():
    """The Microsoft-enrolled AAVMF template and Ubuntu's descriptor pairing it with the code."""
    return firmware.template('microsoft', 'aarch64'), firmware.enrolled_descriptor('aarch64')


def write_firmware_provenance(root, original_trust, descriptor):
    provenance = {'schema_version': 1, 'code_sha256': sha(CODE), 'vars_sha256': sha(TEMPLATE),
                  'descriptor_sha256': sha(descriptor),
                  'db_certificate_sha256': sorted(firmware.certificates(original_trust['db'][1]))}
    package_record = Path('/usr/share/kedra-release-fixture/firmware/packages.txt')
    if package_record.is_file():
        provenance['firmware_packages'] = package_record.read_text()
    else:
        provenance['firmware_packages'] = command('dpkg-query', '-W', '-f=${Package} ${Version}\n',
            'qemu-efi-aarch64', 'python3-virt-firmware', capture_output=True, text=True).stdout
    (root / 'firmware-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')


class PhaseDisks(NamedTuple):
    variables: Path
    disk: Path
    sentinel: Path
    manifest: Path


def prepare_phase_disks(root, iso, insecure, media_boot):
    state_root = root / 'insecure-installer' if insecure else root
    if insecure:
        state_root.mkdir(mode=0o700)
    disks = PhaseDisks(state_root / 'AAVMF_VARS.fd', state_root / 'installed.qcow2',
                       state_root / 'sentinel.raw', state_root / 'disks.json')
    if media_boot:
        create_install_disks(disks, iso, insecure)
    else:
        require(iso is None and disks.manifest.is_file() and disks.disk.is_file(), 'Installed phase must not attach an ISO')
    return disks


def create_install_disks(disks, iso, insecure):
    require(iso is not None and iso.is_file() and not iso.is_symlink(), 'Expected private fixture ISO')
    require(not any(path.exists() or path.is_symlink() for path in disks),
            'Install disks must be new')
    firmware.create('microsoft', disks.variables, 'aarch64')
    if insecure:
        command('virt-fw-vars', '--inplace', disks.variables, '--set-false', 'SecureBootEnable',
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    command('qemu-img', 'create', '-f', 'qcow2', disks.disk, '96G', stdout=subprocess.DEVNULL)
    with disks.sentinel.open('xb') as stream:
        stream.write(os.urandom(16 * 1024**2))
    disks.manifest.write_text(json.dumps({'schema_version': 1, 'blank_target_sha256': sha(disks.disk),
        'sentinel_sha256': sha(disks.sentinel), 'iso_sha256': sha(iso),
        'iso': str(iso.resolve())}, sort_keys=True) + '\n')


def qemu_argv(disks, root, qmp, log, iso, accelerator):
    argv = [qemu.qemu_binary('aarch64'), *qemu.machine_argv(
        'aarch64', accelerator, code=CODE, variables=disks.variables, qmp=qmp, serial=log, cpus=4, memory_mib=8192)]
    argv += qemu.disk_argv('target', disks.disk, serial='KEDRA_INSTALL_ONLY')
    argv += qemu.disk_argv('sentinel', disks.sentinel, image_format='raw', serial='KEDRA_KEEP_DATA')
    argv += qemu.disk_argv('cases', root / 'cases.raw', image_format='raw', readonly=True, serial='KEDRA_CASES')
    argv += ['-device', 'virtio-gpu-pci', '-device', 'virtio-keyboard-pci', *qemu.network_argv(True),
             '-display', 'none', '-no-reboot']
    if iso is not None:
        argv.extend(['-device', 'virtio-scsi-pci,id=scsi0', '-drive',
                     f'if=none,id=cdrom,format=raw,media=cdrom,readonly=on,file={qemu.option_path(iso.resolve())}',
                     '-device', 'scsi-cd,drive=cdrom,bus=scsi0.0', '-boot', 'order=d'])
    return argv


def record_installer_progress(text, phase, progress, started):
    if phase == 'install':
        installer_stages.observe_health(text, progress, int((time.monotonic() - started) * 1000))
    stages = progress.setdefault('installer_stages', {})
    for token in re.findall(r'(?:^|\n)KEDRA_INSTALL_STAGE_([A-Z0-9_]{1,64})\r?(?=\n|$)', text):
        if token in installer_stages.TOKENS and len(stages) < 256:
            stages.setdefault(token, time.monotonic() - started)


def quit_after_refusal(text, qmp, process, deadline, progress, started):
    require('KEDRA_FIXTURE_INSTALL_COMPLETE' not in text, 'Insecure installation unexpectedly completed')
    progress['refusal_seen_elapsed_seconds'] = time.monotonic() - started
    progress['quit_attempted'] = True
    qmp_quit(qmp, process, deadline)
    progress.update(quit_completed=True, quit_completed_elapsed_seconds=time.monotonic() - started)


def boot_phase(args, root, private, disks, log, insecure, media_boot, progress):
    qmp = root / (args.phase + '.qmp.sock')
    require(not qmp.exists() and not log.exists(), 'Phase output already exists')
    argv = qemu_argv(disks, root, qmp, log, args.iso if media_boot else None, progress['accelerator'])
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
                if args.phase == 'install':
                    installer_stages.sample_host_health(process, root, progress, int((time.monotonic() - started) * 1000))
                if log.exists():
                    require(log.stat().st_size <= LIMIT, 'ARM serial output exceeded its bound')
                    text = log.read_text(errors='replace')
                    record_installer_progress(text, args.phase, progress, started)
                    require('systemd[1]: Freezing execution.' not in text, 'Guest PID 1 froze')
                    if insecure and not refused and 'installation requires UEFI Secure Boot and must not start' in text:
                        quit_after_refusal(text, qmp, process, deadline, progress, started)
                        refused = True
                    if not media_boot and not entered and 'Please enter passphrase for disk' in text:
                        qmp_key(qmp, password)
                        entered = True
                time.sleep(1)
            require(process.returncode == 0, 'QEMU failed')
        finally:
            qemu.stop(process)
    return started, entered, refused


def require_fixture_intact(disks, selected, expected_trust):
    require(sha(disks.sentinel) == selected['sentinel_sha256'], 'Unselected sentinel disk changed')
    require(sha(Path(selected['iso'])) == selected['iso_sha256'], 'Original installer media changed')
    require(firmware.trust(disks.variables) == expected_trust, 'Firmware authority changed during boot')
    command('qemu-img', 'check', disks.disk, stdout=subprocess.DEVNULL)


def write_phase_report(root, phase, started, insecure, media_boot, entered, accelerator):
    report = {'schema_version': 1, 'phase': phase, 'outcome': 'refused' if insecure else 'pass',
              'accelerator': accelerator,
              'elapsed_seconds': time.monotonic() - started, 'sentinel_unchanged': True,
              'iso_unchanged': True, 'firmware_trust_unchanged': True,
              'iso_attached': media_boot, 'luks_unlock_observed': entered,
              'stop': 'qmp-quit-after-verifier-refusal' if insecure else 'guest-poweroff'}
    (root / (phase + '-result.json')).write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, sort_keys=True))


def phase_main(progress=None):
    if progress is None:
        progress = {}
    args = parse_phase_arguments()
    fixture = load_context(args.fixture_context)
    root = args.root.resolve()
    require(root == Path(fixture['runner_temp']) / 'kedra-ghcr',
            'Unexpected fixture directory')
    private = root.parent / 'kedra-ghcr-private'
    require(root.is_dir() and private.is_dir(), 'Fixture preparation is absent')
    insecure = args.phase == 'refuse-insecure'
    media_boot = args.phase in ('refuse-insecure', 'install')
    progress['accelerator'] = qemu.default_accelerator('aarch64')
    original_trust, descriptor = verified_firmware_trust()
    write_firmware_provenance(root, original_trust, descriptor)
    disks = prepare_phase_disks(root, args.iso, insecure, media_boot)
    selected = json.loads(disks.manifest.read_text())
    require(sha(disks.sentinel) == selected['sentinel_sha256'], 'Sentinel changed before boot')
    expected_trust = dict(original_trust)
    if insecure:
        expected_trust['SecureBootEnable'] = (original_trust['SecureBootEnable'][0], b'\x00')
    require(firmware.trust(disks.variables) == expected_trust, 'ARM firmware trust changed')
    log = private / (args.phase + '.serial.log') if media_boot else root / (args.phase + '.serial.log')
    started, entered, refused = boot_phase(args, root, private, disks, log, insecure, media_boot, progress)
    text = log.read_text(errors='replace')
    marker = 'KEDRA_FIXTURE_INSTALL_COMPLETE' if media_boot else 'KEDRA_GHCR_' + args.phase + '_PASS'
    if insecure:
        require(refused and marker not in text and sha(disks.disk) == selected['blank_target_sha256'],
                'Secure-Boot-disabled refusal or untouched target evidence is absent')
    else:
        require(marker in text and 'KEDRA_GHCR_FAIL' not in text, 'Expected phase result is absent or failed')
    if not media_boot:
        require('KEDRA_SECUREBOOT_PASS' in text and entered, 'Installed boot lacks security/unlock evidence')
    require_fixture_intact(disks, selected, expected_trust)
    write_phase_report(root, args.phase, started, insecure, media_boot, entered, progress['accelerator'])


def main():
    if '--help' in sys.argv:
        return phase_main()
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--phase', choices=('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK'), required=True)
    add_context_argument(parser)
    args, _ = parser.parse_known_args()
    fixture = load_context(args.fixture_context)
    root = args.root.resolve()
    require(root == Path(fixture['runner_temp']) / 'kedra-ghcr' and root.is_dir(),
            'Unexpected fixture directory')
    descriptor = os.open(root / 'host-launch.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'r+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        progress = {}
        try:
            phase_main(progress)
        except Exception as error:
            publish_phase_failure(fixture, root, args.phase, error, progress)
            # phase_main has already reaped its owned QEMU in finally; record
            # a deadline separately from other failures.
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
