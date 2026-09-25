#!/usr/bin/env python3
"""Build one local Kedra ISO from an explicitly reviewed, signed GHCR digest."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid

ROOT = Path(__file__).resolve().parents[1]
# One closed target table, shared with the release tooling; never guess a target.
sys.dont_write_bytecode = True
sys.path[:0] = [str(ROOT / 'build/release'), str(ROOT / 'installer')]
from material import TARGETS  # noqa: E402
import smoke  # noqa: E402


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
    repositories = ', '.join(spec['repository'] for spec in TARGETS.values())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True, help='Reviewed REPOSITORY@sha256:... of ' + repositories)
    parser.add_argument('--output-dir', required=True, type=Path, help='New local directory; never overwrite existing output')
    parser.add_argument('--base-image', help='Explicit reviewed Fedora base digest; needed for legacy images without resolved inputs')
    parser.add_argument('--smoke', action='store_true',
                        help='Also boot the ISO diskless through UEFI Secure Boot and check offline verification/Anaconda startup (requires KVM)')
    args = parser.parse_args()
    require(sys.platform == 'linux' and os.getuid() != 0 and os.geteuid() == os.getuid(),
            'Run as an ordinary user on a Linux build host; sudo is requested for container tools')
    repository, _, digest = args.image.partition('@sha256:')
    matches = [name for name, spec in TARGETS.items() if spec['repository'] == repository]
    require(len(matches) == 1 and re.fullmatch('[a-f0-9]{64}', digest),
            'An exact reviewed Kedra GHCR digest of an enabled target is required, not a mutable tag: ' + repositories)
    target_name = matches[0]
    spec = TARGETS[target_name]
    # The pinned image-builder builds media only for its own architecture.
    require(platform.machine() == spec['architecture'],
            f'Build {target_name} media on a native {spec["architecture"]} Linux host, not {platform.machine()}')
    base_pattern = r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}'
    if args.base_image:
        require(re.fullmatch(base_pattern, args.base_image), 'Invalid reviewed Fedora base digest')
    machine = smoke.MACHINES[spec['architecture']]
    names = ['sudo', 'podman', 'skopeo', 'openssl'] + ([machine['qemu']] if args.smoke else [])
    tools = {name: shutil.which(name, path='/usr/sbin:/usr/bin:/sbin:/bin') for name in names}
    require(all(tools.values()), 'Install the documented local build prerequisites first: ' + ', '.join(names))
    if args.smoke:
        require(os.access('/dev/kvm', os.R_OK | os.W_OK), '--smoke requires KVM access for the invoking user')
        require(all(path.is_file() for path in (machine['code'], machine['vars'])),
                '--smoke requires the Secure Boot firmware ' + str(machine['code']) + ' and Microsoft-enrolled '
                'variables ' + str(machine['vars']) + ' (Debian/Ubuntu package ' + machine['package'] + ')')
    output = args.output_dir.absolute()
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(),
            '--output-dir must name a new directory under an existing parent')
    require(not any(char in str(output) + str(ROOT) for char in ':,\n\r'),
            'Local build paths must not contain container-volume separators or newlines')
    pins = json_file(ROOT / 'installer/inputs.json')
    require(pins['schema_version'] == 2, 'Unexpected installer input schema')
    builder = pins['platforms'][spec['oci_architecture']]['builder']
    require(re.fullmatch(r'ghcr\.io/osbuild/image-builder@sha256:[a-f0-9]{64}', builder),
            'Unexpected pinned builder scope')
    environment = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C.UTF-8',
                   'HOME': os.environ.get('HOME', str(Path.home()))}

    def run(arguments, *, capture=False, timeout=7200):
        result = subprocess.run([str(item) for item in arguments], check=False,
                                stdin=subprocess.DEVNULL, env=environment,
                                stdout=subprocess.PIPE if capture else None, timeout=timeout)
        require(result.returncode == 0, 'Command failed: ' + str(arguments[0]))
        return result.stdout if capture else b''

    # Check the fixed public authority before downloading or running image code.
    public = ROOT / spec['public_key']
    recorded = ROOT / spec['key_sha256']
    require(public.is_file() and not public.is_symlink() and recorded.is_file() and not recorded.is_symlink()
            and recorded.stat().st_size == 65, 'Missing or unsafe public authority for ' + target_name)
    fingerprint = recorded.read_text(encoding='ascii').removesuffix('\n')
    require(re.fullmatch('[a-f0-9]{64}', fingerprint), 'Malformed public authority fingerprint')
    der = run([tools['openssl'], 'pkey', '-pubin', '-in', public, '-outform', 'DER'], capture=True, timeout=30)
    require(hashlib.sha256(der).hexdigest() == fingerprint, 'Public authority fingerprint differs')
    print(f'kedra installer: {target_name} ({spec["architecture"]}) public key SPKI SHA-256 {fingerprint}; '
          'confirm it independently', file=sys.stderr)
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
                       'signedIdentity': {'type': 'exactRepository', 'dockerRepository': repository}}
        policy.write_text(json.dumps({'default': [{'type': 'reject'}],
            'transports': {'docker': {repository: [requirement]}, 'containers-storage': {'': [requirement]}}}) + '\n')
        run([tools['sudo'], tools['skopeo'], '--policy', policy, '--registries.d', registry_config,
             'copy', '--preserve-digests', '--digestfile', work / 'payload.digest',
             'docker://' + args.image, 'containers-storage:' + args.image])
        require((work / 'payload.digest').read_text().strip() == 'sha256:' + digest, 'Verified image digest changed')

        def architecture(image):
            return run([tools['sudo'], tools['podman'], 'image', 'inspect', '--format', '{{.Architecture}}',
                        image], capture=True).decode().strip()
        require(architecture(args.image) == spec['oci_architecture'],
                'Signed payload is not ' + spec['oci_architecture'])
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
        require(target['id'] == target_name and target['architecture'] == spec['architecture']
                and target['fedora_release'] == 44 and target['image'] == repository,
                'Signed image source has the wrong target')
        release_policy = json_file(trust / 'release-policy.json')
        require(release_policy['key_fingerprint'] == fingerprint and release_policy['scope'] == {
            'target': target_name, 'architecture': spec['architecture'], 'fedora_release': 44, 'repository': repository},
            'Installed release policy differs from reviewed scope')
        installed_requirement = dict(requirement, keyPath='/usr/lib/sysroot/trust/release.pub')
        require(json_file(trust / 'policy.json') == {'default': [{'type': 'reject'}], 'transports': {
            'docker': {repository: [installed_requirement]}, 'containers-storage': {'': [installed_requirement]}}},
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
        require(architecture(builder) == spec['oci_architecture'], 'Pinned builder is not ' + spec['oci_architecture'])
        run([tools['sudo'], tools['podman'], 'build', '--pull=always', '--no-cache',
             '--build-arg', 'BASE_IMAGE=' + base, '--build-arg', 'PAYLOAD_IMAGE=' + args.image,
             '--build-arg', 'SOURCE_DATE_EPOCH=' + str(int(time.time())), '--tag', media_tag, context])
        require(architecture(media_tag) == spec['oci_architecture'],
                'Installer environment is not ' + spec['oci_architecture'])
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
        smoke_record = None
        if args.smoke:
            smoke_record = smoke.run(spec['architecture'], tools[machine['qemu']], iso, work / 'smoke',
                                     environment)
        name = f'kedra-{target_name}-44-{digest[:16]}.iso'
        record = {'schema_version': 1, 'target': target_name, 'architecture': spec['architecture'],
                  'image': args.image, 'source_revision': source['source_revision'],
                  'public_key_fingerprint': fingerprint, 'base': base, 'builder': builder,
                  'installer': dict(filename=name, **identity(iso)), 'diskless_smoke_passed': args.smoke,
                  'diskless_smoke': smoke_record, 'fresh_installation_performed': False, 'uploaded': False}
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
