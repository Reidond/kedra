"""Bounded image identity/material validation after native OCI authentication.

Production first performs a policy-enforcing Skopeo copy of the exact manifest.
This module validates its config and non-executed installed files, not signatures.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import time

LIMIT = 4 * 1024**2
LABEL = 'org.kedra.image.identity'
REPOSITORY = 'ghcr.io/reidond/kedra-desktop'
FIELDS = {'schema_version', 'project', 'target', 'architecture', 'fedora_release', 'repository', 'channel',
          'source_revision', 'source_manifest_sha256', 'resolved_inputs_sha256', 'workflow', 'epoch',
          'run_number', 'run_attempt', 'resolved_at', 'minimum_protocol'}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def read(path, limit=LIMIT):
    require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
            'Missing, nonregular or oversized input: ' + path.name)
    with path.open('rb') as stream:
        result = stream.read(limit + 1)
    require(len(result) <= limit, 'Input grew beyond its bound')
    return result


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'Duplicate JSON member')
        result[key] = value
    return result


def document(data):
    return json.loads(data, object_pairs_hook=unique)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')


def packages(data):
    rows, identities = [], set()
    for line in data.decode('utf-8').splitlines():
        row = line.split('\t')
        require(len(row) == 7, 'Unsupported installed RPM row')
        if row[0] == 'gpg-pubkey':
            continue
        require(row[1].isdigit() and row[4] in ('x86_64', 'noarch', 'i686')
                and all(re.fullmatch('[a-f0-9]{64}', value) for value in row[5:]), 'Missing RPM content identity')
        identity = tuple(row[:5])
        require(identity not in identities, 'Duplicate RPM identity')
        identities.add(identity)
        rows.append(row)
    require(rows, 'Empty RPM inventory')
    return sorted(rows)


def bootc_compatibility(rows):
    contract = document(read(Path(__file__).with_name('compatibility.json'), 4096))
    require(isinstance(contract, dict) and set(contract) == {'schema_version', 'bootc_version'}
            and type(contract['schema_version']) is int and contract['schema_version'] == 1
            and isinstance(contract['bootc_version'], str), 'Invalid bootc compatibility contract')
    parts = contract['bootc_version'].split('.')
    require(len(parts) == 3 and all(0 < len(part) <= 10 and re.fullmatch(r'0|[1-9][0-9]*', part)
                                  and int(part) <= 2**32 - 1 for part in parts), 'Invalid bootc version triplet')
    selected = [row for row in rows if row[0] == 'bootc']
    compatible = (len(selected) == 1 and selected[0][1] == '0' and selected[0][2] == contract['bootc_version']
                  and selected[0][4] == 'x86_64')
    return {'compatible': compatible, 'qualified_bootc_version': contract['bootc_version'],
            'installed_bootc_packages': selected}


def require_bootc(rows):
    result = bootc_compatibility(rows)
    require(result['compatible'], 'Unqualified bootc package; expected version ' + result['qualified_bootc_version']
            + ', observed ' + repr(result['installed_bootc_packages']) + '. Qualify/update the shared compatibility contract before signing.')
    return result


def validate(identity):
    require(isinstance(identity, dict) and set(identity) == FIELDS, 'Unsupported image identity fields')
    require(identity['schema_version'] == 2 and type(identity['schema_version']) is int
            and identity['project'] == 'Kedra' and identity['target'] == 'desktop'
            and identity['architecture'] == 'x86_64' and identity['fedora_release'] == 44
            and type(identity['fedora_release']) is int and identity['repository'] == REPOSITORY
            and identity['channel'] == 'stable' and identity['workflow'] == '.github/workflows/release.yml',
            'Image identity scope differs from the installed channel')
    require(all(type(identity[key]) is int and 0 < identity[key] <= 2**63 - 1
                for key in ('epoch', 'run_number', 'run_attempt', 'resolved_at', 'minimum_protocol'))
            and identity['epoch'] == 1 and identity['minimum_protocol'] <= 2, 'Unsupported image rank/protocol')
    require(isinstance(identity['source_revision'], str) and re.fullmatch('[a-f0-9]{40}', identity['source_revision'])
            and all(isinstance(identity[key], str) and re.fullmatch('[a-f0-9]{64}', identity[key])
                    for key in ('source_manifest_sha256', 'resolved_inputs_sha256')), 'Invalid image content identity')
    require(identity['resolved_at'] <= int(time.time()) + 300, 'Image resolution time is in the future')
    return identity


def verify(config, identity_bytes, inputs_bytes, source_bytes):
    identity = validate(document(identity_bytes))
    require(config.get('os') == 'linux' and config.get('architecture') == 'amd64', 'Wrong OCI platform')
    require(config.get('config', {}).get('Labels', {}).get(LABEL) == identity_bytes.decode('utf-8'),
            'Signed config label and installed image identity differ')
    require(identity_bytes == canonical(identity), 'Installed identity is not canonical')
    inputs = document(inputs_bytes)
    require(isinstance(inputs, dict) and set(inputs) == {'schema_version', 'base', 'source', 'artifacts', 'recipes', 'packages'}
            and inputs.get('schema_version') == 1 and inputs_bytes == canonical(inputs), 'Unsupported material record')
    require(sha(inputs_bytes) == identity['resolved_inputs_sha256']
            and sha(source_bytes) == identity['source_manifest_sha256'], 'Installed material/source hash differs')
    source = document(source_bytes)
    require(source.get('schema_version') == 1 and source['source_revision'] == identity['source_revision']
            and source['target']['id'] == identity['target'] and source['target']['image'] == REPOSITORY
            and source['target']['architecture'] == identity['architecture']
            and source['target']['fedora_release'] == identity['fedora_release']
            and inputs['source'] == {key: value for key, value in source.items()
                                    if key not in ('source_revision', 'input_scope')}, 'Source provenance differs')
    require(re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', inputs['base']), 'Unreviewed Fedora base')
    require(all(isinstance(inputs[key], dict) and inputs[key]
                and all(isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value) for value in inputs[key].values())
                for key in ('artifacts', 'recipes')), 'Incomplete artifact/recipe identity')
    rows = inputs['packages']
    require(packages(('\n'.join('\t'.join(row) for row in rows) + '\n').encode()) == rows, 'Noncanonical RPM material')
    return identity, inputs


def order(incoming, previous, digest, previous_digest):
    validate(incoming)
    require(re.fullmatch('sha256:[a-f0-9]{64}', digest) is not None, 'Invalid incoming digest')
    if previous is None:
        return 'first-image'
    validate(previous)
    current_rank = tuple(incoming[key] for key in ('epoch', 'run_number', 'run_attempt'))
    old_rank = tuple(previous[key] for key in ('epoch', 'run_number', 'run_attempt'))
    require(current_rank >= old_rank, 'Stable image rank would regress')
    require(incoming['resolved_at'] >= previous['resolved_at'], 'Signed package-resolution time would regress')
    if current_rank == old_rank:
        require(digest == previous_digest and incoming == previous, 'Different digest or identity at the same image rank')
        return 'already-current'
    return 'advance'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='operation', required=True)
    inspect = sub.add_parser('verify')
    for name in ('config', 'identity', 'inputs', 'source'):
        inspect.add_argument('--' + name, type=Path, required=True)
    inspect.add_argument('--previous-identity', type=Path)
    inspect.add_argument('--digest')
    inspect.add_argument('--previous-digest')
    compare = sub.add_parser('compare')
    compare.add_argument('--current', type=Path, required=True)
    compare.add_argument('--previous', type=Path, required=True)
    compatibility = sub.add_parser('bootc')
    compatibility.add_argument('--inputs', type=Path, required=True)
    args = parser.parse_args()
    if args.operation == 'bootc':
        print(json.dumps(require_bootc(document(read(args.inputs))['packages']), sort_keys=True))
    elif args.operation == 'compare':
        current, previous = document(read(args.current)), document(read(args.previous))
        print(json.dumps({'decision': 'unchanged' if current == previous else 'changed'}))
    else:
        identity, inputs = verify(document(read(args.config)), read(args.identity, 16384), read(args.inputs), read(args.source, 1024**2))
        result = {'identity': identity, 'material_verified': True, 'oci_signature_verified_by_this_command': False}
        if args.previous_identity:
            require(args.digest is not None and args.previous_digest is not None, 'Ordering needs both digests')
            result['ordering'] = order(identity, document(read(args.previous_identity, 16384)), args.digest, args.previous_digest)
        print(json.dumps(result, sort_keys=True))
