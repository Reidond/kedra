#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Run context of the boot-level drivers: a hosted runner or a declared disposable host.

The drivers change host configuration (/etc/hosts, container registry trust,
/usr/libexec, /dev/kvm), start privileged image builders and boot generated
disks. They run only where that is harmless: the removed hosted Actions runners,
or a Linux host that the operator declares disposable with KEDRA_DISPOSABLE_HOST=1.
A declared host must run Ubuntu 24.04, the runner image whose firmware and
packages the drivers pin. It must not be a booted bootc/OSTree system such as a
Kedra workstation, and the driver runs as an ordinary user with non-interactive
sudo, like the runner account.

On a declared host this supplies what the runner supplied: RUNNER_TEMP is a
fresh private directory, GITHUB_SHA the clean committed HEAD, GITHUB_RUN_ID a
local run number and GITHUB_RUN_ATTEMPT 1. It builds the workspace release
binaries for that HEAD, as each workflow did before its driver, and refuses an
existing evidence directory so that separate runs never mix their evidence. It
relaxes none of a driver's own checks.

Python drivers call enter(); bash drivers source disposable-host.sh, which runs
this file and exports the printed values.
"""
import argparse
import json
import os
import platform
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
DECLARATION = 'KEDRA_DISPOSABLE_HOST'
ARCHITECTURES = ('x86_64', 'aarch64')
CONTAINERD_STORE = ['driver-type', 'io.containerd.snapshotter.v1']


class HostError(RuntimeError):
    pass


def require(ok, message):
    if not ok:
        raise HostError(message)


def capture(*command, timeout=60):
    result = subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, capture_output=True,
                            text=True, timeout=timeout, check=False)
    require(result.returncode == 0, f'`{" ".join(command[:3])}` failed: {result.stderr.strip()[-500:]}')
    return result.stdout.strip()


def hosted_runner():
    return os.environ.get('GITHUB_ACTIONS') == 'true'


def os_release():
    values = {}
    for line in Path('/etc/os-release').read_text().splitlines():
        key, separator, value = line.partition('=')
        if separator:
            values[key] = value.strip().strip('"')
    return values


def check_declared_host(release):
    require(os.environ.get(DECLARATION) == '1',
            f'Boot-level drivers change host configuration. Run them only on a disposable host, '
            f'declared with {DECLARATION}=1; never on a workstation')
    require(release.get('ID') == 'ubuntu' and release.get('VERSION_ID') == '24.04',
            'The disposable host must run Ubuntu 24.04, the image of the removed hosted runners')
    require(not Path('/run/ostree-booted').exists(),
            'Refusing a booted bootc/OSTree system; use a disposable Ubuntu 24.04 host')
    require(os.getuid() != 0, 'Run as an ordinary user with non-interactive sudo, like the runner account')
    require(shutil.which('sudo') is not None
            and subprocess.run(['sudo', '--non-interactive', 'true'], stdin=subprocess.DEVNULL,
                               capture_output=True, timeout=30, check=False).returncode == 0,
            'The driver needs non-interactive sudo for its disposable privileged steps')


def check_hosted_runner():
    require(os.environ.get('RUNNER_OS') == 'Linux' and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra'
            and os.environ.get('RUNNER_TEMP'), 'Unexpected hosted runner context')


def check_platform(architecture, kvm, commands, docker_containerd):
    require(sys.platform == 'linux', 'Boot-level drivers need a Linux host')
    machine = platform.machine()
    require(architecture is None or machine == architecture,
            f'This driver needs a native {architecture} host, not {machine}')
    if kvm:
        require(Path('/dev/kvm').exists() and stat.S_ISCHR(os.stat('/dev/kvm').st_mode),
                'This driver boots its VMs with KVM, and /dev/kvm is missing')
    missing = sorted(command for command in set(commands) if shutil.which(command) is None)
    require(not missing, 'Missing host commands: ' + ' '.join(missing)
            + '; install the Ubuntu packages that usr/src/kedra/tests/README.md lists for this driver')
    if docker_containerd:
        status = json.loads(capture('docker', 'info', '--format', '{{json .DriverStatus}}') or 'null')
        require(isinstance(status, list) and CONTAINERD_STORE in status,
                'Docker must use the containerd image store (the removed workflows enabled the '
                '"containerd-snapshotter" daemon feature); configure it before running this driver')


def committed_head():
    require(not capture('git', 'status', '--porcelain=v1', '--untracked-files=all'),
            'Commit or remove local changes first: evidence names one committed revision')
    return capture('git', 'rev-parse', '--verify', 'HEAD^{commit}')


def build_release():
    # Same build each removed workflow ran before its driver; its output goes to stderr
    # so that the bash shim reads only the context from stdout.
    result = subprocess.run(['cargo', 'build', '--workspace', '--release', '--locked'], cwd=ROOT,
                            stdin=subprocess.DEVNULL, stdout=sys.stderr, check=False)
    require(result.returncode == 0, 'cargo build --workspace --release --locked failed')


def private_directory():
    # An explicit RUNNER_TEMP selects the filesystem; every run gets a new directory in it.
    parent = Path(os.environ.get('RUNNER_TEMP') or '/var/tmp').resolve()
    require(parent.is_dir(), f'RUNNER_TEMP parent {parent} is not a directory')
    return Path(tempfile.mkdtemp(prefix='kedra-boot-', dir=parent))


def hosted_values():
    check_hosted_runner()
    return {'mode': 'actions', 'source_revision': os.environ['GITHUB_SHA'],
            'runner_temp': str(Path(os.environ['RUNNER_TEMP']).resolve()),
            'run_id': os.environ['GITHUB_RUN_ID'], 'run_attempt': os.environ['GITHUB_RUN_ATTEMPT']}


def host_values(build):
    head = committed_head()
    if build:
        build_release()
    return {'mode': 'disposable-host', 'source_revision': head, 'runner_temp': str(private_directory()),
            'run_id': str(int(time.time())), 'run_attempt': '1'}


def record(evidence, value, release):
    directory = ROOT / evidence
    require(not directory.exists() and not directory.is_symlink(),
            f'{evidence} already exists; move earlier evidence away before a new run')
    directory.mkdir(parents=True)
    document = {'schema_version': 1, **value, 'machine': platform.machine(), 'kernel': platform.release(),
                'operating_system': release.get('PRETTY_NAME'), 'kvm': Path('/dev/kvm').exists()}
    (directory / 'execution.json').write_text(json.dumps(document, sort_keys=True, indent=2) + '\n')


def context(*, evidence=None, architecture=None, kvm=False, commands=(), docker_containerd=False, build=True):
    """Validate the host, export the run variables for child scripts and return them."""
    require(Path.cwd().resolve() == ROOT, f'Run the driver from the repository root {ROOT}')
    release = os_release()
    if not hosted_runner():
        check_declared_host(release)
    tools = ('git', 'cargo') if build and not hosted_runner() else ('git',)
    check_platform(architecture, kvm, (*commands, *tools), docker_containerd)
    value = hosted_values() if hosted_runner() else host_values(build)
    if evidence is not None:
        record(evidence, value, release)
    os.environ.update(RUNNER_TEMP=value['runner_temp'], GITHUB_SHA=value['source_revision'],
                      GITHUB_RUN_ID=value['run_id'], GITHUB_RUN_ATTEMPT=value['run_attempt'])
    return value


def enter(**options):
    """context() for a driver's entry point: a refused host ends it with the reason."""
    try:
        return context(**options)
    except (HostError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        raise SystemExit('disposable host: ' + str(error)) from error


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--evidence', help='New evidence directory, relative to the repository root')
    parser.add_argument('--architecture', choices=ARCHITECTURES, help='Required native host architecture')
    parser.add_argument('--kvm', action='store_true', help='Require /dev/kvm')
    parser.add_argument('--command', action='append', default=[], dest='commands', help='Required host command')
    parser.add_argument('--docker-containerd', action='store_true', help='Require the Docker containerd image store')
    parser.add_argument('--no-build', action='store_true', help='Skip the workspace release build')
    args = parser.parse_args()
    value = enter(evidence=args.evidence, architecture=args.architecture, kvm=args.kvm, commands=args.commands,
                  docker_containerd=args.docker_containerd, build=not args.no_build)
    print(json.dumps(value, sort_keys=True))


if __name__ == '__main__':
    main()
