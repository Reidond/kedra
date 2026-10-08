#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Native private agent runtime test on a disposable host.

The steps of the removed test-agents.yml for the release target of this host's
architecture: fetch the pinned official Codex inputs, verify their signatures
into the private image package, install the bundled runtime where the image
puts it, then run probe.py as the invoking user without network access. It
installs into /usr/libexec/sysroot/agents, so it runs only on a declared
disposable host (common/disposable_host.py).
"""
import json
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'common'))
import disposable_host

EVIDENCE = 'output/r05-evidence'
INSTALLED = Path('/usr/libexec/sysroot/agents/codex')
AGENTS = 'usr/src/kedra/image/agents/'


def native_target():
    table = json.loads((ROOT / 'usr/src/kedra/image/release/targets.json').read_text())
    machine = platform.machine()
    matches = [name for name, item in table['targets'].items() if item['architecture'] == machine]
    if len(matches) != 1:
        raise SystemExit(f'No single release target runs natively on {machine}')
    return matches[0]


def run(command, **options):
    return subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, check=True, **options)


def fetch_and_package(target, inputs, temporary, evidence):
    run(['uv', 'run', AGENTS + 'fetch.py', '--target', target, '--output', str(inputs), '--research-tools'])
    shutil.copyfile(ROOT / AGENTS / 'inputs.json', evidence / 'inputs.json')
    shutil.copytree(inputs / 'notices', evidence / 'notices', symlinks=True)
    with (evidence / 'notice-hashes.txt').open('w') as hashes:
        run(['sha256sum', *map(str, sorted((inputs / 'notices').iterdir()))], stdout=hashes)
    run(['uv', 'run', AGENTS + 'package.py', '--target', target, '--inputs', str(inputs),
         '--output', str(temporary / 'kedra-agents.tar'), '--evidence', str(evidence / 'image-package.json')])


def install_bundled(inputs):
    run(['sudo', 'mkdir', '-p', str(INSTALLED.parent)])
    run(['sudo', 'cp', '-a', str(inputs / 'codex'), str(INSTALLED)])
    run(['sudo', 'chown', '-R', 'root:root', str(INSTALLED)])


def probe(inputs, temporary, evidence):
    # Drop privileges before running checkout test code, with no network access.
    # sudo resets PATH and HOME: pass uv by path, this home and an offline cache
    # (the probe script has no dependencies).
    command = ['sudo', 'unshare', '--net', '--', 'setpriv', f'--reuid={os.getuid()}', f'--regid={os.getgid()}',
               '--init-groups', 'env', f'HOME={os.environ["HOME"]}',
               f'UV_CACHE_DIR={temporary / "kedra-r05-uv-cache"}', 'UV_OFFLINE=1',
               shutil.which('uv'), 'run', str(HERE / 'probe.py'), '--sysroot', str(ROOT / 'target/release/sysroot'),
               '--packages', str(inputs), '--work', str(temporary / 'kedra-r05-probe'), '--evidence', str(evidence)]
    result = subprocess.run(command, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, text=True,
                            check=False)
    print(result.stdout, end='', flush=True)
    (evidence / 'probe-result.txt').write_text(result.stdout)
    if result.returncode:
        raise SystemExit(f'agents: probe exited {result.returncode}')


def main():
    context = disposable_host.enter(evidence=EVIDENCE, commands=('sudo', 'unshare', 'setpriv', 'sha256sum'))
    if INSTALLED.exists():
        raise SystemExit(f'agents: {INSTALLED} already exists; use a fresh disposable host')
    target = native_target()
    temporary, evidence = Path(context['runner_temp']), ROOT / EVIDENCE
    inputs = temporary / 'kedra-r05-inputs'
    try:
        fetch_and_package(target, inputs, temporary, evidence)
        install_bundled(inputs)
    except subprocess.CalledProcessError as error:
        raise SystemExit(f'agents: {error}') from error
    probe(inputs, temporary, evidence)


if __name__ == '__main__':
    main()
