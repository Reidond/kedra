"""Generated bootc disks for the disposable VM drivers (imported, not run).

A driver passes the disposable-host guard first. Then it imports one candidate
image into rootful Podman, layers its test-only observer fixture, and builds a
QCOW2 disk with the pinned bootc-image-builder and a generated account
password. Everything is written below the driver's new work directory; the
password, blueprint and generated signing keys stay in its private 0700
directory and the builder log is kept only with password hashes redacted.

A Kedra candidate carries `enforce-container-sigpolicy`, so bootc refuses an
unsigned fixture. sign_fixture admits it as the native lab does
(tests/container/qemu/sign-fixture.sh): a generated key signs the fixture under
a unique name, and a separate buildroot adds exactly two exact-image-ID rules
for that key to the candidate's own policy, for the ordinary and the builder's
storage. The fixture's production trust is unchanged and nothing is disabled.
"""
import hashlib
import json
import os
import re
import secrets
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
INPUTS = ROOT / 'usr/src/kedra/image/inputs.json'
TOOLS = ('sudo', 'podman', 'skopeo', 'openssl')
FIXTURE_KEY = '/usr/share/kedra-research/fixture-signing.pub'
GRAPH_ROOT = '/var/lib/containers/storage'
# Policy scopes and transport sources by image ID, for rootful Podman's store
# and the copy bootc-image-builder installs from (osbuild's storage2).
SCOPES = {'ordinary': f'[overlay@{GRAPH_ROOT}]@{{id}}', 'bib': '[overlay@/run/osbuild/containers/storage2]@{id}'}
SOURCES = {
    'ordinary': f'containers-storage:[overlay@{GRAPH_ROOT}+/run/containers/storage]{{id}}',
    'bib': 'containers-storage:[overlay@/run/osbuild/containers/storage2+/run/containers/storage:'
           'additionalimagestore=/run/osbuild/containers/storage]{id}',
}
REFUSALS = {'unsigned': 'A signature was required, but no signature exists',
            'wrong': 'cryptographic signature verification failed'}


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


def fixture_policy(production, image_id, reference):
    """The candidate's policy plus exactly two exact-ID rules for the fixture key."""
    policy = json.loads(production)
    storage = policy.get('transports', {}).get('containers-storage', {})
    scopes = [scope.format(id=image_id) for scope in SCOPES.values()]
    require(policy.get('default') == [{'type': 'reject'}] and storage.get('')
            and not any(scope in storage for scope in scopes), 'Unexpected candidate signature policy')
    requirement = {'type': 'sigstoreSigned', 'keyPath': FIXTURE_KEY,
                   'signedIdentity': {'type': 'exactReference', 'dockerReference': reference}}
    for scope in scopes:
        storage[scope] = [requirement]
    without = json.loads(json.dumps(policy))
    for scope in scopes:
        del without['transports']['containers-storage'][scope]
    require(without == json.loads(production), 'Fixture policy changed more than its two rules')
    return policy


def generate_keys(private, evidence):
    keys = private / 'keys'
    keys.mkdir(mode=0o700)
    (keys / 'passphrase').write_text(secrets.token_urlsafe(32))
    (keys / 'passphrase').chmod(0o600)
    for name in ('allowed', 'wrong'):
        run(['skopeo', 'generate-sigstore-key', '--output-prefix', keys / name,
             '--passphrase-file', keys / 'passphrase'], output=evidence / f'key-{name}.log')
    return keys


def remove_keys(keys):
    for path in keys.iterdir():
        path.unlink()
    keys.rmdir()


def build_buildroot(work, reference, policy, keys, token):
    """FROM the fixture: its fixture public key and the two-rule policy, nothing else."""
    context = work / 'buildroot'
    context.mkdir(mode=0o700)
    shutil.copyfile(keys / 'allowed.pub', context / 'fixture.pub')
    shutil.copyfile(keys / 'allowed.pub', work / 'evidence' / 'fixture-signing.pub')
    (context / 'policy.json').write_text(json.dumps(policy, indent=2, sort_keys=True) + '\n')
    (context / 'Containerfile').write_text(f'FROM {reference}\nCOPY fixture.pub {FIXTURE_KEY}\n'
                                           'COPY policy.json /etc/containers/policy.json\n')
    podman('build', '--pull=never', '--network=none', '--iidfile', context / 'image.id',
           '--tag', f'localhost/kedra-vm-buildroot:{token}', context, output=work / 'evidence' / 'buildroot-build.log')
    image_id = (context / 'image.id').read_text().strip().removeprefix('sha256:')
    digest = podman('image', 'inspect', image_id, '--format', '{{.Digest}}', capture=True).strip()
    require(re.fullmatch('sha256:[a-f0-9]{64}', digest), 'Buildroot has no manifest digest')
    reference = 'localhost/kedra-vm-buildroot@' + digest
    require(podman('image', 'inspect', reference, '--format', '{{.Id}}', capture=True).strip() == image_id,
            'Buildroot reference resolves to another image')
    installed = podman('run', '--rm', '--pull=never', '--network', 'none', '--entrypoint', '/usr/bin/cat',
                       image_id, '/etc/containers/policy.json', capture=True)
    require(json.loads(installed) == policy, 'Buildroot policy differs from the fixture policy')
    return image_id, reference


