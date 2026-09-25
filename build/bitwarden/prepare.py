"""Prepare pinned native Bitwarden files for image construction in Actions."""
import argparse
import hashlib
import http.client
import io
import json
import os
import pathlib
import re
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request

# The closed release target table selects the architecture; inputs.json pins
# exactly one official package per architecture.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / 'release'))
from material import TARGETS  # noqa: E402

parser = argparse.ArgumentParser()
parser.add_argument('--target', choices=sorted(TARGETS), required=True)
parser.add_argument('--context', type=pathlib.Path, required=True)
parser.add_argument('--evidence', type=pathlib.Path, required=True)
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true' or not args.context.is_dir():
    raise SystemExit('Use an explicit Actions build context')
pins = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())
architecture = TARGETS[args.target]['architecture']
package = pins['packages'].get(architecture)
if package is None:
    raise RuntimeError(f'No pinned Bitwarden package for {architecture}')

# Reviewed flat layout of the official 2026.8.0 arm64 tarball (97 entries).
# Any other name, link, special entry or special permission bit needs review.
TARBALL_FILES = {
    'LICENSE.electron.txt', 'LICENSES.chromium.html', 'bitwarden', 'bitwarden-app', 'chrome-sandbox',
    'chrome_100_percent.pak', 'chrome_200_percent.pak', 'chrome_crashpad_handler', 'desktop_proxy',
    'icudtl.dat', 'libEGL.so', 'libGLESv2.so', 'libffmpeg.so', 'libprocess_isolation.so',
    'libvk_swiftshader.so', 'libvulkan.so.1', 'resources.pak', 'snapshot_blob.bin',
    'v8_context_snapshot.bin', 'vk_swiftshader_icd.json', 'resources/app-update.yml', 'resources/app.asar',
    'resources/apparmor-profile', 'resources/com.bitwarden.desktop.desktop', 'resources/package-type',
    'resources/app.asar.unpacked/node_modules/@bitwarden/desktop-napi/desktop_napi.linux-arm64-gnu.node',
    'resources/app.asar.unpacked/node_modules/@bitwarden/desktop-napi/index.js',
}
TARBALL_DIRECTORIES = {
    '.', 'locales', 'resources', 'resources/icons', 'resources/app.asar.unpacked',
    'resources/app.asar.unpacked/node_modules', 'resources/app.asar.unpacked/node_modules/@bitwarden',
    'resources/app.asar.unpacked/node_modules/@bitwarden/desktop-napi',
}
TARBALL_LOCALE = re.compile(r'locales/[a-z]{2,3}(-[A-Z0-9]{2,3})?\.pak')
TARBALL_ICON = re.compile(r'resources/icons/(16|32|64|128|256|512|1024)x\1\.png')
TARBALL_ICON_SIZES = {'16', '32', '64', '128', '256', '512', '1024'}

def transient(error):
    if isinstance(error, urllib.error.HTTPError):
        return error.code == 429 or error.code >= 500
    return isinstance(error, (urllib.error.URLError, TimeoutError, ConnectionError, http.client.HTTPException))

def download(record, path):
    digest, size = hashlib.sha256(), 0
    with path.open('xb') as output, urllib.request.urlopen(record['url'], timeout=60) as response:
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > record['size']:
                raise RuntimeError('Input exceeded pinned size')
            digest.update(chunk)
            output.write(chunk)
    if size != record['size'] or digest.hexdigest() != record['sha256']:
        raise RuntimeError('Pinned Bitwarden input mismatch')

def fetch(record, path):
    # Retry only transport failures; the pinned size and digest checks stay final.
    for attempt in range(1, 5):
        try:
            download(record, path)
            return
        except (OSError, http.client.HTTPException) as error:
            if isinstance(error, FileExistsError) or not transient(error) or attempt == 4:
                raise
            path.unlink()
            print(f'Transient download failure for {path.name}: {error}; retrying', flush=True)
            time.sleep(15 * attempt)

