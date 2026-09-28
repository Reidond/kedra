#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Prepare the pinned native QEMU/HVF/ANGLE runtime in a private prefix."""
import argparse
import fcntl
import hashlib
import json
import os
import platform
import plistlib
import shlex
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE / 'build-tools'


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def run(argv, *, cwd=None, env=None, capture=False):
    print('+ ' + shlex.join(map(str, argv)), flush=True)
    return subprocess.run(list(map(str, argv)), cwd=cwd, env=env, check=True,
                          text=True, capture_output=capture, timeout=7200).stdout


def uv(*args):
    return ['uv', 'run', '--project', str(TOOLS), '--locked', *map(str, args)]


def download(entry, path):
    if not path.is_file() or sha256(path) != entry['sha256']:
        pending = path.with_suffix('.download')
        run(['curl', '--fail', '--location', '--retry', '3', '--proto', '=https', '-o', pending, entry['url']])
        if sha256(pending) != entry['sha256']:
            raise ValueError(f'download checksum mismatch: {path.name}')
        os.replace(pending, path)


def checkout(root, name, pin):
    path = root / name
    if not path.exists():
        run(['git', 'init', '-q', path])
        run(['git', '-C', path, 'remote', 'add', 'origin', pin['url']])
    revision = subprocess.run(['git', '-C', path, 'rev-parse', 'HEAD'], capture_output=True, text=True, check=False).stdout.strip()
    if revision != pin['revision']:
        run(['git', '-C', path, 'diff', '--exit-code'])
        run(['git', '-C', path, 'fetch', '--depth', '1', 'origin', pin['revision']])
        run(['git', '-C', path, 'checkout', '--detach', 'FETCH_HEAD'])
    run(['git', '-C', path, 'diff', '--exit-code'])
    return path


def dependencies(binary):
    result = run(['otool', '-L', binary], capture=True)
    return [line.strip().split(' (', 1)[0] for line in result.splitlines()[1:]]


def bundle(prefix, destination):
    """Copy only runtime executables and their dylib closure; retain no Homebrew paths."""
    (destination / 'bin').mkdir(parents=True)
    (destination / 'lib').mkdir()
    queue = []
    for name in ['qemu-system-aarch64', 'qemu-img', 'swtpm']:
        source = prefix / 'bin' / name
        shutil.copy2(source, destination / 'bin' / name)
        queue.append((source, destination / 'bin' / name))
    # Epoxy dlopens these, so they are roots even when not in Mach-O load commands.
    for name in ['libEGL.dylib', 'libGLESv2.dylib', 'libGLESv1_CM.dylib']:
        source = prefix / 'lib' / name
        shutil.copy2(source, destination / 'lib' / name)
        queue.append((source, destination / 'lib' / name))
    seen = set()
    while queue:
        source, target = queue.pop()
        if target in seen:
            continue
        seen.add(target)
        for reference in dependencies(source):
            if reference.startswith(('/usr/lib/', '/System/Library/')):
                continue
            candidate = Path(reference)
            if reference.startswith('@') or not candidate.is_absolute():
                candidate = prefix / 'lib' / Path(reference).name
            if not candidate.is_file() or 'UTM.app' in str(candidate):
                raise ValueError(f'unresolved or forbidden runtime dependency: {reference}')
            # A dylib's first entry may be its own install name.
            if candidate.resolve() == source.resolve():
                continue
            copied = destination / 'lib' / candidate.name
            if not copied.exists():
                shutil.copy2(candidate, copied)
                queue.append((candidate, copied))
            replacement = '@loader_path/' + os.path.relpath(copied, target.parent)
            run(['install_name_tool', '-change', reference, replacement, target])
        if target.suffix == '.dylib':
            run(['install_name_tool', '-id', '@loader_path/' + target.name, target])
    entitlement = destination / 'hypervisor.plist'
    entitlement.write_bytes(plistlib.dumps({'com.apple.security.hypervisor': True}))
    for path in sorted(seen):
        arguments = ['codesign', '--force', '--sign', '-']
        if path.name == 'qemu-system-aarch64':
            arguments += ['--entitlements', entitlement]
        run([*arguments, path])
        run(['codesign', '--verify', '--strict', path])
        for reference in dependencies(path):
            if not reference.startswith(('/usr/lib/', '/System/Library/', '@loader_path/')):
                raise ValueError(f'nonportable load command: {reference}')
    # Give Cocoa/LaunchServices a real application identity for window selection.
    app = destination / 'Kedra QEMU.app'
    executable = app / 'Contents/MacOS/qemu-system-aarch64'
    executable.parent.mkdir(parents=True)
    shutil.copy2(destination / 'bin/qemu-system-aarch64', executable)
    for reference in dependencies(executable):
        if reference.startswith('@loader_path/'):
            run(['install_name_tool', '-change', reference, '@loader_path/../../../lib/' + Path(reference).name, executable])
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier': 'dev.kedra.qemu-lab', 'CFBundleName': 'Kedra QEMU',
        'CFBundleDisplayName': 'Kedra QEMU', 'CFBundleExecutable': 'qemu-system-aarch64',
        'CFBundlePackageType': 'APPL', 'CFBundleVersion': '1', 'NSHighResolutionCapable': True,
    }))
    run(['codesign', '--force', '--sign', '-', '--entitlements', entitlement, app])
    run(['codesign', '--verify', '--strict', app])
    return seen


