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
import selectors
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote

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


def api(endpoint, missing=False, timeout=30):
    result = subprocess.run(['gh', 'api', f'repos/{REPOSITORY}/{endpoint}'],
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            check=False, timeout=timeout)
    value = json.loads(result.stdout)
    if missing and result.returncode and value.get('status') == '404':
        return None
    require(result.returncode == 0, 'GitHub API refused public release inspection')
    return value


def release_for_tag(tag):
    # The tag endpoint returns published releases only. Authenticated listings
    # can include drafts, subject to the caller's repository access. The final
    # mutation boundary uses the publisher's contents:write token.
    matches = {}
    deadline = time.monotonic() + 60
    def lookup(endpoint):
        remaining = deadline - time.monotonic()
        require(remaining > 0, 'Release lookup timed out; absence cannot be established')
        return api(endpoint, timeout=min(30, remaining))
    for page in range(1, 101):
        releases = lookup(f'releases?per_page=100&page={page}')
        require(isinstance(releases, list) and len(releases) <= 100, 'Invalid release listing')
        for release in releases:
            require(isinstance(release, dict) and type(release.get('id')) is int
                    and release['id'] > 0 and isinstance(release.get('tag_name'), str), 'Invalid release identity')
            if release.get('tag_name') == tag:
                matches[release['id']] = release
        require(len(matches) <= 1, 'Multiple releases/drafts use this tag; explicit recovery is required')
        if len(releases) < 100:
            if not matches:
                return None
            release_id = next(iter(matches))
            release = lookup(f'releases/{release_id}')
            require(release['id'] == release_id and release['tag_name'] == tag,
                    'Release identity changed during lookup')
            return release
    raise SystemExit('Release listing exceeded its bound; absence cannot be established')


def checked_draft(release_id, tag, source, title, body):
    release = release_for_tag(tag)
    require(release is not None and release['id'] == release_id and release['tag_name'] == tag
            and release['target_commitish'] == source and release['name'] == title
            and release['body'] == body and release['draft'] is True
            and release['prerelease'] is False and release['published_at'] is None,
            'Draft metadata differs from this operation; preserve it for explicit recovery')
    return release


def tag_commit(tag):
    reference = api(f'git/ref/tags/{tag}', missing=True)
    if reference is None:
        return None
    target, seen = reference['object'], set()
    for _ in range(8):
        digest = target['sha']
        require(isinstance(digest, str) and re.fullmatch('[a-f0-9]{40}', digest)
                and digest not in seen, 'Invalid or cyclic Git tag target')
        seen.add(digest)
        if target['type'] == 'commit':
            return digest
        require(target['type'] == 'tag', 'Release tag does not resolve to a commit')
        target = api(f'git/tags/{digest}')['object']
    raise SystemExit('Annotated release tag exceeded its resolution bound')


def release_metadata(release):
    return {key: release[key] for key in ['id', 'tag_name', 'target_commitish', 'name', 'body',
                                         'draft', 'prerelease', 'published_at']}


def asset_identity(asset):
    require(isinstance(asset, dict) and type(asset.get('id')) is int and asset['id'] > 0
            and isinstance(asset.get('name'), str) and type(asset.get('size')) is int
            and 0 < asset['size'] <= 1900 * 1024**2 and asset.get('state') == 'uploaded',
            'Release asset identity is invalid or incomplete')
    return {key: asset[key] for key in ['id', 'name', 'size', 'state']}


def asset_inventory(release):
    assets = [asset_identity(asset) for asset in release['assets']]
    require(len({asset['id'] for asset in assets}) == len(assets)
            and len({asset['name'] for asset in assets}) == len(assets), 'Duplicate release asset identity/name')
    return {asset['id']: asset for asset in assets}


