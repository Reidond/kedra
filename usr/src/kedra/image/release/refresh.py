#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Public Fedora base resolution and main-only signed GHCR release operations.

No GitHub Releases, ISO artifacts, metadata signatures or checkpoints are used.
Only the automatically signed, validated OCI manifest authorizes an update.
"""
import argparse
import io
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import uuid
from pathlib import Path

import material as m
import trust

ROOT = Path(__file__).resolve().parents[5]
IMAGE = 'usr/src/kedra/image/'
WORKFLOW = '.github/workflows/release.yml'
OUTPUT = ROOT / 'output'
BINARIES = OUTPUT / 'release-binaries'
DERIVATIVE = OUTPUT / 'release-context'
# Foundation build context: compose.py freezes the payload archive itself.
CONTEXT_FILES = {'Containerfile': IMAGE + 'Containerfile', 'assemble.sh': IMAGE + 'assemble.sh',
                 'install-rpms.sh': IMAGE + 'catalog/install-rpms.sh'}
# The engine's bound for an installed composition receipt.
COMPOSITION_LIMIT = 64 * 1024**2


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


def accepted_source():
    """The dispatched accepted-main revision, checked out. No registry or API call."""
    m.require(os.environ.get('GITHUB_ACTIONS') == 'true' and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra'
              and os.environ.get('GITHUB_REF') == 'refs/heads/main', 'Only accepted-main Actions may publish images')
    source = os.environ['GITHUB_SHA']
    m.require(re.fullmatch('[a-f0-9]{40}', source) is not None
              and run('git', 'rev-parse', 'HEAD').stdout.decode().strip() == source, 'Checkout differs from the dispatched source')
    return source


def require_current_main(source):
    """The exact-main guard: once at the end of preflight and once right before stable
    advances. The isolated signer checks it once more before it uses the private key."""
    current = m.document(run('gh', 'api', 'repos/Reidond/kedra/git/ref/heads/main', '--jq', '.object').stdout)['sha']
    m.require(current == source, 'Accepted main changed')


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


def target(name):
    spec = m.target_spec(name)
    # Builds, candidate inspection and publication run natively on the target's runner.
    m.require(platform.machine() == spec['architecture'], 'Release target does not match the native runner architecture')
    return dict(spec, id=name, stable=spec['repository'] + ':stable')


def unchanged(spec, previous, source):
    """Stable still names the preflight's digest and main is still this source."""
    m.require(manifest_digest(spec['stable'], missing=True) == previous, 'Stable changed after preflight; refusing publication')
    require_current_main(source)


def host_trust(spec, work, approved=None):
    """Signature policy and attachment discovery for this target's verified pulls.

    The committed key must match its committed fingerprint. Publication also
    passes the fingerprint the isolated signer checked against the protected
    environment's authority.
    """
    key = ROOT / spec['public_key']
    fingerprint = m.sha(run('openssl', 'pkey', '-pubin', '-in', key, '-outform', 'DER').stdout)
    expected = (ROOT / spec['key_sha256']).read_text().strip()
    m.require(fingerprint == expected and approved in (None, fingerprint), 'Independent public authority differs')
    policy, registries = trust.host_configuration(work / 'host-trust', spec['repository'], key)
    return {'fingerprint': fingerprint, 'policy': policy, 'registries': registries}


def copied_file(container, path, maximum):
    archive = run('sudo', 'podman', 'cp', container + ':' + path, '-', maximum=maximum + 16384).stdout
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:') as stream:
        members = stream.getmembers()
        m.require(len(members) == 1 and members[0].isfile() and 0 < members[0].size <= maximum,
                  'Installed image record must be one bounded regular file')
        data = stream.extractfile(members[0]).read(maximum + 1)
    m.require(len(data) <= maximum, 'Installed record exceeded its bound')
    return data


