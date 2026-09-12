#!/usr/bin/env python3
"""Build one local Kedra ISO from an explicitly reviewed, signed GHCR digest."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = 'ghcr.io/reidond/kedra-desktop'
FINGERPRINT = 'a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e'


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def json_file(path):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 4 * 1024**2,
            'Missing, unsafe or oversized JSON input: ' + str(path))
    return json.loads(path.read_bytes())


def identity(path):
    with path.open('rb') as stream:
        return {'size_bytes': path.stat().st_size,
                'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True, help='Reviewed ghcr.io/reidond/kedra-desktop@sha256:...')
    parser.add_argument('--output-dir', required=True, type=Path, help='New local directory; never overwrite existing output')
    parser.add_argument('--base-image', help='Explicit reviewed Fedora base digest; needed for legacy images without resolved inputs')
    parser.add_argument('--smoke', action='store_true', help='Also run diskless offline verification/Anaconda startup (requires KVM)')
    args = parser.parse_args()
    require(sys.platform == 'linux' and os.getuid() != 0 and os.geteuid() == os.getuid(),
            'Run as an ordinary user on a Linux build host; sudo is requested for container tools')
    require(re.fullmatch(re.escape(REPOSITORY) + r'@sha256:[a-f0-9]{64}', args.image),
            'An exact reviewed Kedra desktop GHCR digest is required, not a mutable tag')
    base_pattern = r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}'
    if args.base_image:
        require(re.fullmatch(base_pattern, args.base_image), 'Invalid reviewed Fedora base digest')
    names = ['sudo', 'podman', 'skopeo', 'openssl'] + (['xorriso', 'qemu-system-x86_64'] if args.smoke else [])
    tools = {name: shutil.which(name, path='/usr/sbin:/usr/bin:/sbin:/bin') for name in names}
    require(all(tools.values()), 'Install the documented local build prerequisites first: ' + ', '.join(names))
    if args.smoke:
        require(os.access('/dev/kvm', os.R_OK | os.W_OK), '--smoke requires KVM access for the invoking user')
    output = args.output_dir.absolute()
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(),
            '--output-dir must name a new directory under an existing parent')
    require(not any(char in str(output) + str(ROOT) for char in ':,\n\r'),
            'Local build paths must not contain container-volume separators or newlines')
    pins = json_file(ROOT / 'installer/inputs.json')
    builder = pins['builder']
    require(pins['architecture'] == 'amd64' and re.fullmatch(
        r'ghcr\.io/osbuild/image-builder@sha256:[a-f0-9]{64}', builder), 'Unexpected pinned builder scope')
    environment = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C.UTF-8',
                   'HOME': os.environ.get('HOME', str(Path.home()))}

    def run(arguments, *, capture=False, timeout=7200):
        result = subprocess.run([str(item) for item in arguments], check=False,
                                stdin=subprocess.DEVNULL, env=environment,
                                stdout=subprocess.PIPE if capture else None, timeout=timeout)
        require(result.returncode == 0, 'Command failed: ' + str(arguments[0]))
        return result.stdout if capture else b''

    # Check the fixed public authority before downloading or running image code.
    public = ROOT / 'build/release/authority/desktop.pub'
    der = run([tools['openssl'], 'pkey', '-pubin', '-in', public, '-outform', 'DER'], capture=True, timeout=30)
    require(hashlib.sha256(der).hexdigest() == FINGERPRINT, 'Public authority fingerprint differs')
    graphroot = run([tools['sudo'], tools['podman'], 'info', '--format', '{{.Store.GraphRoot}}'], capture=True).decode().strip()
    require(graphroot == '/var/lib/containers/storage', 'The pinned builder requires default rootful Podman storage')
    output.mkdir(mode=0o700)
    work = Path(tempfile.mkdtemp(prefix='kedra-installer-', dir=output.parent)).resolve()
    container = None
    media_container = 'kedra-local-builder-' + uuid.uuid4().hex
    media_tag = 'localhost/kedra-anaconda:local-' + uuid.uuid4().hex
    succeeded = False
    try:
        context, media, evidence = (work / name for name in ('context', 'media', 'evidence'))
        for directory in (context, media, evidence):
            directory.mkdir()
        registry_config = work / 'registries.d'
        registry_config.mkdir()
        (registry_config / 'kedra.yaml').write_text('docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n')
        policy = work / 'policy.json'
        requirement = {'type': 'sigstoreSigned', 'keyPath': str(public),
                       'signedIdentity': {'type': 'exactRepository', 'dockerRepository': REPOSITORY}}
        policy.write_text(json.dumps({'default': [{'type': 'reject'}],
            'transports': {'docker': {REPOSITORY: [requirement]}, 'containers-storage': {'': [requirement]}}}) + '\n')
        run([tools['sudo'], tools['skopeo'], '--policy', policy, '--registries.d', registry_config,
             'copy', '--preserve-digests', '--digestfile', work / 'payload.digest',
             'docker://' + args.image, 'containers-storage:' + args.image])
        require((work / 'payload.digest').read_text().strip() == args.image.split('@')[1], 'Verified image digest changed')
        architecture = run([tools['sudo'], tools['podman'], 'image', 'inspect', '--format', '{{.Architecture}}',
                            args.image], capture=True).decode().strip()
        require(architecture == 'amd64', 'Signed payload is not AMD64')
        container = run([tools['sudo'], tools['podman'], 'create', '--network=none',
                         '--entrypoint', '/usr/bin/true', args.image], capture=True).decode().strip()
        require(re.fullmatch('[a-f0-9]{64}', container), 'Unexpected inspection container ID')
        trust = context / 'trust'
        trust.mkdir()
        files = {
            '/usr/share/sysroot/source.json': trust / 'source.json',
            '/usr/libexec/sysroot/helper': context / 'helper',
            '/usr/lib/sysroot/trust/release.pub': trust / 'release.pub',
            '/usr/lib/sysroot/trust/release-policy.json': trust / 'release-policy.json',
            '/etc/containers/policy.json': trust / 'policy.json',
            '/etc/containers/registries.d/kedra.yaml': trust / 'registries.yaml',
            '/usr/lib/bootc/install/10-kedra.toml': trust / 'install.toml',
        }
        for source, destination in files.items():
            run([tools['sudo'], tools['podman'], 'cp', container + ':' + source, destination])
            require(destination.is_file() and not destination.is_symlink(), 'Signed payload input is not a regular file')
            run([tools['sudo'], 'chown', '--no-dereference', str(os.getuid()) + ':' + str(os.getgid()), destination])
        require((trust / 'release.pub').read_bytes() == public.read_bytes(), 'Installed public key differs')
        source = json_file(trust / 'source.json')
        target = source['target']
        require(target['id'] == 'desktop' and target['architecture'] == 'x86_64'
                and target['fedora_release'] == 44 and target['image'] == REPOSITORY,
                'Signed image source has the wrong target')
        release_policy = json_file(trust / 'release-policy.json')
        require(release_policy['key_fingerprint'] == FINGERPRINT and release_policy['scope'] == {
            'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44, 'repository': REPOSITORY},
            'Installed release policy differs from reviewed scope')
        installed_requirement = dict(requirement, keyPath='/usr/lib/sysroot/trust/release.pub')
        require(json_file(trust / 'policy.json') == {'default': [{'type': 'reject'}], 'transports': {
            'docker': {REPOSITORY: [installed_requirement]}, 'containers-storage': {'': [installed_requirement]}}},
            'Signed payload does not retain the strict container policy')
        require(tomllib.loads((trust / 'install.toml').read_text()) == {'install': {'enforce-container-sigpolicy': True}},
                'Signed payload does not enforce initial-install signature policy')
        require((trust / 'registries.yaml').read_text() == 'docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n',
                'Signed payload attachment discovery differs')
        if args.base_image:
            base = args.base_image
        else:
            resolved = work / 'resolved-inputs.json'
            run([tools['sudo'], tools['podman'], 'cp', container + ':/usr/share/sysroot/resolved-inputs.json', resolved])
            require(resolved.is_file() and not resolved.is_symlink(), 'Signed resolved inputs are not a regular file')
            run([tools['sudo'], 'chown', '--no-dereference', str(os.getuid()) + ':' + str(os.getgid()), resolved])
            base = json_file(resolved)['base']
        require(re.fullmatch(base_pattern, base), 'Signed image does not name an exact Fedora base; use --base-image for legacy media')
        for name in ('Containerfile', 'iso.yaml', 'prepare.sh', 'boot-probe.service', 'boot-probe.sh',
                     'anaconda-adapter.py', 'finalize-fstab.py', 'verify-payload.service', 'require-verification.conf'):
            shutil.copyfile(ROOT / 'installer' / name, context / name)
        builder_help = run([tools['sudo'], tools['podman'], 'run', '--rm', builder, 'build', '--help'], capture=True)
        require(b'--bootc-installer-payload-ref' in builder_help, 'Pinned builder interface differs')
        run([tools['sudo'], tools['podman'], 'build', '--pull=always', '--no-cache',
             '--build-arg', 'BASE_IMAGE=' + base, '--build-arg', 'PAYLOAD_IMAGE=' + args.image,
             '--build-arg', 'SOURCE_DATE_EPOCH=' + str(int(time.time())), '--tag', media_tag, context])
        run([tools['sudo'], tools['podman'], 'run', '--rm', '--name', media_container, '--privileged',
             '--security-opt', 'label=type:unconfined_t', '-e', 'KEDRA_INSTALLER_PAYLOAD=' + args.image,
             '-e', 'KEDRA_INSTALLER_IMAGE=' + media_tag, '-v', str(media) + ':/output',
             '-v', str(evidence) + ':/evidence', '-v', str(ROOT / 'installer') + ':/kedra-installer:ro',
             '-v', '/var/lib/containers/storage:/var/lib/containers/storage',
             '--entrypoint', '/bin/bash', builder, '/kedra-installer/build-iso.sh'])
        images = list(media.rglob('*.iso'))
        require(len(images) == 1 and images[0].is_file() and not images[0].is_symlink(), 'Expected one complete local ISO')
        iso = images[0]
        run([tools['sudo'], 'chown', '--no-dereference', str(os.getuid()) + ':' + str(os.getgid()), iso])
        if args.smoke:
            kernel = work / 'kernel'
            run([tools['xorriso'], '-osirrox', 'on', '-indev', iso, '-extract', '/images/pxeboot', kernel])
            run([sys.executable, ROOT / 'installer/smoke.py', '--iso', iso,
                 '--kernel-dir', kernel, '--work', work / 'smoke'], timeout=360)
        name = 'kedra-desktop-44-' + args.image.split(':')[-1][:16] + '.iso'
        record = {'schema_version': 1, 'image': args.image, 'source_revision': source['source_revision'],
                  'public_key_fingerprint': FINGERPRINT, 'base': base, 'builder': builder,
                  'installer': dict(filename=name, **identity(iso)), 'diskless_smoke_passed': args.smoke,
                  'fresh_installation_performed': False, 'uploaded': False}
        # Same-filesystem exclusive link: never publish a partial or overwrite an existing ISO.
        os.link(iso, output / name)
        with (output / 'installer.json').open('x', encoding='utf-8') as stream:
            stream.write(json.dumps(record, indent=2) + '\n')
        with (output / 'SHA256SUMS').open('x', encoding='utf-8') as stream:
            stream.write(record['installer']['sha256'] + '  ' + name + '\n')
        succeeded = True
        print(output / name)
    finally:
        if container:
            subprocess.run([tools['sudo'], tools['podman'], 'rm', '--force', container], env=environment, check=False)
        subprocess.run([tools['sudo'], tools['podman'], 'rm', '--force', media_container], env=environment,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        if succeeded:
            require(work.parent == output.parent.resolve() and work.name.startswith('kedra-installer-'), 'Unsafe scratch cleanup')
            run([tools['sudo'], 'rm', '-rf', '--', work])
        else:
            print('Build failed; local scratch retained for inspection: ' + str(work), file=sys.stderr)


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        print('kedra installer: ' + str(error), file=sys.stderr)
        raise SystemExit(1)