def rpm_entries(rpm, root):
    identity = subprocess.check_output(['rpm', '-qp', '--qf', '%{NAME} %{VERSION} %{ARCH}', str(rpm)]).decode()
    if identity != f'bitwarden {pins["version"]} {architecture}':
        raise RuntimeError('Unexpected RPM identity')
    listing = subprocess.check_output(['rpm', '-qpl', str(rpm)]).decode().splitlines()
    if not listing or any(not name.startswith(('/opt/Bitwarden/', '/usr/share/icons/hicolor/'))
                          and name != '/usr/share/applications/bitwarden.desktop' for name in listing):
        raise RuntimeError('Review changed RPM layout')
    extracted = root / 'extracted'
    extracted.mkdir()
    # Extract trusted pinned data as the ordinary runner. Never execute RPM scripts.
    with (root / 'payload.cpio').open('xb') as output:
        subprocess.run(['rpm2cpio', str(rpm)], stdout=output, check=True)
    with (root / 'payload.cpio').open('rb') as payload:
        subprocess.run(['cpio', '--extract', '--make-directories', '--no-absolute-filenames', '--quiet'],
                       stdin=payload, cwd=extracted, check=True)
    entries = {}
    for path in extracted.rglob('*'):
        if path.is_symlink() or not (path.is_dir() or path.is_file()):
            raise RuntimeError('Unexpected RPM link or special entry')
        if path.is_dir():
            continue
        name = path.relative_to(extracted).as_posix()
        if name.startswith('opt/Bitwarden/'):
            name = 'usr/lib/bitwarden/' + name.removeprefix('opt/Bitwarden/')
        data = path.read_bytes()
        if name == 'usr/share/applications/bitwarden.desktop':
            old = b'Exec=/opt/Bitwarden/bitwarden %U'
            if data.count(old) != 1:
                raise RuntimeError('Unexpected desktop entry')
            data = data.replace(old, b'Exec=/usr/bin/bitwarden %U')
        entries[name] = (data, 0o755 if path.stat().st_mode & 0o111 else 0o644)
    return entries, {'rpm_identity': identity}

def tarball_entries(tarball):
    if architecture != 'aarch64':
        raise RuntimeError('The reviewed tarball layout is arm64-only')
    entries, directories, locales, icons = {}, set(), set(), set()
    # Read members from the verified archive into memory; nothing is extracted to
    # disk, so links, devices and path traversal cannot reach the runner.
    with tarfile.open(tarball, mode='r|gz') as archive:
        for member in archive:
            if member.name != '.' and not member.name.startswith('./'):
                raise RuntimeError('Unexpected tarball root')
            name = member.name.removeprefix('./') or '.'
            if name != '.' and (name.startswith('/') or '\\' in name
                                or any(part in ('', '.', '..') for part in name.split('/'))):
                raise RuntimeError('Unsafe tarball path')
            if member.mode & 0o7000:
                raise RuntimeError('Setuid, setgid or sticky tarball entry')
            if member.isdir():
                if name not in TARBALL_DIRECTORIES or name in directories:
                    raise RuntimeError('Review changed tarball directory layout')
                directories.add(name)
                continue
            if not member.isfile() or member.size > 512 * 1024 * 1024:
                raise RuntimeError('Unexpected tarball link, special or oversized entry')
            if TARBALL_LOCALE.fullmatch(name):
                locales.add(name)
            elif match := TARBALL_ICON.fullmatch(name):
                icons.add(match.group(1))
            elif name not in TARBALL_FILES:
                raise RuntimeError('Review changed tarball layout')
            destination = 'usr/lib/bitwarden/' + name
            if destination in entries:
                raise RuntimeError('Duplicate tarball entry')
            with archive.extractfile(member) as stream:
                data = stream.read()
            if len(data) != member.size:
                raise RuntimeError('Truncated tarball entry')
            entries[destination] = (data, 0o755 if member.mode & 0o111 else 0o644)
    present = {name.removeprefix('usr/lib/bitwarden/') for name in entries} - locales
    present -= {f'resources/icons/{size}x{size}.png' for size in icons}
    if (present != TARBALL_FILES or icons != TARBALL_ICON_SIZES or directories != TARBALL_DIRECTORIES
            or 'locales/en-US.pak' not in locales):
        raise RuntimeError('Incomplete reviewed tarball layout')
    # The upstream entry names the application icon com.bitwarden.desktop and
    # starts `bitwarden` from PATH; pin it to the image wrapper instead.
    desktop = entries['usr/lib/bitwarden/resources/com.bitwarden.desktop.desktop'][0]
    old = b'\nExec=bitwarden %u\n'
    if (desktop.count(b'Exec=') != 1 or desktop.count(old) != 1
            or desktop.count(b'\nIcon=com.bitwarden.desktop\n') != 1):
        raise RuntimeError('Unexpected desktop entry')
    entries['usr/share/applications/com.bitwarden.desktop.desktop'] = (
        desktop.replace(old, b'\nExec=/usr/bin/bitwarden %u\n'), 0o644)
    for size in sorted(icons):
        data = entries[f'usr/lib/bitwarden/resources/icons/{size}x{size}.png'][0]
        entries[f'usr/share/icons/hicolor/{size}x{size}/apps/com.bitwarden.desktop.png'] = (data, 0o644)
    return entries, {'tarball_layout': 'reviewed flat arm64 layout', 'locales': len(locales)}

