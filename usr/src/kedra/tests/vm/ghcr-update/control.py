#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Loopback-only controller for an isolated test registry; no production destinations."""
import argparse
import hashlib
import http.server
import json
import re
import ssl
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

from fixture import add_context_argument, load_context

parser = argparse.ArgumentParser(description=__doc__)
add_context_argument(parser)
parser.add_argument('--root', type=Path, help='The fixture root that run.sh or run-hvf.py created')
parser.add_argument('--engine', choices=('podman', 'docker'), default='podman',
                    help='Engine of the kedra-ghcr-registry container: rootful Podman on Linux, Docker on macOS')
args = parser.parse_args()
ENGINE = ['sudo', 'podman'] if args.engine == 'podman' else ['docker']
if args.fixture_context is not None:
    fixture = load_context(args.fixture_context)
    root = Path(fixture['runner_temp']) / 'kedra-ghcr'
else:
    # run.sh and run-hvf.py fixtures have no ARM release context; their drivers admitted the host.
    assert args.root is not None and args.root.name == 'kedra-ghcr' and args.root.is_dir()
    root = args.root.resolve()
cases = json.loads((root / 'cases/cases.json').read_bytes())
context = ssl.create_default_context(cafile=str(root / 'context/tls.crt'))
TARGET = cases.get('target', 'desktop')
assert TARGET in ('desktop', 'qemu-arm64')
REPOSITORY = 'reidond/kedra-' + TARGET
MANIFESTS = 'https://127.0.0.1/v2/' + REPOSITORY + '/manifests/'
# Registry combined access-log line of one completed blob download.
BLOB_GET = re.compile(r'"GET /v2/' + re.escape(REPOSITORY) + r'/blobs/(sha256:[0-9a-f]{64}) HTTP/[0-9.]+" 2[0-9][0-9] ')


def manifest(digest):
    request = urllib.request.Request(MANIFESTS + digest, headers={'Accept': 'application/vnd.oci.image.manifest.v1+json'})
    with urllib.request.urlopen(request, context=context, timeout=20) as response:
        payload = response.read(1048577)
    assert len(payload) <= 1048576
    return payload, json.loads(payload)


def layer_gets(variant):
    """Completed registry downloads of each layer of one signed test image so far."""
    counts = dict.fromkeys((layer['digest'] for layer in manifest(cases['digests'][variant])[1]['layers']), 0)
    assert counts
    log = subprocess.run([*ENGINE, 'logs', 'kedra-ghcr-registry'], check=True, timeout=120,
                         capture_output=True, text=True, errors='replace')
    for line in (log.stdout + log.stderr).splitlines():
        match = BLOB_GET.search(line)
        if match and match.group(1) in counts:
            counts[match.group(1)] += 1
    return counts


def unsign(variant):
    """Withdraw one test image's sigstore attachment; the signed image bytes stay."""
    tag = 'sha256-' + cases['digests'][variant].removeprefix('sha256:') + '.sig'
    payload, _ = manifest(tag)
    request = urllib.request.Request(MANIFESTS + 'sha256:' + hashlib.sha256(payload).hexdigest(), method='DELETE')
    with urllib.request.urlopen(request, context=context, timeout=20) as response:
        assert response.status == 202
    try:
        manifest(tag)
    except urllib.error.HTTPError as error:
        assert error.code == 404, error.code
    else:
        raise AssertionError('Signature attachment is still served after deletion')


def registry_power(online):
    """Podman stops and starts the registry. Docker pauses it, so an unreachable
    registry needs no second read of its TLS key and the guest's request stalls."""
    if args.engine == 'podman':
        subprocess.run([*ENGINE, 'start' if online else 'stop', 'kedra-ghcr-registry'],
                       check=True, timeout=30, stdout=subprocess.DEVNULL)
        return
    paused = subprocess.run(['docker', 'inspect', '--format', '{{.State.Paused}}', 'kedra-ghcr-registry'],
                            check=True, timeout=30, capture_output=True, text=True).stdout.strip() == 'true'
    if paused == online:
        subprocess.run(['docker', 'unpause' if online else 'pause', 'kedra-ghcr-registry'],
                       check=True, timeout=30, stdout=subprocess.DEVNULL)


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        try:
            operation = self.path.removeprefix('/')
            body = b'ok\n'
            if operation.startswith('layer-gets/'):
                body = json.dumps(layer_gets(operation.removeprefix('layer-gets/'))).encode()
            elif operation.startswith('unsign/'):
                variant = operation.removeprefix('unsign/')
                assert variant in cases['digests']
                unsign(variant)
            elif operation in ('offline', 'online'):
                registry_power(operation == 'online')
                if operation == 'online':
                    # start acknowledges the container before its HTTPS listener is ready.
                    # Poll only this read-only loopback endpoint; never repeat start or a PUT.
                    deadline = time.monotonic() + 10
                    while True:
                        remaining = deadline - time.monotonic()
                        if remaining <= 0:
                            raise TimeoutError('Local registry did not become ready within 10 seconds')
                        try:
                            with urllib.request.urlopen('https://127.0.0.1/v2/', context=context,
                                                        timeout=min(2, remaining)) as response:
                                assert response.status == 200
                            break
                        except (urllib.error.URLError, TimeoutError):
                            time.sleep(min(0.1, max(0, deadline - time.monotonic())))
            else:
                assert operation in cases['digests']
                payload, value = manifest(cases['digests'][operation])
                request = urllib.request.Request(MANIFESTS + 'stable', payload, method='PUT',
                                                headers={'Content-Type': value['mediaType']})
                with urllib.request.urlopen(request, context=context, timeout=20) as response:
                    assert response.status == 201
            self.send_response(200)
            self.end_headers()
            self.wfile.write(body)
        except Exception as error:  # noqa: BLE001 - the fixture reports any failure as HTTP 500
            self.send_error(500, str(error))


http.server.HTTPServer(('127.0.0.1', 18080), Handler).serve_forever()
