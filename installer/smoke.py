"""Boot the unchanged ISO diskless through UEFI Secure Boot and wait for probe markers."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import time

# Debian/Ubuntu firmware: Secure Boot code plus a variable store with Microsoft
# UEFI CAs enrolled and Secure Boot enabled. The guest probe independently
# reports the firmware state; the smoke fails unless that report is present.
MACHINES = {
    'x86_64': {
        'qemu': 'qemu-system-x86_64', 'package': 'ovmf',
        'code': Path('/usr/share/OVMF/OVMF_CODE_4M.secboot.fd'),
        'vars': Path('/usr/share/OVMF/OVMF_VARS_4M.ms.fd'),
        # OVMF Secure Boot builds require SMM and a secure pflash varstore.
        'arguments': ['-machine', 'q35,smm=on,accel=kvm', '-cpu', 'host',
                      '-global', 'driver=cfi.pflash01,property=secure,value=on',
                      '-global', 'ICH9-LPC.disable_s3=1', '-vga', 'std',
                      '-device', 'ide-cd,drive=installer,bootindex=0'],
    },
    'aarch64': {
        'qemu': 'qemu-system-aarch64', 'package': 'qemu-efi-aarch64',
        'code': Path('/usr/share/AAVMF/AAVMF_CODE.secboot.fd'),
        'vars': Path('/usr/share/AAVMF/AAVMF_VARS.ms.fd'),
        'arguments': ['-machine', 'virt,gic-version=max,accel=kvm', '-cpu', 'host',
                      '-device', 'virtio-gpu-pci', '-device', 'virtio-scsi-pci,id=scsi',
                      '-device', 'scsi-cd,bus=scsi.0,drive=installer,bootindex=0'],
    },
}
MARKERS = ('KEDRA_INSTALLER_SECUREBOOT_ENABLED', 'KEDRA_INSTALLER_SIGNED_PAYLOAD_PASS',
           'KEDRA_INSTALLER_ANACONDA_STARTED')
# UEFI, the unchanged 60 s ISO menu timeout, boot, verification (<=330 s) and Anaconda.
DEADLINE = 600


def identity(path):
    with path.open('rb') as stream:
        return {'path': str(path), 'size_bytes': path.stat().st_size,
                'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}


def command(machine, qemu, iso, variables, events, serial):
    return [
        qemu, *machine['arguments'], '-smp', '2', '-m', '6144', '-device', 'virtio-rng-pci',
        '-drive', f'if=pflash,format=raw,unit=0,readonly=on,file={machine["code"]}',
        '-drive', f'if=pflash,format=raw,unit=1,file={variables}',
        '-drive', f'if=none,id=installer,media=cdrom,format=raw,readonly=on,file={iso}',
        # Explicit research trigger without editing the media's boot menu.
        '-smbios', 'type=11,value=io.systemd.credential:kedra.research=1',
        '-device', 'virtio-serial-pci', '-chardev', f'file,id=events,path={events}',
        '-device', 'virtserialport,chardev=events,name=org.kedra.events',
        '-display', 'none', '-nic', 'none', '-monitor', 'none', '-no-reboot',
        '-serial', f'file:{serial}',
    ]


def run(architecture, qemu, iso, work, environment):
    machine = MACHINES[architecture]
    work.mkdir(mode=0o700)
    variables = work / 'efi-vars.fd'
    shutil.copyfile(machine['vars'], variables)
    events = work / 'events.log'
    version = subprocess.run([qemu, '--version'], check=False, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                             env=environment, timeout=30)
    if version.returncode != 0:
        raise RuntimeError('Command failed: ' + qemu)
    with (work / 'qemu.log').open('w') as log:
        process = subprocess.Popen(command(machine, qemu, iso, variables, events, work / 'serial.log'),
                                   stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, env=environment)
        try:
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
                    break
                time.sleep(1)
            else:
                raise RuntimeError('Secure Boot installer smoke did not pass; see ' + str(work))
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    return {'architecture': architecture, 'qemu': version.stdout.decode().splitlines()[0],
            'firmware_code': identity(machine['code']), 'firmware_variables_template': identity(machine['vars']),
            'markers': list(MARKERS)}
