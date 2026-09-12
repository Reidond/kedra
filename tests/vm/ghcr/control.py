"""Loopback-only controller for an isolated test registry; no production destinations."""
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import urllib.request

root = Path(os.environ['RUNNER_TEMP']).resolve() / 'kedra-ghcr'
assert os.environ.get('GITHUB_ACTIONS') == 'true'
cases = json.loads((root / 'cases/cases.json').read_bytes())
context = ssl.create_default_context(cafile=str(root / 'context/tls.crt'))


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        try:
            operation = self.path.removeprefix('/')
            if operation in ('offline', 'online'):
                subprocess.run(['sudo', 'podman', 'stop' if operation == 'offline' else 'start', 'kedra-ghcr-registry'],
                               check=True, timeout=30, stdout=subprocess.DEVNULL)
            else:
                assert operation in cases['digests']
                digest = cases['digests'][operation]
                url = 'https://127.0.0.1/v2/reidond/kedra-desktop/manifests/'
                request = urllib.request.Request(url + digest, headers={'Accept': 'application/vnd.oci.image.manifest.v1+json'})
                with urllib.request.urlopen(request, context=context, timeout=20) as response:
                    payload = response.read(1048577)
                assert len(payload) <= 1048576
                manifest = json.loads(payload)
                request = urllib.request.Request(url + 'stable', payload, method='PUT',
                                                headers={'Content-Type': manifest['mediaType']})
                with urllib.request.urlopen(request, context=context, timeout=20) as response:
                    assert response.status == 201
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'ok\n')
        except Exception as error:
            self.send_error(500, str(error))


http.server.HTTPServer(('127.0.0.1', 18080), Handler).serve_forever()
