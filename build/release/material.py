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
# Closed release target table, shared by refresh.py, prepare-trust.py and the
# local installer. Each target has its own repository, key and signing
# environment; any other (target, architecture) pair is refused.
# installer/utm/kedra-utm.py imports this module with macOS /usr/bin/python3
# (3.9): keep module-level code and syntax Python 3.9 compatible.
TARGETS = {
    'desktop': {'architecture': 'x86_64', 'oci_architecture': 'amd64',
                'repository': 'ghcr.io/reidond/kedra-desktop', 'builds': 'ghcr.io/reidond/kedra-desktop-builds',
                'runner': 'ubuntu-24.04', 'environment': 'kedra-desktop-signing', 'prefix': 'KEDRA_DESKTOP',
                'public_key': 'build/release/authority/desktop.pub', 'key_sha256': 'build/release/authority/desktop.sha256'},
    'utm': {'architecture': 'aarch64', 'oci_architecture': 'arm64',
            'repository': 'ghcr.io/reidond/kedra-utm', 'builds': 'ghcr.io/reidond/kedra-utm-builds',
            'runner': 'ubuntu-24.04-arm', 'environment': 'kedra-utm-signing', 'prefix': 'KEDRA_UTM',
            'public_key': 'build/release/authority/utm.pub', 'key_sha256': 'build/release/authority/utm.sha256'},
}
FIELDS = {'schema_version', 'project', 'target', 'architecture', 'fedora_release', 'repository', 'channel',
          'source_revision', 'source_manifest_sha256', 'resolved_inputs_sha256', 'workflow', 'epoch',
          'run_number', 'run_attempt', 'resolved_at', 'minimum_protocol'}


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def enabled(target, architecture):
    spec = TARGETS.get(target) if isinstance(target, str) else None
    return spec if spec is not None and spec['architecture'] == architecture else None


def target_spec(target):
    require(isinstance(target, str) and target in TARGETS, 'Unsupported release target: ' + repr(target))
    return TARGETS[target]


def rpm_architectures(architecture):
    require(architecture in {spec['architecture'] for spec in TARGETS.values()}, 'Unsupported RPM architecture')
    return {architecture, 'noarch'} | ({'i686'} if architecture == 'x86_64' else set())


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


def packages(data, architecture):
    allowed = rpm_architectures(architecture)
    rows, identities = [], set()
    for line in data.decode('utf-8').splitlines():
        row = line.split('\t')
        require(len(row) == 7, 'Unsupported installed RPM row')
        if row[0] == 'gpg-pubkey':
            continue
        require(row[4] in allowed, 'RPM architecture is not allowed for this target')
        require(row[1].isdigit() and all(re.fullmatch('[a-f0-9]{64}', value) for value in row[5:]),
                'Missing RPM content identity')
        identity = tuple(row[:5])
        require(identity not in identities, 'Duplicate RPM identity')
        identities.add(identity)
        rows.append(row)
    require(rows, 'Empty RPM inventory')
    return sorted(rows)


def bootc_compatibility(rows, architecture):
    contract = document(read(Path(__file__).with_name('compatibility.json'), 4096))
    require(isinstance(contract, dict) and set(contract) == {'schema_version', 'bootc_version'}
            and type(contract['schema_version']) is int and contract['schema_version'] == 1
            and isinstance(contract['bootc_version'], str), 'Invalid bootc compatibility contract')
    parts = contract['bootc_version'].split('.')
    require(len(parts) == 3 and all(0 < len(part) <= 10 and re.fullmatch(r'0|[1-9][0-9]*', part)
                                  and int(part) <= 2**32 - 1 for part in parts), 'Invalid bootc version triplet')
    selected = [row for row in rows if row[0] == 'bootc']
    compatible = (len(selected) == 1 and selected[0][1] == '0' and selected[0][2] == contract['bootc_version']
                  and selected[0][4] == architecture)
    return {'compatible': compatible, 'qualified_bootc_version': contract['bootc_version'],
            'installed_bootc_packages': selected}