def verified_image(spec, digest, work, label, host):
    m.require(re.fullmatch('sha256:[a-f0-9]{64}', digest), 'Invalid registry digest')
    reference = spec['repository'] + '@' + digest
    local = 'localhost/kedra-verified:' + label
    log = run('sudo', 'skopeo', '--policy', host['policy'], '--registries.d', host['registries'], 'copy',
              '--preserve-digests', '--digestfile', work / (label + '.digest'),
              'docker://' + reference, 'containers-storage:' + local)
    (work / (label + '-pull.log')).write_bytes(log.stdout + log.stderr)
    m.require((work / (label + '.digest')).read_text().strip() == digest, 'Verified copy changed manifest identity')
    return inspect_stored(spec, local, digest, work, label)


def inspect_stored(spec, local, digest, work, label, selected=None):
    config = m.document(run('sudo', 'skopeo', 'inspect', '--config', '--raw', 'containers-storage:' + local).stdout)
    m.require(config.get('os') == 'linux' and config.get('architecture') == spec['oci_architecture'],
              'Wrong signed image platform')
    identity_text = config.get('config', {}).get('Labels', {}).get(m.LABEL)
    if identity_text is None:
        return {'digest': digest, 'identity': None, 'inputs': None}
    m.require(isinstance(identity_text, str) and len(identity_text.encode()) <= 16384, 'Oversized image identity label')
    m.validate(m.document(identity_text), spec['id'])
    container = 'kedra-inspect-' + uuid.uuid4().hex
    run('sudo', 'podman', 'create', '--name', container, '--network=none', '--entrypoint=/usr/bin/false', local)
    try:
        identity_bytes = copied_file(container, '/usr/share/sysroot/image-identity.json', 16384)
        inputs_bytes = copied_file(container, '/usr/share/sysroot/resolved-inputs.json', m.LIMIT)
        source_bytes = copied_file(container, '/usr/share/sysroot/source.json', 1024**2)
        identity, inputs = m.verify(config, identity_bytes, inputs_bytes, source_bytes, spec['id'])
        rpm_bytes = copied_file(container, '/usr/share/sysroot/package-material.txt', m.LIMIT)
        m.require(m.packages(rpm_bytes, spec['architecture']) == inputs['packages'],
                  'Installed RPM inventory differs from the RPM lock')
        if 'rpm-lock' in inputs['artifacts']:
            lock_bytes = copied_file(container, '/usr/share/sysroot/rpm-lock.json', m.LOCK_LIMIT)
            lock = m.document(lock_bytes)
            m.require(m.canonical(lock) == lock_bytes and m.lock_pins_sha256(lock) == inputs['artifacts']['rpm-lock']
                      and lock['foundation']['packages'] == inputs['packages'],
                      'Installed RPM lock differs from the signed resolved inputs')
            (work / (label + '-rpm-lock.json')).write_bytes(lock_bytes)
        native = None
        # Images built through the engine carry their target's declared steps.
        if m.declared_steps(inputs):
            native_bytes = copied_file(container, '/usr/share/sysroot/native-receipt.json', m.LIMIT)
            native = m.verify_native(config, native_bytes, inputs, spec['id'], selected)
            (work / (label + '-native.json')).write_bytes(native_bytes)
            # The binary cache admits a substituted package only if it equals one of
            # these receipts of a strictly verified image (schema 2 names runtime roots).
            composition_bytes = copied_file(container, '/usr/share/sysroot/composition.json', COMPOSITION_LIMIT)
            if m.document(composition_bytes).get('schema') == 2:
                (work / (label + '-composition.json')).write_bytes(composition_bytes)
        (work / (label + '-identity.json')).write_bytes(identity_bytes)
        (work / (label + '-inputs.json')).write_bytes(inputs_bytes)
        (work / (label + '-source.json')).write_bytes(source_bytes)
        (work / (label + '-config.json')).write_bytes(m.canonical(config))
        return {'digest': digest, 'identity': identity, 'inputs': inputs, 'native': native}
    finally:
        run('sudo', 'podman', 'rm', container)


