#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Boot-level qemu-arm64 qualification on a disposable aarch64 Linux host.

Checks the candidate's bootc contract, layers the observer fixture, builds a
generated disk and boots it under UEFI Secure Boot through an explicit
\\EFI\\fedora\\shimaa64.efi NVRAM entry: KVM when /dev/kvm is usable, TCG
otherwise. The guest reports Secure Boot, bootc, SELinux and unit markers on
its org.kedra.events port.

  uv run usr/src/kedra/tests/vm/qemu-arm64/run.py --disposable-host --work /new/dir \\
      --candidate ghcr.io/reidond/kedra-qemu-arm64:stable
"""
import argparse
import json
import platform
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import disk
import qemu
import secure_boot

COMMON = HERE.parents[1] / 'common'
CANDIDATE = 'localhost/kedra-qemu-arm64:research'
FIXTURE = 'localhost/kedra-qemu-arm64-test:secureboot'
FILES = {'Containerfile': HERE / 'Containerfile', 'check.sh': HERE / 'check.sh', 'check.service': HERE / 'check.service',
         'secure_boot_check.py': COMMON / 'secure_boot_check.py'}
HOST_TOOLS = ('qemu-system-aarch64', 'qemu-img', 'virt-fw-vars', 'dpkg-query')
MARKERS = ('KEDRA_SECUREBOOT_PASS', 'KEDRA_QEMU_ARM64_SECUREBOOT_PASS', 'KEDRA_QEMU_ARM64_BOOTC_PASS',
           'KEDRA_QEMU_ARM64_SELINUX_PASS', 'KEDRA_QEMU_ARM64_UNITS_PASS')
# TCG on a slow host stays well inside this; KVM finishes in minutes.
BOUND = 90 * 60


def arguments():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--disposable-host', action='store_true', help='Required: this throwaway host may be changed')
    parser.add_argument('--work', type=Path, required=True, help='New directory for disks, evidence and secrets')
    parser.add_argument('--candidate', required=True,
                        help='docker:REF from the local Docker Engine, podman:REF in rootful Podman, or a registry reference')
    parser.add_argument('--accelerator', choices=('kvm', 'tcg'))
    return parser.parse_args()


def contract(evidence):
    """The candidate carries at least the bootc version the release contract names."""
    minimum = json.loads((HERE.parents[2] / 'image/release/compatibility.json').read_bytes())['minimum_bootc_version']
    podman = ('run', '--rm', '--network=none', CANDIDATE)
    disk.podman(*podman, 'bootc', 'container', 'lint', output=evidence / 'final-lint.log')
    actual = disk.podman(*podman, 'rpm', '-q', '--qf', '%{VERSION} %{ARCH}', 'bootc', capture=True)
    version, _, architecture = actual.partition(' ')
    numeric = re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version)
    disk.require(architecture == 'aarch64' and numeric is not None
                 and tuple(map(int, version.split('.'))) >= tuple(map(int, minimum.split('.'))),
                 f'Candidate bootc {actual!r} is older than the contract minimum {minimum}')


def command(work, generated, accelerator):
    evidence = work / 'evidence'
    code = secure_boot.firmware('aarch64')['code']
    argv = [qemu.qemu_binary('aarch64'), *qemu.machine_argv(
        'aarch64', accelerator, code=code, variables=work / 'AAVMF_VARS.fd', qmp=work / 'qmp.sock',
        serial=evidence / 'serial.log', cpus=4, memory_mib=4096)]
    argv += qemu.disk_argv('system', generated, snapshot=True)
    argv += qemu.events_argv(evidence / 'events.log')
    # The image's qemu-guest-agent unit must bind to the host-provided channel.
    argv += ['-chardev', f'socket,id=qga,path={qemu.option_path(work / "qga.sock")},server=on,wait=off',
             '-device', 'virtserialport,chardev=qga,name=org.qemu.guest_agent.0']
    argv += qemu.network_argv(True)
    # The explicit entry needs no fallback reset; poweroff ends the run.
    return argv + ['-device', 'virtio-gpu-pci', '-display', 'none', '-no-reboot']


def boot(work, generated, accelerator, reference, digest):
    evidence = work / 'evidence'
    qemu.create_variables('aarch64', work / 'AAVMF_VARS.fd', boot_entry=qemu.AARCH64_SHIM)
    entries = subprocess.run(['virt-fw-vars', '--input', work / 'AAVMF_VARS.fd', '--print'], check=True,
                             capture_output=True, text=True, timeout=60).stdout
    (evidence / 'firmware-boot-vars.txt').write_text(entries)
    disk.require('devpath=FilePath(\\EFI\\fedora\\shimaa64.efi)' in entries, 'The explicit shim entry is absent')
    started = time.monotonic()
    process = qemu.launch(command(work, generated, accelerator), evidence / 'qemu.log')
    try:
        qemu.wait_for_socket(work / 'qmp.sock', process)
        qemu.watch(process, work / 'qmp.sock', [evidence / 'serial.log', evidence / 'events.log'],
                   deadline=started + BOUND, required=(*MARKERS, 'KEDRA_QEMU_ARM64_IMAGE_DIGEST=' + digest,
                                                       'KEDRA_QEMU_ARM64_IMAGE_REFERENCE=' + reference),
                   failures=('KEDRA_QEMU_ARM64_FAIL',))
    finally:
        qemu.stop(process)
    secure_boot.verify(work / 'AAVMF_VARS.fd', 'aarch64')
    # Firmware selected the explicit entry, not the removable-media fallback.
    disk.require('from \\EFI\\fedora\\shimaa64.efi' in qemu.read_log(evidence / 'serial.log'),
                 'Firmware did not boot the explicit shim entry')
    return time.monotonic() - started


def main():
    args = arguments()
    qemu.require_disposable_host(args.disposable_host)
    disk.require(platform.machine() == 'aarch64', 'The qemu-arm64 driver needs an aarch64 Linux host')
    disk.require_tools(HOST_TOOLS)
    accelerator = args.accelerator or qemu.default_accelerator('aarch64')
    work = disk.new_work(args.work)
    evidence = work / 'evidence'
    try:
        disk.import_candidate(args.candidate, CANDIDATE, evidence)
        contract(evidence)
        disk.build_fixture(work / 'context', FILES, FIXTURE, evidence)
        # The candidate enforces its signature policy at install: admit the
        # fixture through a generated key rather than disabling verification.
        reference, buildroot, digest = disk.sign_fixture(work, FIXTURE)
        # Headless: the account only lets the observer run doctor as a non-root user.
        disk.blueprint(work / 'private', keep_password=False)
        generated = disk.build_disk('arm64', reference, work, build_container=buildroot)
        secure_boot.provenance(evidence, 'aarch64')
        elapsed = boot(work, generated, accelerator, reference, digest)
    finally:
        disk.sanitize(work)
    print(json.dumps({'schema_version': 1, 'target': 'qemu-arm64', 'accelerator': accelerator,
                      'boot_seconds': round(elapsed), 'image_digest': digest, 'markers': list(MARKERS)}, sort_keys=True))
    print(f'PASS: qemu-arm64 disk booted shim -> GRUB -> kernel under UEFI Secure Boot ({accelerator}) '
          'and reported every marker.')


if __name__ == '__main__':
    try:
        main()
    except (disk.DiskError, qemu.VmError, secure_boot.FirmwareError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit('qemu-arm64 VM: ' + str(error)) from None