def unchanged_channel(previous, *, removed=False):
    current = release_for_tag(CHANNEL)
    if not previous:
        require(current is None and tag_commit(CHANNEL) is None,
                'Channel appeared after review; explicit recovery is required')
        return
    require(current is not None and release_metadata(current) == release_metadata(previous['release']),
            'Channel release metadata changed after review')
    expected = asset_inventory(previous['release'])
    if removed:
        expected.pop(previous['asset']['id'])
    require(asset_inventory(current) == expected, 'Channel asset identity changed after review')


def mutate(method, endpoint, *, input_path=None, content_type=None, timeout=30):
    arguments = ['gh', 'api', '--method', method, endpoint]
    if input_path is not None:
        arguments.extend(['--input', str(input_path)])
    if content_type is not None:
        arguments.extend(['--header', 'Content-Type: ' + content_type])
    result = subprocess.run(arguments, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False, timeout=timeout)
    require(result.returncode == 0, 'Release mutation could not be confirmed; preserve state for explicit recovery: '
            + result.stderr.decode(errors='replace')[:1500])
    return result.stdout


def upload_asset(release_id, path, expected, before_write):
    # Recheck metadata after hashing and immediately before writing to the pinned
    # numeric ID; another matching tag cannot redirect the transfer.
    require(type(release_id) is int and release_id > 0, 'Invalid upload release ID')
    require(file_identity(path, expected['size_bytes']) == expected, 'Upload bytes changed after verification')
    before_write()
    result = document(mutate('POST', f'https://uploads.github.com/repos/{REPOSITORY}/releases/{release_id}/assets'
                             f'?name={quote(path.name, safe="")}', input_path=path,
                             content_type='application/octet-stream', timeout=900))
    asset = asset_identity(result)
    require(asset['name'] == path.name and asset['size'] == expected['size_bytes'], 'Upload identity differs')
    return asset


def download_asset(asset, path):
    asset = asset_identity(asset)
    # Stream only the checked asset ID, with an exact byte bound and a deadline.
    # stderr is kept separately so it cannot deadlock the binary stdout pipe.
    deadline, written = time.monotonic() + 900, 0
    with tempfile.TemporaryFile() as errors, path.open('xb') as destination:
        child = subprocess.Popen(['gh', 'api', f'repos/{REPOSITORY}/releases/assets/{asset["id"]}',
                                  '--header', 'Accept: application/octet-stream'],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=errors)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(child.stdout, selectors.EVENT_READ)
                while True:
                    remaining = deadline - time.monotonic()
                    require(remaining > 0 and selector.select(remaining), 'Release asset download timed out')
                    chunk = os.read(child.stdout.fileno(), min(1024**2, asset['size'] - written + 1))
                    if not chunk:
                        break
                    written += len(chunk)
                    require(written <= asset['size'], 'Release asset download exceeds its checked size')
                    destination.write(chunk)
            require(child.wait(timeout=max(0.1, deadline - time.monotonic())) == 0
                    and written == asset['size'], 'Release asset download failed or is truncated')
        finally:
            if child.poll() is None:
                child.kill()
            child.wait()
            child.stdout.close()


def create_draft(tag, source, title, body, work, previous):
    notes = work / f'{tag}-notes.md'
    notes.write_bytes(body.encode())
    unchanged_channel(previous)
    require(release_for_tag(tag) is None and tag_commit(tag) is None,
            'Release/draft/tag already exists; inspect interrupted publication before explicit recovery')
    current_source()
    failure = None
    try:
        result = subprocess.run(['gh', 'release', 'create', tag, '--repo', REPOSITORY,
                                 '--target', source, '--draft', '--title', title, '--notes-file', str(notes)],
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                check=False, timeout=60)
        if result.returncode != 0:
            failure = f'exit {result.returncode}: ' + result.stderr.decode(errors='replace')[:1500]
    except subprocess.TimeoutExpired:
        # subprocess.run kills and reaps the CLI; the server may have committed.
        failure = 'timed out after 60 seconds'
    if failure is not None:
        print('Draft create ' + failure, file=sys.stderr, flush=True)
    # Bounded readback, never another create. A 5xx/lost response may already have
    # created a draft. Only this invocation's exact, uniquely marked empty draft
    # can continue; a pre-existing draft was refused before the request.
    release = None
    for delay in (0, 2, 5, 10, 20):
        if delay:
            time.sleep(delay)
        release = release_for_tag(tag)
        if release is not None:
            break
    require(release is not None, 'Draft creation could not be confirmed; inspect before explicit recovery')
    release = checked_draft(release['id'], tag, source, title, body)
    require(release['assets'] == [], 'New draft is not empty; preserve it for explicit recovery')
    if failure is not None:
        print('Authenticated readback confirmed exact empty draft '
              f'{release["id"]}. No create retry was submitted.', flush=True)
    return release['id']


