"""Public signed helper fixtures; generated private keys stay outside artifacts."""
import argparse
import base64
import hashlib
import json
import os
import pathlib
import subprocess
import time
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('phase', choices=['prepare', 'requests'])
parser.add_argument('--root', type=pathlib.Path, required=True)
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true':
    raise SystemExit('Disposable Actions fixture only')
root = args.root.resolve()
if root != pathlib.Path(os.environ['RUNNER_TEMP']).resolve() / 'kedra-r01':
    raise SystemExit('Unexpected fixture directory')
private = root.parent / 'kedra-r01-private'
context = root / 'context'
scope = {'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44,
         'repository': 'registry.kedra.test:5000/kedra/r01'}
pins = json.loads(pathlib.Path('build/agents/inputs.json').read_text())['cosign_test_tool']

def dump(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')

if args.phase == 'prepare':
    binary = root / 'cosign'
    digest, size = hashlib.sha256(), 0
    url = f'https://github.com/sigstore/cosign/releases/download/v{pins["version"]}/cosign-linux-amd64'
    with urllib.request.urlopen(url, timeout=60) as response, binary.open('xb') as output:
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > pins['size']:
                raise RuntimeError('Cosign exceeded pinned size')
            digest.update(chunk)
            output.write(chunk)
    if size != pins['size'] or digest.hexdigest() != pins['sha256']:
        raise RuntimeError('Cosign identity mismatch')
    binary.chmod(0o755)
    public_der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(context / 'public/release.pub'), '-outform', 'DER'])
    dump(context / 'release-policy.json', {'schema_version': 1, 'scope': scope,
        'key_fingerprint': hashlib.sha256(public_der).hexdigest()})
    source = {'schema_version': 1, 'source_revision': os.environ['GITHUB_SHA'], 'input_scope': 'generated-r01-fixture',
              'target': {'id': 'desktop', 'architecture': 'x86_64', 'image': scope['repository'],
                         'fedora_release': 44, 'candidate_target': True, 'hardware_status': 'synthetic'},
              'files': [], 'packages': [], 'remove_packages': []}
    dump(context / 'source.json', source)
    requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
                   'signedIdentity': {'type': 'exactRepository', 'dockerRepository': scope['repository']}}
    policy = {'default': [{'type': 'reject'}], 'transports': {'docker': {scope['repository']: [requirement]},
              'containers-storage': {'': [requirement]}}}
    dump(context / 'policy.json', policy)
    raise SystemExit(0)

cosign = root / 'cosign'
if cosign.stat().st_size != pins['size'] or hashlib.sha256(cosign.read_bytes()).hexdigest() != pins['sha256']:
    raise RuntimeError('Cosign changed before signing')
source_hash = hashlib.sha256((context / 'source.json').read_bytes()).hexdigest()
now = int(time.time())
signing_home = private / 'cosign-home'
signing_home.mkdir(mode=0o700)
signing_environment = {'PATH': os.environ['PATH'], 'HOME': str(signing_home), 'LANG': 'C.UTF-8',
                       'COSIGN_PASSWORD': (private / 'passphrase').read_text().strip()}
counter = 0

def signed(value, key='allowed'):
    global counter
    counter += 1
    payload = json.dumps(value, sort_keys=True, separators=(',', ':'))
    path, bundle = private / f'payload-{counter}.json', private / f'bundle-{counter}.json'
    path.write_text(payload)
    subprocess.run([str(cosign), 'sign-blob', '--yes', '--key', str(private / f'{key}.private'),
                    '--use-signing-config=false', '--tlog-upload=false', '--bundle', str(bundle), str(path)],
                   env=signing_environment, stdout=subprocess.DEVNULL, check=True)
    data = json.loads(bundle.read_text())['messageSignature']
    if data['messageDigest']['algorithm'] != 'SHA2_256' or base64.b64decode(data['messageDigest']['digest']) != hashlib.sha256(payload.encode()).digest():
        raise RuntimeError('Unexpected native Cosign blob algorithm or digest')
    if 'image_digest' in value and value['approval'] == 'promoted' and value['scope']['target'] == 'desktop' and key == 'allowed':
        signature_path = private / f'signature-{counter}.txt'
        signature_path.write_text(data['signature'])
        subprocess.run([str(pathlib.Path('target/release/sysroot').resolve()), 'release', 'verify',
                        '--manifest', str(path), '--signature', str(signature_path),
                        '--public-key', str(context / 'public/release.pub')], stdout=subprocess.DEVNULL, check=True)
    return {'payload': payload, 'signature': data['signature']}

