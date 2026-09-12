"""Prepare public desktop image trust without creating keys or signing images."""
import argparse
import json
import pathlib
import re
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--source', type=pathlib.Path, required=True)
parser.add_argument('--public-key', type=pathlib.Path, required=True)
parser.add_argument('--expected-fingerprint', required=True)
parser.add_argument('--sysroot', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
if not re.fullmatch('[a-f0-9]{64}', args.expected_fingerprint):
    raise SystemExit('Provide the independently reviewed lowercase SHA-256 key fingerprint')
def bounded(path, limit):
    with path.open('rb') as stream:
        value = stream.read(limit + 1)
    if len(value) > limit:
        raise SystemExit('Public input exceeds its bound')
    return value


source = json.loads(bounded(args.source, 1_048_576))
public_key = bounded(args.public_key, 4096)
target = source.get('target', {})
if (source.get('schema_version') != 1 or source.get('input_scope') != 'committed HEAD only'
        or not re.fullmatch('[a-f0-9]{40}', source.get('source_revision', ''))
        or target.get('id') != 'desktop' or target.get('architecture') != 'x86_64'
        or target.get('fedora_release') != 44 or target.get('image') != 'ghcr.io/reidond/kedra-desktop'):
    raise SystemExit('Expected the actual committed desktop source plan and owner registry scope')
with tempfile.TemporaryDirectory(prefix='kedra-public-key-') as temporary:
    captured = pathlib.Path(temporary) / 'release.pub'
    captured.write_bytes(public_key)
    identity = subprocess.run([str(args.sysroot.resolve()), 'release', 'key', '--public-key', str(captured), '--json'],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
if identity.returncode != 0:
    raise SystemExit('The supplied public key is not a valid bounded P-256 SPKI key')
identity = json.loads(identity.stdout)
if identity['key_fingerprint_sha256'] != args.expected_fingerprint:
    raise SystemExit('Public key differs from the reviewed fingerprint')
scope = {'target': target['id'], 'architecture': target['architecture'],
         'fedora_release': target['fedora_release'], 'repository': target['image']}
requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
               'signedIdentity': {'type': 'exactRepository', 'dockerRepository': scope['repository']}}
policy = {'default': [{'type': 'reject'}], 'transports': {
    'docker': {scope['repository']: [requirement]}, 'containers-storage': {'': [requirement]}}}
args.output.mkdir(parents=True, exist_ok=False)
(args.output / 'release.pub').write_bytes(public_key)
(args.output / 'release-policy.json').write_text(json.dumps({'schema_version': 1, 'scope': scope,
    'key_fingerprint': args.expected_fingerprint}, sort_keys=True, indent=2) + '\n')
(args.output / 'policy.json').write_text(json.dumps(policy, sort_keys=True, indent=2) + '\n')
(args.output / 'registries.yaml').write_text('docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n')
(args.output / 'install.toml').write_text('[install]\nenforce-container-sigpolicy = true\n')
print(json.dumps({'public_trust_prepared': True, 'scope': scope, 'key_fingerprint_sha256': args.expected_fingerprint,
                  'source_revision': source['source_revision'], 'image_signed': False, 'stable_updated': False}))
