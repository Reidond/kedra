#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Bounded image identity/material validation after native OCI authentication.

Production first performs a policy-enforcing Skopeo copy of the exact manifest.
This module validates its config and non-executed installed files, not signatures.
"""
import argparse
import hashlib
import json
import re
import subprocess
import time
from pathlib import Path

LIMIT = 4 * 1024**2
LOCK_LIMIT = 8 * 1024**2
LABEL = 'org.kedra.image.identity'
IMAGE = 'usr/src/kedra/image/'
# Closed release target table, shared with the Rust helper (embedded at build
# time), the image input tooling and the local installer. Each target has its
# own repository, key and signing environment; any other (target, architecture)
# pair is refused.
TABLE = json.loads(Path(__file__).with_name('targets.json').read_text())
if TABLE.get('schema_version') != 1:
    raise RuntimeError('Unsupported release target table')
TARGETS = TABLE['targets']
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


def catalog_pins(root, binaries, target, destination):
    """Derive the run's pin envelope once, through the selected frontend.

    This is trusted public preflight, before signing: the pinned `sysroot` derives
    the envelope from the committed intent and this function writes it to a new
    file, so no caller-supplied pin file or claimed hash is ever trusted.
    """
    process = subprocess.run([str(binaries / 'sysroot'), 'catalog', 'pins',
                              '--input-root', str(root / (IMAGE + 'packages')),
                              '--entry', 'catalog.kedra', '--lock', 'packages.lock.json',
                              '--target', target, '--target-policy', str(root / (IMAGE + 'package-policy.json'))],
                             stdin=subprocess.DEVNULL, capture_output=True, timeout=60, check=False)
    require(process.returncode == 0 and 0 < len(process.stdout) <= 8 * 1024**2,
            'Catalog pin derivation failed: ' + process.stderr.decode(errors='replace')[:1500])
    with destination.open('xb') as stream:
        stream.write(process.stdout)
    return process.stdout


def language_material(root, source, pins, target):
    frontend = source.get('package_frontend')
    require(isinstance(frontend, dict) and frontend.get('schema_version') == 1
            and frontend.get('format') == 'kedra' and pins.get('schema_version') == 2
            and set(pins) == {'schema_version', 'format', 'frontend_version', 'namespace',
                              'target', 'intent_sha256', 'inputs', 'packages', 'archives',
                              'builders', 'templates'}
            and pins['format'] == 'kedra' and pins['frontend_version'] == 1
            and pins['namespace'] == 'kedra' and pins['target'] == target
            and pins['intent_sha256'] == frontend.get('intent_sha256')
            and pins['builders'] == frontend.get('builders')
            and isinstance(pins['packages'], dict) and 0 < len(pins['packages']) <= 256
            and sorted(pins['packages']) == frontend.get('selected_packages'),
            'Language material differs from the independently selected committed intent')
    inputs = frontend.get('inputs')
    require(isinstance(inputs, dict) and 0 < len(inputs) <= 4227,
            'Missing bounded committed frontend inventory')
    for path, expected in inputs.items():
        require(isinstance(path, str) and path.startswith(IMAGE)
                and all(part not in ('', '.', '..') for part in path.split('/'))
                and sha(read(root / path, 8 * 1024**2)) == expected,
                'Frontend input differs from committed admission')
    return sorted(inputs)


RPM_LOCK_FIELDS = {'schema_version', 'format', 'target', 'architecture', 'fedora_release', 'base',
                   'repositories', 'intent_sha256', 'rpms', 'foundation', 'builders'}
RPM_FIELDS = {'name', 'epoch', 'version', 'release', 'arch', 'size', 'sha256', 'header_sha256',
              'payload_sha256', 'source_rpm', 'repository', 'durable_url', 'mirror_url', 'object'}
ROLE_FIELDS = {'requests', 'remove', 'install', 'packages'}


def lock_pins_sha256(lock):
    """Identity of what an RPM lock installs: the lock without per-run mirror URLs."""
    pins = dict(lock, rpms={name: {key: value for key, value in rpm.items() if key != 'mirror_url'}
                            for name, rpm in lock['rpms'].items()})
    return sha(canonical(pins))


def lock_role(lock, role, architecture):
    require(isinstance(role, dict) and set(role) == ROLE_FIELDS
            and all(isinstance(role[key], list) and role[key] == sorted(set(role[key]))
                    for key in ('requests', 'remove', 'install')), 'Invalid RPM lock role')
    rows = role['packages']
    require(isinstance(rows, list) and rows and all(isinstance(row, list) for row in rows)
            and packages(('\n'.join('\t'.join(row) for row in rows) + '\n').encode(), architecture) == rows,
            'Noncanonical RPM lock inventory')
    installed = {tuple(row) for row in rows}
    for name in role['install']:
        rpm = lock['rpms'].get(name)
        require(isinstance(rpm, dict) and tuple(rpm[key] for key in (
            'name', 'epoch', 'version', 'release', 'arch', 'header_sha256', 'payload_sha256')) in installed,
            'RPM lock installs a file outside its inventory: ' + str(name))
    return rows


def rpm_lock(data, source, base, target):
    """Check a run's RPM lock against the committed source it was resolved for."""
    spec = target_spec(target)
    lock = document(data)
    require(isinstance(lock, dict) and set(lock) == RPM_LOCK_FIELDS and canonical(lock) == data,
            'Unsupported or noncanonical RPM lock')
    frontend = source.get('package_frontend')
    require(isinstance(frontend, dict) and lock['schema_version'] == 1 and lock['format'] == 'kedra-rpm-lock'
            and lock['fedora_release'] == 44 and lock['target'] == target
            and lock['architecture'] == spec['architecture'] and lock['base'] == base
            and lock['intent_sha256'] == frontend.get('intent_sha256'),
            'RPM lock belongs to another target, base or package intent')
    rpms = lock['rpms']
    require(isinstance(rpms, dict) and all(
        isinstance(rpm, dict) and set(rpm) == RPM_FIELDS and isinstance(rpm['size'], int) and rpm['size'] > 0
        and re.fullmatch('[a-f0-9]{64}', rpm['sha256'])
        and name == f"{rpm['name']}-{rpm['version']}-{rpm['release']}.{rpm['arch']}.rpm"
        for name, rpm in rpms.items()), 'Invalid RPM lock entry')
    foundation, builders = lock['foundation'], lock['builders']
    require(isinstance(builders, dict) and isinstance(foundation, dict)
            and all(isinstance(value, dict) for value in builders.values())
            and foundation.get('requests') == sorted(set(source['packages']))
            and foundation.get('remove') == sorted(set(source['remove_packages']))
            and {role: value.get('requests') for role, value in builders.items()} == frontend.get('builders'),
            'RPM lock requests differ from the committed package selection')
    lock_role(lock, foundation, spec['architecture'])
    for role in builders.values():
        lock_role(lock, role, spec['architecture'])
    used = set(foundation['install']).union(*(role['install'] for role in builders.values()))
    require(used == set(rpms), 'RPM lock lists files no role installs')
    return lock


