#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Re-admit an explicitly pinned local candidate after fixture-only corrections."""
import argparse
import json
import os
import re
import signal
import subprocess
import sys
import uuid
from pathlib import Path

from candidate import interrupted, stop_group
from fixture import ROOT, add_context_argument, load_context

sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'usr/src/kedra/image/release'))
import compose
import material as m

FIXTURE_PREFIX = 'usr/src/kedra/tests/vm/ghcr-update/'
FIXTURE_FILES = frozenset(FIXTURE_PREFIX + name for name in (
    'fixture.py', 'candidate.py', 'resume-arm64.py', 'prepare-arm64.py', 'prepare-install-arm64.py',
    'boot-arm64.py', 'run-arm64.sh', 'arm64.Containerfile', 'arm64-variant.Containerfile',
    'controller.Containerfile', 'controller-entrypoint.sh', 'check.py', 'control.py',
    'arm64-check.service', 'identity-recovery.py'))
MATERIAL_FILES = {
    'source.json': '/usr/share/sysroot/source.json',
    'package-material.txt': '/usr/share/sysroot/package-material.txt',
    'native-receipt.json': '/usr/share/sysroot/native-receipt.json',
}


def documentation(path):
    return path in ('README.md', 'worklog.md') or path.startswith(('.specs/', 'usr/src/kedra/docs/'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    add_context_argument(parser)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    fixture = load_context(args.fixture_context)
    m.require(fixture['mode'] == 'local' and fixture['retained_candidate'] is not None,
              'Retained admission is available only for an explicitly selected local candidate')
    root = args.root.resolve(strict=True)
    m.require(root == Path(fixture['runner_temp']) / 'kedra-ghcr', 'Unexpected fixture root')
    work = compose.private_directory(root / 'candidate', create=True)
    sequence = 0
    environment = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_OPTIONAL_LOCKS='0')

    def run(label, arguments, accepted=(0,)):
        nonlocal sequence
        sequence += 1
        with (work / f'{sequence:02d}-{label}.stderr').open('xb') as log:
            process = subprocess.Popen(list(map(str, arguments)), cwd=ROOT, env=environment,
                                       stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=log,
                                       start_new_session=True)
            try:
                output, _ = process.communicate(timeout=60)
            except BaseException:
                previous = {number: signal.signal(number, signal.SIG_IGN)
                            for number in (signal.SIGTERM, signal.SIGINT)}
                try:
                    stop_group(process)
                finally:
                    for number, handler in previous.items():
                        signal.signal(number, handler)
                raise
        m.require(process.returncode in accepted and len(output) <= 8 * 1024**2,
                  label + ' failed or exceeded its output bound')
        return subprocess.CompletedProcess(arguments, process.returncode, output)

    def git(*arguments):
        return run('source-proof', ['/usr/bin/git', '--no-replace-objects', '-C', ROOT, *arguments]).stdout

    prior = Path(fixture['retained_candidate'])
    candidate, candidate_bytes = compose.read_json(prior, fixture['retained_candidate_sha256'], 65536)
    m.require(set(candidate) == {'schema_version', 'target', 'source_revision', 'candidate', 'reference',
                                'signed', 'production_publication', 'fresh_installation_performed'}
              and type(candidate['schema_version']) is int and candidate['schema_version'] == 1
              and candidate['target'] == 'qemu-arm64' and candidate['source_revision'] == fixture['source_revision']
              and candidate['reference'] == 'localhost/kedra-ghcr-arm:composed'
              and candidate['signed'] is False and candidate['production_publication'] is False
              and candidate['fresh_installation_performed'] is False, 'Unsupported retained unsigned candidate')
    selected = candidate['candidate']
    m.require(selected.get('source_revision') == fixture['source_revision'], 'Candidate source binding differs')
    inputs, input_bytes = compose.read_json(prior.parent / 'resolved-inputs.json',
                                            selected['input_material_sha256'], m.LIMIT)
    compose.validate_inputs(inputs, input_bytes)
    git('merge-base', '--is-ancestor', fixture['source_revision'], fixture['fixture_revision'])
    changed = git('diff', '--no-ext-diff', '--no-textconv', '--no-renames', '--name-only', '-z',
                  fixture['source_revision'], fixture['fixture_revision'], '--').decode().split('\0')
    changed = [name for name in changed if name]
    m.require(all(name in FIXTURE_FILES or documentation(name) for name in changed),
              'Retained candidates cannot cross a product, build, signer or payload change')
    dirty = git('diff', '--no-ext-diff', '--no-textconv', '--no-renames', '--name-only', '-z',
                'HEAD', '--').decode().split('\0')
    m.require(all(not name or documentation(name) for name in dirty), 'Runtime source has uncommitted changes')
    for name in FIXTURE_FILES:
        actual = compose.ordinary(ROOT / name)
        m.require(m.read(actual) == git('cat-file', 'blob', fixture['fixture_revision'] + ':' + name),
                  'Fixture tool differs from its selected commit: ' + name)
    for name, expected in inputs['recipes'].items():
        path = Path(name)
        m.require(not path.is_absolute() and all(part not in ('', '.', '..', '.git') for part in path.parts),
                  'Unsafe production recipe name')
        data = m.read(compose.ordinary(ROOT / path))
        m.require(data == git('cat-file', 'blob', fixture['source_revision'] + ':' + name),
                  'Production recipe differs from original candidate source: ' + name)
        if name == m.NATIVE_RECIPE:
            data = m.canonical(m.document(data))
        m.require(m.sha(data) == expected, 'Production recipe differs from retained material: ' + name)
    for name, expected in fixture['binary_sha256'].items():
        m.require(inputs['artifacts'].get(name) == expected, 'Candidate executable provenance differs: ' + name)
    engine = m.document(run('docker-engine', ['/usr/bin/docker', 'info', '--format', '{{json .}}']).stdout)
    m.require(engine.get('ID') == selected['engine'] and engine.get('OSType') == 'linux'
              and engine.get('Architecture') in ('arm64', 'aarch64'), 'Candidate belongs to another Docker engine')
    transfer = selected['transfer']
    identifier = compose.image_id(transfer['image'])
    reference = transfer['reference']
    m.require(re.fullmatch(r'localhost/kedra-compose:[a-f0-9]{32}', reference), 'Unexpected retained transfer reference')
    podman = ['/usr/bin/sudo', '--non-interactive', '/usr/bin/podman']
    skopeo = ['/usr/bin/sudo', '--non-interactive', '/usr/bin/skopeo']

    def image_matches(reference):
        rows = m.document(run('image-inspect', [*podman, 'image', 'inspect', reference]).stdout)
        m.require(isinstance(rows, list) and len(rows) == 1, 'Unexpected retained image inspection')
        observed = rows[0]
        m.require('sha256:' + observed['Id'].removeprefix('sha256:') == identifier
                  and observed.get('Architecture') == 'arm64' and observed.get('Os') == 'linux',
                  'Retained image reference or platform changed')
        return observed

    observed = image_matches(reference)
    image_matches(candidate['reference'])
    manifest = run('image-manifest', [*skopeo, 'inspect', '--raw', 'containers-storage:' + reference]).stdout
    config_bytes = run('image-config', [*skopeo, 'inspect', '--config', '--raw', 'containers-storage:' + reference]).stdout
    m.require('sha256:' + m.sha(manifest) == transfer['destination_manifest_digest']
              and 'sha256:' + m.sha(config_bytes) == transfer['config_digest'] == identifier,
              'Retained image manifest or config changed')
    config = m.document(config_bytes)
    m.require(config.get('config') == observed.get('Config')
              and config.get('rootfs', {}).get('diff_ids') == observed.get('RootFS', {}).get('Layers'),
              'Retained config or filesystem lineage differs')
    nonce = uuid.uuid4().hex
    observer = {'schema_version': 1, 'name': 'kedra-retained-observer-' + nonce,
                'invocation': nonce, 'id': None}
    observer_path = work / 'retained-observer.json'
    compose.write_json(observer_path, observer)
    exists = run('observer-name-absence', [*podman, 'container', 'exists', observer['name']], (0, 1))
    m.require(exists.returncode == 1, 'Retained observer name already exists')

    def inspect_observer():
        rows = m.document(run('observer-inspect', [*podman, 'container', 'inspect', observer['name']]).stdout)
        m.require(isinstance(rows, list) and len(rows) == 1, 'Unexpected retained observer inspection')
        value = rows[0]
        labels = value.get('Config', {}).get('Labels', {})
        found = value.get('Id', '')
        m.require(re.fullmatch('[a-f0-9]{64}', found) and value.get('Name') == observer['name']
                  and labels.get('dev.kedra.lab.owner') == 'kedra-release-fixture'
                  and labels.get('dev.kedra.lab.invocation') == nonce
                  and observer['id'] in (None, found), 'Retained observer ownership changed')
        return found

    try:
        created = run('create-observer', [*podman, 'create', '--pull=never', '--name', observer['name'],
            '--label', 'dev.kedra.lab.owner=kedra-release-fixture', '--label', 'dev.kedra.lab.invocation=' + nonce,
            '--entrypoint', '/usr/bin/true', identifier]).stdout.decode().strip()
        m.require(re.fullmatch('[a-f0-9]{64}', created), 'Invalid retained observer identity')
        observer['id'] = created
        observer_path.write_bytes(m.canonical(observer))
        inspect_observer()
        for name, source in MATERIAL_FILES.items():
            run('copy-material', [*podman, 'cp', created + ':' + source, work / name])
            compose.ordinary(work / name)
            run('own-material', ['/usr/bin/sudo', '--non-interactive', '/usr/bin/chown', '--no-dereference',
                                str(os.getuid()) + ':' + str(os.getgid()), '--', work / name])
            compose.ordinary(work / name)
    finally:
        previous = {number: signal.signal(number, signal.SIG_IGN)
                    for number in (signal.SIGTERM, signal.SIGINT)}
        cleanup = {**observer, 'removed': False, 'cleanup_failed': True}
        try:
            exists = run('observer-exists', [*podman, 'container', 'exists', observer['name']], (0, 1))
            if exists.returncode == 0:
                observer['id'] = inspect_observer()
                cleanup['id'] = observer['id']
                run('remove-observer', [*podman, 'rm', '--force', '--time', '5', observer['id']])
                absent = run('observer-removed', [*podman, 'container', 'exists', observer['id']], (0, 1))
                m.require(absent.returncode == 1, 'Retained observer remains after removal')
            cleanup.update(removed=True, cleanup_failed=False)
        finally:
            compose.write_json(work / 'retained-observer-cleanup.json', cleanup)
            for number, handler in previous.items():
                signal.signal(number, handler)
    for name, field in (('source.json', 'source_manifest_sha256'), ('native-receipt.json', 'native_receipt_sha256')):
        m.require(compose.hash_file(work / name) == selected[field], 'Retained installed material changed: ' + name)
    source, source_bytes = compose.read_json(work / 'source.json')
    m.require(source.get('source_revision') == fixture['source_revision']
              and {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')}
              == inputs['source'], 'Retained source identity differs')
    m.verify_native(config, m.read(work / 'native-receipt.json'), inputs, selected)
    packages = m.packages(m.read(work / 'package-material.txt'), 'aarch64')
    m.require(packages == inputs['packages'] and m.sha(compose.package_bytes(packages)) == selected['rpm_sha256'],
              'Retained RPM closure differs')
    image_matches(reference)
    image_matches(candidate['reference'])
    load_context(args.fixture_context)
    compose.write_new(work / 'resolved-inputs.json', input_bytes)
    compose.write_new(work / 'candidate.json', candidate_bytes)
    compose.write_new(root / 'source-plan.json', source_bytes)
    report = {'schema_version': 1, 'candidate_source_revision': fixture['source_revision'],
              'fixture_revision': fixture['fixture_revision'], 'selected_candidate_sha256': m.sha(candidate_bytes),
              'fixture_only_committed_changes': changed, 'manifest_digest': transfer['destination_manifest_digest'],
              'config_digest': identifier, 'rootfs_diff_ids': observed['RootFS']['Layers'],
              'native_receipt_sha256': selected['native_receipt_sha256'],
              'source_manifest_sha256': selected['source_manifest_sha256'], 'rpm_sha256': selected['rpm_sha256'],
              'production_recipes_and_executables': 'unchanged', 'observer_removed': True}
    compose.write_json(Path(fixture['evidence']) / 'retained-candidate-verification.json', report)
    print(json.dumps(candidate, sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit('retained candidate: ' + str(error)) from error
