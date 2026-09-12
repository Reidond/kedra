"""Resolve the reviewed Fedora test stream to one checked immutable platform."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


def require(ok, message):
    if not ok:
        raise SystemExit(message)


def native(*arguments):
    result = subprocess.run(arguments, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, timeout=180, check=False)
    require(result.returncode == 0, 'Fedora input resolution failed: ' + result.stderr.decode(errors='replace')[:1500])
    require(0 < len(result.stdout) <= 8 * 1024**2, 'Registry response exceeds its bound')
    return result.stdout


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
require(os.environ.get('GITHUB_ACTIONS') == 'true' and os.environ.get('RUNNER_OS') == 'Linux'
        and os.environ.get('GITHUB_REPOSITORY') == 'Reidond/kedra', 'Use only a disposable Kedra Actions runner')
root = Path(__file__).resolve().parents[1]
inputs = json.loads((root / 'build/inputs.json').read_bytes())
tag = inputs['base_tag']
require(tag == 'quay.io/fedora/fedora-bootc:44' and inputs['architecture'] == 'amd64',
        'Unreviewed Fedora stream or platform')
raw = native('skopeo', 'inspect', '--raw', 'docker://' + tag)
manifest = json.loads(raw)
discovery_digest = 'sha256:' + hashlib.sha256(raw).hexdigest()
if 'manifests' in manifest:
    choices = [item for item in manifest['manifests']
               if item.get('platform', {}).get('os') == 'linux'
               and item.get('platform', {}).get('architecture') == 'amd64'
               and not item.get('platform', {}).get('variant')]
    require(len(choices) == 1, 'Fedora index does not select exactly one Linux/AMD64 platform without variant')
    digest = choices[0]['digest']
else:
    digest = discovery_digest
require(isinstance(digest, str) and re.fullmatch(r'sha256:[a-f0-9]{64}', digest), 'Invalid Fedora platform digest')
reference = 'quay.io/fedora/fedora-bootc@' + digest
platform_raw = native('skopeo', 'inspect', '--raw', 'docker://' + reference)
require('sha256:' + hashlib.sha256(platform_raw).hexdigest() == digest, 'Immutable platform manifest hash differs')
platform = json.loads(platform_raw)
require('manifests' not in platform and isinstance(platform.get('config'), dict), 'Selected image is not a platform manifest')
config = json.loads(native('skopeo', 'inspect', '--config', 'docker://' + reference))
require(config.get('os') == 'linux' and config.get('architecture') == 'amd64', 'Fedora image config platform differs')
evidence = {'schema_version': 1, 'source_tag': tag, 'discovery_digest': discovery_digest,
            'platform_digest': digest, 'reference': reference, 'os': 'linux', 'architecture': 'amd64',
            'resolved_at': int(time.time()), 'skopeo': native('skopeo', '--version').decode().strip()}
with args.output.open('x', encoding='utf-8', newline='\n') as stream:
    json.dump(evidence, stream, sort_keys=True, indent=2)
    stream.write('\n')
print(reference)