def resolve_base(spec, work):
    inputs = m.document(m.read(ROOT / IMAGE / 'inputs.json', 16384))
    architecture = spec['oci_architecture']
    m.require(inputs.get('schema_version') == 2 and inputs['base_tag'] == 'quay.io/fedora/fedora-bootc:44'
              and isinstance(inputs.get('platforms'), dict) and architecture in inputs['platforms'],
              'Unreviewed upstream scope')
    raw = run('skopeo', 'inspect', '--raw', 'docker://' + inputs['base_tag']).stdout
    manifest = m.document(raw)
    if 'manifests' in manifest:
        matches = [item for item in manifest['manifests'] if item.get('platform', {}).get('os') == 'linux'
                   and item.get('platform', {}).get('architecture') == architecture
                   and not item.get('platform', {}).get('variant')]
        m.require(len(matches) == 1, 'Ambiguous upstream platform')
        digest = matches[0]['digest']
    else:
        digest = 'sha256:' + m.sha(raw)
    m.require(re.fullmatch('sha256:[a-f0-9]{64}', digest), 'Invalid base digest')
    base = 'quay.io/fedora/fedora-bootc@' + digest
    m.require(manifest_digest(base) == digest, 'Base manifest changed')
    config = m.document(run('skopeo', 'inspect', '--config', '--raw', 'docker://' + base).stdout)
    m.require(config.get('os') == 'linux' and config.get('architecture') == architecture, 'Unexpected base platform')
    write(work / 'base-resolution.json', {'base': base, 'source_tag': inputs['base_tag'],
          'discovery_digest': 'sha256:' + m.sha(raw), 'resolved_at': int(time.time())})
    return base


def stage_inputs(spec, work):
    """Pinned binaries, committed source plan and foundation build context.

    The plan's bytes are the image's installed source manifest: `source archive`
    writes the same serialization, and the build checks the foundation's copy.
    """
    for directory in (BINARIES, OUTPUT / (spec['id'] + '-context'), DERIVATIVE):
        directory.mkdir(parents=True, exist_ok=False)
    context = OUTPUT / (spec['id'] + '-context')
    for name in ('sysroot', 'sysroot-helper'):
        for destination in (BINARIES / name, context / name):
            shutil.copyfile(ROOT / 'target/release' / name, destination)
            destination.chmod(0o755)
    for name, recipe in CONTEXT_FILES.items():
        shutil.copyfile(ROOT / recipe, context / name)
        (context / name).chmod((ROOT / recipe).stat().st_mode & 0o777)
    printed = run(BINARIES / 'sysroot', 'source', 'plan', '--host', spec['id'], '--json', maximum=1024**2 + 1).stdout
    m.require(printed.endswith(b'\n'), 'Unexpected source plan output')
    source_bytes = printed[:-1]
    (work / 'source-plan.json').write_bytes(source_bytes)
    return context, source_bytes, m.document(source_bytes)


def check_plan(spec, plan, source):
    m.require(plan['source_revision'] == source and plan['input_scope'] == 'committed HEAD only',
              'Source plan differs from accepted main')
    m.require(plan['target']['id'] == spec['id'] and plan['target']['architecture'] == spec['architecture']
              and plan['target']['image'] == spec['repository'] and plan['target']['fedora_release'] == 44
              and plan['target']['candidate_target'] is True, 'Source plan belongs to another release target')


def resolve_inputs(spec, args, plan, context):
    """The run's single package resolution and its recorded build inputs.

    Every later build installs offline from this lock's fixed-output store objects,
    so a mirror push mid-run cannot make the build differ from what was recorded.
    """
    base = resolve_base(spec, args.work)
    lock_path = args.work / 'rpm-lock.json'
    run(BINARIES / 'sysroot', 'catalog', 'lock', 'resolve', '--input-root', ROOT / (IMAGE + 'packages'),
        '--entry', 'catalog.kedra', '--lock', 'packages.lock.json', '--target', spec['id'],
        '--target-policy', ROOT / (IMAGE + 'package-policy.json'), '--base', base, '--store', args.store,
        '--log-dir', args.work / 'rpm-resolution', '--output', lock_path, timeout=3600)
    inputs = m.resolved_inputs(ROOT, plan, base, context, BINARIES, m.read(lock_path, m.LOCK_LIMIT),
                               spec['id'], catalog={'pins': BINARIES / 'catalog-pins.json'})
    m.require(m.declared_steps(inputs), 'The committed target declares no image steps')
    write(args.work / 'preflight-bootc-compatibility.json', m.bootc_compatibility(inputs['packages'], spec['architecture']))
    return base, lock_path, inputs


