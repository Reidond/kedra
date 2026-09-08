"""Public Actions orchestration; never receives release private keys.

The workflow serializes candidate builds and publication for this single scope.
All authority comes from exact signatures and independently reviewed inputs.
"""
import argparse
import base64
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

REPOSITORY = 'Reidond/kedra'
CHANNEL = 'desktop-44-x86_64-channel'
ROOT = pathlib.Path(__file__).resolve().parents[2]


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def run(*arguments):
    result = subprocess.run(list(map(str, arguments)), stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    require(result.returncode == 0, 'Public operation failed: ' + result.stderr.decode(errors='replace')[:1500])
    return result.stdout


def api(endpoint, missing=False):
    result = subprocess.run(['gh', 'api', f'repos/{REPOSITORY}/{endpoint}'],
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    value = json.loads(result.stdout)
    if missing and result.returncode and value.get('status') == '404':
        return None
    require(result.returncode == 0, 'GitHub API refused public release inspection')
    return value


def read(path, limit=65_536):
    require(path.is_file() and not path.is_symlink(), 'Expected an ordinary public input file')
    with path.open('rb') as stream:
        value = stream.read(limit + 1)
    require(0 < len(value) <= limit, 'Public input is empty or exceeds its bound')
    return value


def sha(value):
    return hashlib.sha256(value).hexdigest()


def file_identity(path, expected_size):
    require(path.is_file() and not path.is_symlink(), 'Expected an ordinary release asset')
    require(type(expected_size) is int and 0 < expected_size <= 64 * 1024**3, 'Invalid release asset size')
    hasher, size = hashlib.sha256(), 0
    with path.open('rb') as stream:
        while chunk := stream.read(min(1024**2, expected_size - size + 1)):
            size += len(chunk)
            require(size <= expected_size, 'Release asset exceeds its reviewed size')
            hasher.update(chunk)
    require(size == expected_size, 'Release asset is truncated')
    return {'sha256': hasher.hexdigest(), 'size_bytes': size}


def unique(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'Duplicate JSON member')
        value[key] = item
    return value


def document(value):
    value = json.loads(value, object_pairs_hook=unique)
    require(isinstance(value, dict), 'Expected a public JSON object')
    return value


def current_source():
    require(os.environ.get('GITHUB_ACTIONS') == 'true'
            and os.environ.get('GITHUB_REPOSITORY') == REPOSITORY
            and os.environ.get('GITHUB_REF') == 'refs/heads/main', 'Main-only Actions operation')
    source = os.environ['GITHUB_SHA']
    require(api('git/ref/heads/main')['object']['sha'] == source, 'Main advanced; prepare a current candidate')
    require(run('git', 'rev-parse', 'HEAD').decode().strip() == source, 'Checkout differs from accepted source')
    return source


def channel(directory):
    release = api(f'releases/tags/{CHANNEL}', missing=True)
    if release is None:
        return None
    require(not release['draft'], 'An incomplete channel draft requires explicit recovery')
    assets = [asset for asset in release['assets'] if asset['name'] == 'channel.json']
    require(len(assets) == 1 and assets[0]['state'] == 'uploaded' and 0 < assets[0]['size'] <= 300_000,
            'Channel asset is absent, incomplete or outside its bound')
    directory.mkdir(parents=True, exist_ok=False)
    run('gh', 'release', 'download', CHANNEL, '--repo', REPOSITORY,
        '--pattern', 'channel.json', '--dir', directory)
    bundle = read(directory / 'channel.json', 300_000)
    value = document(bundle)
    require(set(value) == {'schema_version', 'release', 'checkpoint'} and type(value['schema_version']) is int
            and value['schema_version'] == 1, 'Unsupported channel bundle')
    for name in ['release', 'checkpoint']:
        signed = value[name]
        require(isinstance(signed, dict) and set(signed) == {'payload', 'signature'}
                and isinstance(signed['payload'], str) and isinstance(signed['signature'], str), 'Invalid signed document')
        payload, signature = signed['payload'].encode(), signed['signature'].encode()
        require(0 < len(payload) <= 65_536 and 0 < len(signature) <= 1024, 'Signed document exceeds its bound')
        (directory / f'{name}.json').write_bytes(payload)
        (directory / f'{name}.sig').write_bytes(signature)
    return sha(bundle)


def verify_channel(directory, cli):
    return document(run(cli, 'release', 'channel', '--manifest', directory / 'release.json',
                        '--signature', directory / 'release.sig', '--checkpoint', directory / 'checkpoint.json',
                        '--checkpoint-signature', directory / 'checkpoint.sig',
                        '--public-key', ROOT / 'build/release/authority/desktop.pub',
                        '--target', 'desktop', '--repository', 'ghcr.io/reidond/kedra-desktop', '--json'))


def candidate_identity(run_id, attempt, source):
    require(re.fullmatch('[1-9][0-9]{0,19}', run_id) and re.fullmatch('[1-9][0-9]{0,9}', attempt), 'Invalid run identity')
    value = api(f'actions/runs/{run_id}/attempts/{attempt}')
    require(value['conclusion'] == 'success' and value['status'] == 'completed'
            and value['head_sha'] == source and value['head_branch'] == 'main'
            and value['event'] == 'workflow_dispatch' and value['path'] == '.github/workflows/release.yml'
            and value['repository']['full_name'] == REPOSITORY
            and value['run_attempt'] == int(attempt), 'Candidate is not the exact successful current-main build')


def assemble(candidate_dir, record, output):
    build = record['build']
    name = f'kedra-desktop-44-{build["run_id"]}-{build["run_attempt"]}.iso'
    require(record['installer']['filename'] == name, 'Unexpected ISO filename')
    size = record['installer']['size_bytes']
    require(type(size) is int and 0 < size <= 64 * 1024**3, 'ISO size outside its bound')
    count = (size + 1900 * 1024**2 - 1) // (1900 * 1024**2)
    names = [name + f'.part{index:02d}' for index in range(count)]
    require(record['parts'] == names, 'Unexpected ordered ISO parts')
    output.mkdir(parents=True, exist_ok=False)
    hasher, written = hashlib.sha256(), 0
    with (output / name).open('xb') as destination:
        for part in names:
            path = candidate_dir / 'release-installer' / part
            require(path.is_file() and not path.is_symlink(), 'Missing ordinary installer part')
            with path.open('rb') as stream:
                while chunk := stream.read(1024**2):
                    written += len(chunk)
                    require(written <= size, 'Installer exceeds its declared size')
                    destination.write(chunk)
                    hasher.update(chunk)
    require(written == size and hasher.hexdigest() == record['installer']['sha256'], 'Complete installer hash/size mismatch')
    return output / name


def prepare(args):
    source = current_source()
    run_id, attempt = os.environ['CANDIDATE_RUN'], os.environ['CANDIDATE_ATTEMPT']
    candidate_identity(run_id, attempt, source)
    evidence = args.candidate_dir / 'release-evidence'
    candidate = read(evidence / 'candidate.json')
    record = document(candidate)
    require(sha(candidate) == os.environ['REVIEWED_CANDIDATE_SHA256'], 'Candidate differs from independently reviewed hash')
    require(record['build']['run_id'] == int(run_id) and record['build']['run_attempt'] == int(attempt)
            and record['source_revision'] == source, 'Candidate artifact identity differs from successful workflow')
    qualification = os.environ['QUALIFICATION_JSON'].encode()
    require(0 < len(qualification) <= 65_536, 'Qualification exceeds its bound')
    qualification_path = args.work / 'qualification.json'
    qualification_path.write_bytes(qualification)
    iso = assemble(args.candidate_dir, record, args.work / 'assembled')
    previous = args.work / 'previous'
    previous_hash = channel(previous)
    bootstrap = os.environ['BOOTSTRAP'] == 'true'
    require((previous_hash is None) == bootstrap, 'Bootstrap must match an actually absent channel')
    command = [sys.executable, ROOT / 'build/release/prepare-release.py',
               '--candidate', evidence / 'candidate.json', '--reviewed-candidate-sha256', sha(candidate),
               '--source', evidence / 'source.json', '--installer', iso,
               '--installer-parts-dir', args.candidate_dir / 'release-installer',
               '--packages', evidence / 'packages.txt', '--provenance', evidence / 'provenance.json',
               '--qualification', qualification_path,
               '--public-key', ROOT / 'build/release/authority/desktop.pub',
               '--expected-fingerprint', read(ROOT / 'build/release/authority/desktop.sha256').decode().strip(),
               '--sysroot', args.sysroot, '--output', args.work / 'unsigned']
    command.extend(['--bootstrap'] if bootstrap else ['--previous-channel', previous])
    run(*command)
    request_path = args.work / 'unsigned/signing-request.json'
    request = document(read(request_path))
    request['expected_previous_bundle_sha256'] = previous_hash
    request_path.write_text(json.dumps(request, sort_keys=True, indent=2) + '\n')
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
        for name, filename in [('release', 'release.json'), ('checkpoint', 'checkpoint.json'),
                               ('signing_request', 'signing-request.json'), ('candidate', 'candidate.json'),
                               ('checksums', 'SHA256SUMS')]:
            output.write(name + '=' + base64.b64encode(read(args.work / 'unsigned' / filename)).decode() + '\n')
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as summary:
        summary.write(f'Proposed desktop release {request["sequence"]}, checkpoint {request["generation"]}.\n\n'
                      f'Candidate SHA-256: `{sha(candidate)}`\n\nQualification SHA-256: `{sha(qualification)}`\n\n'
                      f'SHA256SUMS SHA-256: `{request["sha256sums_sha256"]}`\n\n'
                      f'Source: `{source}`. Inspect qualification.json and exact installation evidence before approving signing.\n')


def publish(args):
    source = current_source()
    signed = args.work / 'signed'
    signed.mkdir(exist_ok=False)
    for name, variable in [('release.json', 'RELEASE_B64'), ('checkpoint.json', 'CHECKPOINT_B64'),
                           ('release.sig', 'RELEASE_SIG_B64'), ('checkpoint.sig', 'CHECKPOINT_SIG_B64'),
                           ('SHA256SUMS', 'CHECKSUMS_B64'), ('SHA256SUMS.sig', 'CHECKSUMS_SIG_B64'),
                           ('signing-request.json', 'REQUEST_B64')]:
        value = base64.b64decode(os.environ[variable], validate=True)
        require(0 < len(value) <= (1024 if name.endswith('.sig') else 65_536), 'Signed input exceeds its bound')
        (signed / name).write_bytes(value)
    request = document(read(signed / 'signing-request.json'))
    release_bytes, checkpoint_bytes = read(signed / 'release.json'), read(signed / 'checkpoint.json')
    require(sha(release_bytes) == request['release_sha256'] and sha(checkpoint_bytes) == request['checkpoint_sha256']
            and sha(read(signed / 'SHA256SUMS')) == request['sha256sums_sha256']
            and request['source_revision'] == source, 'Signed bytes differ from the reviewed request')
    identity = document(run(args.sysroot, 'release', 'key', '--public-key',
                            ROOT / 'build/release/authority/desktop.pub', '--json'))
    require(identity['key_fingerprint_sha256'] == request['key_fingerprint_sha256'], 'Checksum authority differs')
    # Same detached base64 DER ECDSA/SHA-256 encoding as the metadata signatures.
    checksum_der = args.work / 'checksums.der'
    checksum_der.write_bytes(base64.b64decode(read(signed / 'SHA256SUMS.sig', 1024).strip(), validate=True))
    run('openssl', 'dgst', '-sha256', '-verify', ROOT / 'build/release/authority/desktop.pub',
        '-signature', checksum_der, signed / 'SHA256SUMS')
    release = document(release_bytes)
    candidate_identity(str(release['build']['run_id']), str(release['build']['run_attempt']), source)
    verified = verify_channel(signed, args.sysroot)
    require(verified['next_trust_state']['highest_release_sequence'] == request['sequence'], 'Signed sequence differs')
    previous = args.work / 'previous'
    previous_hash = channel(previous)
    require(previous_hash == request['expected_previous_bundle_sha256'], 'Channel changed after review; no publication')
    if previous_hash is not None:
        state = verify_channel(previous, args.sysroot)['next_trust_state']
        require(state['highest_release_sha256'] == request['expected_previous_release_sha256']
                and state['checkpoint_sha256'] == request['expected_previous_checkpoint_sha256']
                and request['sequence'] == state['highest_release_sequence'] + 1
                and request['generation'] == state['generation'] + 1, 'Channel ordering differs from reviewed request')
    else:
        require(request['sequence'] == 1 and request['generation'] == 1
                and request['expected_previous_release_sha256'] is None
                and request['expected_previous_checkpoint_sha256'] is None, 'Invalid bootstrap sequence')
    evidence = args.candidate_dir / 'release-evidence'
    candidate_bytes = read(evidence / 'candidate.json')
    require(sha(candidate_bytes) == request['candidate_sha256'], 'Publication candidate changed')
    candidate = document(candidate_bytes)
    require(candidate['schema_version'] == 2 and all(candidate[key] == release[key] for key in
            ['project', 'scope', 'source_revision', 'build', 'image_digest', 'home_manifest_sha256', 'installer']),
            'Publication candidate differs from signed release')
    iso = assemble(args.candidate_dir, candidate, args.work / 'assembled')
    run(args.sysroot, 'release', 'verify', '--manifest', signed / 'release.json', '--signature', signed / 'release.sig',
        '--public-key', ROOT / 'build/release/authority/desktop.pub', '--target', 'desktop', '--artifact', iso, '--json')
    qualification = os.environ['QUALIFICATION_JSON'].encode()
    require(sha(qualification) == request['qualification_sha256'], 'Qualification changed after review')
    (signed / 'qualification.json').write_bytes(qualification)
    (signed / 'candidate.json').write_bytes(candidate_bytes)
    (signed / 'source.json').write_bytes(read(evidence / 'source.json', 1_048_576))
    require(sha(read(signed / 'source.json', 1_048_576)) == release['home_manifest_sha256'], 'Publication source differs')
    for name, limit in [('packages.txt', 4 * 1024**2), ('provenance.json', 65_536)]:
        value = read(evidence / name, limit)
        require(sha(value) == candidate[name.split('.')[0] + '_sha256'], 'Publication inventory/provenance differs')
        (signed / name).write_bytes(value)
    (signed / 'release.pub').write_bytes(read(ROOT / 'build/release/authority/desktop.pub', 4096))
    (signed / 'release-key.sha256').write_bytes(read(ROOT / 'build/release/authority/desktop.sha256'))
    checksum_paths = {name: signed / name for name in ['candidate.json', 'source.json', 'qualification.json',
                      'packages.txt', 'provenance.json', 'release.json', 'checkpoint.json',
                      'release.pub', 'release-key.sha256']}
    checksum_paths[iso.name] = iso
    checksum_paths.update({name: args.candidate_dir / 'release-installer' / name for name in candidate['parts']})
    require(set(request['assets']) == set(checksum_paths), 'Reviewed checksum inventory differs from fixed release assets')
    for name, path in checksum_paths.items():
        expected = request['assets'][name]
        require(file_identity(path, expected['size_bytes']) == expected, 'Asset differs from reviewed signing request: ' + name)
    expected_checksums = ''.join(f'{request["assets"][name]["sha256"]}  {name}\n'
                                 for name in sorted(checksum_paths)).encode('ascii')
    require(read(signed / 'SHA256SUMS') == expected_checksums, 'Signed checksums differ from actual release assets')
    bundle = {'schema_version': 1,
              'release': {'payload': release_bytes.decode(), 'signature': read(signed / 'release.sig', 1024).decode()},
              'checkpoint': {'payload': checkpoint_bytes.decode(), 'signature': read(signed / 'checkpoint.sig', 1024).decode()}}
    (signed / 'channel.json').write_text(json.dumps(bundle, sort_keys=True, indent=2) + '\n')
    tag = f'desktop-44-x86_64-r{request["sequence"]}'
    require(api(f'releases/tags/{tag}', missing=True) is None and api(f'git/ref/tags/{tag}', missing=True) is None,
            'Release/tag already exists; inspect interrupted publication before explicit recovery')
    notes = args.work / 'release-notes.md'
    notes.write_text(f'Kedra desktop / Fedora 44 / x86_64, release {request["sequence"]}.\n\n'
                     f'Source `{source}`. Candidate build [{release["build"]["run_id"]}]'
                     f'(https://github.com/{REPOSITORY}/actions/runs/{release["build"]["run_id"]}).\n\n'
                     'Download all numbered ISO parts and verify/assemble them with sysroot release assemble. '
                     'Read docs/INSTALL.md and docs/RELEASES.md at the source revision. '
                     'Qualification and signed checksums are attached; physical hardware qualification is separate.\n')
    current_source()
    run('gh', 'release', 'create', tag, '--repo', REPOSITORY, '--target', source, '--draft',
        '--title', f'Kedra desktop 44 / release {request["sequence"]}', '--notes-file', notes)
    assets = list(sorted(signed.iterdir())) + [args.candidate_dir / 'release-installer' / name for name in candidate['parts']]
    upload_identities = {}
    for path in assets:
        identity = file_identity(path, path.stat().st_size)
        if path.name in request['assets']:
            require(identity == request['assets'][path.name], 'Release asset changed before upload: ' + path.name)
        upload_identities[path.name] = identity
        run('gh', 'release', 'upload', tag, path, '--repo', REPOSITORY)
    uploaded = api(f'releases/tags/{tag}')
    require(len(uploaded['assets']) == len(assets)
            and {asset['name']: asset['size'] for asset in uploaded['assets'] if asset['state'] == 'uploaded'}
            == {name: item['size_bytes'] for name, item in upload_identities.items()}, 'Draft asset inventory differs; leaving draft for inspection')
    # Verify remote bytes, not merely names/sizes, before making the version/channel discoverable.
    readback = args.work / 'version-readback'
    readback.mkdir(exist_ok=False)
    run('gh', 'release', 'download', tag, '--repo', REPOSITORY, '--dir', readback)
    for name, expected in upload_identities.items():
        require(file_identity(readback / name, expected['size_bytes']) == expected,
                'Draft asset readback differs; leaving draft for inspection: ' + name)
    current_source()
    run('gh', 'release', 'edit', tag, '--repo', REPOSITORY, '--draft=false', '--latest=false')
    with tempfile.TemporaryDirectory(prefix='kedra-channel-recheck-') as temporary:
        require(channel(pathlib.Path(temporary) / 'current') == previous_hash, 'Channel changed before final upload')
    if previous_hash is None:
        run('gh', 'release', 'create', CHANNEL, '--repo', REPOSITORY, '--target', source, '--draft',
            '--title', 'Kedra desktop 44 / current channel', '--notes',
            'Discovery only. Verify both signed documents and retained replay state. Versioned releases contain installer assets.')
        run('gh', 'release', 'upload', CHANNEL, signed / 'channel.json', '--repo', REPOSITORY)
        run('gh', 'release', 'edit', CHANNEL, '--repo', REPOSITORY, '--draft=false', '--latest=false')
    else:
        # A replacement may temporarily be unavailable. One bundle prevents mixed signed pairs.
        run('gh', 'release', 'upload', CHANNEL, signed / 'channel.json', '--repo', REPOSITORY, '--clobber')
    with tempfile.TemporaryDirectory(prefix='kedra-channel-readback-') as temporary:
        require(channel(pathlib.Path(temporary) / 'current') == sha(read(signed / 'channel.json', 300_000)),
                'Published channel readback differs; inspect before retry')
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as summary:
        summary.write(f'Published [desktop release {request["sequence"]}](https://github.com/{REPOSITORY}/releases/tag/{tag}) '
                      f'and verified exact versioned asset and single-bundle channel readback. '
                      'No machine was enrolled, staged or rebooted.\n')


parser = argparse.ArgumentParser()
parser.add_argument('operation', choices=['prepare', 'publish'])
parser.add_argument('--candidate-dir', type=pathlib.Path, required=True)
parser.add_argument('--work', type=pathlib.Path, required=True)
parser.add_argument('--sysroot', type=pathlib.Path, required=True)
arguments = parser.parse_args()
arguments.sysroot = arguments.sysroot.resolve()
arguments.work.mkdir(parents=True, exist_ok=False)
if arguments.operation == 'prepare':
    prepare(arguments)
else:
    publish(arguments)