def release(variant, sequence, **changes):
    value = {'schema_version': 1, 'project': 'Kedra', 'scope': scope, 'sequence': sequence,
             'source_revision': os.environ['GITHUB_SHA'], 'build': {'repository': 'Reidond/kedra',
             'workflow': '.github/workflows/release.yml', 'run_id': int(os.environ['GITHUB_RUN_ID']),
             'run_attempt': int(os.environ['GITHUB_RUN_ATTEMPT'])},
             'image_digest': (root / f'{variant}.digest').read_text().strip(),
             'home_manifest_sha256': source_hash, 'installer': {'filename': f'kedra-{changes.get("scope", scope)["target"]}-44-fixture-{variant.lower()}.iso',
             'size_bytes': 7, 'sha256': hashlib.sha256(b'fixture').hexdigest()},
             'approval': 'promoted', 'minimum_protocol': 1}
    value.update(changes)
    return value

def checkpoint(document, generation, expired=False):
    return {'schema_version': 1, 'project': 'Kedra', 'scope': scope, 'generation': generation,
            'release_sha256': hashlib.sha256(document['payload'].encode()).hexdigest(),
            'issued_at': now - 7200 if expired else now, 'expires_at': now - 3600 if expired else now + 3600,
            'last_successful_resolution': now - 7200 if expired else now}

def envelope(name, operation, document=None, generation=None, expired=False, **extra):
    request = {'operation': operation, **extra}
    if document is not None:
        request['release'] = document
    if generation is not None:
        request['checkpoint'] = signed(checkpoint(document, generation, expired))
    dump(root / 'cases' / f'helper-{name}.json', {'schema_version': 1, 'request': request})

a, b = signed(release('A', 1)), signed(release('B', 6))
initial_latest = signed(release('B', 2))
envelope('status', 'status')
envelope('enroll', 'enroll', initial_latest, 2, installed_release=a)
envelope('enroll-mismatch', 'enroll', initial_latest, 2)
envelope('stage-b', 'stage', b, 6, replace_staged=None, resume=False)
envelope('rollback-a', 'rollback', a, replace_staged=None)
envelope('resume-b', 'stage', b, 6, replace_staged=None, resume=True)
envelope('replay-a', 'stage', a, 1, replace_staged=None, resume=False)
envelope('expired', 'stage', b, 6, expired=True, replace_staged=None, resume=False)
for variant, sequence in [('U', 3), ('W', 4), ('M', 5)]:
    envelope(f'oci-{variant}', 'stage', signed(release(variant, sequence)), sequence, replace_staged=None, resume=False)
envelope('pending', 'stage', signed(release('U', 7)), 7, replace_staged=None, resume=False)
envelope('wrong-key', 'stage', signed(release('B', 6), 'wrong'), 6, replace_staged=None, resume=False)
envelope('wrong-target', 'stage', signed(release('B', 6, scope={**scope, 'target': 'other'})), 6, replace_staged=None, resume=False)
envelope('candidate', 'stage', signed(release('B', 6, approval='candidate')), 6, replace_staged=None, resume=False)
dump(root / 'cases/helper-extra-field.json', {'schema_version': 1, 'request': {'operation': 'status', 'verified': True}})
print('Prepared public Cosign-signed helper requests; no private key enters a context or artifact')
