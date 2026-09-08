"""Verify and package the official private Codex runtime as explicit image input."""
import argparse
import hashlib
import io
import json
import pathlib
import re
import subprocess
import tarfile

parser = argparse.ArgumentParser()
parser.add_argument('--inputs', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--evidence', type=pathlib.Path, required=True)
args = parser.parse_args()
pins = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())
record = pins['codex']

def verify(path, digest, size=None):
    if not path.is_file() or path.is_symlink():
        raise RuntimeError(f'Expected a regular input: {path.name}')
    hashed = hashlib.sha256()
    count = 0
    with path.open('rb') as source:
        while chunk := source.read(1024 * 1024):
            count += len(chunk)
            hashed.update(chunk)
    if (size is not None and count != size) or hashed.hexdigest() != digest:
        raise RuntimeError(f'Pinned input identity changed: {path.name}')

archive = args.inputs / 'codex.tar.gz'
source_archive = args.inputs / 'codex-corresponding-source.tar.gz'
verify(archive, record['archive_sha256'], record['archive_size'])
verify(source_archive, record['source_sha256'], record['source_size'])
tool = pins['cosign_test_tool']
cosign = args.inputs / 'cosign'
verify(cosign, tool['sha256'], tool['size'])
identity = f'https://github.com/openai/codex/.github/workflows/rust-release.yml@refs/tags/rust-v{record["version"]}'
if set(record['signatures']) != {'codex', 'codex-code-mode-host', 'bwrap'}:
    raise RuntimeError('Signature set changed; review the package layout')
for binary, digest in record['signatures'].items():
    bundle = args.inputs / f'{binary}.sigstore'
    verify(bundle, digest)
    relative = 'codex-resources/bwrap' if binary == 'bwrap' else f'bin/{binary}'
    result = subprocess.run([str(cosign), 'verify-blob', '--bundle', str(bundle),
                    '--certificate-identity', identity,
                    '--certificate-oidc-issuer', 'https://token.actions.githubusercontent.com',
                    str(args.inputs / 'codex' / relative)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (args.evidence.parent / f'{binary}-verification.txt').write_bytes(result.stdout)
    if result.returncode != 0:
        raise RuntimeError(f'Official signature verification failed: {binary}')

notices = {}
with tarfile.open(source_archive) as source:
    prefix = f'codex-{record["source_revision"]}/'
    for relative, name in [
        ('LICENSE', 'codex-LICENSE'), ('NOTICE', 'codex-NOTICE'),
        ('codex-rs/vendor/bubblewrap/COPYING', 'bubblewrap-COPYING'),
        ('codex-rs/shell-escalation/patches/zsh-exec-wrapper.patch', 'codex-zsh-exec-wrapper.patch'),
    ]:
        member = source.getmember(prefix + relative)
        if not member.isfile() or member.size > 1024 * 1024:
            raise RuntimeError('Unexpected source notice entry')
        with source.extractfile(member) as stream:
            notices[name] = stream.read()
for notice in pins['component_notices']:
    if re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*', notice['filename']) is None:
        raise RuntimeError('Notice name must be a safe basename')
    path = args.inputs / 'notices' / notice['filename']
    verify(path, notice['sha256'], notice['size'])
    notices[notice['filename']] = path.read_bytes()

summary = {'schema_version': 1, 'agent': 'codex', 'version': record['version'],
           'target': record['target'], 'archive_sha256': record['archive_sha256'],
           'source_revision': record['source_revision'], 'source_sha256': record['source_sha256'],
           'signature_identity': identity, 'private_entrypoint': '/usr/libexec/sysroot/agents/codex/bin/codex',
           'notices': {name: hashlib.sha256(data).hexdigest() for name, data in sorted(notices.items())}}
expected = {'bin/codex', 'bin/codex-code-mode-host', 'codex-package.json',
            'codex-path/rg', 'codex-resources/bwrap', 'codex-resources/zsh/bin/zsh'}
checked = set()
with tarfile.open(archive, mode='r|gz') as official:
    for member in official:
        if member.isdir():
            continue
        if not member.isfile() or member.name not in expected or member.name in checked or member.size > 1024 * 1024 * 1024:
            raise RuntimeError('Official package layout changed')
        checked.add(member.name)
        with official.extractfile(member) as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        verify(args.inputs / 'codex' / member.name, digest, member.size)
if checked != expected:
    raise RuntimeError('Incomplete package')
manifest = json.loads((args.inputs / 'codex/codex-package.json').read_text())
if manifest != {'layoutVersion': 1, 'version': record['version'], 'target': record['target'],
                'variant': 'codex', 'entrypoint': 'bin/codex', 'resourcesDir': 'codex-resources', 'pathDir': 'codex-path'}:
    raise RuntimeError('Unexpected official package manifest')

def append(output, name, size, stream, mode=0o644):
    item = tarfile.TarInfo(name)
    item.size, item.mode, item.uid, item.gid, item.mtime = size, mode, 0, 0, 0
    output.addfile(item, stream)

with args.output.open('xb') as destination, tarfile.open(fileobj=destination, mode='w', format=tarfile.GNU_FORMAT) as output:
    seen = set()
    with tarfile.open(archive, mode='r|gz') as official:
        for member in official:
            if member.isdir():
                continue
            if not member.isfile() or member.name not in expected or member.name in seen:
                raise RuntimeError('Official package layout changed')
            seen.add(member.name)
            with official.extractfile(member) as stream:
                append(output, f'usr/libexec/sysroot/agents/codex/{member.name}', member.size, stream,
                       0o644 if member.name == 'codex-package.json' else 0o755)
    if seen != expected:
        raise RuntimeError('Incomplete official package')
    for name, data in sorted(notices.items()):
        append(output, f'usr/share/licenses/sysroot-agents/codex/{name}', len(data), io.BytesIO(data))
    with source_archive.open('rb') as source:
        append(output, 'usr/share/licenses/sysroot-agents/codex/corresponding-source.tar.gz', record['source_size'], source)
    metadata = (json.dumps(summary, indent=2) + '\n').encode()
    append(output, 'usr/share/sysroot/agents/codex.json', len(metadata), io.BytesIO(metadata))
args.evidence.write_text(json.dumps(summary, indent=2) + '\n')
print(f'Verified official Codex {record["version"]} packaged at the private sysroot path')