def prepare(spec, args):
    source = accepted_source()
    context, source_bytes, plan = stage_inputs(spec, args.work)
    check_plan(spec, plan, source)
    host = host_trust(spec, args.work)
    output('fingerprint', host['fingerprint'])
    previous_digest = manifest_digest(spec['stable'], missing=True)
    previous = verified_image(spec, previous_digest, args.work, 'previous', host) if previous_digest else {
        'digest': None, 'identity': None, 'inputs': None}
    base, lock_path, inputs = resolve_inputs(spec, args, plan, context)
    material_bytes = m.canonical(inputs)
    (args.work / 'resolved-inputs.json').write_bytes(material_bytes)
    # Source and lock pins are both in the resolved inputs: equal inputs mean the
    # stable image was built from this source and these exact RPMs.
    changed = previous['inputs'] != inputs
    write(args.work / 'comparison.json', {'decision': 'changed' if changed else 'unchanged',
          'reason': 'verified stable inputs differ or are absent' if changed else 'verified stable resolved inputs match',
          'previous_digest': previous_digest})
    unchanged(spec, previous_digest, source)
    output('changed', str(changed).lower())
    if not changed:
        return
    run(BINARIES / 'sysroot', 'catalog', 'lock', 'repo', '--store', args.store, '--lock', lock_path,
        '--foundation', '--output', context / 'rpms', timeout=1800)
    run(sys.executable, ROOT / (IMAGE + 'release/prepare-trust.py'), '--source', args.work / 'source-plan.json',
        '--public-key', ROOT / spec['public_key'], '--expected-fingerprint', host['fingerprint'],
        '--sysroot', BINARIES / 'sysroot', '--output', DERIVATIVE / 'trust')
    identity = {'schema_version': 2, 'project': 'Kedra', 'target': spec['id'], 'architecture': spec['architecture'],
                'fedora_release': 44, 'repository': spec['repository'], 'channel': 'stable', 'source_revision': source,
                'source_manifest_sha256': m.sha(source_bytes), 'resolved_inputs_sha256': m.sha(material_bytes),
                'workflow': WORKFLOW, 'epoch': 1, 'run_number': int(os.environ['GITHUB_RUN_NUMBER']),
                'run_attempt': int(os.environ['GITHUB_RUN_ATTEMPT']), 'resolved_at': int(time.time()), 'minimum_protocol': 2}
    m.validate(identity, spec['id'])
    if previous['identity']:
        m.require(tuple(identity[k] for k in ('epoch', 'run_number', 'run_attempt')) >
                  tuple(previous['identity'][k] for k in ('epoch', 'run_number', 'run_attempt')), 'Build rank is obsolete')
        m.require(identity['resolved_at'] >= previous['identity']['resolved_at'], 'Package-resolution time moved backwards')
    for destination in (args.work, DERIVATIVE):
        write(destination / 'image-identity.json', identity)
        (destination / 'resolved-inputs.json').write_bytes(material_bytes)
    # The previous stable's signature-verified identity, for publication's rank check.
    write(args.work / 'build-state.json', {'schema_version': 1, 'target': spec['id'], 'source': source,
          'run_id': int(os.environ['GITHUB_RUN_ID']), 'run_attempt': identity['run_attempt'],
          'previous_digest': previous_digest, 'previous_identity': previous['identity'], 'identity': identity})
    output('base', base)


