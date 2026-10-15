#!/usr/bin/python3
# Runs inside a disposable VM guest with the guest's own python3 and standard
# library; uv applies to host-side scripts only. Every VM fixture installs this
# one file; drivers and the native lab import or execute it, never a copy.
"""The guest's UEFI Secure Boot check.

Shim must report Secure Boot enabled, the firmware SecureBoot/SetupMode
variables must be 1/0, kernel lockdown must select integrity or confidentiality
and the kernel journal must keep the kernel's own report, which a quiet console
hides. With --boot-entry, firmware must have booted the BootCurrent entry naming
that loader. Prints one JSON result line, then KEDRA_SECUREBOOT_PASS or
KEDRA_SECUREBOOT_FAIL with the reason; a failure exits 1.
"""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

EFI_GLOBAL = '8be4df61-93ca-11d2-aa0d-00e098032b8c'
EFIVARS = Path('/sys/firmware/efi/efivars')
LOCKDOWN = Path('/sys/kernel/security/lockdown')
KERNEL_REPORT = 'secureboot: Secure boot enabled'
PASS = 'KEDRA_SECUREBOOT_PASS'
FAIL = 'KEDRA_SECUREBOOT_FAIL'
ENVIRONMENT = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LC_ALL': 'C'}


class SecureBootError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise SecureBootError(message)


def run(argv):
    result = subprocess.run(argv, capture_output=True, text=True, errors='replace', timeout=120,
                            check=False, env=ENVIRONMENT)
    require(result.returncode == 0, f'{argv[0]} exited {result.returncode}: {result.stderr.strip()[-500:]}')
    require(len(result.stdout) <= 16 * 1024**2, f'{argv[0]} output exceeded its bound')
    return result.stdout


def efi_variable(name):
    # efivarfs prefixes each value with 4 attribute bytes.
    with (EFIVARS / f'{name}-{EFI_GLOBAL}').open('rb') as stream:
        data = stream.read(6)
    require(len(data) == 5, f'unexpected EFI {name} variable size {len(data)}')
    return data[4]


def booted_entry(loader):
    listing = run(['efibootmgr', '-v'])
    current = re.search(r'^BootCurrent: ([0-9A-Fa-f]{4})$', listing, re.MULTILINE)
    require(current is not None, 'firmware reports no BootCurrent')
    entry = re.search(r'^Boot' + current[1] + r'[* ].*$', listing, re.MULTILINE)
    require(entry is not None and loader.lower() in entry[0].lower(),
            'firmware did not boot ' + loader + ': ' + (entry[0] if entry else 'no entry'))
    return entry[0], listing


def check(boot_entry=None):
    """Return the evidence or raise SecureBootError; the caller emits markers."""
    shim = run(['mokutil', '--sb-state']).rstrip('\n')
    require(shim == 'SecureBoot enabled', 'shim reports ' + repr(shim))
    efi = {name: efi_variable(name) for name in ('SecureBoot', 'SetupMode')}
    require(efi == {'SecureBoot': 1, 'SetupMode': 0}, 'firmware variables ' + json.dumps(efi, sort_keys=True))
    lockdown = LOCKDOWN.read_text().strip()
    require('[integrity]' in lockdown or '[confidentiality]' in lockdown, 'kernel lockdown is ' + lockdown)
    kernel = run(['journalctl', '-k', '-b', '--no-pager', '-o', 'cat'])
    require(KERNEL_REPORT in kernel.splitlines(), 'the kernel journal lacks ' + repr(KERNEL_REPORT))
    result = {'schema_version': 1, 'shim': shim, 'efi': efi, 'lockdown': lockdown, 'kernel': KERNEL_REPORT}
    if boot_entry is not None:
        result['boot_entry'], result['efibootmgr'] = booted_entry(boot_entry)
    return result


def publish(path, result):
    """Replace a world-readable result so an unprivileged guest user can read it."""
    path.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    pending = path.with_name(path.name + '.pending')
    descriptor = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o644)
    with os.fdopen(descriptor, 'w') as stream:
        json.dump(result, stream, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(pending, path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--boot-entry', help='Loader the firmware BootCurrent entry must name, e.g. shimaa64.efi')
    parser.add_argument('--output', type=Path, help='Also publish the JSON result here (mode 0644)')
    args = parser.parse_args()
    boot_id = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    try:
        result = {**check(args.boot_entry), 'passed': True}
    except (OSError, ValueError, SecureBootError, subprocess.SubprocessError) as error:
        result = {'schema_version': 1, 'passed': False, 'error': str(error)}
    result['boot_id'] = boot_id
    if args.output is not None:
        publish(args.output, result)
    print(json.dumps({key: value for key, value in result.items() if key != 'efibootmgr'}, sort_keys=True), flush=True)
    print(PASS if result['passed'] else FAIL + ' ' + result['error'], flush=True)
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