def check_admission(work, buildroot, phase, scope, image_id):
    """An independent consumer with the buildroot's default policy: refuse or admit."""
    checks = work / 'private' / 'signature-check'
    checks.mkdir(mode=0o700, exist_ok=True)
    result = subprocess.run(['sudo', 'podman', 'run', '--rm', '--pull=never', '--privileged', '--network', 'none',
                             '--read-only', '--security-opt', 'label=type:unconfined_t',
                             '--tmpfs', '/run:rw,nosuid,nodev,mode=0755', '--tmpfs', '/tmp:rw,nosuid,nodev,noexec,size=64m',
                             '-v', f'{GRAPH_ROOT}:{GRAPH_ROOT}', '-v', f'{GRAPH_ROOT}:/run/osbuild/containers/storage',
                             '-v', f'{checks}:/verification', '--entrypoint', '/usr/bin/skopeo', buildroot,
                             'copy', '--preserve-digests', SOURCES[scope].format(id=image_id), f'dir:/verification/{scope}'],
                            stdin=subprocess.DEVNULL, capture_output=True, timeout=1800, check=False)
    stderr = result.stderr.decode(errors='replace')
    (work / 'evidence' / f'verify-{phase}-{scope}.txt').write_text(f'exit {result.returncode}\n{stderr}')
    if phase != 'allowed':
        require(result.returncode != 0 and REFUSALS[phase] in stderr, f'{phase} fixture was not refused in {scope} storage')
        return
    require(result.returncode == 0, f'Signed fixture was refused in {scope} storage')
    # Written by the root consumer; read it with the same privilege.
    manifest = subprocess.run(['sudo', 'cat', '--', checks / scope / 'manifest.json'], stdin=subprocess.DEVNULL,
                              capture_output=True, check=True, timeout=60).stdout
    expected = (work / 'evidence' / 'allowed.digest').read_text().strip()
    require('sha256:' + hashlib.sha256(manifest).hexdigest() == expected, 'Admitted manifest differs from the signed one')
    run(['sudo', 'rm', '-r', '--one-file-system', '--', checks / scope])


def sign_fixture(work, fixture):
    """Admit the observer fixture through a generated key; return its boot and buildroot references."""
    evidence = work / 'evidence'
    store = podman('info', '--format', '{{.Store.GraphRoot}} {{.Store.GraphDriverName}}', capture=True).split()
    require(store == [GRAPH_ROOT, 'overlay'], 'The pinned builder needs default rootful overlay Podman storage')
    image_id = podman('image', 'inspect', fixture, '--format', '{{.Id}}', capture=True).strip()
    require(re.fullmatch('[a-f0-9]{64}', image_id), 'Fixture has no image ID')
    token = secrets.token_hex(16)
    reference = f'localhost/kedra-vm-fixture/{token}:boot'
    podman('tag', image_id, reference)
    production = podman('run', '--rm', '--pull=never', '--network', 'none', '--entrypoint', '/usr/bin/cat',
                        image_id, '/etc/containers/policy.json', capture=True)
    (evidence / 'production-policy.json').write_text(production)
    policy = fixture_policy(production, image_id, reference)
    keys = generate_keys(work / 'private', evidence)
    try:
        buildroot_id, buildroot = build_buildroot(work, reference, policy, keys, token)
        for phase in ('unsigned', 'wrong', 'allowed'):
            if phase != 'unsigned':
                podman('push', '--remove-signatures', '--sign-by-sigstore-private-key', keys / f'{phase}.private',
                       '--sign-passphrase-file', keys / 'passphrase', '--digestfile', evidence / f'{phase}.digest',
                       image_id, f'containers-storage:{reference}', output=evidence / f'sign-{phase}.log')
            require(podman('image', 'inspect', reference, '--format', '{{.Id}}', capture=True).strip() == image_id,
                    'Signing changed the fixture image')
            for scope in SCOPES:
                check_admission(work, buildroot_id, phase, scope, image_id)
    finally:
        # Private keys never reach the builder; signatures stay in the store it reads.
        remove_keys(keys)
    digest = (evidence / 'allowed.digest').read_text().strip()
    boot = reference.rsplit(':', 1)[0] + '@' + digest
    require(podman('image', 'inspect', boot, '--format', '{{.Id}}', capture=True).strip() == image_id,
            'Signed boot reference resolves to another image')
    (evidence / 'signing.json').write_text(json.dumps({
        'schema_version': 1, 'signed_reference': reference, 'boot_reference': boot, 'signed_manifest_digest': digest,
        'target_storage_id': image_id, 'buildroot_reference': buildroot, 'key_path': FIXTURE_KEY,
        'scopes': [scope.format(id=image_id) for scope in SCOPES.values()],
        'admission': {'unsigned': 'refused', 'wrong_key_only': 'refused', 'allowed_key_only': 'pass'},
        'consumer_policy': 'default', 'target_production_trust': 'unchanged', 'private_keys_removed': True,
    }, indent=2, sort_keys=True) + '\n')
    return boot, buildroot, digest


def build_disk(platform, image, work, build_container=None):
    """The pinned bootc-image-builder's QCOW2 disk of the (signed) fixture image."""
    builder = json.loads(INPUTS.read_bytes())['platforms'][platform]['builder']
    require(re.fullmatch(r'quay\.io/centos-bootc/bootc-image-builder@sha256:[a-f0-9]{64}', builder),
            'Unexpected pinned builder')
    log = work / 'private' / 'builder.log'
    podman('pull', builder, output=log)
    podman('run', '--rm', '--privileged', '--security-opt', 'label=type:unconfined_t',
           '-v', f'{work / "image"}:/output', '-v', f'{work / "private" / "blueprint.toml"}:/config.toml:ro',
           '-v', '/var/lib/containers/storage:/var/lib/containers/storage',
           builder, '--type', 'qcow2', '--rootfs', 'ext4', '--use-librepo=True',
           *(('--build-container', build_container) if build_container else ()), image, output=log)
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
    if (private / 'keys').exists():
        remove_keys(private / 'keys')
