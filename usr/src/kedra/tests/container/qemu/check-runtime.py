#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Read-only native QEMU prerequisite/receipt inspection; never installs tools."""
import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent


def command(argv):
    try:
        result = subprocess.run(argv, capture_output=True, text=True, timeout=10, check=False)
        output = (result.stdout or result.stderr).strip()
        return result.returncode == 0, output[:3000] if result.returncode == 0 else output[-1500:]
    except (OSError, subprocess.TimeoutExpired) as error:
        return False, str(error)


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load_pins():
    pins = json.loads((HERE / 'inputs.json').read_text())
    if (not isinstance(pins, dict) or pins.get('schema_version') != 1 or pins.get('platform') != 'macos-arm64'
            or any(not re.fullmatch('[a-f0-9]{40}', entry['revision'])
                   for entry in pins['sources'].values())):
        raise ValueError('unsupported or malformed native runtime inputs')
    return pins


def check_build_tools(pins, add):
    for name in ('clang', 'ninja', 'pkg-config', 'git', 'uv', 'autoconf', 'automake', 'glibtoolize'):
        found = shutil.which(name)
        add(name, found is not None, found or 'not found; install only with owner authorization')
    for name in ('glib-2.0', 'pixman-1', 'slirp', 'json-glib-1.0', 'openssl', 'libtasn1'):
        ok, detail = command(['pkg-config', '--modversion', name])
        add(name, ok, detail)
    # Do not run uv resolution from a read-only check: it could install Meson.
    meson = shutil.which('meson')
    ok, version = command(['uv', 'run', '--offline', '--no-project', meson, '--version']) if meson else (False, 'not installed; requires Meson ' + pins['build_tools']['meson'])
    add('meson', ok and version == pins['build_tools']['meson'], version)
    ok, version = command(['xcrun', 'metal', '--version'])
    add('metal-toolchain', ok, version)


def receipt_matches_inputs(receipt):
    required = {'bin/qemu-system-aarch64', 'bin/qemu-img', 'bin/swtpm',
                'Kedra QEMU.app/Contents/MacOS/qemu-system-aarch64',
                'Kedra QEMU.app/Contents/Info.plist',
                'firmware/AAVMF_CODE.secboot.fd', 'firmware/AAVMF_VARS.ms.fd'}
    return (isinstance(receipt, dict) and receipt.get('schema_version') == 1 and isinstance(receipt.get('files'), dict)
            and required.issubset(receipt['files'])
            and receipt.get('inputs_sha256') == sha256(HERE / 'inputs.json')
            and receipt.get('recipe_sha256') == sha256(HERE / 'prepare-runtime.py')
            and receipt.get('angle_dependencies_sha256') == sha256(HERE / 'angle-dependencies.json')
            and receipt.get('python_lock_sha256') == sha256(HERE / 'build-tools/uv.lock'))


def runtime_file_failures(runtime, files):
    failures = []
    for name, expected in files.items():
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts:
            failures.append('unsafe receipt path')
            break
        path = runtime / relative
        if (path.is_symlink() or not path.is_file()
                or not path.resolve().is_relative_to(runtime.resolve())
                or sha256(path) != expected):
            failures.append(name)
    return failures


def check_qemu_features(runtime, add):
    binary = str(runtime / 'bin/qemu-system-aarch64')
    for name, arguments, expected in [
        ('hvf', ['-accel', 'help'], 'hvf'),
        ('cocoa', ['-display', 'help'], 'cocoa'),
        ('virtio-gl', ['-device', 'virtio-gpu-gl-pci,help'], 'virtio-gpu-gl-pci options:'),
    ]:
        ok, output = command([binary, *arguments])
        add(name, ok and expected in output, output, 'runtime')


def check_runtime_receipt(runtime, add):
    receipt_path = runtime / 'receipt.json'
    if not receipt_path.is_file():
        add('runtime-receipt', False, 'no prepared standalone runtime at ' + str(runtime), 'runtime')
        return
    receipt = json.loads(receipt_path.read_text())
    if not receipt_matches_inputs(receipt):
        add('runtime-receipt', False, 'unsupported, incomplete or mismatched receipt', 'runtime')
        return
    failures = runtime_file_failures(runtime, receipt['files'])
    add('runtime-receipt', not failures,
        'missing/changed runtime files: ' + ', '.join(failures[:5]) if failures
        else 'runtime file hashes checked; GPU qualification is separate', 'runtime')
    if not failures:
        check_qemu_features(runtime, add)


def inspect(runtime):
    pins = load_pins()
    checks = []

    def add(name, passed, detail, category='build'):
        checks.append({'name': name, 'category': category,
                       'status': 'pass' if passed else 'blocked', 'detail': detail})

    add('host', platform.system() == 'Darwin' and platform.machine() == 'arm64',
        f'{platform.system()} {platform.release()} {platform.machine()}', 'runtime')
    check_build_tools(pins, add)
    add('dependency-lock', pins.get('closure_locked') is True,
        'complete ANGLE Git/CIPD/GCS and tool dependency closure required before preparation', 'runtime')
    check_runtime_receipt(runtime, add)
    return {'schema_version': 1, 'runtime': str(runtime), 'gpu_qualified': False,
            'build_prerequisites_ready': all(check['status'] == 'pass' for check in checks if check['category'] == 'build'),
            'ready_for_qualification': all(check['status'] == 'pass' for check in checks if check['category'] == 'runtime'),
            'checks': checks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', required=True, type=Path)
    args = parser.parse_args()
    try:
        result = inspect(args.runtime.expanduser().absolute())
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.exit(1, f'native runtime check: {error}\n')
    print(json.dumps(result, indent=2))
    return 0 if result['ready_for_qualification'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
