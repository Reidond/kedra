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
import subprocess
import sys
from pathlib import Path

from fixture import add_context_argument, load_context

ROOT = Path(__file__).resolve().parents[6]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'usr/src/kedra/image/release'))
import material as m


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--context', required=True, type=Path)
    parser.add_argument('--source-plan', required=True, type=Path)
    parser.add_argument('--base-image', required=True)
    parser.add_argument('--work', required=True, type=Path)
    parser.add_argument('--tag', required=True, choices=(
        'localhost/kedra-qemu-arm64:research', 'localhost/kedra-ghcr-arm:composed'))
    add_context_argument(parser)
    args = parser.parse_args()
    fixture = load_context(args.fixture_context)
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

    def run(label, argv, timeout=2400, capture=False):
        nonlocal sequence
        sequence += 1
        logfile = work / f'{sequence:02d}-{label}.log'
        with logfile.open('xb') as log:
            result = subprocess.run(list(map(str, argv)), cwd=ROOT, stdin=subprocess.DEVNULL,
                                    stdout=subprocess.PIPE if capture else log, stderr=log,
                                    check=False, timeout=timeout)
        m.require(result.returncode == 0, label + ' failed; see ' + str(logfile))
        if capture:
            m.require(len(result.stdout) <= 8 * 1024**2, 'Oversized fixture response')
            return result.stdout
        return b''

    head = run('source-head', ['git', '--no-replace-objects', '-C', ROOT, 'rev-parse', 'HEAD'], capture=True).decode().strip()
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
    run('resolve-packages', [
        'sudo', 'podman', 'run', '--rm', '--pull=always',
        '--volume', str(context) + ':/context:ro', '--volume', str(resolution) + ':/resolution',
        '--entrypoint', '/bin/bash', args.base_image, '-euc',
        ('tar -xf /context/payload.tar -C /; tar -xf /context/agents.tar -C /; '
        'tar -xf /context/bitwarden.tar -C /; mkdir -p /usr/libexec/sysroot; '
        'cp /context/sysroot /usr/bin/sysroot; cp /context/sysroot-helper /usr/libexec/sysroot/helper; '
        '/bin/bash /context/assemble.sh --resolve-packages'),
    ])
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
                       '--context', context, '--output-dir', work / 'foundation'])
    foundation = work / 'foundation/foundation.json'
    run('compose', ['uv', 'run', composer, 'compose', *common,
                    '--foundation-receipt', foundation, '--expected-foundation-receipt-sha256', sha(foundation),
                    '--output-dir', work / 'native'])
    selected = m.document(m.read(work / 'native/candidate.json'))
    transfer = selected['transfer']
    raw = run('read-final-manifest', ['sudo', 'skopeo', 'inspect', '--raw',
                                     'containers-storage:' + transfer['reference']], capture=True)
    m.require('sha256:' + m.sha(raw) == transfer['destination_manifest_digest'], 'Candidate transfer changed')
    names = run('refuse-existing-tag', ['sudo', 'podman', 'image', 'list', '--filter',
                                       'reference=' + args.tag, '--format', '{{.ID}}'], capture=True)
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
