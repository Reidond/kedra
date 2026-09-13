"""Public A/P/B source and signing fixtures for a disposable Actions VM."""
import argparse
import base64
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import time
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('phase', choices=['images', 'requests'])
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true' or os.environ.get('GITHUB_REPOSITORY') != 'Reidond/kedra':
    raise SystemExit('Disposable Kedra Actions fixture only')
repo = pathlib.Path.cwd().resolve()
root = pathlib.Path(os.environ['RUNNER_TEMP']).resolve() / 'kedra-r04'
private = root.parent / 'kedra-r04-private'
tool = repo / 'target/release/sysroot'
scope = {'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44,
         'repository': 'registry.kedra.test:5000/kedra/r04'}
pins = json.loads((repo / 'build/agents/inputs.json').read_text())['cosign_test_tool']


def dump(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')


def git(*args, data=None):
    environment = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    environment.update({'GIT_AUTHOR_NAME': 'Kedra R04 fixture', 'GIT_COMMITTER_NAME': 'Kedra R04 fixture',
                        'GIT_AUTHOR_EMAIL': 'r04@example.invalid', 'GIT_COMMITTER_EMAIL': 'r04@example.invalid',
                        'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null'})
    if args[0] in {'read-tree', 'update-index', 'write-tree'}:
        environment['GIT_INDEX_FILE'] = str(root / 'fixture-index')
    return subprocess.check_output(['git', '--no-replace-objects', '-C', str(repo), *args], input=data, env=environment)


if args.phase == 'images':
    a = git('rev-parse', 'HEAD').decode().strip()
    if a != os.environ['GITHUB_SHA']:
        raise RuntimeError('source differs from the dispatched revision')
    source_path = 'home/.config/niri/config.kdl'
    original = git('show', f'{a}:{source_path}').decode()
    published = original.replace('width 2', 'width 3')
    # Generate the independent incoming gap edit from the current baseline.
    # Native validation and the actual home CLI exercise the resulting inputs;
    # a particular historical desktop spacing is not a fixture prerequisite.
    incoming = re.sub(r'(?m)^([ \t]*)gaps[ \t]+[0-9]+[ \t]*$', r'\g<1>gaps 18', published, count=1)
    incoming = incoming.replace('\nbinds {\n', '\ncursor {\n    xcursor-size 28\n}\n\nbinds {\n')

    def commit(parent, text, message):
        git('read-tree', parent)
        blob = git('hash-object', '-w', '--stdin', data=text.encode()).decode().strip()
        git('update-index', '--cacheinfo', '100644', blob, source_path)
        tree = git('write-tree').decode().strip()
        return git('commit-tree', tree, '-p', parent, '-m', message).decode().strip()

    publication = commit(a, published, 'R04 generated selected-line publication')
    b = commit(publication, incoming, 'R04 generated incoming image baseline')
    git('update-ref', 'refs/heads/kedra-r04-b', b)
    git('bundle', 'create', str(root / 'source.bundle'), 'refs/heads/kedra-r04-b')
    source_b = root / 'source-b'
    git('worktree', 'add', '--detach', str(source_b), b)
    fixture = {'a': a, 'publication': publication, 'b': b, 'scope': scope}
    dump(root / 'fixture.json', fixture)
    public = (private / 'allowed.pub').read_bytes()
    der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(private / 'allowed.pub'), '-outform', 'DER'])
    requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
                   'signedIdentity': {'type': 'exactRepository', 'dockerRepository': scope['repository']}}
    for variant, directory, text in [('A', repo, original), ('B', source_b, incoming)]:
        context = root / variant
        context.mkdir()
        source = json.loads(subprocess.check_output([str(tool), 'source', 'plan', '--host', 'desktop', '--repo', str(directory), '--json']))
        source['target']['image'] = scope['repository']
        source['input_scope'] = 'generated-r04-signed-home-transition-fixture'
        dump(context / 'source.json', source)
        dump(context / 'release-policy.json', {'schema_version': 1, 'scope': scope, 'key_fingerprint': hashlib.sha256(der).hexdigest()})
        dump(context / 'policy.json', {'default': [{'type': 'reject'}], 'transports': {'docker': {scope['repository']: [requirement]}, 'containers-storage': {'': [requirement]}}})
        (context / 'release.pub').write_bytes(public)
        (context / 'config.kdl').write_text(text)
        (context / 'registries.yaml').write_text('docker:\n  registry.kedra.test:5000:\n    use-sigstore-attachments: true\n')
        (context / 'install.toml').write_text('[install]\nenforce-container-sigpolicy = true\n')
        for name in ['Containerfile', 'check.sh', 'check.service', 'home.py']:
            shutil.copyfile(repo / 'tests/vm/home-transition' / name, context / name)
        shutil.copyfile(repo / 'tests/vm/console.toml', context / 'console.toml')
        shutil.copyfile(repo / 'tests/vm/desktop/test-profile.toml', context / 'test-profile.toml')
        for name in ['source.bundle', 'fixture.json', 'tls.crt']:
            shutil.copyfile(root / name, context / name)
    raise SystemExit(0)

