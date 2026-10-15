"""Boot the unchanged ISO diskless through UEFI Secure Boot and wait for probe markers."""
import hashlib
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tests/common'))
import qemu
import secure_boot

# Debian/Ubuntu firmware through the shared VM launcher: Secure Boot code plus
# a variable store with Microsoft UEFI CAs enrolled and Secure Boot enabled.
# The guest probe independently reports the firmware state; the smoke fails
# unless that report is present.
MACHINES = {
    architecture: {'qemu': qemu.qemu_binary(architecture), 'package': spec['packages'][0],
                   'code': spec['code'], 'vars': spec['templates']['microsoft']}
    for architecture, spec in secure_boot.FIRMWARE.items()
}
MARKERS = ('KEDRA_INSTALLER_SECUREBOOT_ENABLED', 'KEDRA_INSTALLER_SIGNED_PAYLOAD_PASS',
           'KEDRA_INSTALLER_ANACONDA_STARTED')
# UEFI, the unchanged 60 s ISO menu timeout, boot, verification (<=330 s) and Anaconda.
DEADLINE = 600


def identity(path):
    with path.open('rb') as stream:
        return {'path': str(path), 'size_bytes': path.stat().st_size,
                'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}


def command(architecture, qemu_path, iso, variables, events, work):
    machine = qemu.machine_argv(architecture, 'kvm', code=MACHINES[architecture]['code'], variables=variables,
                                qmp=work / 'qmp.sock', serial=work / 'serial.log', cpus=2, memory_mib=6144)
    if architecture == 'x86_64':
        media = ['-global', 'ICH9-LPC.disable_s3=1', '-vga', 'std', '-device', 'ide-cd,drive=installer,bootindex=0']
    else:
        media = ['-device', 'virtio-gpu-pci', '-device', 'virtio-scsi-pci,id=scsi',
                 '-device', 'scsi-cd,bus=scsi.0,drive=installer,bootindex=0']
    return [
        qemu_path, *machine, *media,
        '-drive', f'if=none,id=installer,media=cdrom,format=raw,readonly=on,file={qemu.option_path(iso)}',
        # Explicit research trigger without editing the media's boot menu.
        '-smbios', 'type=11,value=io.systemd.credential:kedra.research=1',
        *qemu.events_argv(events), *qemu.network_argv(False), '-display', 'none', '-no-reboot',
    ]


def wait(process, events):
    deadline = time.monotonic() + DEADLINE
    while process.poll() is None and time.monotonic() < deadline:
        text = events.read_text(errors='replace') if events.exists() else ''
        if 'KEDRA_INSTALLER_PROBE_FAIL' in text:
            raise RuntimeError('Installer boot probe failed; see ' + str(events))
        if 'KEDRA_INSTALLER_ANACONDA_STARTED' in text:
            if not all(marker in text for marker in MARKERS):
                raise RuntimeError('Anaconda started without Secure Boot or embedded-payload verification')
            print('PASS: UEFI Secure Boot, offline signed payload verification and Anaconda startup; '
                  'no disks attached')
            return
        time.sleep(1)
    raise RuntimeError('Secure Boot installer smoke did not pass; see ' + str(events.parent))


def run(architecture, qemu_path, iso, work, environment):
    machine = MACHINES[architecture]
    work.mkdir(mode=0o700)
    variables = work / 'efi-vars.fd'
    shutil.copyfile(machine['vars'], variables)
    events = work / 'events.log'
    version = subprocess.run([qemu_path, '--version'], check=False, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, env=environment, timeout=30)
    if version.returncode != 0:
        raise RuntimeError('Command failed: ' + qemu_path)
    process = qemu.launch(command(architecture, qemu_path, iso, variables, events, work), work / 'qemu.log',
                          env=environment)
    try:
        wait(process, events)
    finally:
        qemu.stop(process, timeout=10)
    return {'architecture': architecture, 'qemu': version.stdout.decode().splitlines()[0],
            'firmware_code': identity(machine['code']), 'firmware_variables_template': identity(machine['vars']),
            'markers': list(MARKERS)}
