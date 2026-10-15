#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Signed GHCR update workflow on an Apple Silicon Mac under HVF.

The qemu-arm64 counterpart of run.sh. A disposable privileged controller
container in the local Docker engine builds the small Fedora bootc fixture with
the given Linux sysroot binaries, signs its variants with generated keys into a
disposable TLS registry published on 127.0.0.1:443, and installs signed A to a
new disk with bootc-image-builder, which runs A's own bootc and policy. The kedra-lab QEMU runtime then boots that disk
three times under HVF and UEFI Secure Boot; the guest reaches the registry as
ghcr.io at 10.0.2.2. Nothing is installed on the Mac.

  uv run usr/src/kedra/tests/vm/ghcr-update/run-hvf.py --binaries DIR --work NEW_DIRECTORY

DIR holds Linux aarch64 `sysroot` and `sysroot-helper` built from this commit.
Prepare the runtime first with `kedra-lab vm tools prepare`.
"""
import argparse
import json
import os
import platform
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
sys.path.insert(0, str(ROOT / 'usr/src/kedra/tests/common'))
import qemu

REGISTRY = 'docker.io/library/registry@sha256:3ffcae348822784850e836f23449ff1d0503933524cef57ddcbdbd263eca0c52'
BASE_TAG = 'quay.io/fedora/fedora-bootc:44'
OWNER = 'dev.kedra.lab.owner=kedra-ghcr-hvf'
VARIANTS = ['A', 'B', 'C', 'D', 'E', 'U', 'W', 'R', 'N', 'T', 'X', 'H', 'M']
CONSOLE = 'kargs = ["console=ttyAMA0,115200"]\nmatch-architectures = ["aarch64"]\n'
# bootc-image-builder appends console=tty0 console=ttyS0, so /dev/console is the absent
# ttyS0 on QEMU virt: the guest workflow writes to the PL011 the host reads instead,
# before the serial getty starts there (its vhangup would cut the workflow's output).
SERIAL_ORDER = '[Unit]\nBefore=serial-getty@ttyAMA0.service\n'
SERIAL_OUTPUT = '[Service]\nStandardOutput=tty\nStandardError=tty\nTTYPath=/dev/ttyAMA0\n'
FIXTURE_FILES = ('Containerfile', 'variant.Containerfile', 'check.py', 'check.service', 'identity-recovery.py')


def run(*argv, check=True, timeout=3600, **options):
    return subprocess.run([str(item) for item in argv], check=check, timeout=timeout, **options)


def capture(*argv):
    return run(*argv, capture_output=True, text=True).stdout.strip()


def require(condition, message):
    if not condition:
        raise SystemExit('run-hvf: ' + message)


def admit(args):
    require(sys.platform == 'darwin' and platform.machine() == 'arm64', 'needs an Apple Silicon Mac (HVF)')
    for name in ('sysroot', 'sysroot-helper'):
        header = (args.binaries / name).read_bytes()[:20]
        require(header[:6] == b'\x7fELF\x02\x01' and int.from_bytes(header[18:20], 'little') == 183,
                name + ' must be a Linux aarch64 ELF binary')
    for path in ('Kedra QEMU.app/Contents/MacOS/qemu-system-aarch64', 'firmware/AAVMF_CODE.secboot.fd',
                 'firmware/AAVMF_VARS.ms.fd'):
        require((args.runtime / path).is_file(), 'prepare the runtime first (kedra-lab vm tools prepare): ' + path)
    run('docker', 'info', stdout=subprocess.DEVNULL)
    require(run('docker', 'container', 'inspect', 'kedra-ghcr-registry', check=False, capture_output=True).returncode != 0,
            'a kedra-ghcr-registry container already exists')
    for port in (443, 18080):
        with socket.socket() as probe:
            require(probe.connect_ex(('127.0.0.1', port)) != 0, f'127.0.0.1:{port} is already in use')
    require(not args.work.exists(), '--work must name a new directory')


class Fixture:
    """Disposable Docker resources; each is removed on exit."""

    def __init__(self, args):
        self.work = args.work.resolve()
        self.root, self.private, self.evidence = (self.work / name for name in ('kedra-ghcr', 'kedra-ghcr-private', 'evidence'))
        token = secrets.token_hex(6)
        self.network, self.volume, self.controller = (f'kedra-ghcr-hvf-{token}', f'kedra-ghcr-hvf-{token}',
                                                       f'kedra-ghcr-hvf-controller-{token}')
        self.started = []
        self.control = None

    def stage(self, *arguments):
        run('docker', 'exec', self.controller, 'bash', '/fixture/hvf-build.sh', *arguments)

    def start(self, base):
        run('docker', 'network', 'create', '--label', OWNER, self.network, stdout=subprocess.DEVNULL)
        self.started.append(('network', self.network))
        run('docker', 'volume', 'create', '--label', OWNER, self.volume, stdout=subprocess.DEVNULL)
        self.started.append(('volume', self.volume))
        run('docker', 'run', '-d', '--name', self.controller, '--label', OWNER, '--privileged', '--cgroupns', 'private',
            '--network', self.network, '--platform', 'linux/arm64',
            '-v', f'{self.root}:/fixture', '-v', f'{self.private}:/private', '-v', f'{self.evidence}:/evidence',
            '--mount', f'type=volume,source={self.volume},target=/var/lib/containers',
            '--entrypoint', 'sleep', base, 'infinity', stdout=subprocess.DEVNULL)
        self.started.append(('container', self.controller))
        self.stage('setup')

    def registry(self):
        run('docker', 'run', '-d', '--name', 'kedra-ghcr-registry', '--label', OWNER, '--network', self.network,
            '--network-alias', 'ghcr.io', '-p', '127.0.0.1:443:443',
            '-v', f'{self.root}/context/tls.crt:/certs/tls.crt:ro', '-v', f'{self.private}/tls.key:/certs/tls.key:ro',
            '-e', 'REGISTRY_HTTP_ADDR=:443', '-e', 'REGISTRY_HTTP_TLS_CERTIFICATE=/certs/tls.crt',
            '-e', 'REGISTRY_HTTP_TLS_KEY=/certs/tls.key', '-e', 'REGISTRY_STORAGE_DELETE_ENABLED=true',
            '-e', 'OTEL_TRACES_EXPORTER=none', REGISTRY, stdout=subprocess.DEVNULL)
        self.started.append(('container', 'kedra-ghcr-registry'))
        for _ in range(30):
            if run('curl', '--silent', '--fail', '--cacert', self.root / 'context/tls.crt', 'https://127.0.0.1/v2/',
                   check=False, stdout=subprocess.DEVNULL).returncode == 0:
                return
            time.sleep(1)
        raise SystemExit('run-hvf: the local registry did not start')

    def cleanup(self, keep=False):
        if self.control is not None:
            self.control.terminate()
            self.control.wait(timeout=30)
        for kind, name in reversed(self.started):
            if name == 'kedra-ghcr-registry':
                with (self.evidence / 'registry.log').open('wb') as log:
                    run('docker', 'logs', name, check=False, stdout=log, stderr=subprocess.STDOUT)
            if kind == 'container' and name == self.controller:
                # Root-owned files the controller wrote stay removable by the owner.
                run('docker', 'exec', name, 'chmod', '-R', 'u+rwX,go+rX', '/fixture', '/evidence', check=False)
            command = {'container': ('rm', '-f'), 'volume': ('volume', 'rm'), 'network': ('network', 'rm')}[kind]
            run('docker', *command, name, check=False, stdout=subprocess.DEVNULL)
        if not keep:
            shutil.rmtree(self.private, ignore_errors=True)


def resolve_base(fixture):
    run('docker', 'pull', '--platform', 'linux/arm64', BASE_TAG, stdout=subprocess.DEVNULL)
    base = capture('docker', 'image', 'inspect', '--format', '{{index .RepoDigests 0}}', BASE_TAG)
    require(base.startswith('quay.io/fedora/fedora-bootc@sha256:'), 'unexpected base digest ' + base)
    (fixture.root / 'base.txt').write_text(base + '\n')
    return base


def prepare(fixture, args):
    for directory in (fixture.root / 'context', fixture.root / 'cases', fixture.private, fixture.evidence):
        directory.mkdir(mode=0o700, parents=True)
    shutil.copy2(HERE / 'hvf-build.sh', fixture.root / 'hvf-build.sh')
    base = resolve_base(fixture)
    fixture.start(base)
    fixture.stage('keys')
    revision = capture('git', '-C', ROOT, 'rev-parse', '--verify', 'HEAD^{commit}')
    run('uv', 'run', HERE / 'prepare.py', '--root', fixture.root, '--source-revision', revision,
        '--target', 'qemu-arm64', '--binaries', args.binaries)
    context = fixture.root / 'context'
    for name in FIXTURE_FILES:
        shutil.copy2(HERE / name, context / name)
    shutil.copy2(ROOT / 'usr/src/kedra/tests/common/secure_boot_check.py', context / 'secure_boot_check.py')
    (context / 'console.toml').write_text(CONSOLE)
    unit = (context / 'check.service').read_text().replace('StandardOutput=journal+console\nStandardError=journal+console\n', '')
    unit = unit.replace('[Unit]\n', SERIAL_ORDER, 1).replace('[Service]\n', SERIAL_OUTPUT, 1)
    (context / 'check.service').write_text(unit)
    shutil.copy2(args.binaries / 'sysroot', context / 'sysroot')
    shutil.copy2(args.binaries / 'sysroot-helper', context / 'helper')
    fixture.registry()
    fixture.stage('base', base)
    run('uv', 'run', HERE / 'prepare.py', '--root', fixture.root, 'finalize')
    fixture.stage('variants')
    digests = {variant: (fixture.root / (variant + '.digest')).read_text().strip() for variant in VARIANTS}
    cases = {'schema_version': 1, 'target': 'qemu-arm64', 'require_fresh_installation': False, 'digests': digests}
    for path in (fixture.root / 'cases/cases.json', fixture.evidence / 'cases.json'):
        path.write_text(json.dumps(cases, indent=2) + '\n')
    builder = json.loads((ROOT / 'usr/src/kedra/image/inputs.json').read_text())['platforms']['arm64']['builder']
    fixture.stage('disk', builder)


def boot(fixture, args, phase):
    work = fixture.root / ('vm-' + phase)
    work.mkdir(mode=0o700)
    # macOS limits Unix socket paths to 104 bytes; the QMP socket gets a short private directory.
    sockets = Path(tempfile.mkdtemp(prefix='kedra-ghcr-', dir='/tmp'))
    serial = fixture.evidence / (phase + '.serial.log')
    argv = [args.runtime / 'Kedra QEMU.app/Contents/MacOS/qemu-system-aarch64', *qemu.machine_argv(
        'aarch64', 'hvf', code=args.runtime / 'firmware/AAVMF_CODE.secboot.fd', variables=fixture.root / 'vars.fd',
        qmp=sockets / 'qmp.sock', serial=serial, cpus=4, memory_mib=4096)]
    argv += qemu.disk_argv('system', fixture.root / 'disk.qcow2')
    argv += qemu.disk_argv('cases', fixture.root / 'cases.raw', image_format='raw', readonly=True)
    argv += [*qemu.network_argv(True), '-display', 'none']
    started = time.monotonic()
    environment = {**os.environ, 'DYLD_FALLBACK_LIBRARY_PATH': str(args.runtime / 'lib')}
    process = qemu.launch(argv, work / 'qemu.log', env=environment)
    try:
        qemu.wait_for_socket(sockets / 'qmp.sock', process)
        qemu.watch(process, sockets / 'qmp.sock', [serial], deadline=started + 3600,
                   required=['KEDRA_SECUREBOOT_PASS', f'KEDRA_GHCR_{phase}_PASS'], failures=['KEDRA_GHCR_FAIL'])
    finally:
        qemu.stop(process)
        shutil.rmtree(sockets, ignore_errors=True)
    result = {'schema_version': 1, 'phase': phase, 'accelerator': 'hvf', 'elapsed_seconds': round(time.monotonic() - started, 1)}
    (fixture.evidence / (phase + '-result.json')).write_text(json.dumps(result, sort_keys=True) + '\n')
    print(json.dumps(result, sort_keys=True), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--binaries', type=Path, required=True, help='Directory with Linux aarch64 sysroot and sysroot-helper')
    parser.add_argument('--work', type=Path, required=True, help='New directory for fixture, disk and evidence')
    parser.add_argument('--runtime', type=Path, default=Path.home() / 'Library/Caches/kedra/qemu/runtime')
    parser.add_argument('--keep-on-failure', action='store_true',
                        help='After a failure keep the Docker resources and generated keys for inspection')
    args = parser.parse_args()
    args.binaries, args.runtime = args.binaries.resolve(), args.runtime.resolve()
    admit(args)
    args.work.mkdir(mode=0o700)
    fixture = Fixture(args)
    try:
        prepare(fixture, args)
        shutil.copy2(args.runtime / 'firmware/AAVMF_VARS.ms.fd', fixture.root / 'vars.fd')
        run('uv', 'run', '--project', ROOT / 'usr/src/kedra/tests/container/qemu/build-tools', '--locked', 'virt-fw-vars',
            '--inplace', fixture.root / 'vars.fd', '--append-boot-filepath', qemu.AARCH64_SHIM, stdout=subprocess.DEVNULL)
        with (fixture.evidence / 'controller.log').open('wb') as log:
            fixture.control = subprocess.Popen(['uv', 'run', str(HERE / 'control.py'), '--root', str(fixture.root),
                                                '--engine', 'docker'], stdout=log, stderr=subprocess.STDOUT)
        time.sleep(2)
        run('curl', '--silent', '--show-error', '--fail', '-X', 'POST', 'http://127.0.0.1:18080/A', stdout=subprocess.DEVNULL)
        for phase in ('A', 'B', 'ROLLBACK'):
            boot(fixture, args, phase)
        (fixture.evidence / 'scope.json').write_text(json.dumps({
            'schema_version': 1, 'target': 'qemu-arm64', 'accelerator': 'hvf', 'boots': ['A', 'B', 'A'],
            'authority': 'generated fixture keys only', 'production_keys_used': False,
            'fixture': 'Fedora bootc 44 base with the tested sysroot binaries, not the full qemu-arm64 image',
        }, indent=2) + '\n')
        print('PASS: signed A/B update, refusals and rollback hold under HVF; evidence in', fixture.evidence)
    except BaseException:
        if args.keep_on_failure:
            print('Kept for inspection with the generated keys; remove with docker rm/volume rm/network rm:',
                  fixture.started, file=sys.stderr)
            fixture.started.clear()
        fixture.cleanup(keep=args.keep_on_failure)
        raise
    fixture.cleanup()


if __name__ == '__main__':
    main()
