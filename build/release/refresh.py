"""Actions-only GHCR image resolution, comparison and stable publication.

No GitHub Releases, ISO artifacts, metadata signatures or checkpoints are used.
Only the manually signed OCI manifest authorizes an update.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import threading
import time
import uuid

import material as m

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = m.REPOSITORY
STABLE = REPOSITORY + ':stable'
WORKFLOW = '.github/workflows/release.yml'


def run(*arguments, timeout=900, maximum=8 * 1024**2, allow_failure=False):
    command = list(map(str, arguments))
    with tempfile.TemporaryFile() as errors:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=errors, cwd=ROOT)
        timer = threading.Timer(timeout, child.kill)
        timer.start()
        content = bytearray()
        try:
            while chunk := child.stdout.read(min(1024**2, maximum - len(content) + 1)):
                content.extend(chunk)
                m.require(len(content) <= maximum, 'Native response exceeds its bound')
            code = child.wait(timeout=30)
            errors.seek(0)
            result = subprocess.CompletedProcess(command, code, bytes(content), errors.read(16384))
        finally:
            timer.cancel()
            if child.poll() is None:
                child.kill()
            child.wait()
            child.stdout.close()
    m.require(allow_failure or result.returncode == 0,
              'Native operation failed: ' + result.stderr.decode(errors='replace')[:1500])
    m.require(len(result.stdout) <= maximum, 'Native response exceeds its bound')
    return result


def output(name, value):
    m.require('\n' not in str(value), 'Invalid Actions output')
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as stream:
        stream.write(f'{name}={value}\n')


def write(path, value):
    temporary = path.with_name(path.name + '.' + uuid.uuid4().hex + '.tmp')
    with temporary.open('xb') as stream:
        stream.write(m.canonical(value))
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


def current_source():
    m.require(os.environ.get('GITHUB_ACTIONS') == 'true' and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra'
              and os.environ.get('GITHUB_REF') == 'refs/heads/main', 'Only accepted-main Actions may publish images')
    source = os.environ['GITHUB_SHA']
    current = m.document(run('gh', 'api', 'repos/Reidond/kedra/git/ref/heads/main', '--jq', '.object').stdout)['sha']
    m.require(current == source and run('git', 'rev-parse', 'HEAD').stdout.decode().strip() == source, 'Accepted main changed')
    return source


def manifest_digest(reference, missing=False):
    result = run('skopeo', 'inspect', '--raw', 'docker://' + reference, allow_failure=missing)
    if result.returncode:
        error = result.stderr.decode(errors='replace').lower()
        m.require(missing and 'manifest unknown' in error, 'Registry lookup failed; absence is not established')
        return None
    manifest = m.document(result.stdout)
    m.require('manifests' not in manifest and isinstance(manifest.get('config'), dict),
              'Stable/candidate must name a single platform manifest')
    return 'sha256:' + m.sha(result.stdout)


def unchanged(previous):
    m.require(manifest_digest(STABLE, missing=True) == previous, 'Stable changed after preflight; refusing publication')
    current_source()


def host_policy(work):
    # Build uses the already validated source fingerprint. Publication receives
    # this value from the isolated signer, which checks protected environment authority.
    key = ROOT / 'build/release/authority/desktop.pub'
    fingerprint = m.sha(run('openssl', 'pkey', '-pubin', '-in', key, '-outform', 'DER').stdout)
    expected = (ROOT / 'build/release/authority/desktop.sha256').read_text().strip()
    m.require(fingerprint == expected == os.environ['APPROVED_FINGERPRINT'], 'Independent public authority differs')
    requirement = {'type': 'sigstoreSigned', 'keyPath': str(key), 'signedIdentity': {
        'type': 'exactRepository', 'dockerRepository': REPOSITORY}}
    path = work / 'host-policy.json'
    write(path, {'default': [{'type': 'reject'}], 'transports': {
        'docker': {REPOSITORY: [requirement]}, 'containers-storage': {'': [requirement]}}})
    return path


def copied_file(container, path, maximum):
    archive = run('sudo', 'podman', 'cp', container + ':' + path, '-', maximum=maximum + 16384).stdout
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:') as stream:
        members = stream.getmembers()
        m.require(len(members) == 1 and members[0].isfile() and 0 < members[0].size <= maximum,
                  'Installed image record must be one bounded regular file')
        data = stream.extractfile(members[0]).read(maximum + 1)
    m.require(len(data) <= maximum, 'Installed record exceeded its bound')
    return data


def verified_image(digest, work, label, policy):
    m.require(re.fullmatch('sha256:[a-f0-9]{64}', digest), 'Invalid registry digest')
    reference = REPOSITORY + '@' + digest
    local = 'localhost/kedra-verified:' + label
    log = run('sudo', 'skopeo', '--policy', policy, 'copy', '--preserve-digests',
              '--digestfile', work / (label + '.digest'), 'docker://' + reference, 'containers-storage:' + local)
    (work / (label + '-pull.log')).write_bytes(log.stdout + log.stderr)
    m.require((work / (label + '.digest')).read_text().strip() == digest, 'Verified copy changed manifest identity')
    return inspect_stored(local, digest, work, label)


def inspect_stored(local, digest, work, label):
    config = m.document(run('sudo', 'skopeo', 'inspect', '--config', '--raw', 'containers-storage:' + local).stdout)
    m.require(config.get('os') == 'linux' and config.get('architecture') == 'amd64', 'Wrong signed image platform')
    identity_text = config.get('config', {}).get('Labels', {}).get(m.LABEL)
    if identity_text is None:
        return {'digest': digest, 'identity': None, 'inputs': None}
    m.require(isinstance(identity_text, str) and len(identity_text.encode()) <= 16384, 'Oversized image identity label')
    m.validate(m.document(identity_text))
    container = 'kedra-inspect-' + uuid.uuid4().hex
    run('sudo', 'podman', 'create', '--name', container, '--network=none', '--entrypoint=/usr/bin/false', local)
    try:
        identity_bytes = copied_file(container, '/usr/share/sysroot/image-identity.json', 16384)
        inputs_bytes = copied_file(container, '/usr/share/sysroot/resolved-inputs.json', m.LIMIT)
        source_bytes = copied_file(container, '/usr/share/sysroot/source.json', 1024**2)
        identity, inputs = m.verify(config, identity_bytes, inputs_bytes, source_bytes)
        rpm_bytes = copied_file(container, '/usr/share/sysroot/package-material.txt', m.LIMIT)
        m.require(m.packages(rpm_bytes) == inputs['packages'], 'Actual candidate RPM inventory differs from preflight material')
        (work / (label + '-identity.json')).write_bytes(identity_bytes)
        (work / (label + '-inputs.json')).write_bytes(inputs_bytes)
        (work / (label + '-source.json')).write_bytes(source_bytes)
        (work / (label + '-config.json')).write_bytes(m.canonical(config))
        return {'digest': digest, 'identity': identity, 'inputs': inputs}
    finally:
        run('sudo', 'podman', 'rm', container)


def resolve_base(work):
    inputs = m.document(m.read(ROOT / 'build/inputs.json', 16384))
    m.require(inputs['base_tag'] == 'quay.io/fedora/fedora-bootc:44' and inputs['architecture'] == 'amd64',
              'Unreviewed upstream scope')
    raw = run('skopeo', 'inspect', '--raw', 'docker://' + inputs['base_tag']).stdout
    manifest = m.document(raw)
    if 'manifests' in manifest:
        matches = [item for item in manifest['manifests'] if item.get('platform', {}).get('os') == 'linux'
                   and item.get('platform', {}).get('architecture') == 'amd64' and not item.get('platform', {}).get('variant')]
        m.require(len(matches) == 1, 'Ambiguous upstream platform')
        digest = matches[0]['digest']
    else:
        digest = 'sha256:' + m.sha(raw)
    m.require(re.fullmatch('sha256:[a-f0-9]{64}', digest), 'Invalid base digest')
    base = 'quay.io/fedora/fedora-bootc@' + digest
    m.require(manifest_digest(base) == digest, 'Base manifest changed')
    config = m.document(run('skopeo', 'inspect', '--config', '--raw', 'docker://' + base).stdout)
    m.require(config.get('os') == 'linux' and config.get('architecture') == 'amd64', 'Unexpected base platform')
    write(work / 'base-resolution.json', {'base': base, 'source_tag': inputs['base_tag'],
          'discovery_digest': 'sha256:' + m.sha(raw), 'resolved_at': int(time.time())})
    return base


def prepare(args):
    source = current_source()
    context = ROOT / 'output/desktop-context'
    derivative = ROOT / 'output/release-context'
    plan = m.document(m.read(args.work / 'source-plan.json', 1024**2))
    m.require(plan['source_revision'] == source, 'Source plan differs from accepted main')
    with tarfile.open(context / 'payload.tar', mode='r:') as archive:
        member = archive.getmember('usr/share/sysroot/source.json')
        m.require(member.isfile() and member.size <= 1024**2, 'Invalid archived source manifest')
        source_bytes = archive.extractfile(member).read()
    m.require(m.document(source_bytes) == plan, 'Archive and plan differ')
    policy = host_policy(args.work)
    previous_digest = manifest_digest(STABLE, missing=True)
    previous = verified_image(previous_digest, args.work, 'previous', policy) if previous_digest else {
        'digest': None, 'identity': None, 'inputs': None}
    base = resolve_base(args.work)
    resolution = args.work / 'resolution'
    resolution.mkdir()
    log = run('sudo', 'podman', 'run', '--rm', '--pull=always', '--volume', str(context) + ':/context:ro',
              '--volume', str(resolution.resolve()) + ':/resolution', '--entrypoint', '/bin/bash', base, '-euc',
              'tar -xf /context/payload.tar -C /; tar -xf /context/agents.tar -C /; '
              'tar -xf /context/bitwarden.tar -C /; mkdir -p /usr/libexec/sysroot; '
              'cp /context/sysroot /usr/bin/sysroot; cp /context/sysroot-helper /usr/libexec/sysroot/helper; '
              '/bin/bash /context/assemble.sh --resolve-packages', timeout=2400)
    (resolution / 'native.log').write_bytes(log.stdout + log.stderr)
    run('sudo', 'chown', f'{os.getuid()}:{os.getgid()}', resolution / 'package-material.txt')
    recipes = ('Containerfile', 'build/assemble.sh', 'build/release/Containerfile', 'build/release/prepare-trust.py',
               'build/release/refresh.py', 'build/release/material.py', 'build/agents/prepare.sh', 'build/agents/package.py',
               'build/agents/fetch.py', 'build/agents/inputs.json', 'build/bitwarden/prepare.py', 'build/bitwarden/inputs.json',
               'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.github/workflows/release.yml',
               'build/release/authority/desktop.pub', 'build/release/authority/desktop.sha256',
               'build/release/compatibility.json')
    inputs = {'schema_version': 1, 'base': base,
              'source': {key: value for key, value in plan.items() if key not in ('source_revision', 'input_scope')},
              'recipes': {name: m.sha((ROOT / name).read_bytes()) for name in recipes},
              'artifacts': {}, 'packages': m.packages(m.read(resolution / 'package-material.txt'))}
    for name in ('sysroot', 'sysroot-helper', 'agents.tar', 'bitwarden.tar'):
        with (context / name).open('rb') as stream:
            inputs['artifacts'][name] = hashlib.file_digest(stream, 'sha256').hexdigest()
    material_bytes = m.canonical(inputs)
    (args.work / 'resolved-inputs.json').write_bytes(material_bytes)
    write(args.work / 'preflight-bootc-compatibility.json', m.bootc_compatibility(inputs['packages']))
    m.require_bootc(inputs['packages'])
    changed = previous['inputs'] != inputs
    write(args.work / 'comparison.json', {'decision': 'changed' if changed else 'unchanged',
          'reason': 'verified stable inputs differ or are absent' if changed else 'verified stable resolved inputs match',
          'previous_digest': previous_digest})
    unchanged(previous_digest)
    output('changed', str(changed).lower())
    if not changed:
        return
    identity = {'schema_version': 2, 'project': 'Kedra', 'target': 'desktop', 'architecture': 'x86_64',
                'fedora_release': 44, 'repository': REPOSITORY, 'channel': 'stable', 'source_revision': source,
                'source_manifest_sha256': m.sha(source_bytes), 'resolved_inputs_sha256': m.sha(material_bytes),
                'workflow': WORKFLOW, 'epoch': 1, 'run_number': int(os.environ['GITHUB_RUN_NUMBER']),
                'run_attempt': int(os.environ['GITHUB_RUN_ATTEMPT']), 'resolved_at': int(time.time()), 'minimum_protocol': 2}
    m.validate(identity)
    if previous['identity']:
        m.require(tuple(identity[k] for k in ('epoch', 'run_number', 'run_attempt')) >
                  tuple(previous['identity'][k] for k in ('epoch', 'run_number', 'run_attempt')), 'Build rank is obsolete')
        m.require(identity['resolved_at'] >= previous['identity']['resolved_at'], 'Package-resolution time moved backwards')
    for destination in (args.work, derivative):
        write(destination / 'image-identity.json', identity)
        (destination / 'resolved-inputs.json').write_bytes(material_bytes)
    write(args.work / 'build-state.json', {'schema_version': 1, 'source': source, 'run_id': int(os.environ['GITHUB_RUN_ID']),
          'run_attempt': identity['run_attempt'], 'previous_digest': previous_digest, 'identity': identity})
    output('base', base)


def inspect_candidate(args):
    source = current_source()
    local = 'localhost/kedra-desktop:release-candidate'
    digest = 'sha256:' + m.sha(run('sudo', 'skopeo', 'inspect', '--raw', 'containers-storage:' + local).stdout)
    observed = inspect_stored(local, digest, args.work, 'built')
    state = m.document(m.read(args.work / 'build-state.json', 65536))
    m.require(observed['identity'] == state['identity'] and observed['identity']['source_revision'] == source,
              'Built image differs from prepared identity')
    write(args.work / 'built-bootc-compatibility.json', m.bootc_compatibility(observed['inputs']['packages']))
    m.require_bootc(observed['inputs']['packages'])
    output('identity_sha256', m.sha(m.canonical(observed['identity'])))


def publish(args):
    source = current_source()
    state = m.document(m.read(args.work / 'build-state.json', 65536))
    m.require(state['source'] == source and state['run_id'] == int(os.environ['GITHUB_RUN_ID'])
              and state['run_attempt'] == int(os.environ['GITHUB_RUN_ATTEMPT']), 'Build artifact belongs to another run')
    digest = os.environ['KEDRA_SIGNED_DIGEST']
    policy = host_policy(args.work)
    unchanged(state['previous_digest'])
    previous = verified_image(state['previous_digest'], args.work, 'prior-final', policy) if state['previous_digest'] else {
        'digest': None, 'identity': None, 'inputs': None}
    candidate = verified_image(digest, args.work, 'candidate-final', policy)
    write(args.work / 'publish-bootc-compatibility.json', m.bootc_compatibility(candidate['inputs']['packages']))
    m.require_bootc(candidate['inputs']['packages'])
    m.require(candidate['identity'] == state['identity'], 'Signed image identity differs from reviewed build')
    m.require(candidate['identity']['source_revision'] == source
              and candidate['identity']['run_number'] == int(os.environ['GITHUB_RUN_NUMBER']), 'Signed source/run rank differs')
    m.order(candidate['identity'], previous['identity'], digest, previous['digest'])
    unchanged(state['previous_digest'])
    receipt = args.work / 'stable-publication.json'
    write(receipt, {'state': 'submitting', 'previous_digest': previous['digest'], 'digest': digest, 'source': source})
    # The registry manifest PUT is atomic; workflow concurrency serializes Kedra writers.
    # Pre/post checks detect changes by other writers. Never retry an uncertain PUT.
    run('sudo', 'skopeo', '--policy', policy, 'copy', '--preserve-digests', '--authfile', os.environ['KEDRA_REGISTRY_AUTH'],
        'docker://' + REPOSITORY + '@' + digest, 'docker://' + STABLE)
    m.require(manifest_digest(STABLE) == digest, 'Stable readback differs; preserve receipt for recovery')
    final = verified_image(digest, args.work, 'stable-final', policy)
    m.require(final['identity'] == candidate['identity'], 'Stable signed identity changed')
    m.order(final['identity'], previous['identity'], digest, previous['digest'])
    m.require(manifest_digest(STABLE) == digest, 'Stable changed during final verification; preserve receipt')
    write(receipt, {'state': 'verified', 'previous_digest': previous['digest'], 'digest': digest, 'source': source})
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as summary:
        summary.write(f'Verified signed stable image: {REPOSITORY}@{digest}. No machine was staged or rebooted.\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('prepare', 'inspect-candidate', 'publish'))
    parser.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    {'prepare': prepare, 'inspect-candidate': inspect_candidate, 'publish': publish}[args.operation](args)