def prepare(args):
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        raise ValueError('this runtime builds on Apple Silicon macOS')
    pins = json.loads((HERE / 'inputs.json').read_text())
    work = args.workdir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    if args.runtime.exists():
        run(['uv', 'run', '--offline', '--script', HERE / 'check-runtime.py', '--runtime', args.runtime])
        print('Reusing verified runtime: ' + str(args.runtime))
        return
    with (work / 'prepare.lock').open('w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if built(work):
            finish(args, pins, work, work / 'prefix')
            return
        run(['uv', 'sync', '--project', TOOLS, '--locked', '--python', '3.12'])
        run(['xcrun', 'metal', '--version'])
        for name, pin in pins['sources'].items():
            patches = pins.get('patches', {}).get(name, [])
            for entry in patches:
                patch = HERE / entry['path']
                if sha256(patch) != entry['sha256']:
                    raise ValueError('runtime patch checksum mismatch')
                applied = subprocess.run(['git', '-C', str(work / name), 'apply', '--reverse', '--check', str(patch)],
                                         capture_output=True, check=False)
                if applied.returncode == 0:
                    run(['git', '-C', work / name, 'apply', '--reverse', patch])
            checkout(work, name, pin)
            for entry in patches:
                run(['git', '-C', work / name, 'apply', HERE / entry['path']])
        prefix = work / 'prefix'
        prefix.mkdir(exist_ok=True)
        shims = work / 'shims'
        shims.mkdir(exist_ok=True)
        for name in ['python3', 'uv-python', 'vpython3']:
            shim = shims / name
            shim.write_text('#!/bin/sh\nexec ' + shlex.join(uv('python')) + ' "$@"\n')
            shim.chmod(0o755)
        env = dict(os.environ, DEPOT_TOOLS_UPDATE='0',
                   GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_TERMINAL_PROMPT='0',
                   PATH=str(shims) + ':' + str(work / 'depot_tools') + ':' + os.environ['PATH'],
                   PKG_CONFIG_PATH=str(prefix / 'lib/pkgconfig') + ':' + os.environ.get('PKG_CONFIG_PATH', ''))
        angle = work / 'angle'
        (angle / '.gclient').write_text("solutions = [{'name': '.', 'url': " + repr(pins['sources']['angle']['url']) +
                                      ", 'managed': False, 'custom_vars': {'checkout_angle_cl_deps': False, "
                                      "'checkout_angle_dawn_deps': False, 'checkout_angle_internal': False, "
                                      "'checkout_angle_mesa': False, 'checkout_angle_restricted_traces': False}}]\n"
                                      "target_os = ['mac']\ntarget_os_only = True\ntarget_cpu = ['arm64']\ntarget_cpu_only = True\n")
        gclient = uv('python', work / 'depot_tools/gclient.py')
        run([*gclient, 'sync', '--nohooks', '--noprehooks', '--no-history', '--shallow', '--jobs', '4', '--ignore-dep-type', 'gcs'], cwd=angle, env=env)
        run([*gclient, 'revinfo', '-a', '--output-json', work / 'angle-revisions.json'], cwd=angle, env=env)
        revisions = json.loads((work / 'angle-revisions.json').read_text())
        revisions = {key: value for key, value in revisions.items() if not value['url'].startswith('gs:')}
        if revisions != json.loads((HERE / 'angle-dependencies.json').read_text()):
            raise ValueError('ANGLE dependency closure differs from its lock')
        llvm = angle / 'third_party/llvm-build/Release+Asserts'
        llvm.mkdir(parents=True, exist_ok=True)
        for name, entry in pins['archives'].items():
            archive = work / (name + '.tar.xz')
            download(entry, archive)
            with tarfile.open(archive) as packed:
                packed.extractall(llvm, filter='data')
        (llvm / 'cr_build_revision').write_text('llvmorg-23-init-2224-g5bd8dadb-3,mac\n')
        lto = llvm / 'lib/libLTO.dylib'
        if not lto.exists():
            developer = run(['xcode-select', '-p'], capture=True).strip()
            lto.symlink_to(Path(developer) / 'Toolchains/XcodeDefault.xctoolchain/usr/lib/libLTO.dylib')
        out = angle / 'out/kedra'
        out.mkdir(parents=True, exist_ok=True)
        arguments = {'is_debug': False, 'is_component_build': False, 'target_cpu': 'arm64',
                     'angle_enable_metal': True, 'angle_enable_gl': False, 'angle_enable_vulkan': False,
                     'angle_enable_null': False, 'angle_enable_swiftshader': False, 'angle_enable_cl': False,
                     'angle_build_tests': False, 'build_angle_deqp_tests': False, 'use_system_xcode': True,
                     'clang_use_chrome_plugins': False, 'use_custom_libcxx': False, 'use_lld': False,
                     'use_siso': False, 'use_remoteexec': False, 'enable_rust': False, 'enable_rust_cxx': False,
                     'treat_warnings_as_errors': False, 'install_prefix': str(prefix)}
        (out / 'args.gn').write_text('\n'.join(f'{key} = {json.dumps(value)}' for key, value in arguments.items()) + '\n')
        run([angle / 'buildtools/mac/gn', 'gen', out, '--root=' + str(angle), '--script-executable=' + str(shims / 'uv-python')], env=env)
        run(['ninja', '-C', out, '-j', str(args.jobs), 'install_angle'], env=env)
        for name, options in [('libepoxy', ['-Degl=yes', '-Dglx=no', '-Dx11=false', '-Dtests=false', '-Dc_args=-I' + str(prefix / 'include')]),
                              ('virglrenderer', ['-Dplatforms=egl', '-Dvenus=false', '-Dtests=false', '-Dfuzzer=false', '-Dvideo=false', '-Ddrm-renderers=[]'])]:
            build = work / 'build' / name
            reconfigure = ['--wipe'] if (build / 'meson-private/coredata.dat').exists() else []
            run(uv('meson', 'setup', *reconfigure, build, work / name, '--prefix=' + str(prefix), '--buildtype=release', *options), env=env)
            run(uv('meson', 'install', '-C', build), env=env)
        crypto = dict(env, CPPFLAGS=run(['pkg-config', '--cflags', 'openssl'], capture=True).strip(),
                      LDFLAGS=run(['pkg-config', '--libs-only-L', 'openssl'], capture=True).strip())
        for name, options in [('libtpms', ['--without-tpm1']), ('swtpm', ['--without-cuse', '--without-seccomp', '--disable-tests'])]:
            run(['./autogen.sh', '--prefix=' + str(prefix), '--with-openssl', '--disable-static', *options], cwd=work / name, env=crypto)
            run(['make', '-j' + str(args.jobs)], cwd=work / name, env=crypto)
            run(['make', 'install'], cwd=work / name, env=crypto)
        build = work / 'build/qemu'
        build.mkdir(parents=True, exist_ok=True)
        run(uv('sh', work / 'qemu/configure', '--prefix=' + str(prefix), '--target-list=aarch64-softmmu',
               '--without-default-features', '--enable-hvf', '--enable-cocoa', '--enable-opengl',
               '--enable-virglrenderer', '--enable-slirp', '--enable-tools', '--enable-tpm', '--enable-pixman',
               '--enable-coreaudio', '--disable-docs', '--disable-guest-agent', '--disable-rust'), cwd=build, env=env)
        run(uv('ninja', '-C', build, '-j', str(args.jobs), 'install'), env=env)
        write_build_receipt(work, prefix)
        finish(args, pins, work, prefix)


def build_identity():
    return {name: sha256(HERE / name) for name in ['inputs.json', 'angle-dependencies.json', 'build-tools/uv.lock', 'prepare-runtime.py']}


def write_build_receipt(work, prefix):
    paths = [prefix / 'bin' / name for name in ['qemu-system-aarch64', 'qemu-img', 'swtpm']]
    paths += list((prefix / 'lib').glob('*.dylib'))
    receipt = {'schema_version': 1, 'inputs': build_identity(),
               'files': {str(path.relative_to(prefix)): sha256(path) for path in paths}}
    (work / 'build-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')


def built(work):
    record = work / 'build-receipt.json'
    if not record.is_file():
        return False
    receipt = json.loads(record.read_text())
    if receipt.get('schema_version') != 1 or receipt.get('inputs') != build_identity():
        return False
    files = receipt.get('files', {})
    if not {'bin/qemu-system-aarch64', 'bin/qemu-img', 'bin/swtpm'}.issubset(files):
        return False
    prefix = work / 'prefix'
    for name, checksum in files.items():
        path = prefix / name
        if not path.resolve().is_relative_to(prefix) or not path.is_file() or sha256(path) != checksum:
            return False
    return True


def finish(args, pins, work, prefix):
    # Publish only after every binary, dylib, firmware and signature check succeeds.
    destination = Path(tempfile.mkdtemp(prefix='bundle-', dir=work))
    destination.rmdir()
    bundle(prefix, destination)
    licenses = destination / 'licenses'
    licenses.mkdir()
    for name in pins['sources']:
        for source in (work / name).iterdir():
            if source.is_file() and source.name.upper().startswith(('COPYING', 'LICENSE', 'NOTICE')):
                shutil.copy2(source, licenses / (name + '-' + source.name))
    archive = work / 'firmware.deb'
    download(pins['firmware'], archive)
    firmware = work / 'firmware'
    firmware.mkdir(exist_ok=True)
    run(['ar', '-x', archive], cwd=firmware)
    run(['tar', '-xf', firmware / 'data.tar.zst', '-C', firmware], env=dict(os.environ, PATH='/opt/homebrew/bin:' + os.environ['PATH']))
    (destination / 'firmware').mkdir()
    for field in ['code', 'vars']:
        source = firmware / 'usr/share/AAVMF' / pins['firmware'][field]
        if sha256(source) != pins['firmware'][field + '_sha256']:
            raise ValueError('firmware file checksum mismatch')
        shutil.copy2(source, destination / 'firmware' / source.name)
    variables = run(uv('virt-fw-vars', '--input', destination / 'firmware' / pins['firmware']['vars'], '--print', '--verbose'), capture=True)
    blocks = variables.split('name=')
    if not (any(block.startswith('db ') and 'subject CN=Microsoft UEFI CA 2023' in block for block in blocks)
            and any(block.startswith('SecureBootEnable ') and 'bool: ON' in block for block in blocks)
            and any(block.startswith('PK ') and 'subject CN=' in block for block in blocks)):
        raise ValueError('firmware lacks the required Secure Boot enrollment')
    (destination / 'firmware/variables.txt').write_text(variables)
    env_runtime = dict(os.environ, DYLD_FALLBACK_LIBRARY_PATH=str(destination / 'lib'))
    for name in ['qemu-system-aarch64', 'qemu-img', 'swtpm']:
        run([destination / 'bin' / name, '--version'], env=env_runtime)
    receipt = {'schema_version': 1, 'inputs_sha256': sha256(HERE / 'inputs.json'),
               'recipe_sha256': sha256(Path(__file__)),
               'angle_dependencies_sha256': sha256(HERE / 'angle-dependencies.json'),
               'python_lock_sha256': sha256(TOOLS / 'uv.lock'), 'gpu_qualified': False,
               'xcode': run(['xcodebuild', '-version'], capture=True),
               'files': {str(p.relative_to(destination)): sha256(p) for p in destination.rglob('*') if p.is_file()}}
    (destination / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    args.runtime.parent.mkdir(parents=True, exist_ok=True)
    os.rename(destination, args.runtime)
    print('Prepared runtime: ' + str(args.runtime) + '; GPU qualification still required')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', required=True, type=Path)
    parser.add_argument('--workdir', required=True, type=Path)
    parser.add_argument('--jobs', type=int, default=8)
    args = parser.parse_args()
    args.runtime = args.runtime.resolve()
    if not 1 <= args.jobs <= 32:
        parser.error('--jobs must be between 1 and 32')
    prepare(args)


if __name__ == '__main__':
    main()
