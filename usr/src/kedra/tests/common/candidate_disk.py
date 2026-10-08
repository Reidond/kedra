"""Shared steps of the candidate-disk boot drivers, vm/desktop/run.py and vm/qemu-arm64/run.py.

Each step is one inline step of the removed test-desktop.yml or test-qemu-arm64.yml
(at 3e33867), with the same commands, files and checks. Host scripts import this
module; it runs under their uv interpreter.
"""
import json
import os
import re
import secrets
import shutil
import subprocess
from pathlib import Path

import prepare_inputs

ROOT = Path(__file__).resolve().parents[5]
SYSROOT = ROOT / 'target/release/sysroot'
IMAGE = ROOT / 'usr/src/kedra/image'
PODMAN = ['sudo', 'podman']
# Removed from every builder log before it enters evidence.
PASSWORD_HASH = re.compile(r'\$6\$[^\s"\']+')


def run(command, *, stdout=None, stderr=None):
    subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, check=True)


def output(command):
    return subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, text=True,
                          check=True).stdout


def to_file(path, command, *, combined=False):
    """Run a command with its standard output, and optionally its errors, in one evidence file."""
    with path.open('w') as stream:
        run(command, stdout=stream, stderr=subprocess.STDOUT if combined else None)


def environment(path, commands):
    with path.open('w') as stream:
        for command in commands:
            run(command, stdout=stream)


def prepare_payload(target, context, evidence):
    """Committed payload, binaries, release build definition and pinned inputs."""
    context.mkdir(parents=True)
    to_file(evidence / 'source.json', [str(SYSROOT), 'source', 'plan', '--host', target, '--json'])
    run([str(SYSROOT), 'source', 'archive', '--host', target, '--output', str(context / 'payload.tar')])
    for source in (SYSROOT, SYSROOT.with_name('sysroot-helper'), IMAGE / 'Containerfile', IMAGE / 'assemble.sh'):
        shutil.copy2(source, context / source.name)
    shutil.copyfile(IMAGE / 'inputs.json', evidence / 'inputs.json')
    prepare_inputs.prepare(target, context, evidence)
    to_file(evidence / 'build-input-hashes.txt', ['sha256sum', *map(str, sorted(context.iterdir()))])
    return json.loads((evidence / 'source.json').read_text())


def resolve_base(evidence, architecture):
    return output(['uv', 'run', 'usr/src/kedra/tests/common/resolve-fedora-base.py', '--architecture', architecture,
                   '--output', str(evidence / 'base-resolution.json')]).strip()


def record_candidate(tag, evidence):
    to_file(evidence / 'image-inspect.json', [*PODMAN, 'image', 'inspect', tag])
    to_file(evidence / 'packages.txt', [*PODMAN, 'run', '--rm', tag, 'cat', '/usr/share/sysroot/packages.txt'])
    to_file(evidence / 'image-var-files.txt', [*PODMAN, 'run', '--rm', tag, 'find', '/var', '-type', 'f'])


def build_observer(files, tag, context, evidence):
    """The non-promotable test layer over the candidate."""
    context.mkdir(parents=True)
    for source in files:
        shutil.copyfile(source, context / source.name)
    to_file(evidence / 'test-image-build.log', [*PODMAN, 'build', '--pull=never', '--tag', tag, str(context)],
            combined=True)


def private_directory(path):
    path.mkdir(mode=0o700)
    return path


def write_private(path, text):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as stream:
        stream.write(text)


def write_blueprint(private, keep_password):
    """Generated disposable account; the password file is kept only for a driver that types it."""
    password = secrets.token_hex(16)
    if keep_password:
        write_private(private / 'password', password)
    hashed = subprocess.check_output(['openssl', 'passwd', '-6', '-stdin'], input=password.encode()).decode().strip()
    write_private(private / 'blueprint.toml', '[[customizations.user]]\nname = "kedra-test"\npassword = '
                  + json.dumps(hashed) + '\ngroups = ["wheel"]\n')


def build_disk(builder, tag, image, private):
    image.mkdir(parents=True)
    to_file(private / 'builder.log', [
        *PODMAN, 'run', '--rm', '--privileged', '--security-opt', 'label=type:unconfined_t',
        '--volume', f'{image}:/output', '--volume', f'{private / "blueprint.toml"}:/config.toml:ro',
        '--volume', '/var/lib/containers/storage:/var/lib/containers/storage',
        builder, '--type', 'qcow2', '--rootfs', 'ext4', '--use-librepo=True', tag], combined=True)


def single_disk(image):
    disks = sorted(path for path in image.rglob('*.qcow2') if path.is_file())
    if len(disks) != 1:
        raise RuntimeError(f'Expected exactly one generated QCOW2 disk, found {len(disks)}')
    return disks[0]


def bounded(command, path, timeout):
    """Run a VM step with its output in an evidence file, then show that output."""
    with path.open('w') as stream:
        process = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=stream)
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.terminate()
            process.wait(timeout=60)
            raise RuntimeError(f'{command[0]} exceeded its {timeout} s bound') from None
    print(path.read_text(), end='', flush=True)
    if code:
        raise subprocess.CalledProcessError(code, command)


def sanitize(private, evidence, names):
    """Keep a redacted builder log as evidence and remove the disposable account secrets."""
    log = private / 'builder.log'
    if log.exists():
        text = PASSWORD_HASH.sub('<synthetic-password-hash-redacted>', log.read_text(errors='replace'))
        (evidence / 'qcow2-build.log').write_text(text)
    for name in names:
        (private / name).unlink(missing_ok=True)
