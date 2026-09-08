"""Fetch pinned official Codex packages for image assembly or isolated research."""
import argparse
import hashlib
import json
import pathlib
import tarfile
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--research-tools', action='store_true')
args = parser.parse_args()
pins = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())
args.output.mkdir(parents=True, exist_ok=False)

def fetch(url, destination, digest, size=None):
    hashed = hashlib.sha256()
    received = 0
    with urllib.request.urlopen(url, timeout=60) as response, destination.open('xb') as output:
        while chunk := response.read(1024 * 1024):
            received += len(chunk)
            if received > (size if size is not None else 65536):
                raise RuntimeError('Download exceeded pinned size bound')
            hashed.update(chunk)
            output.write(chunk)
    if (size is not None and received != size) or hashed.hexdigest() != digest:
        raise RuntimeError(f'Pinned artifact identity mismatch: {destination.name}')

def package(record, name):
    version, target = record['version'], record['target']
    archive = args.output / f'{name}.tar.gz'
    base = f'https://github.com/openai/codex/releases/download/rust-v{version}'
    fetch(f'{base}/codex-package-{target}.tar.gz', archive, record['archive_sha256'], record['archive_size'])
    directory = args.output / name
    expected = {'bin/codex', 'bin/codex-code-mode-host', 'codex-package.json',
                'codex-path/rg', 'codex-resources/bwrap', 'codex-resources/zsh/bin/zsh'}
    with tarfile.open(archive) as stream:
        members = stream.getmembers()
        if {m.name for m in members if m.isfile()} != expected:
            raise RuntimeError('Official package layout changed; review required')
        if any(not (m.isfile() or m.isdir()) for m in members):
            raise RuntimeError('Package links or special entries are not accepted')
        stream.extractall(directory, filter='data')
    manifest = json.loads((directory / 'codex-package.json').read_text())
    if manifest != {'layoutVersion': 1, 'version': version, 'target': target, 'variant': 'codex',
                    'entrypoint': 'bin/codex', 'resourcesDir': 'codex-resources', 'pathDir': 'codex-path'}:
        raise RuntimeError('Unexpected package manifest')
    for binary, digest in record.get('signatures', {}).items():
        fetch(f'{base}/{binary}-{target}.sigstore', args.output / f'{binary}.sigstore', digest)
    print(f'Pinned package verified: {name} {version}', flush=True)

package(pins['codex'], 'codex')
record = pins['codex']
fetch(f'https://codeload.github.com/openai/codex/tar.gz/{record["source_revision"]}',
      args.output / 'codex-corresponding-source.tar.gz', record['source_sha256'], record['source_size'])
# Keep upstream notices beside the unmodified package, including the vendored
# bubblewrap license. Its complete source/build files remain in the archive.
notices = args.output / 'notices'
notices.mkdir()
with tarfile.open(args.output / 'codex-corresponding-source.tar.gz') as source:
    prefix = f'codex-{record["source_revision"]}/'
    for relative, name in [
        ('LICENSE', 'codex-LICENSE'), ('NOTICE', 'codex-NOTICE'),
        ('codex-rs/vendor/bubblewrap/COPYING', 'bubblewrap-COPYING'),
        ('codex-rs/shell-escalation/patches/zsh-exec-wrapper.patch', 'codex-zsh-exec-wrapper.patch'),
    ]:
        member = source.getmember(prefix + relative)
        if not member.isfile() or member.size > 1024 * 1024:
            raise RuntimeError('Expected bounded upstream notice/source file')
        stream = source.extractfile(member)
        if stream is None:
            raise RuntimeError('Missing upstream notice/source bytes')
        with stream:
            (notices / name).write_bytes(stream.read())
for notice in pins['component_notices']:
    if pathlib.PurePosixPath(notice['filename']).name != notice['filename']:
        raise RuntimeError('Notice name must be a basename')
    fetch(notice['url'], notices / notice['filename'], notice['sha256'], notice['size'])
if args.research_tools:
    package(pins['personal_test_codex'], 'personal-codex')
    tool = pins['cosign_test_tool']
    fetch(f'https://github.com/sigstore/cosign/releases/download/v{tool["version"]}/cosign-linux-amd64',
          args.output / 'cosign', tool['sha256'], tool['size'])
    (args.output / 'cosign').chmod(0o755)