cosign = root / 'cosign'
url = f'https://github.com/sigstore/cosign/releases/download/v{pins["version"]}/cosign-linux-amd64'
size, digest = 0, hashlib.sha256()
with urllib.request.urlopen(url, timeout=60) as response, cosign.open('xb') as output:
    while chunk := response.read(1024 * 1024):
        size += len(chunk)
        if size > pins['size']:
            raise RuntimeError('Cosign exceeded the pinned size')
        digest.update(chunk)
        output.write(chunk)
if size != pins['size'] or digest.hexdigest() != pins['sha256']:
    raise RuntimeError('Cosign identity mismatch')
cosign.chmod(0o755)
fixture = json.loads((root / 'fixture.json').read_text())
signing_home = private / 'signing-home'
signing_home.mkdir(mode=0o700)
environment = {'PATH': os.environ['PATH'], 'HOME': str(signing_home), 'LANG': 'C.UTF-8',
               'COSIGN_PASSWORD': (private / 'passphrase').read_text().strip()}
counter = 0


def signed(value):
    global counter
    counter += 1
    payload = json.dumps(value, sort_keys=True, separators=(',', ':'))
    path, bundle = private / f'payload-{counter}.json', private / f'bundle-{counter}.json'
    path.write_text(payload)
    subprocess.run([str(cosign), 'sign-blob', '--yes', '--key', str(private / 'allowed.private'),
                    '--use-signing-config=false', '--tlog-upload=false', '--bundle', str(bundle), str(path)],
                   env=environment, stdout=subprocess.DEVNULL, check=True)
    message = json.loads(bundle.read_text())['messageSignature']
    if message['messageDigest']['algorithm'] != 'SHA2_256' or base64.b64decode(message['messageDigest']['digest']) != hashlib.sha256(payload.encode()).digest():
        raise RuntimeError('Unexpected Cosign blob digest')
    return {'payload': payload, 'signature': message['signature']}


def release(variant, sequence):
    return signed({'schema_version': 1, 'project': 'Kedra', 'scope': scope, 'sequence': sequence,
                   'source_revision': fixture[variant.lower()],
                   'build': {'repository': 'Reidond/kedra', 'workflow': '.github/workflows/release.yml',
                             'run_id': int(os.environ['GITHUB_RUN_ID']), 'run_attempt': int(os.environ['GITHUB_RUN_ATTEMPT'])},
                   'image_digest': (root / f'{variant}.digest').read_text().strip(),
                   'home_manifest_sha256': hashlib.sha256((root / variant / 'source.json').read_bytes()).hexdigest(),
                   'installer': {'filename': f'kedra-desktop-44-r04-{variant.lower()}.iso', 'size_bytes': 7,
                                 'sha256': hashlib.sha256(b'fixture').hexdigest()},
                   'approval': 'promoted', 'minimum_protocol': 1})


def checkpoint(document, generation):
    now = int(time.time())
    return signed({'schema_version': 1, 'project': 'Kedra', 'scope': scope, 'generation': generation,
                   'release_sha256': hashlib.sha256(document['payload'].encode()).hexdigest(),
                   'issued_at': now, 'expires_at': now + 3600, 'last_successful_resolution': now})


a, b = release('A', 1), release('B', 2)
cases = root / 'cases'
cases.mkdir()
for name, request in {
    'status': {'operation': 'status'},
    'enroll': {'operation': 'enroll', 'release': a, 'checkpoint': checkpoint(a, 1), 'installed_release': a},
    'stage-b': {'operation': 'stage', 'release': b, 'checkpoint': checkpoint(b, 2), 'replace_staged': None, 'resume': False},
    'rollback-a': {'operation': 'rollback', 'release': a, 'replace_staged': None},
}.items():
    dump(cases / f'helper-{name}.json', {'schema_version': 1, 'request': request})
print('Prepared public source/metadata fixtures with separate disposable authority')
