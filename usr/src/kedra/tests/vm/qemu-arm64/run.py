#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""TCG UEFI Secure Boot boot of the qemu-arm64 candidate (README.md in this directory).

The steps of the removed test-qemu-arm64.yml, on a declared disposable native
aarch64 host (common/disposable_host.py): compose the full candidate with the
release composer through vm/ghcr-update/candidate.py, require its bootc lint,
platform and bootc contract, layer this directory's non-promotable observer,
build a disposable disk with a generated account whose password is never
stored, then boot it through boot.sh, which requires every guest marker.
The hosted runner's Docker had to be switched to the containerd image store
first; this driver requires that store instead of reconfiguring the daemon, and
runs the unchanged public image-retention preflight (retention.py) before any
candidate work.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
HERE = Path(__file__).resolve().parent
COMMON = HERE.parents[1] / 'common'
sys.path.insert(0, str(COMMON))
import candidate_disk as step
import disposable_host
import retention

EVIDENCE = 'output/qemu-arm64-evidence'
CANDIDATE = 'localhost/kedra-qemu-arm64:research'
OBSERVER = 'localhost/kedra-qemu-arm64-test:secureboot'
OBSERVER_FILES = (HERE / 'Containerfile', HERE / 'check.sh', HERE / 'check.service')
COMMANDS = ('sudo', 'docker', 'podman', 'skopeo', 'openssl', 'sha256sum', 'jq', 'dpkg-query',
            'qemu-system-aarch64', 'virt-fw-vars')
# The removed boot step's bound; boot.sh bounds the emulator to 90 minutes.
BOOT_SECONDS = 100 * 60


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def compose_candidate(work, evidence):
    source = step.prepare_payload('qemu-arm64', work / 'context', evidence)
    target = source['target']
    require(target['id'] == 'qemu-arm64' and target['architecture'] == 'aarch64' and target['candidate_target'],
            'Source plan is not the qemu-arm64 candidate target')
    base = step.resolve_base(evidence, 'arm64')
    step.environment(evidence / 'environment.txt', [
        ['date', '--utc', '--iso-8601=seconds'], ['uname', '-a'], ['nproc'], ['free', '-h'], ['df', '-h'],
        ['podman', '--version'], ['skopeo', '--version']])
    step.to_file(evidence / 'composed-candidate.json', [
        'uv', 'run', str(HERE.parent / 'ghcr-update/candidate.py'), '--diagnostic-directory', str(evidence),
        '--context', str(work / 'context'), '--source-plan', str(evidence / 'source.json'), '--base-image', base,
        '--work', str(work / 'candidate'), '--tag', CANDIDATE])


def check_candidate(evidence):
    step.to_file(evidence / 'final-lint.log',
                 [*step.PODMAN, 'run', '--rm', '--network=none', CANDIDATE, 'bootc', 'container', 'lint'])
    step.record_candidate(CANDIDATE, evidence)
    inspected = json.loads((evidence / 'image-inspect.json').read_text())
    require(inspected[0]['Os'] == 'linux' and inspected[0]['Architecture'] == 'arm64',
            'Candidate image platform is not linux/arm64')
    # Image contents (guest agent RPC block list, graphics defaults, kargs,
    # Bitwarden, Codex, boot-chain packages) are container scenarios; the boot
    # below needs this contract.
    contract = json.loads((step.IMAGE / 'release/compatibility.json').read_text())['bootc_version']
    installed = step.output([*step.PODMAN, 'run', '--rm', CANDIDATE, 'rpm', '-q', '--qf', '%{VERSION} %{ARCH}', 'bootc']).rstrip('\n')
    require(installed == f'{contract} aarch64', f'bootc {installed} differs from the {contract} contract')


def build_disk(work, evidence, private):
    builder = json.loads((step.IMAGE / 'inputs.json').read_text())['platforms']['arm64']['builder']
    step.to_file(evidence / 'builder-pull.log', [*step.PODMAN, 'pull', builder], combined=True)
    platform = step.output([*step.PODMAN, 'image', 'inspect', builder, '--format', '{{.Os}}/{{.Architecture}}'])
    require(platform.strip() == 'linux/arm64', 'Pinned image builder is not linux/arm64')
    step.build_disk(builder, OBSERVER, work / 'image', private)
    # The emulator needs no privilege: hand only the generated disk to this user.
    step.run(['sudo', 'chown', '-R', f'{os.getuid()}:{os.getgid()}', str(work / 'image')])
    step.to_file(evidence / 'disk-space.txt', ['df', '-h'])


def boot(work, evidence):
    disk = step.single_disk(work / 'image')
    digest = step.output([*step.PODMAN, 'image', 'inspect', OBSERVER, '--format', '{{.Digest}}']).strip()
    step.bounded(['bash', str(HERE / 'boot.sh'), str(disk), str(work / 'vm'), str(evidence / 'vm'), digest],
                 evidence / 'vm-result.txt', BOOT_SECONDS)


def main():
    context = disposable_host.enter(evidence=EVIDENCE, architecture='aarch64', docker_containerd=True,
                                    commands=COMMANDS)
    work = Path(context['runner_temp']) / 'kedra-qemu-arm64'
    private = step.private_directory(Path(context['runner_temp']) / 'kedra-qemu-arm64-private')
    evidence = ROOT / EVIDENCE
    if retention.check(evidence, step.SYSROOT):
        raise SystemExit(f'qemu-arm64: the Docker backend did not retain the public probe image; inspect {EVIDENCE}')
    try:
        compose_candidate(work, evidence)
        check_candidate(evidence)
        step.build_observer(OBSERVER_FILES, OBSERVER, work / 'test-context', evidence)
        # Headless run: the account only lets the observer run doctor as a
        # non-root user, so the generated password is never written down.
        step.write_blueprint(private, keep_password=False)
        build_disk(work, evidence, private)
        boot(work, evidence)
    except (subprocess.CalledProcessError, RuntimeError, KeyError) as error:
        raise SystemExit(f'qemu-arm64: {error}; inspect {EVIDENCE}') from error
    finally:
        step.sanitize(private, evidence, ('blueprint.toml', 'builder.log'))


if __name__ == '__main__':
    main()
