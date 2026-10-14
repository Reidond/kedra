#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Boot-level desktop qualification on a disposable x86_64 Linux host with KVM.

Layers the observer fixture over one desktop candidate, builds a generated disk
with a disposable account, then boots it under UEFI Secure Boot: tuigreet
password login on VT 1, PAM login-keyring unlock, the session on the virtio
GPU, doctor's session gate and the Xwayland and Qt file choosers.

  uv run usr/src/kedra/tests/vm/desktop/run.py --disposable-host --work /new/dir \\
      --candidate docker:kedra-local-desktop:KEY
"""
import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import disk
import qemu

COMMON = HERE.parents[1] / 'common'
CANDIDATE = 'localhost/kedra-desktop:research'
FIXTURE = 'localhost/kedra-desktop-test:r07'
FILES = {
    'Containerfile': HERE / 'Containerfile', 'check.sh': HERE / 'check.sh', 'check.service': HERE / 'check.service',
    'test-profile.toml': COMMON / 'test-profile.toml', 'console.toml': COMMON / 'console.toml',
    'secure_boot_check.py': COMMON / 'secure_boot_check.py',
    # The guest toolkit fixture is shared with the container scenarios.
    'toolkit-app.py': HERE.parents[1] / 'container/lab/probes/toolkit-app.py',
}
HOST_TOOLS = ('qemu-system-x86_64', 'virt-fw-vars', 'xvfb-run', 'xdotool', 'import', 'dpkg-query')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--disposable-host', action='store_true', help='Required: this throwaway host may be changed')
    parser.add_argument('--work', type=Path, required=True, help='New directory for disks, evidence and secrets')
    parser.add_argument('--candidate', required=True,
                        help='docker:REF from the local Docker Engine, podman:REF in rootful Podman, or a registry reference')
    return parser.parse_args()


def main():
    args = arguments()
    qemu.require_disposable_host(args.disposable_host)
    disk.require(platform.machine() == 'x86_64' and os.access('/dev/kvm', os.R_OK | os.W_OK),
                 'The desktop VM needs an x86_64 host with KVM access for the invoking user')
    disk.require_tools(HOST_TOOLS)
    work = disk.new_work(args.work)
    evidence = work / 'evidence'
    try:
        disk.import_candidate(args.candidate, CANDIDATE, evidence)
        disk.build_fixture(work / 'context', FILES, FIXTURE, evidence)
        password = disk.blueprint(work / 'private')
        generated = disk.build_disk('amd64', FIXTURE, work)
        subprocess.run(['uv', 'run', COMMON / 'secure_boot.py', 'provenance', '--evidence', evidence],
                       check=True, timeout=300)
        # sudo resets PATH; hand root the pinned uv by absolute path. Root opens
        # /dev/kvm and the GTK GL display on a private Xvfb server.
        uv = shutil.which('uv')
        with (evidence / 'vm-result.txt').open('wb') as result:
            subprocess.run(['sudo', 'xvfb-run', '-a', '-s', '-screen 0 1280x768x24', 'env', 'LIBGL_ALWAYS_SOFTWARE=1',
                            uv, 'run', HERE / 'session.py', '--disk', generated, '--work', evidence / 'vm',
                            '--password-file', password], check=True, stdout=result, timeout=3600)
        print((evidence / 'vm-result.txt').read_text(), end='')
    finally:
        disk.sanitize(work)


if __name__ == '__main__':
    try:
        main()
    except (disk.DiskError, qemu.VmError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit('desktop VM: ' + str(error)) from None
