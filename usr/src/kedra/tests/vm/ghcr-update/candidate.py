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
    # The release's single package resolution: an RPM lock of fixed-output store
    # objects, from which the foundation installs offline (image/catalog/install-rpms.sh).
    store = work / 'store'
    lock_path = work / 'rpm-lock.json'
    image = ROOT / 'usr/src/kedra/image'
    run('resolve-rpm-lock', [binaries / 'sysroot', 'catalog', 'lock', 'resolve',
                             '--input-root', image / 'packages', '--entry', 'catalog.kedra',
                             '--lock', 'packages.lock.json', '--target', 'qemu-arm64',
                             '--target-policy', image / 'package-policy.json', '--base', args.base_image,
                             '--store', store, '--log-dir', work / 'rpm-resolution', '--output', lock_path],
        timeout=3600)
    inputs = m.resolved_inputs(ROOT, plan, args.base_image, context, binaries,
                               m.read(lock_path, m.LOCK_LIMIT), 'qemu-arm64')
    m.require(context.joinpath('install-rpms.sh').is_file(), 'Build context lacks install-rpms.sh')
    m.require(not context.joinpath('rpms').exists(), 'Build context already has an RPM repository')
    run('foundation-rpms', [binaries / 'sysroot', 'catalog', 'lock', 'repo', '--store', store,
                            '--lock', lock_path, '--foundation', '--output', context / 'rpms'], timeout=1800)
    inputs_path = work / 'resolved-inputs.json'
    inputs_path.write_bytes(m.canonical(inputs))
    common = ['--target', 'qemu-arm64', '--repo', ROOT, '--source-revision', source_revision,
              '--inputs', inputs_path, '--expected-inputs-sha256', sha(inputs_path),
              '--store', store, '--binaries', binaries]
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
