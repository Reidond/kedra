#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Graphical login and session of the desktop candidate under UEFI Secure Boot.

The steps of the removed test-desktop.yml, on a declared disposable x86_64 host
with KVM (common/disposable_host.py): build the full desktop candidate from the
committed payload and pinned inputs, layer this directory's non-promotable
observer, build a disposable disk with a generated account, then boot it through
common/run_vm.py under Ubuntu's Microsoft-enrolled OVMF. run_vm.py types the
generated password into tuigreet on VT 1 and drives the Xwayland and Qt file
choosers; check.sh asserts Secure Boot, SELinux, the PAM keyring and the session.
Installed image contents are container scenarios; this qualifies boot and login.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
HERE = Path(__file__).resolve().parent
COMMON = HERE.parents[1] / 'common'
sys.path.insert(0, str(COMMON))
import candidate_disk as step
import disposable_host

EVIDENCE = 'output/desktop-evidence'
CANDIDATE = 'localhost/kedra-desktop:research'
OBSERVER = 'localhost/kedra-desktop-test:r07'
OBSERVER_FILES = (HERE / 'Containerfile', HERE / 'check.sh', HERE / 'check.service',
                  COMMON / 'test-profile.toml', COMMON / 'console.toml',
                  # The guest toolkit fixture is shared with the container scenarios.
                  HERE.parents[1] / 'container/lab/probes/toolkit-app.py')
COMMANDS = ('sudo', 'podman', 'skopeo', 'openssl', 'sha256sum', 'dpkg-query', 'qemu-system-x86_64',
            'virt-fw-vars', 'xvfb-run', 'xauth', 'xdotool', 'import')
# The removed job's bound; run_vm.py bounds the guest session itself.
VM_SECONDS = 60 * 60


def build_candidate(work, evidence):
    step.prepare_payload('desktop', work / 'context', evidence)
    base = step.resolve_base(evidence, 'amd64')
    step.environment(evidence / 'environment.txt', [
        ['date', '--utc', '--iso-8601=seconds'], ['uname', '-a'], ['free', '-h'], ['df', '-h'],
        ['podman', '--version'], ['skopeo', '--version']])
    step.to_file(evidence / 'build.log', [
        *step.PODMAN, 'build', '--pull=always', '--no-cache', '--build-arg', f'BASE_IMAGE={base}',
        '--tag', CANDIDATE, str(work / 'context')], combined=True)
    # Installed-image contents (programs, validated defaults, unit enablement,
    # toolkit packages, Bitwarden, Codex) are container scenarios; this VM
    # qualifies boot and login.
    step.record_candidate(CANDIDATE, evidence)


def boot(work, evidence, private):
    step.run(['uv', 'run', str(COMMON / 'secure_boot.py'), 'provenance', '--evidence', str(evidence)])
    disk = step.single_disk(work / 'image')
    # sudo resets PATH; hand root the pinned uv by absolute path.
    uv = shutil.which('uv')
    step.bounded(['sudo', 'xvfb-run', '-a', '-s', '-screen 0 1280x768x24', 'env', 'LIBGL_ALWAYS_SOFTWARE=1',
                  uv, 'run', str(COMMON / 'run_vm.py'), '--disk', str(disk), '--work', str(evidence / 'vm'),
                  '--password-file', str(private / 'password')], evidence / 'vm-result.txt', VM_SECONDS)


def main():
    context = disposable_host.enter(evidence=EVIDENCE, architecture='x86_64', kvm=True, commands=COMMANDS)
    work = Path(context['runner_temp']) / 'kedra-r07'
    private = step.private_directory(Path(context['runner_temp']) / 'kedra-r07-private')
    evidence = ROOT / EVIDENCE
    builder = json.loads((step.IMAGE / 'inputs.json').read_text())['platforms']['amd64']['builder']
    try:
        build_candidate(work, evidence)
        step.build_observer(OBSERVER_FILES, OBSERVER, work / 'test-context', evidence)
        step.write_blueprint(private, keep_password=True)
        step.build_disk(builder, OBSERVER, work / 'image', private)
        boot(work, evidence, private)
    except (subprocess.CalledProcessError, RuntimeError) as error:
        raise SystemExit(f'desktop: {error}; inspect {EVIDENCE}') from error
    finally:
        step.sanitize(private, evidence, ('password', 'blueprint.toml', 'builder.log'))


if __name__ == '__main__':
    main()