with tempfile.TemporaryDirectory(prefix='kedra-bitwarden-', dir=os.environ['RUNNER_TEMP']) as temporary:
    root = pathlib.Path(temporary)
    downloaded, source = root / f'bitwarden.{package["format"]}', root / 'source.tar.gz'
    fetch(package, downloaded)
    fetch(pins['source'], source)
    if package['format'] == 'rpm':
        entries, details = rpm_entries(downloaded, root)
        origin = (b'Unmodified Bitwarden runtime from the pinned official RPM. Kedra relocates\n'
                  b'its files from /opt/Bitwarden to /usr/lib/bitwarden for bootc image updates\n'
                  b'and adjusts only the desktop launcher. Upstream scripts are not executed.\n')
    elif package['format'] == 'tarball':
        entries, details = tarball_entries(downloaded)
        origin = (b'Unmodified Bitwarden runtime from the pinned official arm64 tarball. Kedra\n'
                  b'places its flat tree at /usr/lib/bitwarden for bootc image updates, installs\n'
                  b'its desktop entry with an absolute launcher path and copies its hicolor icons.\n')
    else:
        raise RuntimeError('Unsupported Bitwarden package format')
    entries['usr/bin/bitwarden'] = (b'#!/bin/sh\nexec /usr/lib/bitwarden/bitwarden "$@"\n', 0o755)
    licenses = 'usr/share/licenses/kedra-bitwarden/'
    entries[licenses + 'clients-source.tar.gz'] = (source.read_bytes(), 0o644)
    with tarfile.open(source) as archive:
        prefix = f'clients-{pins["source"]["revision"]}/'
        for name in ['LICENSE.txt', 'LICENSE_GPL.txt', 'LICENSE_BITWARDEN.txt']:
            member = archive.getmember(prefix + name)
            if not member.isfile() or member.size > 1024 * 1024:
                raise RuntimeError('Unexpected source notice')
            entries[licenses + name] = (archive.extractfile(member).read(), 0o644)
    entries[licenses + 'README.txt'] = (
        origin +
        b'Electron/Chromium notices remain beside the runtime. Corresponding client\n'
        b'source, license terms, build scripts and dependency locks are included here.\n'
        b'Kedra is independent of Bitwarden Inc. Updates arrive through OS images.\n', 0o644)
    receipt = {'schema_version': pins['schema_version'], 'version': pins['version'], 'target': args.target,
               'architecture': architecture, 'package': package, 'source': pins['source'], **details,
               'runtime_root': '/usr/lib/bitwarden', 'package_scripts_executed': False, 'files': len(entries)}
    entries['usr/share/sysroot/bitwarden.json'] = ((json.dumps(receipt, indent=2) + '\n').encode(), 0o644)
    with tarfile.open(args.context / 'bitwarden.tar', 'x', format=tarfile.GNU_FORMAT) as archive:
        for name, (data, mode) in sorted(entries.items()):
            member = tarfile.TarInfo(name)
            member.size, member.mode, member.mtime = len(data), mode, 0
            member.uid = member.gid = 0
            archive.addfile(member, io.BytesIO(data))
    args.evidence.write_text(json.dumps(receipt, indent=2) + '\n')
print(f'Pinned native Bitwarden image input prepared for {args.target} ({architecture})')
