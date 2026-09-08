"""Prepare unsigned release/checkpoint bytes from reviewed exact-media evidence.

This ordinary-user producer has no signing, networking or publication authority.
The protected publisher must recheck the expected previous channel under its lock.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--candidate', type=pathlib.Path, required=True)
parser.add_argument('--reviewed-candidate-sha256', required=True)
parser.add_argument('--source', type=pathlib.Path, required=True)
parser.add_argument('--installer', type=pathlib.Path, required=True)
parser.add_argument('--installer-parts-dir', type=pathlib.Path, required=True)
parser.add_argument('--packages', type=pathlib.Path, required=True)
parser.add_argument('--provenance', type=pathlib.Path, required=True)
parser.add_argument('--qualification', type=pathlib.Path, required=True)
parser.add_argument('--public-key', type=pathlib.Path, required=True)
parser.add_argument('--expected-fingerprint', required=True)
parser.add_argument('--sysroot', type=pathlib.Path, required=True)
history = parser.add_mutually_exclusive_group(required=True)
history.add_argument('--previous-channel', type=pathlib.Path)
history.add_argument('--bootstrap', action='store_true')
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
scope = {'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44,
         'repository': 'ghcr.io/reidond/kedra-desktop'}
checks = {'encrypted_install', 'unselected_disk_preserved', 'iso_free_boot',
          'owner_desktop_login', 'selinux_enforcing', 'exact_booted_image',
          'container_policy', 'home_writable', 'root_read_only', 'doctor'}


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'Duplicate JSON member')
        result[key] = value
    return result


def bounded(path, limit=65_536):
    require(path.is_file() and not path.is_symlink(), 'Expected an ordinary public input file')
    with path.open('rb') as stream:
        value = stream.read(limit + 1)
    require(0 < len(value) <= limit, 'Public input exceeds its size bound or is empty')
    return value


def document(value):
    result = json.loads(value, object_pairs_hook=unique)
    require(isinstance(result, dict), 'Expected a JSON object')
    return result


def number(value, maximum=2**64 - 1):
    return type(value) is int and 0 < value <= maximum


def digest(value):
    return isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value) is not None


def cli(*arguments):
    result = subprocess.run([str(args.sysroot.resolve()), 'release', *map(str, arguments), '--json'],
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    require(result.returncode == 0, 'Native release verification refused public inputs: ' + result.stderr.decode(errors='replace')[:1000])
    return document(result.stdout)


candidate_bytes = bounded(args.candidate)
candidate_hash = hashlib.sha256(candidate_bytes).hexdigest()
require(digest(args.reviewed_candidate_sha256) and candidate_hash == args.reviewed_candidate_sha256,
        'Candidate bytes differ from the independently reviewed digest')
candidate = document(candidate_bytes)
require(set(candidate) == {'schema_version', 'project', 'scope', 'source_revision', 'build',
                          'image_digest', 'home_manifest_sha256', 'installer', 'parts', 'approval',
                          'fresh_install_qualified', 'last_successful_resolution',
                          'packages_sha256', 'provenance_sha256', 'parts_sha256'}, 'Unknown or missing candidate field')
require(type(candidate['schema_version']) is int and candidate['schema_version'] == 2
        and candidate['project'] == 'Kedra' and candidate['scope'] == scope
        and candidate['approval'] == 'candidate' and candidate['fresh_install_qualified'] is False,
        'Expected an unpromoted desktop candidate')
require(isinstance(candidate['source_revision'], str) and re.fullmatch('[a-f0-9]{40}', candidate['source_revision']),
        'Invalid candidate source revision')
require(isinstance(candidate['image_digest'], str) and re.fullmatch('sha256:[a-f0-9]{64}', candidate['image_digest']),
        'Invalid candidate image digest')
build = candidate['build']
require(isinstance(build, dict) and set(build) == {'repository', 'workflow', 'run_id', 'run_attempt'}
        and build['repository'] == 'Reidond/kedra' and build['workflow'] == '.github/workflows/release.yml'
        and number(build['run_id']) and number(build['run_attempt'], 2**32 - 1), 'Invalid candidate build identity')
source_bytes = bounded(args.source, 1_048_576)
source = document(source_bytes)
require(hashlib.sha256(source_bytes).hexdigest() == candidate['home_manifest_sha256']
        and type(source.get('schema_version')) is int and source['schema_version'] == 1
        and source.get('source_revision') == candidate['source_revision']
        and source.get('input_scope') == 'committed HEAD only', 'Installed source bytes differ from the reviewed candidate')
target = source.get('target', {})
require(target.get('id') == scope['target'] and target.get('architecture') == scope['architecture']
        and target.get('fedora_release') == scope['fedora_release'] and target.get('image') == scope['repository'],
        'Installed source target differs from release scope')
installer = candidate['installer']
require(isinstance(installer, dict) and set(installer) == {'filename', 'size_bytes', 'sha256'}
        and installer['filename'] == f'kedra-desktop-44-{build["run_id"]}-{build["run_attempt"]}.iso'
        and number(installer['size_bytes'], 64 * 1024**3) and digest(installer['sha256']), 'Invalid installer identity')
require(args.installer.name == installer['filename'], 'Installer filename differs from the candidate')
require(args.installer.is_file() and not args.installer.is_symlink(), 'Expected an ordinary installer file')
with args.installer.open('rb') as stream:
    hasher, size = hashlib.sha256(), 0
    while chunk := stream.read(min(1_048_576, installer['size_bytes'] - size + 1)):
        size += len(chunk)
        require(size <= installer['size_bytes'], 'Installer exceeds the candidate size')
        hasher.update(chunk)
    observed = hasher.hexdigest()
require(size == installer['size_bytes'] and observed == installer['sha256'], 'Installer size/hash differs from the candidate')
parts = candidate['parts']
require(isinstance(parts, list) and 1 <= len(parts) <= 64
        and len(parts) == (installer['size_bytes'] + 1900 * 1024**2 - 1) // (1900 * 1024**2)
        and parts == [installer['filename'] + f'.part{index:02d}' for index in range(len(parts))], 'Invalid ordered download parts')
require(isinstance(candidate['parts_sha256'], dict) and set(candidate['parts_sha256']) == set(parts)
        and all(digest(value) for value in candidate['parts_sha256'].values()), 'Missing reviewed download part hashes')
assets = {installer['filename']: {'sha256': observed, 'size_bytes': size}}
parts_hasher = hashlib.sha256()
for index, name in enumerate(parts):
    path = args.installer_parts_dir / name
    require(path.is_file() and not path.is_symlink(), 'Expected an ordinary installer part')
    expected_size = min(1900 * 1024**2, installer['size_bytes'] - index * 1900 * 1024**2)
    hasher, size = hashlib.sha256(), 0
    with path.open('rb') as stream:
        while chunk := stream.read(min(1_048_576, expected_size - size + 1)):
            size += len(chunk)
            require(size <= expected_size, 'Download part exceeds the declared split size')
            hasher.update(chunk)
            parts_hasher.update(chunk)
    require(size == expected_size, 'Download part is truncated')
    require(hasher.hexdigest() == candidate['parts_sha256'][name], 'Download part differs from the reviewed candidate')
    assets[name] = {'sha256': hasher.hexdigest(), 'size_bytes': size}
require(parts_hasher.hexdigest() == installer['sha256'], 'Download parts differ from the complete installer')
packages_bytes = bounded(args.packages, 4 * 1024**2)
provenance_bytes = bounded(args.provenance)
require(digest(candidate['packages_sha256']) and digest(candidate['provenance_sha256'])
        and hashlib.sha256(packages_bytes).hexdigest() == candidate['packages_sha256']
        and hashlib.sha256(provenance_bytes).hexdigest() == candidate['provenance_sha256'],
        'Inventory/provenance bytes differ from the reviewed candidate')
provenance = document(provenance_bytes)
provenance_identity = ['project', 'scope', 'source_revision', 'build', 'image_digest',
                       'home_manifest_sha256', 'packages_sha256', 'last_successful_resolution', 'installer']
require(set(provenance) == {'schema_version', 'format', 'installer_inputs', *provenance_identity}
        and type(provenance['schema_version']) is int and provenance['schema_version'] == 1
        and provenance['format'] == 'kedra-candidate-provenance'
        and all(provenance[key] == candidate[key] for key in provenance_identity),
        'Provenance identity differs from the candidate')
inputs = provenance['installer_inputs']
require(isinstance(inputs, dict) and set(inputs) == {'base_image', 'builder_image', 'builder_version'}
        and all(isinstance(inputs[key], str) and re.fullmatch('[a-z0-9./_-]+@sha256:[a-f0-9]{64}', inputs[key])
                for key in ['base_image', 'builder_image'])
        and isinstance(inputs['builder_version'], str) and 0 < len(inputs['builder_version']) <= 4096,
        'Missing resolved installer provenance')
qualification_bytes = bounded(args.qualification)
qualification = document(qualification_bytes)
require(set(qualification) == {'schema_version', 'candidate_sha256', 'method', 'checks', 'evidence'}
        and type(qualification['schema_version']) is int and qualification['schema_version'] == 1
        and qualification['candidate_sha256'] == candidate_hash
        and qualification['method'] in {'manual-vm', 'end-to-end-vm'}, 'Qualification does not bind the reviewed candidate')
require(isinstance(qualification['checks'], dict) and set(qualification['checks']) == checks
        and all(value == 'pass' for value in qualification['checks'].values()), 'Exact-media qualification has missing or unsuccessful cases')
require(isinstance(qualification['evidence'], list) and 1 <= len(qualification['evidence']) <= 32
        and all(isinstance(value, str) and 0 < len(value) <= 2048 for value in qualification['evidence']), 'Missing qualification evidence references')
require(digest(args.expected_fingerprint), 'Expected the independently reviewed public-key fingerprint')
public_key = bounded(args.public_key, 4096)
require(public_key == public_key.strip() + b'\n', 'Expected a canonical public PEM file with one final newline')
previous_record = None
with tempfile.TemporaryDirectory(prefix='kedra-release-public-') as temporary:
    captured = pathlib.Path(temporary)
    key = captured / 'release.pub'
    key.write_bytes(public_key)
    identity = cli('key', '--public-key', key)
    require(identity['key_fingerprint_sha256'] == args.expected_fingerprint, 'Public key differs from independently reviewed authority')
    if args.previous_channel:
        # Capture exact bytes once; the native verifier authenticates all four.
        for name, limit in [('release.json', 65_536), ('release.sig', 1024),
                            ('checkpoint.json', 65_536), ('checkpoint.sig', 1024)]:
            (captured / name).write_bytes(bounded(args.previous_channel / name, limit))
        previous = cli('channel', '--manifest', captured / 'release.json', '--signature', captured / 'release.sig',
                       '--checkpoint', captured / 'checkpoint.json', '--checkpoint-signature', captured / 'checkpoint.sig',
                       '--public-key', key, '--target', scope['target'], '--repository', scope['repository'])
        previous_record = document((captured / 'release.json').read_bytes())
        old_build = previous_record['build']
        require((build['run_id'], build['run_attempt']) > (old_build['run_id'], old_build['run_attempt']),
                'Older or repeated candidate build cannot advance the channel')
        state = previous['next_trust_state']
        sequence, generation = state['highest_release_sequence'] + 1, state['generation'] + 1
        prior_release, prior_checkpoint = state['highest_release_sha256'], state['checkpoint_sha256']
    else:
        sequence, generation = 1, 1
        prior_release = prior_checkpoint = None
require(number(sequence) and number(generation), 'Release/checkpoint sequence is exhausted')
now = int(time.time())
resolved = candidate['last_successful_resolution']
require(number(resolved) and 0 <= now - resolved <= 36 * 3600,
        'Candidate resolution is in the future or over 36 hours old; build a fresh candidate')
if previous_record:
    require(resolved >= state['last_successful_resolution'], 'Resolution history moved backwards')
release = {key: candidate[key] for key in ['schema_version', 'project', 'scope', 'source_revision', 'build',
                                         'image_digest', 'home_manifest_sha256', 'installer']}
release.update(schema_version=1, sequence=sequence, approval='promoted', minimum_protocol=1)
release_bytes = (json.dumps(release, sort_keys=True, indent=2) + '\n').encode()
checkpoint = {'schema_version': 1, 'project': 'Kedra', 'scope': scope, 'generation': generation,
              'release_sha256': hashlib.sha256(release_bytes).hexdigest(), 'issued_at': now,
              'expires_at': now + 7 * 24 * 3600, 'last_successful_resolution': resolved}
checkpoint_bytes = (json.dumps(checkpoint, sort_keys=True, indent=2) + '\n').encode()
files = {'candidate.json': candidate_bytes, 'source.json': source_bytes,
         'qualification.json': qualification_bytes, 'packages.txt': packages_bytes,
         'provenance.json': provenance_bytes, 'release.json': release_bytes,
         'checkpoint.json': checkpoint_bytes, 'release.pub': public_key,
         'release-key.sha256': (args.expected_fingerprint + '\n').encode()}
assets.update({name: {'sha256': hashlib.sha256(value).hexdigest(), 'size_bytes': len(value)}
               for name, value in files.items()})
checksums_bytes = ''.join(f'{assets[name]["sha256"]}  {name}\n' for name in sorted(assets)).encode('ascii')
request = {'schema_version': 1, 'candidate_sha256': candidate_hash,
           'qualification_sha256': hashlib.sha256(qualification_bytes).hexdigest(),
           'key_fingerprint_sha256': args.expected_fingerprint,
           'expected_previous_release_sha256': prior_release, 'expected_previous_checkpoint_sha256': prior_checkpoint,
           'release_sha256': hashlib.sha256(release_bytes).hexdigest(),
           'checkpoint_sha256': hashlib.sha256(checkpoint_bytes).hexdigest(),
           'sha256sums_sha256': hashlib.sha256(checksums_bytes).hexdigest(), 'assets': assets,
           'source_revision': candidate['source_revision'], 'build': build,
           'sequence': sequence, 'generation': generation, 'signed': False, 'published': False}
args.output.mkdir(parents=True, exist_ok=False)
for name, value in files.items():
    (args.output / name).write_bytes(value)
(args.output / 'SHA256SUMS').write_bytes(checksums_bytes)
(args.output / 'signing-request.json').write_text(json.dumps(request, sort_keys=True, indent=2) + '\n')
print(json.dumps(request))
