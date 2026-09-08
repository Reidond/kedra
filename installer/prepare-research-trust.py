"""Generate only public image/media inputs; ephemeral private keys stay elsewhere."""
import argparse
import hashlib
import json
import os
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--public-key', type=pathlib.Path, required=True)
parser.add_argument('--source', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true':
    raise SystemExit('Research trust assembly runs only on a disposable Actions runner')
args.output.mkdir(parents=True, exist_ok=False)
source = json.loads(args.source.read_text())
if source['target']['id'] != 'desktop' or source['target']['architecture'] != 'x86_64' or source['target']['fedora_release'] != 44:
    raise SystemExit('Expected the explicitly planned desktop source')
# The fixture retains source file/package provenance but has a separate registry
# scope. It cannot enroll as the owner target or be promoted as an owner release.
repository = 'registry.kedra.test:5000/kedra/r02'
source['target']['image'] = repository
source['input_scope'] = 'generated-r02-signed-installer-fixture'
scope = {'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44, 'repository': repository}
key = args.public_key.read_bytes()
if not key.startswith(b'-----BEGIN PUBLIC KEY-----') or b'PRIVATE' in key or len(key) > 4096:
    raise SystemExit('Only a bounded public key may enter an image context')
der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(args.public_key), '-outform', 'DER'])
def write(name, value):
    (args.output / name).write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')
(args.output / 'release.pub').write_bytes(key)
write('source.json', source)
write('release-policy.json', {'schema_version': 1, 'scope': scope, 'key_fingerprint': hashlib.sha256(der).hexdigest()})
requirement = {'type': 'sigstoreSigned', 'keyPath': '/usr/lib/sysroot/trust/release.pub',
               'signedIdentity': {'type': 'exactRepository', 'dockerRepository': repository}}
write('policy.json', {'default': [{'type': 'reject'}], 'transports': {
    'docker': {repository: [requirement]}, 'containers-storage': {'': [requirement]}}})
(args.output / 'registries.yaml').write_text('docker:\n  registry.kedra.test:5000:\n    use-sigstore-attachments: true\n')
(args.output / 'install.toml').write_text('[install]\nenforce-container-sigpolicy = true\n')
(args.output / 'research-only').write_text('Disposable R02 signing key and registry scope. Not an owner release.\n')