def inspect_candidate(spec, args):
    """Bind the built candidate to the preflight before it leaves the runner.

    Equal identity means equal resolved inputs (whose lock already passed the bootc
    check), and the installed RPM inventory, native receipt and source manifest
    must equal them.
    """
    source = accepted_source()
    local = 'localhost/kedra-' + spec['id'] + ':release-candidate'
    digest = 'sha256:' + m.sha(run('sudo', 'skopeo', 'inspect', '--raw', 'containers-storage:' + local).stdout)
    selected = m.document(m.read(args.work / 'native-candidate.json', m.LIMIT))
    observed = inspect_stored(spec, local, digest, args.work, 'built', selected)
    state = m.document(m.read(args.work / 'build-state.json', 65536))
    m.require(state['target'] == spec['id'] and observed['identity'] == state['identity']
              and observed['identity']['source_revision'] == source, 'Built image differs from prepared identity')
    m.require(selected.get('source_revision') == source
              and selected.get('source_manifest_sha256') == state['identity']['source_manifest_sha256'],
              'Native selection differs from accepted source')
    output('identity_sha256', m.sha(m.canonical(observed['identity'])))


def publish(spec, args):
    """Advance stable to the signed candidate.

    The candidate's signature is verified here, once, by a strict pull. The previous
    stable's identity is the one preflight verified; the digest guard right before
    the write proves stable still names that image.
    """
    source = accepted_source()
    state = m.document(m.read(args.work / 'build-state.json', 65536))
    m.require(state['target'] == spec['id'] and state['source'] == source
              and state['run_id'] == int(os.environ['GITHUB_RUN_ID'])
              and state['run_attempt'] == int(os.environ['GITHUB_RUN_ATTEMPT']), 'Build artifact belongs to another run/target')
    digest = os.environ['KEDRA_SIGNED_DIGEST']
    host = host_trust(spec, args.work, os.environ['APPROVED_FINGERPRINT'])
    candidate = verified_image(spec, digest, args.work, 'candidate-final', host)
    m.require(candidate['identity'] == state['identity'], 'Signed image identity differs from reviewed build')
    m.require(candidate['identity']['source_revision'] == source
              and candidate['identity']['run_number'] == int(os.environ['GITHUB_RUN_NUMBER']), 'Signed source/run rank differs')
    m.order(candidate['identity'], state['previous_identity'], digest, state['previous_digest'], spec['id'])
    unchanged(spec, state['previous_digest'], source)
    receipt = args.work / 'stable-publication.json'
    write(receipt, {'state': 'submitting', 'previous_digest': state['previous_digest'], 'digest': digest, 'source': source})
    # The registry manifest PUT is atomic; workflow concurrency serializes Kedra writers.
    # Pre/post checks detect changes by other writers. Never retry an uncertain PUT.
    run('sudo', 'skopeo', '--policy', host['policy'], '--registries.d', host['registries'], 'copy', '--preserve-digests',
        '--authfile', os.environ['KEDRA_REGISTRY_AUTH'],
        'docker://' + spec['repository'] + '@' + digest, 'docker://' + spec['stable'])
    m.require(manifest_digest(spec['stable']) == digest, 'Stable readback differs; preserve receipt for recovery')
    write(receipt, {'state': 'verified', 'previous_digest': state['previous_digest'], 'digest': digest, 'source': source})
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as summary:
        summary.write(f'Verified signed stable image: {spec["repository"]}@{digest}. No machine was staged or rebooted.\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('resolve-base', 'prepare', 'inspect-candidate', 'publish'))
    parser.add_argument('--target', choices=sorted(m.TARGETS), required=True)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--store', type=Path, help='Private engine store for the RPM lock objects (prepare)')
    args = parser.parse_args()
    spec = target(args.target)
    args.work.mkdir(parents=True, exist_ok=True)
    if args.operation == 'prepare' and args.store is None:
        parser.error('prepare needs --store')
    if args.operation == 'resolve-base':
        print(resolve_base(spec, args.work))
    else:
        {'prepare': prepare, 'inspect-candidate': inspect_candidate, 'publish': publish}[args.operation](spec, args)
