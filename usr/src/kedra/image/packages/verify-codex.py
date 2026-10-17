#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Verify an official Codex package's Sigstore signatures before pinning it.

The pin-update step for codex.kedra (docs/PACKAGES.md): download the official
`codex-package-<arch>-unknown-linux-musl.tar.gz` of one release, verify the
release workflow's Sigstore bundles for its codex, codex-code-mode-host and
bwrap binaries with the pinned Cosign, and print the archive SHA-256 to pin
with pin.py. Builds then trust that SHA-256 alone. Runs on a Linux runner,
because the pinned Cosign builds are Linux ones.
"""
import argparse
import hashlib
import pathlib
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request

# Cosign executes here, so its build follows this machine, never the target.
COSIGN_VERSION = '3.1.3'
COSIGN = {
    'x86_64': ('cosign-linux-amd64', 141178250, '4629c757b7618056f8ddd7e2625ae9fdd94c0372a65049520bc7d9df9efc7f71'),
    'aarch64': ('cosign-linux-arm64', 132747403, 'c5d324e091826b0d7a78eb16fef316450b4eb9aaec045611c08ba06f5e73220a'),
}
# The signed binaries and their place in the package.
SIGNED = {'codex': 'bin/codex', 'codex-code-mode-host': 'bin/codex-code-mode-host',
          'bwrap': 'codex-resources/bwrap'}
ISSUER = 'https://token.actions.githubusercontent.com'


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def fetch(url, destination):
    with urllib.request.urlopen(url, timeout=120) as response, destination.open('xb') as output:
        shutil.copyfileobj(response, output, 1024 * 1024)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def cosign(work):
    machine = platform.machine()
    require(platform.system() == 'Linux' and machine in COSIGN, f'No pinned Cosign for {platform.system()} {machine}')
    asset, size, sha256 = COSIGN[machine]
    binary = work / 'cosign'
    fetch(f'https://github.com/sigstore/cosign/releases/download/v{COSIGN_VERSION}/{asset}', binary)
    require(binary.stat().st_size == size and digest(binary) == sha256, 'Cosign differs from its pin')
    binary.chmod(0o755)
    return binary


def signed_binaries(archive, work):
    """Extract only the signed binaries, as regular files, without following links."""
    extracted = {}
    with tarfile.open(archive, mode='r:gz') as package:
        for name, relative in SIGNED.items():
            member = package.getmember(relative)
            require(member.isfile(), f'{relative} is not a regular file in the package')
            path = work / name
            with package.extractfile(member) as stream, path.open('xb') as output:
                shutil.copyfileobj(stream, output, 1024 * 1024)
            extracted[name] = path
    return extracted


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--version', required=True, help='Codex release, for example 0.153.4')
    parser.add_argument('--architecture', choices=['aarch64', 'x86_64'], required=True)
    parser.add_argument('--archive', type=pathlib.Path, help='already downloaded package archive')
    args = parser.parse_args()
    require(re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', args.version), 'Expected a plain release version')
    triple = f'{args.architecture}-unknown-linux-musl'
    base = f'https://github.com/openai/codex/releases/download/rust-v{args.version}'
    identity = f'https://github.com/openai/codex/.github/workflows/rust-release.yml@refs/tags/rust-v{args.version}'
    with tempfile.TemporaryDirectory(prefix='kedra-codex-verify-') as temporary:
        work = pathlib.Path(temporary)
        archive = work / 'package.tar.gz'
        url = f'{base}/codex-package-{triple}.tar.gz'
        if args.archive is None:
            fetch(url, archive)
        else:
            shutil.copyfile(args.archive, archive)
        verifier = cosign(work)
        binaries = signed_binaries(archive, work)
        for name, binary in binaries.items():
            bundle = work / f'{name}.sigstore'
            fetch(f'{base}/{name}-{triple}.sigstore', bundle)
            result = subprocess.run([str(verifier), 'verify-blob', '--bundle', str(bundle),
                                     '--certificate-identity', identity, '--certificate-oidc-issuer', ISSUER,
                                     str(binary)], capture_output=True, text=True, check=False)
            require(result.returncode == 0, f'Sigstore verification failed for {name}: {result.stderr.strip()}')
            print(f'verified {name}: {identity}')
        print(f'url {url}')
        print(f'sha256 {digest(archive)}')


if __name__ == '__main__':
    main()
