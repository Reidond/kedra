"""Prepare pinned native Bitwarden files for image construction in Actions."""
import argparse
import hashlib
import io
import json
import os
import pathlib
import subprocess
import tarfile
import tempfile
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('--context', type=pathlib.Path, required=True)
parser.add_argument('--evidence', type=pathlib.Path, required=True)
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true' or not args.context.is_dir():
    raise SystemExit('Use an explicit Actions build context')
pins = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())

def fetch(record, path):
    digest, size = hashlib.sha256(), 0
    with urllib.request.urlopen(record['url'], timeout=60) as response, path.open('xb') as output:
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > record['size']:
                raise RuntimeError('Input exceeded pinned size')
            digest.update(chunk)
            output.write(chunk)
    if size != record['size'] or digest.hexdigest() != record['sha256']:
        raise RuntimeError('Pinned Bitwarden input mismatch')

with tempfile.TemporaryDirectory(prefix='kedra-bitwarden-', dir=os.environ['RUNNER_TEMP']) as temporary:
    root = pathlib.Path(temporary)
    rpm, source = root / 'bitwarden.rpm', root / 'source.tar.gz'
    fetch(pins['rpm'], rpm)
    fetch(pins['source'], source)
    identity = subprocess.check_output(['rpm', '-qp', '--qf', '%{NAME} %{VERSION} %{ARCH}', str(rpm)]).decode()
    if identity != f'bitwarden {pins["version"]} x86_64':
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
        b'Unmodified Bitwarden runtime from the pinned official RPM. Kedra relocates\n'
        b'its files from /opt/Bitwarden to /usr/lib/bitwarden for bootc image updates\n'
        b'and adjusts only the desktop launcher. Upstream scripts are not executed.\n'
        b'Electron/Chromium notices remain beside the runtime. Corresponding client\n'
        b'source, license terms, build scripts and dependency locks are included here.\n'
        b'Kedra is independent of Bitwarden Inc. Updates arrive through OS images.\n', 0o644)
    receipt = {**pins, 'rpm_identity': identity, 'runtime_root': '/usr/lib/bitwarden',
               'rpm_scripts_executed': False, 'files': len(entries)}
    entries['usr/share/sysroot/bitwarden.json'] = ((json.dumps(receipt, indent=2) + '\n').encode(), 0o644)
    with tarfile.open(args.context / 'bitwarden.tar', 'x', format=tarfile.GNU_FORMAT) as archive:
        for name, (data, mode) in sorted(entries.items()):
            member = tarfile.TarInfo(name)
            member.size, member.mode, member.mtime = len(data), mode, 0
            member.uid = member.gid = 0
            archive.addfile(member, io.BytesIO(data))
    args.evidence.write_text(json.dumps(receipt, indent=2) + '\n')
print('Pinned native Bitwarden image input prepared')