def require_bootc(rows, architecture):
    result = bootc_compatibility(rows, architecture)
    require(result['compatible'], 'Unqualified bootc package; expected version ' + result['qualified_bootc_version']
            + ', observed ' + repr(result['installed_bootc_packages']) + '. Qualify/update the shared compatibility contract before signing.')
    return result


def validate(identity, target):
    spec = target_spec(target)
    require(isinstance(identity, dict) and set(identity) == FIELDS, 'Unsupported image identity fields')
    require(identity['schema_version'] == 2 and type(identity['schema_version']) is int
            and identity['project'] == 'Kedra' and identity['target'] == target
            and enabled(identity['target'], identity['architecture']) is spec and identity['fedora_release'] == 44
            and type(identity['fedora_release']) is int and identity['repository'] == spec['repository']
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


def verify(config, identity_bytes, inputs_bytes, source_bytes, target):
    spec = target_spec(target)
    identity = validate(document(identity_bytes), target)
    require(config.get('os') == 'linux' and config.get('architecture') == spec['oci_architecture'], 'Wrong OCI platform')
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
            and source['target']['id'] == identity['target'] and source['target']['image'] == spec['repository']
            and source['target']['architecture'] == identity['architecture']
            and source['target']['fedora_release'] == identity['fedora_release']
            and inputs['source'] == {key: value for key, value in source.items()
                                    if key not in ('source_revision', 'input_scope')}, 'Source provenance differs')
    require(re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', inputs['base']), 'Unreviewed Fedora base')
    require(all(isinstance(inputs[key], dict) and inputs[key]
                and all(isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value) for value in inputs[key].values())
                for key in ('artifacts', 'recipes')), 'Incomplete artifact/recipe identity')
    rows = inputs['packages']
    require(packages(('\n'.join('\t'.join(row) for row in rows) + '\n').encode(), identity['architecture']) == rows,
            'Noncanonical RPM material')
    return identity, inputs


def order(incoming, previous, digest, previous_digest, target):
    validate(incoming, target)
    require(re.fullmatch('sha256:[a-f0-9]{64}', digest) is not None, 'Invalid incoming digest')
    if previous is None:
        return 'first-image'
    validate(previous, target)
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
    inspect.add_argument('--target', choices=sorted(TARGETS), required=True)
    for name in ('config', 'identity', 'inputs', 'source'):
        inspect.add_argument('--' + name, type=Path, required=True)
    inspect.add_argument('--previous-identity', type=Path)
    inspect.add_argument('--digest')
    inspect.add_argument('--previous-digest')
    compare = sub.add_parser('compare')
    compare.add_argument('--current', type=Path, required=True)
    compare.add_argument('--previous', type=Path, required=True)
    compatibility = sub.add_parser('bootc')
    compatibility.add_argument('--target', choices=sorted(TARGETS), required=True)
    compatibility.add_argument('--inputs', type=Path, required=True)
    args = parser.parse_args()
    if args.operation == 'bootc':
        print(json.dumps(require_bootc(document(read(args.inputs))['packages'], TARGETS[args.target]['architecture']),
                         sort_keys=True))
    elif args.operation == 'compare':
        current, previous = document(read(args.current)), document(read(args.previous))
        print(json.dumps({'decision': 'unchanged' if current == previous else 'changed'}))
    else:
        identity, inputs = verify(document(read(args.config)), read(args.identity, 16384), read(args.inputs),
                                  read(args.source, 1024**2), args.target)
        result = {'identity': identity, 'material_verified': True, 'oci_signature_verified_by_this_command': False}
        if args.previous_identity:
            require(args.digest is not None and args.previous_digest is not None, 'Ordering needs both digests')
            result['ordering'] = order(identity, document(read(args.previous_identity, 16384)), args.digest,
                                       args.previous_digest, args.target)
        print(json.dumps(result, sort_keys=True))
