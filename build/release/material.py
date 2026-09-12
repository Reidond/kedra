"""Authenticate published resolved build inputs using signed checksum assets."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

LIMIT = 4 * 1024**2


def require(ok, message):
    if not ok:
        raise SystemExit(message)


def read(path, limit=LIMIT):
    require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= limit,
            'Missing, nonregular or oversized release input: ' + path.name)
    return path.read_bytes()


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'Duplicate JSON key')
        result[key] = value
    return result


def document(data):
    return json.loads(data, object_pairs_hook=unique)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def packages(data):
    rows = []
    names = set()
    for line in data.decode('utf-8').splitlines():
        row = line.split('\t')
        require(len(row) == 7, 'Unsupported RPM material row')
        if row[0] == 'gpg-pubkey':
            continue
        require(row[1].isdigit() and row[4] in ('x86_64', 'noarch', 'i686')
                and all(re.fullmatch('[a-f0-9]{64}', item) for item in row[5:]),
                'RPM material lacks exact header/payload identity')
        identity = tuple(row[:5])
        require(identity not in names, 'Duplicate installed RPM identity')
        names.add(identity)
        rows.append(row)
    require(rows, 'Empty RPM material')
    return sorted(rows)


def verify(directory, public_key, release_hash):
    checksums = read(directory / 'SHA256SUMS', 65536)
    signature = base64.b64decode(read(directory / 'SHA256SUMS.sig', 1024).strip(), validate=True)
    with tempfile.TemporaryDirectory(prefix='kedra-checksums-') as temporary:
        der = Path(temporary) / 'signature.der'
        der.write_bytes(signature)
        subprocess.run(['openssl', 'dgst', '-sha256', '-verify', str(public_key),
                        '-signature', str(der), str(directory / 'SHA256SUMS')],
                       check=True, stdout=subprocess.DEVNULL, timeout=30)
    inventory = {}
    for line in checksums.decode('ascii').splitlines():
        match = re.fullmatch(r'([a-f0-9]{64})  ([A-Za-z0-9][A-Za-z0-9._-]{0,127})', line)
        require(match is not None and match[2] not in inventory, 'Invalid signed checksum inventory')
        inventory[match[2]] = match[1]
    values = {}
    for name in ('release.json', 'provenance.json', 'packages.txt'):
        data = read(directory / name)
        require(inventory.get(name) == sha(data), 'Signed asset hash mismatch: ' + name)
        values[name] = data
    require(sha(values['release.json']) == release_hash, 'Checksum assets belong to another release')
    release = document(values['release.json'])
    provenance = document(values['provenance.json'])
    require(provenance.get('schema_version') == 2 and provenance.get('format') == 'kedra-candidate-provenance',
            'Legacy release has no complete resolved-input identity')
    require(all(provenance[key] == release[key] for key in
                ('project', 'scope', 'source_revision', 'build', 'image_digest', 'home_manifest_sha256', 'installer'))
            and provenance['packages_sha256'] == sha(values['packages.txt']), 'Resolved provenance is not bound to release')
    material = provenance['resolved_inputs']
    require(material.get('schema_version') == 1
            and set(material) == {'schema_version', 'base', 'source', 'artifacts', 'recipes', 'packages'}
            and re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', material['base'])
            and isinstance(material['source'], dict) and isinstance(material['artifacts'], dict)
            and isinstance(material['recipes'], dict) and material['artifacts'] and material['recipes'],
            'Incomplete resolved-input identity')
    for section in ('artifacts', 'recipes'):
        require(all(isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value)
                    for value in material[section].values()), 'Invalid material content hash')
    require(packages(('\n'.join('\t'.join(row) for row in material['packages']) + '\n').encode())
            == material['packages'], 'Noncanonical package material')
    display = sorted(f'{row[0]}-{row[2]}-{row[3]}.{row[4]}' for row in material['packages'])
    displayed = sorted(line for line in values['packages.txt'].decode().splitlines()
                       if not line.startswith('gpg-pubkey-'))
    require(display == displayed, 'Signed package list differs from resolved package material')
    return material


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release-dir', type=Path, required=True)
    parser.add_argument('--public-key', type=Path, required=True)
    parser.add_argument('--release-sha256', required=True)
    parser.add_argument('--current-inputs', type=Path)
    args = parser.parse_args()
    prior = verify(args.release_dir, args.public_key, args.release_sha256)
    if args.current_inputs is None:
        print(sha(canonical(prior)))
    else:
        current = document(read(args.current_inputs))
        print(json.dumps({'decision': 'no-change' if prior == current else 'candidate',
                          'authenticated_previous_inputs_sha256': sha(canonical(prior))}))