def resolved_inputs(root, source, base, context, binaries, rpm_lock_data, target, catalog=None):
    """Record deterministic build inputs for a caller's frozen committed source.

    Production establishes accepted-main authority before calling this function;
    branch fixtures use their own frozen source. This function grants neither
    publication nor installed trust and performs no package or registry operation.
    Every RPM comes from the run's lock: its foundation inventory is the image's
    package material and its pins hash is the `rpm-lock` artifact. With `catalog`,
    the run's pin envelope is derived here once and written to `catalog['pins']`.
    The bootc compatibility of the lock is checked here, once per run.
    """
    spec = target_spec(target)
    require(source.get('schema_version') == 1 and source.get('target', {}).get('id') == target
            and source['target'].get('architecture') == spec['architecture']
            and source['target'].get('image') == spec['repository']
            and source['target'].get('fedora_release') == 44
            and source['target'].get('candidate_target') is True
            and re.fullmatch('[a-f0-9]{40}', source.get('source_revision', '')),
            'Source plan differs from the closed release target')
    require(re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', base),
            'Unreviewed Fedora base')
    lock = rpm_lock(rpm_lock_data, source, base, target)
    # Everything that decides image content. The release workflows only orchestrate
    # these reviewed scripts (an edit to them alone cannot change an image), so
    # they are not recipes and a workflow-only change rebuilds and re-signs nothing.
    recipes = [IMAGE + name for name in (
        'Containerfile', 'assemble.sh', 'catalog/install-rpms.sh', 'catalog/build-packages.sh',
        'release/Containerfile', 'release/build.sh', 'release/compose.py', 'release/prepare-trust.py',
        'release/trust.py', 'release/refresh.py', 'release/material.py', 'release/targets.json',
        'release/compatibility.json')]
    recipes.extend(('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', spec['public_key'], spec['key_sha256']))
    artifacts = {name: context / name for name in ('sysroot', 'sysroot-helper')}
    if catalog is not None:
        require(isinstance(catalog, dict) and set(catalog) == {'pins'}, 'Unsupported catalog material inputs')
        pins = document(catalog_pins(root, binaries, target, catalog['pins']))
        require(isinstance(pins, dict), 'Invalid catalog material')
        recipes.extend((*language_material(root, source, pins, target), IMAGE + 'catalog/prepare-sources.py'))
        artifacts['catalog-pins'] = catalog['pins']
        artifacts['catalog-author'] = binaries / 'sysroot'
    recipe_hashes = {}
    for name in recipes:
        recipe_hashes[name] = sha(read(root / name))
    artifact_hashes = {}
    for name, path in artifacts.items():
        require(path.is_file() and not path.is_symlink() and path.stat().st_nlink == 1,
                'Build artifact must be a single-link regular file: ' + name)
        with path.open('rb') as stream:
            artifact_hashes[name] = hashlib.file_digest(stream, 'sha256').hexdigest()
    artifact_hashes['rpm-lock'] = lock_pins_sha256(lock)
    if catalog is not None:
        require(set(lock['builders']) == set(pins['builders']), 'Compiler roles differ from the RPM lock')
        builder_rows = {role: value['packages'] for role, value in lock['builders'].items()}
        artifact_hashes['catalog-builder-rpms'] = sha(canonical(builder_rows))
        require(artifact_hashes['catalog-author'] == artifact_hashes['sysroot'], 'Catalog author differs from sysroot artifact')
    rows = lock['foundation']['packages']
    require_bootc(rows, spec['architecture'])
    return {'schema_version': 1, 'base': base,
            'source': {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')},
            'recipes': recipe_hashes, 'artifacts': artifact_hashes, 'packages': rows}


def declared_steps(inputs):
    """The image steps the committed source declares for its target (`sysroot derive`)."""
    steps = inputs['source'].get('native_steps', [])
    require(isinstance(steps, list) and all(isinstance(step, dict) and isinstance(step.get('kind'), str)
                                            for step in steps), 'Invalid declared native steps')
    return steps


def verify_native(config, receipt_bytes, inputs, target, selected=None):
    """Validate signed native-receipt bindings; actual artifact execution is separate."""
    receipt = document(receipt_bytes)
    fields = {'schema', 'identity', 'parent_identity', 'implementation', 'driver_sha256',
              'recipe_sha256', 'foundation_image', 'rpm_sha256', 'artifacts', 'kernels', 'tools'}
    require(isinstance(receipt, dict) and set(receipt) == fields
            and type(receipt['schema']) is int and receipt['schema'] == 1,
            'Unsupported native material receipt')
    for field in ('identity', 'parent_identity', 'driver_sha256', 'recipe_sha256', 'rpm_sha256'):
        require(isinstance(receipt[field], str) and re.fullmatch('[a-f0-9]{64}', receipt[field]),
                'Malformed native receipt identity')
    steps = declared_steps(inputs)
    initramfs = any(step['kind'] == 'initramfs' for step in steps)
    require(isinstance(receipt['foundation_image'], str)
            and re.fullmatch('sha256:[a-f0-9]{64}', receipt['foundation_image'])
            and isinstance(receipt['artifacts'], list) and receipt['artifacts']
            and isinstance(receipt['kernels'], list) and bool(receipt['kernels']) == initramfs
            and isinstance(receipt['tools'], dict) and receipt['tools'], 'Incomplete native material')
    labels = config.get('config', {}).get('Labels', {})
    require(config.get('os') == 'linux' and config.get('architecture') == target_spec(target)['oci_architecture']
            and steps
            and labels.get('dev.kedra.native.identity') == receipt['identity']
            and labels.get('dev.kedra.composition.identity') == receipt['parent_identity'],
            'Native receipt differs from image composition labels')
    rpm_bytes = ('\n'.join('\t'.join(row) for row in inputs['packages']) + '\n').encode()
    require(sha(rpm_bytes) == receipt['rpm_sha256'], 'Native receipt differs from release RPM material')
    if selected is not None:
        require(selected.get('schema_version') == 1 and selected.get('target') == target
                and selected.get('input_material_sha256') == sha(canonical(inputs))
                and selected.get('foundation_image') == receipt['foundation_image']
                and selected.get('composition_identity') == receipt['parent_identity']
                and selected.get('native_identity') == receipt['identity']
                and selected.get('rpm_sha256') == receipt['rpm_sha256']
                and selected.get('native_receipt_sha256') == sha(receipt_bytes),
                'Candidate native material differs from independently retained build')
    return receipt


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


def version_triplet(value):
    parts = value.split('.')
    if len(parts) == 3 and all(0 < len(part) <= 10 and re.fullmatch(r'0|[1-9][0-9]*', part)
                               and int(part) <= 2**32 - 1 for part in parts):
        return tuple(int(part) for part in parts)
    return None


def bootc_compatibility(rows, architecture):
    contract = document(read(Path(__file__).with_name('compatibility.json'), 4096))
    require(isinstance(contract, dict) and set(contract) == {'schema_version', 'minimum_bootc_version'}
            and type(contract['schema_version']) is int and contract['schema_version'] == 2
            and isinstance(contract['minimum_bootc_version'], str), 'Invalid bootc compatibility contract')
    minimum = version_triplet(contract['minimum_bootc_version'])
    require(minimum is not None, 'Invalid bootc version triplet')
    selected = [row for row in rows if row[0] == 'bootc']
    installed = version_triplet(selected[0][2]) if len(selected) == 1 else None
    compatible = (installed is not None and installed >= minimum and selected[0][1] == '0'
                  and selected[0][4] == architecture)
    return {'compatible': compatible, 'minimum_bootc_version': contract['minimum_bootc_version'],
            'installed_bootc_packages': selected}


def require_bootc(rows, architecture):
    result = bootc_compatibility(rows, architecture)
    require(result['compatible'], 'Unqualified bootc package; expected version ' + result['minimum_bootc_version']
            + ' or newer, observed ' + repr(result['installed_bootc_packages']) + '. The installed helper refuses older bootc.')
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
