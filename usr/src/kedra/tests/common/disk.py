"""Generated bootc disks for the disposable VM drivers (imported, not run).

A driver passes the disposable-host guard first. Then it imports one candidate
image into rootful Podman, layers its test-only observer fixture, and builds a
QCOW2 disk with the pinned bootc-image-builder and a generated account
password. Everything is written below the driver's new work directory; the
password and blueprint stay in its private 0700 directory and the builder log is
kept only with password hashes redacted.
"""
import json
import os
import re
import secrets
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
INPUTS = ROOT / 'usr/src/kedra/image/inputs.json'
TOOLS = ('sudo', 'podman', 'openssl')


class DiskError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise DiskError(message)


def run(argv, *, output=None, timeout=7200, capture=False):
    """Run with explicit arguments; output goes to a log file or is returned."""
    with (open(output, 'ab') if output else open(os.devnull, 'wb')) as log:
        result = subprocess.run([str(item) for item in argv], stdin=subprocess.DEVNULL, timeout=timeout,
                                stdout=subprocess.PIPE if capture else log, stderr=log, check=False)
    require(result.returncode == 0, f'{argv[0]} {argv[1] if len(argv) > 1 else ""} failed'
            + (f'; see {output}' if output else ''))
    return result.stdout.decode() if capture else ''


def podman(*argv, **kwargs):
    return run(['sudo', 'podman', *argv], **kwargs)


def new_work(path):
    """The driver's new private work tree: evidence/ for results, private/ for secrets."""
    work = path.absolute()
    require(not work.exists() and not work.is_symlink() and work.parent.is_dir(),
            '--work must name a new directory under an existing parent')
    require(not any(character in str(work) for character in ':,\r\n'), 'Unsafe --work path for volume mounts')
    work.mkdir(mode=0o700)
    for name in ('evidence', 'private', 'context', 'image'):
        (work / name).mkdir(mode=0o700)
    return work


def require_tools(names):
    missing = [name for name in (*TOOLS, *names) if shutil.which(name) is None]
    require(not missing, 'Install the documented host tools first: ' + ', '.join(missing))


def import_candidate(candidate, tag, evidence):
    """Tag one candidate in rootful Podman: docker:REF, podman:REF or a registry REF."""
    kind, _, reference = candidate.partition(':')
    if kind == 'docker' and reference:
        archive = evidence.parent / 'private' / 'candidate.tar'
        run(['docker', 'image', 'save', '--output', archive, reference], output=evidence / 'candidate-import.log')
        loaded = podman('load', '--input', archive, capture=True, output=evidence / 'candidate-import.log')
        archive.unlink()
        match = re.search(r'Loaded image(?:\(s\))?: (\S+)', loaded)
        require(match is not None, 'Podman did not report the loaded candidate')
        source = match[1]
    elif kind == 'podman' and reference:
        source = reference
    else:
        podman('pull', candidate, output=evidence / 'candidate-import.log')
        source = candidate
    podman('tag', source, tag, output=evidence / 'candidate-import.log')
    inspect = podman('image', 'inspect', tag, capture=True)
    (evidence / 'candidate-inspect.json').write_text(inspect)
    return json.loads(inspect)[0]


def build_fixture(context, files, tag, evidence):
    """Layer the observer fixture (a Containerfile in files) over the tagged candidate."""
    for name, source in files.items():
        shutil.copyfile(source, context / name)
    podman('build', '--pull=never', '--tag', tag, context, output=evidence / 'fixture-build.log')
    return podman('image', 'inspect', tag, '--format', '{{.Digest}}', capture=True).strip()


def blueprint(private, *, keep_password=True):
    """A generated disposable account password and its hashed builder blueprint.

    Headless drivers never need the password; it is then never written down.
    """
    password = secrets.token_hex(16)
    if keep_password:
        (private / 'password').write_text(password)
        (private / 'password').chmod(0o600)
    hashed = subprocess.run(['openssl', 'passwd', '-6', '-stdin'], input=password.encode(), check=True,
                            capture_output=True, timeout=30).stdout.decode().strip()
    (private / 'blueprint.toml').write_text(
        '[[customizations.user]]\nname = "kedra-test"\npassword = ' + json.dumps(hashed) + '\ngroups = ["wheel"]\n')
    (private / 'blueprint.toml').chmod(0o600)
    return private / 'password' if keep_password else None


def build_disk(platform, image, work):
    """The pinned bootc-image-builder's QCOW2 disk of the fixture image."""
    builder = json.loads(INPUTS.read_bytes())['platforms'][platform]['builder']
    require(re.fullmatch(r'quay\.io/centos-bootc/bootc-image-builder@sha256:[a-f0-9]{64}', builder),
            'Unexpected pinned builder')
    log = work / 'private' / 'builder.log'
    podman('pull', builder, output=log)
    podman('run', '--rm', '--privileged', '--security-opt', 'label=type:unconfined_t',
           '-v', f'{work / "image"}:/output', '-v', f'{work / "private" / "blueprint.toml"}:/config.toml:ro',
           '-v', '/var/lib/containers/storage:/var/lib/containers/storage',
           builder, '--type', 'qcow2', '--rootfs', 'ext4', '--use-librepo=True', image, output=log)
    disks = sorted((work / 'image').rglob('*.qcow2'))
    require(len(disks) == 1, 'Expected one generated QCOW2 disk')
    # The emulator needs no privilege: hand only the generated disk to the invoking user.
    run(['sudo', 'chown', '-R', f'{os.getuid()}:{os.getgid()}', work / 'image'])
    return disks[0]


def sanitize(work):
    """Keep the builder log without password hashes; remove the generated secret."""
    private = work / 'private'
    log = private / 'builder.log'
    if log.exists():
        text = re.sub(r'\$6\$[^\s"\']+', '<synthetic-password-hash-redacted>', log.read_text(errors='replace'))
        (work / 'evidence' / 'qcow2-build.log').write_text(text)
    for name in ('password', 'blueprint.toml', 'builder.log', 'candidate.tar'):
        (private / name).unlink(missing_ok=True)
