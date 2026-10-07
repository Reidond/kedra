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
LABEL = 'org.kedra.image.identity'
IMAGE = 'usr/src/kedra/image/'
NATIVE_RECIPE = IMAGE + 'release/native-steps.json'
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


def language_material(root, source, pins, pins_data, binaries, target):
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
    # This is trusted public preflight, before signing. Rederive the entire pin
    # envelope through the selected frontend rather than trusting claimed hashes.
    process = subprocess.run([str(binaries / 'sysroot'), 'catalog', 'pins',
                              '--input-root', str(root / (IMAGE + 'packages')),
                              '--entry', 'catalog.kedra', '--lock', 'packages.lock.json',
                              '--target', target, '--target-policy', str(root / (IMAGE + 'package-policy.json'))],
                             stdin=subprocess.DEVNULL, capture_output=True, timeout=60, check=False)
    require(process.returncode == 0 and len(process.stdout) <= 8 * 1024**2
            and process.stdout == pins_data, 'Frontend pin envelope differs from independent preflight')
    return sorted(inputs)


def resolved_inputs(root, source, base, context, binaries, package_material, target, catalog=None):
    """Record deterministic build inputs for a caller's frozen committed source.

    Production establishes accepted-main authority before calling this function;
    branch fixtures use their own frozen source. This function grants neither
    publication nor installed trust and performs no package or registry operation.
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
    recipes = [IMAGE + name for name in (
        'Containerfile', 'assemble.sh', 'release/Containerfile', 'release/prepare-trust.py',
        'release/refresh.py', 'release/material.py', 'release/targets.json',
        'release/compatibility.json', 'agents/prepare.sh', 'agents/package.py',
        'agents/fetch.py', 'agents/pins.py', 'agents/inputs.json', 'bitwarden/prepare.py',
        'bitwarden/inputs.json')]
    recipes.extend(('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
                    '.github/workflows/release.yml', '.github/workflows/release-target.yml',
                    spec['public_key'], spec['key_sha256']))
    artifacts = {name: context / name for name in ('sysroot', 'sysroot-helper', 'agents.tar', 'bitwarden.tar')}
    if target == 'qemu-arm64':
        recipes.extend((IMAGE + 'release/compose.py', NATIVE_RECIPE))
        artifacts['kedra-lab'] = binaries / 'kedra-lab'
    if catalog is not None:
        require(target == 'qemu-arm64' and isinstance(catalog, dict)
                and set(catalog) == {'pins', 'builder_rpms'}, 'Unsupported catalog material inputs')
        pins_data = read(catalog['pins'])
        pins = document(pins_data)
        require(isinstance(pins, dict), 'Invalid catalog material')
        if pins.get('schema_version') == 2:
            expected_recipes = language_material(root, source, pins, pins_data, binaries, target)
            expected_recipes.append(IMAGE + 'catalog/prepare-sources.py')
        else:
            require('package_frontend' not in source, 'New-format source cannot use legacy catalog material')
            expected_recipes = legacy_catalog_material(root, pins)
        recipes.extend((*expected_recipes, IMAGE + 'catalog/Containerfile', IMAGE + 'catalog/builder.sh'))
        artifacts['catalog-pins'] = catalog['pins']
        artifacts['catalog-author'] = binaries / 'sysroot'
    recipe_hashes = {}
    for name in recipes:
        data = read(root / name)
        if name == NATIVE_RECIPE:
            data = canonical(document(data))
        recipe_hashes[name] = sha(data)
    artifact_hashes = {}
    for name, path in artifacts.items():
        require(path.is_file() and not path.is_symlink() and path.stat().st_nlink == 1,
                'Build artifact must be a single-link regular file: ' + name)
        with path.open('rb') as stream:
            artifact_hashes[name] = hashlib.file_digest(stream, 'sha256').hexdigest()
    if catalog is not None:
        if pins.get('schema_version') == 2:
            builder_rows = document(read(catalog['builder_rpms']))
            require(isinstance(builder_rows, dict) and set(builder_rows) == set(pins['builders']),
                    'Compiler role material differs from selected intent')
            for rows in builder_rows.values():
                require(isinstance(rows, list) and rows and packages(('\n'.join('\t'.join(row) for row in rows) + '\n').encode(), 'aarch64') == rows,
                        'Noncanonical compiler role RPM material')
        else:
            builder_rows = packages(read(catalog['builder_rpms']), 'aarch64')
        artifact_hashes['catalog-builder-rpms'] = sha(canonical(builder_rows))
        require(artifact_hashes['catalog-author'] == artifact_hashes['sysroot'], 'Catalog author differs from sysroot artifact')
    rows = packages(package_material, spec['architecture'])
    require_bootc(rows, spec['architecture'])
    return {'schema_version': 1, 'base': base,
            'source': {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')},
            'recipes': recipe_hashes, 'artifacts': artifact_hashes, 'packages': rows}


def legacy_catalog_material(root, pins):
    require(pins.get('schema_version') == 1 and pins.get('namespace') == 'kedra'
            and set(pins.get('packages', {})) == {'jq', 'sqlite'},
            'Unsupported legacy catalog pin inventory')
    templates = pins.get('templates')
    require(isinstance(templates, dict) and 0 < len(templates) <= 16
            and isinstance(pins.get('bindings'), dict) and pins['bindings'],
            'Missing catalog template or binding inventory')
    template_root = IMAGE + 'catalog/templates/'
    for name, destination in templates.items():
        require(isinstance(name, str) and name.startswith(template_root),
                'Catalog template is outside the development image tree')
        relative = name.removeprefix(template_root)
        require(relative and all(part not in ('', '.', '..') for part in relative.split('/'))
                and destination == '/' + relative, 'Catalog template destination differs from its path')
    expected_recipes = ('usr/src/kedra/crates/sysroot-catalog/lib.rs',
                        'usr/src/kedra/crates/sysroot-catalog/recipes.rs',
                        'usr/src/kedra/crates/sysroot/catalog.rs', *sorted(templates))
    if 'legacy_data' in pins:
        require(pins['legacy_data'] == 'usr/src/kedra/crates/sysroot-catalog/legacy.json',
                'Unknown legacy catalog data boundary')
        expected_recipes += (pins['legacy_data'],)
    require(set(pins.get('recipes', {})) == set(expected_recipes), 'Catalog recipe inventory differs')
    for name in expected_recipes:
        require(pins['recipes'][name] == sha(read(root / name)), 'Compiled catalog recipe differs: ' + name)
    return expected_recipes


def verify_native(config, receipt_bytes, inputs, selected=None):
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
    require(isinstance(receipt['foundation_image'], str)
            and re.fullmatch('sha256:[a-f0-9]{64}', receipt['foundation_image'])
            and isinstance(receipt['artifacts'], list) and receipt['artifacts']
            and isinstance(receipt['kernels'], list) and receipt['kernels']
            and isinstance(receipt['tools'], dict) and receipt['tools'], 'Incomplete native material')
    labels = config.get('config', {}).get('Labels', {})
    require(config.get('os') == 'linux' and config.get('architecture') == 'arm64'
            and NATIVE_RECIPE in inputs['recipes']
            and labels.get('dev.kedra.native.identity') == receipt['identity']
            and labels.get('dev.kedra.composition.identity') == receipt['parent_identity'],
            'Native receipt differs from image composition labels')
    rpm_bytes = ('\n'.join('\t'.join(row) for row in inputs['packages']) + '\n').encode()
    require(sha(rpm_bytes) == receipt['rpm_sha256'], 'Native receipt differs from release RPM material')
    if selected is not None:
        require(selected.get('schema_version') == 1 and selected.get('target') == 'qemu-arm64'
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
