#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Prepare the full unsigned ARM candidate for existing Actions VM fixtures.

The normal release composer owns composition and native derivation. This fixture
preparer shares deterministic material construction but has no main/publication
override and no signing authority.
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

from fixture import add_context_argument, load_context

ROOT = Path(__file__).resolve().parents[6]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'usr/src/kedra/image/release'))
import compose
import material as m


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def interrupted(number, _frame):
    raise InterruptedError('Interrupted by signal ' + str(number))


def stop_group(process):
    # uv may exit before compose.py has drained its separate-session child.
    # Keep the group alive for that bounded cleanup before escalating.
    if process.returncode is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        process.wait(timeout=5)
        return
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        process.poll()
        try:
            os.killpg(process.pid, 0)
        except ProcessLookupError:
            process.wait(timeout=5)
            return
        time.sleep(0.1)
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait(timeout=5)


def main():
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--context', required=True, type=Path)
    parser.add_argument('--source-plan', required=True, type=Path)
    parser.add_argument('--base-image', required=True)
    parser.add_argument('--work', required=True, type=Path)
    parser.add_argument('--diagnostic-directory', type=Path,
                        help='Existing public evidence directory; defaults to fixture evidence')
    parser.add_argument('--tag', required=True, choices=(
        'localhost/kedra-qemu-arm64:research', 'localhost/kedra-ghcr-arm:composed'))
    add_context_argument(parser)
    args = parser.parse_args()
    fixture = load_context(args.fixture_context)
    diagnostics = args.diagnostic_directory or Path(fixture['evidence'])
    source_revision = fixture['source_revision']
    m.require(re.fullmatch('[a-f0-9]{40}', source_revision), 'Missing dispatched source revision')
    m.require(re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', args.base_image),
              'Expected reviewed immutable Fedora base')
    work = args.work.absolute()
    m.require(not work.exists() and not work.is_symlink() and work.parent.is_dir(), 'Work directory must be new')
    work.mkdir(mode=0o700)
    context = args.context.resolve(strict=True)
    m.require(not args.context.is_symlink() and context.is_dir()
              and not any(c in str(context) + str(work) for c in ':,\r\n'), 'Unsafe build paths')
    sequence = 0

    def run(label, argv, timeout=2400, capture=False, accepted=(0,)):
        nonlocal sequence
        sequence += 1
        logfile = work / f'{sequence:02d}-{label}.log'
        with logfile.open('xb') as log:
            arguments = list(map(str, argv))
            process = subprocess.Popen(arguments, cwd=ROOT, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE if capture else log, stderr=log,
                                       start_new_session=True)
            try:
                output, _ = process.communicate(timeout=timeout)
            except BaseException as error:
                previous = {number: signal.signal(number, signal.SIG_IGN)
                            for number in (signal.SIGTERM, signal.SIGINT)}
                try:
                    stop_group(process)
                finally:
                    for number, handler in previous.items():
                        signal.signal(number, handler)
                if label in ('foundation', 'compose'):
                    compose.publish_failure(diagnostics / 'candidate-failure.json', label,
                                            error, {'step': label, 'tool': 'uv',
                                                    'exit_code': process.returncode,
                                                    'logs': (('combined', logfile),)})
                raise
        if label in ('foundation', 'compose') and process.returncode not in accepted:
            compose.publish_failure(diagnostics / 'candidate-failure.json', label,
                                    None, {'step': label, 'tool': 'uv', 'exit_code': process.returncode,
                                           'logs': (('combined', logfile),)})
        m.require(process.returncode in accepted, label + ' failed; see ' + str(logfile))
        if capture:
            m.require(len(output) <= 8 * 1024**2, 'Oversized fixture response')
        return subprocess.CompletedProcess(arguments, process.returncode, output or b'')

    head = run('source-head', ['git', '--no-replace-objects', '-C', ROOT, 'rev-parse', 'HEAD'], capture=True).stdout.decode().strip()
    m.require(head == source_revision, 'Fixture source differs from dispatch')
    plan = m.document(m.read(args.source_plan, 1024**2))
    m.require(plan.get('source_revision') == source_revision, 'Fixture source plan is stale')
    binaries = work / 'binaries'
    binaries.mkdir(mode=0o700)
    for name in ('sysroot', 'sysroot-helper', 'kedra-lab'):
        shutil.copyfile(Path(fixture['binaries']) / name, binaries / name)
        (binaries / name).chmod(0o500)
    resolution = work / 'resolution'
    resolution.mkdir(mode=0o700)
    nonce = uuid.uuid4().hex
    resolver = {'schema_version': 1, 'name': 'kedra-release-resolver-' + nonce,
                'invocation': nonce, 'id': None}
    resolver_path = work / 'resolver.json'
    resolver_path.write_bytes(m.canonical(resolver))
    podman = ['/usr/bin/sudo', '--non-interactive', '/usr/bin/podman']
    absent = run('resolver-name-absence', [*podman, 'container', 'exists', resolver['name']],
                 timeout=30, accepted=(0, 1))
    m.require(absent.returncode == 1, 'Resolver name already exists')

    def inspect_resolver():
        records = m.document(run('inspect-resolver', [*podman, 'container', 'inspect', resolver['name']],
                                 timeout=30, capture=True).stdout)
        m.require(isinstance(records, list) and len(records) == 1, 'Unexpected resolver inspection')
        record = records[0]
        labels = record.get('Config', {}).get('Labels', {})
        identifier = record.get('Id', '')
        m.require(re.fullmatch('[a-f0-9]{64}', identifier)
                  and record.get('Name') == resolver['name']
                  and labels.get('dev.kedra.lab.owner') == 'kedra-release-fixture'
                  and labels.get('dev.kedra.lab.kind') == 'package-resolver'
                  and labels.get('dev.kedra.lab.invocation') == nonce
                  and resolver['id'] in (None, identifier), 'Resolver ownership or identity changed')
        return identifier

    try:
        created = run('create-resolver', [
            *podman, 'create', '--pull=always', '--name', resolver['name'],
            '--label', 'dev.kedra.lab.owner=kedra-release-fixture',
            '--label', 'dev.kedra.lab.kind=package-resolver',
            '--label', 'dev.kedra.lab.invocation=' + nonce,
            '--volume', str(context) + ':/context:ro', '--volume', str(resolution) + ':/resolution',
            '--entrypoint', '/bin/bash', args.base_image, '-euc',
            ('tar -xf /context/payload.tar -C /; tar -xf /context/agents.tar -C /; '
            'tar -xf /context/bitwarden.tar -C /; mkdir -p /usr/libexec/sysroot; '
            'cp /context/sysroot /usr/bin/sysroot; cp /context/sysroot-helper /usr/libexec/sysroot/helper; '
            '/bin/bash /context/assemble.sh --resolve-packages'),
        ], capture=True)
        resolver['id'] = created.stdout.decode().strip()
        m.require(re.fullmatch('[a-f0-9]{64}', resolver['id']), 'Invalid created resolver identity')
        resolver_path.write_bytes(m.canonical(resolver))
        inspect_resolver()
        run('resolve-packages', [*podman, 'start', '--attach', resolver['id']])
    finally:
        previous = {number: signal.signal(number, signal.SIG_IGN)
                    for number in (signal.SIGTERM, signal.SIGINT)}
        cleanup = {**resolver, 'removed': False, 'cleanup_failed': True}
        try:
            exists = run('resolver-cleanup-exists', [*podman, 'container', 'exists', resolver['name']],
                         timeout=30, accepted=(0, 1))
            if exists.returncode == 0:
                resolver['id'] = inspect_resolver()
                resolver_path.write_bytes(m.canonical(resolver))
                cleanup['id'] = resolver['id']
                run('remove-resolver', [*podman, 'rm', '--force', '--time', '10', resolver['id']], timeout=30)
                absent = run('resolver-removed', [*podman, 'container', 'exists', resolver['id']],
                             timeout=30, accepted=(0, 1))
                m.require(absent.returncode == 1, 'Resolver still exists after removal')
            cleanup.update(removed=True, cleanup_failed=False)
        finally:
            (work / 'resolver-cleanup.json').write_bytes(m.canonical(cleanup))
            for number, handler in previous.items():
                signal.signal(number, handler)
    run('own-material', ['sudo', 'chown', str(os.getuid()) + ':' + str(os.getgid()),
                         resolution / 'package-material.txt'])
    inputs = m.resolved_inputs(ROOT, plan, args.base_image, context, binaries,
                               m.read(resolution / 'package-material.txt'), 'qemu-arm64')
    inputs_path = work / 'resolved-inputs.json'
    inputs_path.write_bytes(m.canonical(inputs))
    common = ['--target', 'qemu-arm64', '--repo', ROOT, '--source-revision', source_revision,
              '--inputs', inputs_path, '--expected-inputs-sha256', sha(inputs_path),
              '--store', work / 'store', '--binaries', binaries]
    composer = ROOT / 'usr/src/kedra/image/release/compose.py'
    run('foundation', ['uv', 'run', composer, 'foundation', *common,
                       '--diagnostic-output', diagnostics / 'foundation-failure.json',
                       '--context', context, '--output-dir', work / 'foundation'])
    foundation = work / 'foundation/foundation.json'
    run('compose', ['uv', 'run', composer, 'compose', *common,
                    '--diagnostic-output', diagnostics / 'compose-failure.json',
                    '--foundation-receipt', foundation, '--expected-foundation-receipt-sha256', sha(foundation),
                    '--output-dir', work / 'native'])
    selected = m.document(m.read(work / 'native/candidate.json'))
    transfer = selected['transfer']
    raw = run('read-final-manifest', ['sudo', 'skopeo', 'inspect', '--raw',
                                     'containers-storage:' + transfer['reference']], capture=True).stdout
    m.require('sha256:' + m.sha(raw) == transfer['destination_manifest_digest'], 'Candidate transfer changed')
    names = run('refuse-existing-tag', ['sudo', 'podman', 'image', 'list', '--filter',
                                       'reference=' + args.tag, '--format', '{{.ID}}'], capture=True).stdout
    m.require(not names.strip(), 'Fixture output tag already exists')
    run('tag-candidate', ['sudo', 'podman', 'tag', transfer['image'], args.tag])
    report = {'schema_version': 1, 'target': 'qemu-arm64', 'source_revision': source_revision,
              'candidate': selected, 'reference': args.tag, 'signed': False,
              'production_publication': False, 'fresh_installation_performed': False}
    (work / 'candidate.json').write_bytes(m.canonical(report))
    print(json.dumps(report, sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit('candidate fixture: ' + str(error)) from error
