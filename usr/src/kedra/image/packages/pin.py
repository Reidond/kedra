#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Add or refresh one archive pin in packages.lock.json.

Downloads the archive (or reads `--archive`), records its SHA-256, extracts it
with the release's own image/catalog/prepare-sources.py and admits the tree
with `sysroot store add-source` in a temporary store, so the recorded `src-`
object is the one the release build will reproduce. Run it on Linux, where the
release extracts. Signature checks for a pin happen before this step
(verify-codex.py for Codex); see docs/PACKAGES.md.
"""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
PREPARE = HERE.parent / 'catalog/prepare-sources.py'


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def acquire(url, archive, destination):
    if archive is not None:
        shutil.copyfile(archive, destination)
        return
    with urllib.request.urlopen(url, timeout=120) as response, destination.open('xb') as output:
        shutil.copyfileobj(response, output, 1024 * 1024)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def sysroot(binary, *arguments):
    result = subprocess.run([str(binary), *map(str, arguments)], capture_output=True, check=False)
    require(result.returncode == 0, f'sysroot {arguments[0]} failed: {result.stderr.decode().strip()}')
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--name', required=True, help='pin name the .kedra archive refers to')
    parser.add_argument('--url', required=True)
    parser.add_argument('--sysroot', type=pathlib.Path, required=True, help='Linux sysroot binary')
    parser.add_argument('--lock', type=pathlib.Path,
                        default=HERE.parents[4] / 'system/packages.lock.json')
    parser.add_argument('--archive', type=pathlib.Path, help='already downloaded archive of --url')
    parser.add_argument('--sha256', help='required archive SHA-256, for example a verified one')
    args = parser.parse_args()
    require(re.fullmatch(r'[a-z0-9][a-z0-9_-]{0,63}', args.name), 'Pin names are lowercase words')
    require(urllib.parse.urlsplit(args.url).scheme == 'https', 'Pins are HTTPS URLs')
    require(sys.platform == 'linux', 'Compute pins on Linux, where the release extracts archives')
    lock = json.loads(args.lock.read_text())
    require(lock.get('schema_version') == 1 and isinstance(lock.get('sources'), dict), 'Unsupported lock')
    with tempfile.TemporaryDirectory(prefix='kedra-pin-') as temporary:
        work = pathlib.Path(temporary)
        archives = work / 'archives'
        archives.mkdir()
        downloaded = work / 'download'
        acquire(args.url, args.archive, downloaded)
        sha256 = digest(downloaded)
        require(args.sha256 in (None, sha256), f'Archive SHA-256 {sha256} differs from --sha256')
        downloaded.rename(archives / f'{sha256}.tar.gz')
        # prepare-sources.py checks the object's form, not its value; the store
        # computes the real one below.
        placeholder = 'src-' + '0' * 64
        pins = work / 'pins.json'
        pins.write_text(json.dumps({'schema_version': 2, 'format': 'kedra', 'archives': {
            placeholder: {'url': args.url, 'sha256': sha256, 'object': placeholder}}}))
        subprocess.run([sys.executable, '-I', str(PREPARE), '--pins', str(pins), '--output', str(work / 'sources'),
                        '--archives', str(archives)], check=True)
        tree = json.loads((work / 'sources/sources.json').read_text())[placeholder]
        store = work / 'store'
        sysroot(args.sysroot, 'store', 'init', '--store', store)
        source = sysroot(args.sysroot, 'store', 'add-source', '--store', store, '--source', tree)
        # The temporary store's objects are read-only; make them removable.
        subprocess.run(['chmod', '-R', 'u+w', str(store)], check=True)
    entry = {'url': args.url, 'sha256': sha256, 'object': source['object']}
    lock['sources'][args.name] = entry
    lock['sources'] = dict(sorted(lock['sources'].items()))
    args.lock.write_text(json.dumps(lock, indent=2) + '\n')
    print(json.dumps({args.name: entry}, indent=2))


if __name__ == '__main__':
    main()
