#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Prepare the pinned Codex and Bitwarden image inputs for a boot-level driver.

A hosted runner runs image/agents/prepare.sh and image/bitwarden/prepare.py
directly. Those release scripts accept no other host; on a declared disposable
host (common/disposable_host.py) they run where the container harness runs them
for full local builds: the disposable Fedora 44 input container of
container/lab/inputs.Containerfile with KEDRA_LOCAL_BUILDER=1, as the invoking
user, with the checkout mounted read-only. Either way the scripts write
agents.tar and bitwarden.tar into the build context and their input records
agent-inputs.json and bitwarden-inputs.json into the evidence directory.
"""
import argparse
import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
TARGETS = ('desktop', 'qemu-arm64')
CONTAINERFILE = ROOT / 'usr/src/kedra/tests/container/lab/inputs.Containerfile'
IMAGE = 'localhost/kedra-boot-inputs:latest'
AGENTS = ['bash', 'usr/src/kedra/image/agents/prepare.sh']
BITWARDEN = ['uv', 'run', 'usr/src/kedra/image/bitwarden/prepare.py']


def run(command):
    subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, check=True)


def on_runner(target, context, evidence):
    run([*AGENTS, target, str(context), str(Path(os.environ['RUNNER_TEMP']) / 'kedra-agent-inputs'),
         str(evidence / 'agent-inputs.json')])
    run([*BITWARDEN, '--target', target, '--context', str(context),
         '--evidence', str(evidence / 'bitwarden-inputs.json')])


def in_container(target, context, evidence):
    with tempfile.TemporaryDirectory(prefix='kedra-inputs-build-') as empty:
        run(['sudo', 'podman', 'build', '--file', str(CONTAINERFILE), '--tag', IMAGE, empty])
    # A private home keeps uv's interpreter and cache between the two runs.
    home = Path(tempfile.mkdtemp(prefix='kedra-inputs-home-', dir=os.environ['RUNNER_TEMP']))
    container = ['sudo', 'podman', 'run', '--rm', '--user', f'{os.getuid()}:{os.getgid()}',
                 '--env', 'KEDRA_LOCAL_BUILDER=1', '--env', 'RUNNER_TEMP=/tmp', '--env', 'HOME=/inputs-home',
                 '--volume', f'{ROOT}:/src:ro', '--volume', f'{context}:/context',
                 '--volume', f'{evidence}:/evidence', '--volume', f'{home}:/inputs-home',
                 '--workdir', '/src', IMAGE]
    run([*container, *AGENTS, target, '/context', '/tmp/kedra-agent-inputs', '/evidence/agent-inputs.json'])
    run([*container, *BITWARDEN, '--target', target, '--context', '/context',
         '--evidence', '/evidence/bitwarden-inputs.json'])


def prepare(target, context, evidence):
    if target not in TARGETS:
        raise SystemExit(f'Unknown target {target}')
    context, evidence = Path(context).resolve(strict=True), Path(evidence).resolve(strict=True)
    if os.environ.get('GITHUB_ACTIONS') == 'true':
        on_runner(target, context, evidence)
    elif os.environ.get('KEDRA_DISPOSABLE_HOST') == '1':
        in_container(target, context, evidence)
    else:
        raise SystemExit('Prepare image inputs only on a hosted runner or declared disposable host')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--target', choices=TARGETS, required=True)
    parser.add_argument('--context', type=Path, required=True, help='Existing image build context')
    parser.add_argument('--evidence', type=Path, required=True, help='Existing evidence directory')
    args = parser.parse_args()
    try:
        prepare(args.target, args.context, args.evidence)
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'prepare inputs: {error}') from error


if __name__ == '__main__':
    main()
