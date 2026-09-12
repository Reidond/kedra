"""Public Fedora resolution and conservative checkpoint renewal; no private keys.

Equality requires authenticated complete resolved inputs, including RPM header
and payload hashes, base, source payload, compiled/external artifacts and recipes.
Build timestamps and OCI layer serialization are not package-change authority.
This runs only in the serialized main-branch release workflow.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import re
import time

import promotion as p
import material


def encoded(value):
    return base64.b64encode(value).decode('ascii')


def output(name, value):
    p.require('\n' not in str(value) and '\r' not in str(value), 'Invalid workflow output')
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as stream:
        stream.write(f'{name}={value}\n')


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n', encoding='utf-8')


def resolve(args):
    p.current_source()
    inputs = p.document(p.read(Path('build/inputs.json')))
    tag = inputs['base_tag']
    p.require(tag == 'quay.io/fedora/fedora-bootc:44' and inputs['architecture'] == 'amd64',
              'Unreviewed Fedora repository, release or architecture')
    raw = p.run('skopeo', 'inspect', '--raw', 'docker://' + tag)
    manifest = p.document(raw)
    if 'manifests' in manifest:
        matches = [item for item in manifest['manifests']
                   if item.get('platform', {}).get('os') == 'linux'
                   and item.get('platform', {}).get('architecture') == 'amd64'
                   and not item.get('platform', {}).get('variant')]
        p.require(len(matches) == 1, 'Fedora index has ambiguous AMD64 platform')
        digest = matches[0]['digest']
        p.require(re.fullmatch(r'sha256:[a-f0-9]{64}', digest), 'Invalid platform digest')
    else:
        digest = 'sha256:' + p.sha(raw)
    reference = 'quay.io/fedora/fedora-bootc@' + digest
    platform_raw = p.run('skopeo', 'inspect', '--raw', 'docker://' + reference)
    p.require('sha256:' + p.sha(platform_raw) == digest, 'Fedora platform manifest hash changed')
    config = p.document(p.run('skopeo', 'inspect', '--config', 'docker://' + reference))
    p.require(config.get('os') == 'linux' and config.get('architecture') == 'amd64',
              'Fedora platform configuration differs')
    inputs.update(base=reference, discovery_manifest_sha256=p.sha(raw),
                  resolved_at=int(time.time()))
    write(args.work / 'image-inputs.json', inputs)
    output('base', reference)


def compare(args):
    source = p.current_source()
    inputs = p.document(p.read(args.work / 'image-inputs.json'))
    plan = p.document(p.read(args.work / 'source-plan.json', 1024**2))
    resolution = args.work / 'resolution'
    resolution.mkdir()
    log = p.run('sudo', 'podman', 'run', '--rm', '--pull=always',
          '--volume', str(Path('output/desktop-context').resolve()) + ':/context:ro',
          '--volume', str(resolution.resolve()) + ':/resolution',
          '--entrypoint', '/bin/bash', inputs['base'], '-euc',
          'tar -xf /context/payload.tar -C /; tar -xf /context/agents.tar -C /; '
          'tar -xf /context/bitwarden.tar -C /; mkdir -p /usr/libexec/sysroot; '
          'cp /context/sysroot /usr/bin/sysroot; cp /context/sysroot-helper /usr/libexec/sysroot/helper; '
          '/bin/bash /context/assemble.sh --resolve-packages')
    (resolution / 'native.log').write_bytes(log)
    p.run('sudo', 'chown', str(os.getuid()) + ':' + str(os.getgid()), resolution / 'package-material.txt')
    resolved = int(time.time())
    recipe_names = ('Containerfile', 'build/assemble.sh', 'build/release/Containerfile',
                    'build/release/prepare-trust.py', 'build/release/refresh.py', 'build/release/material.py',
                    'build/agents/prepare.sh', 'build/agents/package.py', 'build/agents/fetch.py',
                    'build/agents/inputs.json', 'build/bitwarden/prepare.py', 'build/bitwarden/inputs.json',
                    'Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', '.github/workflows/release.yml',
                    'build/release/authority/desktop.pub', 'build/release/authority/desktop.sha256',
                    'build/release/installer.sh')
    recipe_names += tuple(path.as_posix() for path in sorted(Path('installer').iterdir())
                          if path.is_file() and path.suffix != '.md')
    artifact_names = ('sysroot', 'sysroot-helper', 'agents.tar', 'bitwarden.tar')
    current = {'schema_version': 1, 'base': inputs['base'],
               'source': {key: value for key, value in plan.items() if key not in ('source_revision', 'input_scope')},
               'recipes': {name: p.sha(Path(name).read_bytes()) for name in recipe_names},
               'artifacts': {name: p.file_identity(Path('output/desktop-context') / name,
                             (Path('output/desktop-context') / name).stat().st_size)['sha256'] for name in artifact_names},
               'packages': material.packages(material.read(resolution / 'package-material.txt'))}
    write(args.work / 'resolved-inputs.json', current)
    output('resolved_at', resolved)
    previous = {}
    if p.channel(args.work / 'previous', previous) is None:
        output('changed', 'true')
        return
    ordering = p.verify_history(args.work / 'previous', args.sysroot)
    release_bytes = p.read(args.work / 'previous/release.json')
    release = p.document(release_bytes)
    tag = 'desktop-44-x86_64-r' + str(release['sequence'])
    published = p.release_for_tag(tag)
    p.require(published is not None and published['draft'] is False, 'Promoted version is unavailable')
    wanted = {'SHA256SUMS': 65536, 'SHA256SUMS.sig': 1024, 'provenance.json': material.LIMIT, 'packages.txt': material.LIMIT}
    assets = {item['name']: item for item in published['assets']}
    if not wanted.keys() <= assets.keys():
        write(args.work / 'comparison.json', {'decision': 'candidate', 'reason': 'legacy release lacks signed resolved inputs'})
        output('changed', 'true')
        return
    for name, limit in wanted.items():
        p.require(0 < assets[name]['size'] <= limit, 'Prior comparison asset exceeds limit')
        p.download_asset(assets[name], args.work / 'previous' / name)
    legacy = p.document(material.read(args.work / 'previous/provenance.json'))
    recorded = legacy.get('resolved_inputs')
    if (legacy.get('schema_version') != 2 or not isinstance(recorded, dict)
            or not {'schema_version', 'base', 'source', 'artifacts', 'recipes', 'packages'} <= recorded.keys()
            or not all(recorded.get(key) for key in ('source', 'artifacts', 'recipes', 'packages'))):
        write(args.work / 'comparison.json', {'decision': 'candidate', 'reason': 'legacy or incomplete resolved-input record'})
        output('changed', 'true')
        return
    prior = material.verify(args.work / 'previous', Path('build/release/authority/desktop.pub'), p.sha(release_bytes))
    equal = prior == current
    write(args.work / 'comparison.json', {
        'schema_version': 1, 'source_revision': source,
        'promoted_image': release['image_digest'], 'resolved_inputs_equal': equal,
        'comparison': 'signed-complete-resolved-inputs-v1',
        'decision': 'no-change' if equal else 'candidate',
    })
    output('changed', 'false' if equal else 'true')
    if not equal:
        return
    now = int(time.time())
    p.require(now - 36 * 3600 <= resolved <= now
              and resolved >= ordering['last_successful_resolution'], 'Stale/backwards resolution')
    old_checkpoint = p.document(p.read(args.work / 'previous/checkpoint.json'))
    checkpoint = dict(old_checkpoint)
    checkpoint.update(generation=old_checkpoint['generation'] + 1, issued_at=now,
                      expires_at=now + 7 * 86400, last_successful_resolution=resolved)
    checkpoint_bytes = (json.dumps(checkpoint, sort_keys=True, indent=2) + '\n').encode()
    request = {'schema_version': 1, 'source_revision': source,
               'previous_bundle_sha256': previous['sha256'],
               'release_sha256': p.sha(release_bytes),
               'previous_checkpoint_sha256': p.sha(p.read(args.work / 'previous/checkpoint.json')),
               'checkpoint_sha256': p.sha(checkpoint_bytes), 'image_digest': release['image_digest'],
               'resolved_inputs_sha256': material.sha(material.canonical(current)),
               'provenance_sha256': p.sha(material.read(args.work / 'previous/provenance.json')),
               'checksums_sha256': p.sha(material.read(args.work / 'previous/SHA256SUMS'))}
    (args.work / 'checkpoint.json').write_bytes(checkpoint_bytes)
    write(args.work / 'renewal-request.json', request)
    output('checkpoint', encoded(checkpoint_bytes))
    output('renewal_request', encoded(json.dumps(request, sort_keys=True).encode()))


def publish(args):
    source = p.current_source()
    request = p.document(base64.b64decode(os.environ['RENEWAL_REQUEST_B64'], validate=True))
    checkpoint = base64.b64decode(os.environ['CHECKPOINT_B64'], validate=True)
    signature = base64.b64decode(os.environ['CHECKPOINT_SIG_B64'], validate=True)
    p.require(request['source_revision'] == source and p.sha(checkpoint) == request['checkpoint_sha256'],
              'Renewal request changed')
    previous = {}
    p.require(p.channel(args.work / 'previous', previous) == request['previous_bundle_sha256'],
              'Channel changed while renewal awaited approval')
    ordering = p.verify_history(args.work / 'previous', args.sysroot)
    release = p.read(args.work / 'previous/release.json')
    old_checkpoint = p.read(args.work / 'previous/checkpoint.json')
    p.require(p.sha(release) == request['release_sha256']
              and p.sha(old_checkpoint) == request['previous_checkpoint_sha256'], 'Predecessor changed')
    new = p.document(checkpoint)
    old = p.document(old_checkpoint)
    p.require(new['generation'] == old['generation'] + 1
              and new['release_sha256'] == old['release_sha256']
              and new['scope'] == old['scope'], 'Renewal changed release or ordering')
    signed = args.work / 'signed'
    signed.mkdir()
    for name in ('release.json', 'release.sig'):
        (signed / name).write_bytes(p.read(args.work / 'previous' / name))
    (signed / 'checkpoint.json').write_bytes(checkpoint)
    (signed / 'checkpoint.sig').write_bytes(signature)
    write(args.work / 'ordering.json', ordering)
    p.verify_channel(signed, args.sysroot, args.work / 'ordering.json')
    bundle = p.document(p.read(args.work / 'previous/channel.json', 300_000))
    bundle['checkpoint'] = {'payload': checkpoint.decode(), 'signature': signature.decode()}
    write(signed / 'channel.json', bundle)
    path = signed / 'channel.json'
    identity = p.file_identity(path, path.stat().st_size)
    p.replace_channel(previous, path, identity)
    p.require(p.channel(args.work / 'published') == identity['sha256'], 'Published renewal readback differs')
    p.verify_channel(args.work / 'published', args.sysroot, args.work / 'ordering.json')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['resolve', 'compare', 'publish'])
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--sysroot', type=Path, default=Path('target/release/sysroot'))
    args = parser.parse_args()
    args.sysroot = args.sysroot.resolve()
    args.work.mkdir(parents=True, exist_ok=True)
    {'resolve': resolve, 'compare': compare, 'publish': publish}[args.operation](args)
