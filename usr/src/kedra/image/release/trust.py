"""Strict container signature policy and sigstore attachment discovery for one target.

The single generator of `policy.json` and the `registries.d` entry: the installed
public trust layer (prepare-trust.py), host-side signature-verified pulls in the
release preflight and publication (refresh.py) and the local installer
(build-local.py). Only the isolated signer, which runs no repository script with
production keys, writes its attachment configuration inline.
"""
import json
from pathlib import Path

# Installed location of the target's release public key.
INSTALLED_KEY = '/usr/lib/sysroot/trust/release.pub'
# Signatures are sigstore attachments next to the image in GHCR.
REGISTRIES = 'docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n'


def policy(repository, key_path):
    """Reject everything except images signed by the key for exactly this repository."""
    requirement = {'type': 'sigstoreSigned', 'keyPath': str(key_path),
                   'signedIdentity': {'type': 'exactRepository', 'dockerRepository': repository}}
    return {'default': [{'type': 'reject'}], 'transports': {
        'docker': {repository: [requirement]}, 'containers-storage': {'': [requirement]}}}


def host_configuration(directory, repository, key_path):
    """Write a host-side policy and registries.d for `skopeo --policy/--registries.d`.

    Returns both paths; nothing under /etc changes.
    """
    directory = Path(directory)
    registries = directory / 'registries.d'
    registries.mkdir(parents=True, exist_ok=True)
    (registries / 'kedra.yaml').write_text(REGISTRIES)
    path = directory / 'host-policy.json'
    path.write_text(json.dumps(policy(repository, key_path), sort_keys=True, separators=(',', ':')))
    return path, registries
