#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Acquire reviewed archives without package-name knowledge or recipe execution."""
import argparse
import gzip
import hashlib
import io
import json
import re
import tarfile
import time
import urllib.parse
import urllib.request
from pathlib import Path

MAX_ARCHIVE = 32 * 1024**2
MAX_TREE = 128 * 1024**2


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


class ExpandedArchive(io.BufferedIOBase):
    def __init__(self, stream):
        self.stream = stream

    def read(self, size=-1):
        require(0 <= size <= 2 * MAX_TREE, 'Oversized archive metadata read')
        result = self.stream.read(size)
        require(self.stream.tell() <= 2 * MAX_TREE, 'Decompressed archive byte limit exceeded')
        return result

    def tell(self):
        return self.stream.tell()

    def seek(self, offset, whence=0):
        require(whence == 0 and 0 <= offset <= 2 * MAX_TREE, 'Unsafe archive seek')
        return self.stream.seek(offset)


class HTTPSRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, new_url):
        require(urllib.parse.urlsplit(new_url).scheme == 'https', 'Archive redirect must stay HTTPS')
        return super().redirect_request(request, fp, code, message, headers, new_url)


def acquire(pin, destination, local=None):
    require(set(pin) == {'url', 'sha256', 'object'} and isinstance(pin['url'], str) and len(pin['url']) <= 4096
            and re.fullmatch('[a-f0-9]{64}', pin['sha256']) and re.fullmatch('src-[a-f0-9]{64}', pin['object'])
            and urllib.parse.urlsplit(pin['url']).scheme == 'https',
            'Invalid source pin')
    digest = hashlib.sha256()
    total = 0
    deadline = time.monotonic() + 120
    opener = urllib.request.build_opener(HTTPSRedirect())
    source = local.open('rb') if local is not None else opener.open(pin['url'], timeout=120)
    with source, destination.open('xb') as output:
        while chunk := source.read(65536):
            total += len(chunk)
            require(total <= MAX_ARCHIVE and time.monotonic() < deadline, 'Archive byte/time limit exceeded')
            digest.update(chunk)
            output.write(chunk)
    require(total and digest.hexdigest() == pin['sha256'], 'Archive checksum differs from reviewed pin')


def extract(archive, destination):
    destination.mkdir(mode=0o700)
    with gzip.open(archive, 'rb') as compressed, tarfile.open(fileobj=ExpandedArchive(compressed), mode='r:') as source:
        members = []
        names = set()
        total = 0
        for member in source:
            total += member.size
            name = member.name.rstrip('/')
            require(len(members) < 100000 and total <= MAX_TREE and name not in names
                    and (member.isfile() or member.isdir() or member.issym() or member.islnk()),
                    'Archive tree count/size/type/duplicate refused')
            names.add(name)
            members.append(member)
        source.extractall(destination, members=members, filter='data')
    roots = list(destination.iterdir())
    require(len(roots) == 1 and roots[0].is_dir() and not roots[0].is_symlink(), 'Expected single ordinary archive root')
    for path in roots[0].rglob('*'):
        if path.is_symlink():
            continue
        info = path.stat()
        mode = 0o755 if path.is_dir() or info.st_mode & 0o111 else 0o644
        if path.is_file() and info.st_nlink != 1:
            data = path.read_bytes()
            path.unlink()
            path.write_bytes(data)
        path.chmod(mode)
    roots[0].chmod(0o755)
    return roots[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pins', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--archives', type=Path, help='Explicit offline archives named by their reviewed SHA-256')
    args = parser.parse_args()
    pins = json.loads(args.pins.read_bytes())
    require(pins.get('schema_version') == 2 and pins.get('format') == 'kedra'
            and isinstance(pins.get('archives'), dict) and len(pins['archives']) <= 256, 'Unsupported source inventory')
    args.output.mkdir(mode=0o700)
    paths = {}
    for index, (identity, pin) in enumerate(sorted(pins['archives'].items())):
        require(identity == pin['object'] and len(identity) == 68 and identity.startswith('src-'), 'Source object differs')
        archive = args.output / (str(index) + '.tar.gz')
        local = args.archives / (pin['sha256'] + '.tar.gz') if args.archives is not None else None
        acquire(pin, archive, local)
        paths[identity] = str(extract(archive, args.output / str(index)))
    (args.output / 'sources.json').write_text(json.dumps(paths, sort_keys=True, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