def publish_draft(release_id, tag, source, title, body, work, expected_assets, previous):
    if tag != CHANNEL:
        unchanged_channel(previous)
    else:
        require(not previous, 'Only initial channel creation publishes a channel draft')
    draft = checked_draft(release_id, tag, source, title, body)
    require(asset_inventory(draft) == expected_assets, 'Draft assets changed after byte readback')
    require(tag_commit(tag) in (None, source), 'Release tag points at another commit; never force-move it')
    update = work / f'{tag}-publish.json'
    update.write_text(json.dumps({'draft': False, 'make_latest': 'false'}) + '\n')
    current_source()
    mutate('PATCH', f'repos/{REPOSITORY}/releases/{release_id}', input_path=update)
    published = api(f'releases/{release_id}')
    require(published['id'] == release_id and published['tag_name'] == tag
            and published['target_commitish'] == source and published['name'] == title
            and published['body'] == body and published['draft'] is False
            and published['prerelease'] is False and published['published_at'] is not None
            and asset_inventory(published) == expected_assets and tag_commit(tag) == source,
            'Publication metadata readback differs; inspect before retry')


def replace_channel(previous, path, expected):
    require(previous and previous['release']['tag_name'] == CHANNEL
            and previous['release']['draft'] is False and previous['asset']['name'] == 'channel.json'
            and path.name == 'channel.json', 'Only the validated published channel asset may be replaced')
    require(sha(read(path, 300_000)) == expected['sha256'], 'New channel bytes changed')
    unchanged_channel(previous)
    current_source()
    # Delete only the exact old asset whose bytes were downloaded and verified.
    # A failed/uncertain deletion stops; neither deletion nor upload is retried.
    mutate('DELETE', f'repos/{REPOSITORY}/releases/assets/{previous["asset"]["id"]}')
    def before_upload():
        unchanged_channel(previous, removed=True)
        current_source()
    return upload_asset(previous['release']['id'], path, expected, before_upload)


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


def channel(directory, snapshot=None):
    release = release_for_tag(CHANNEL)
    if release is None:
        return None
    require(not release['draft'], 'An incomplete channel draft requires explicit recovery')
    assets = [asset for asset in release['assets'] if asset['name'] == 'channel.json']
    require(len(assets) == 1 and assets[0]['state'] == 'uploaded' and 0 < assets[0]['size'] <= 300_000,
            'Channel asset is absent, incomplete or outside its bound')
    asset_inventory(release)
    directory.mkdir(parents=True, exist_ok=False)
    download_asset(assets[0], directory / 'channel.json')
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
    if snapshot is not None:
        snapshot.update(release=release, asset=asset_identity(assets[0]), sha256=sha(bundle))
    return sha(bundle)


def verify_channel(directory, cli, previous_state=None):
    command = [cli, 'release', 'channel', '--manifest', directory / 'release.json',
               '--signature', directory / 'release.sig', '--checkpoint', directory / 'checkpoint.json',
               '--checkpoint-signature', directory / 'checkpoint.sig',
               '--public-key', ROOT / 'build/release/authority/desktop.pub',
               '--target', 'desktop', '--repository', 'ghcr.io/reidond/kedra-desktop', '--json']
    if previous_state is not None:
        command.extend(['--previous-state', previous_state])
    return document(run(*command))


def verify_history(directory, cli):
    value = document(run(cli, 'release', 'history', '--manifest', directory / 'release.json',
                         '--signature', directory / 'release.sig', '--checkpoint', directory / 'checkpoint.json',
                         '--checkpoint-signature', directory / 'checkpoint.sig',
                         '--public-key', ROOT / 'build/release/authority/desktop.pub',
                         '--target', 'desktop', '--repository', 'ghcr.io/reidond/kedra-desktop', '--json'))
    require(value['historical_only'] is True and value['channel_freshness_verified'] is False
            and value['deployment_authorized'] is False, 'Expected authenticated predecessor ordering only')
    return value['ordering_state']


def candidate_identity(run_id, attempt, source):
    require(re.fullmatch('[1-9][0-9]{0,19}', run_id) and re.fullmatch('[1-9][0-9]{0,9}', attempt), 'Invalid run identity')
    value = api(f'actions/runs/{run_id}/attempts/{attempt}')
    require(value['conclusion'] == 'success' and value['status'] == 'completed'
            and value['head_sha'] == source and value['head_branch'] == 'main'
            and value['event'] in ('workflow_dispatch', 'schedule', 'push')
            and value['path'] == '.github/workflows/release.yml'
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
    previous_channel = {}
    previous_hash = channel(previous, previous_channel)
    require(previous_hash == request['expected_previous_bundle_sha256'], 'Channel changed after review; no publication')
    if previous_hash is not None:
        state = verify_history(previous, args.sysroot)
        require(state['highest_release_sha256'] == request['expected_previous_release_sha256']
                and state['checkpoint_sha256'] == request['expected_previous_checkpoint_sha256']
                and request['sequence'] == state['highest_release_sequence'] + 1
                and request['generation'] == state['generation'] + 1, 'Channel ordering differs from reviewed request')
        ordering_path = args.work / 'previous-ordering-state.json'
        ordering_path.write_text(json.dumps(state, sort_keys=True, indent=2) + '\n')
        # Only the predecessor uses historical verification. The newly signed pair
        # must still pass actual-clock freshness and every native ordering floor.
        verify_channel(signed, args.sysroot, previous_state=ordering_path)
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
    for name, limit in [('packages.txt', 4 * 1024**2), ('provenance.json', 4 * 1024**2)]:
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
    run_id, attempt = os.environ['GITHUB_RUN_ID'], os.environ['GITHUB_RUN_ATTEMPT']
    require(re.fullmatch('[1-9][0-9]{0,19}', run_id) and re.fullmatch('[1-9][0-9]{0,9}', attempt),
            'Invalid publication run identity')
    marker = (f'<!-- kedra-promotion run={run_id} attempt={attempt} '
              f'request-sha256={sha(read(signed / "signing-request.json"))} -->')
    title = f'Kedra desktop 44 / release {request["sequence"]}'
    body = (f'Kedra desktop / Fedora 44 / x86_64, release {request["sequence"]}.\n\n'
            f'Source `{source}`. Candidate build [{release["build"]["run_id"]}]'
            f'(https://github.com/{REPOSITORY}/actions/runs/{release["build"]["run_id"]}).\n\n'
            'Download all numbered ISO parts and verify/assemble them with sysroot release assemble. '
            'Read docs/INSTALL.md and docs/RELEASES.md at the source revision. '
            'Qualification and signed checksums are attached; physical hardware qualification is separate.\n\n'
            + marker + '\n')
    release_id = create_draft(tag, source, title, body, args.work, previous_channel)
    assets = list(sorted(signed.iterdir())) + [args.candidate_dir / 'release-installer' / name for name in candidate['parts']]
    upload_identities = {}
    uploaded_assets = {}
    for path in assets:
        identity = file_identity(path, path.stat().st_size)
        if path.name in request['assets']:
            require(identity == request['assets'][path.name], 'Release asset changed before upload: ' + path.name)
        upload_identities[path.name] = identity
        def before_upload():
            unchanged_channel(previous_channel)
            draft = checked_draft(release_id, tag, source, title, body)
            require(asset_inventory(draft) == uploaded_assets, 'Draft assets changed before upload')
            current_source()
        uploaded_asset = upload_asset(release_id, path, identity, before_upload)
        require(uploaded_asset['id'] not in uploaded_assets, 'Upload reused another asset ID')
        uploaded_assets[uploaded_asset['id']] = uploaded_asset
    uploaded = checked_draft(release_id, tag, source, title, body)
    require(asset_inventory(uploaded) == uploaded_assets, 'Draft asset inventory differs; leaving draft for inspection')
    # Verify remote bytes, not merely names/sizes, before making the version/channel discoverable.
    readback = args.work / 'version-readback'
    readback.mkdir(exist_ok=False)
    for asset in uploaded_assets.values():
        name = asset['name']
        expected = upload_identities[name]
        download_asset(asset, readback / name)
        require(file_identity(readback / name, expected['size_bytes']) == expected,
                'Draft asset readback differs; leaving draft for inspection: ' + name)
    publish_draft(release_id, tag, source, title, body, args.work, uploaded_assets, previous_channel)
    unchanged_channel(previous_channel)
    with tempfile.TemporaryDirectory(prefix='kedra-channel-recheck-') as temporary:
        require(channel(pathlib.Path(temporary) / 'current') == previous_hash, 'Channel changed before final upload')
    channel_path = signed / 'channel.json'
    channel_identity = file_identity(channel_path, channel_path.stat().st_size)
    if previous_hash is None:
        channel_title = 'Kedra desktop 44 / current channel'
        channel_body = ('Discovery only. Verify both signed documents and retained replay state. '
                        'Versioned releases contain installer assets.\n\n' + marker + '\n')
        channel_id = create_draft(CHANNEL, source, channel_title, channel_body, args.work, previous_channel)
        def before_channel_upload():
            channel_draft = checked_draft(channel_id, CHANNEL, source, channel_title, channel_body)
            require(asset_inventory(channel_draft) == {}, 'Channel draft is no longer empty')
            current_source()
        channel_asset = upload_asset(channel_id, channel_path, channel_identity, before_channel_upload)
        channel_draft = checked_draft(channel_id, CHANNEL, source, channel_title, channel_body)
        expected_channel_assets = {channel_asset['id']: channel_asset}
        require(asset_inventory(channel_draft) == expected_channel_assets,
                'Channel draft inventory differs; inspect before explicit recovery')
        draft_readback = args.work / 'channel-draft-readback.json'
        download_asset(channel_asset, draft_readback)
        require(file_identity(draft_readback, channel_identity['size_bytes']) == channel_identity,
                'Channel draft bytes differ before publication')
        publish_draft(channel_id, CHANNEL, source, channel_title, channel_body, args.work,
                      expected_channel_assets, previous_channel)
    else:
        # A replacement may temporarily be unavailable. One bundle prevents mixed signed pairs.
        channel_id = previous_channel['release']['id']
        channel_asset = replace_channel(previous_channel, channel_path, channel_identity)
    with tempfile.TemporaryDirectory(prefix='kedra-channel-readback-') as temporary:
        final_channel = {}
        require(channel(pathlib.Path(temporary) / 'current', final_channel) == channel_identity['sha256']
                and final_channel['release']['id'] == channel_id
                and final_channel['asset'] == channel_asset,
                'Published channel readback differs; inspect before retry')
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as summary:
        summary.write(f'Published [desktop release {request["sequence"]}](https://github.com/{REPOSITORY}/releases/tag/{tag}) '
                      f'and verified exact versioned asset and single-bundle channel readback. '
                      'No machine was enrolled, staged or rebooted.\n')


if __name__ == '__main__':
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
