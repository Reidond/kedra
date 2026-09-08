"""Add the missing v82 generic ISO SELinux labeling stage, fail on layout drift."""
import json
import pathlib
import sys

source, destination = map(pathlib.Path, sys.argv[1:])
manifest = json.loads(source.read_text())
if manifest.get('version') != '2':
    raise SystemExit('Expected osbuild manifest version 2')
trees = [p for p in manifest['pipelines'] if p['name'] == 'os-tree']
if len(trees) != 1:
    raise SystemExit('Expected one generic installer os-tree')
tree = trees[0]
if tree['stages'][0]['type'] != 'org.osbuild.container-deploy':
    raise SystemExit('Unexpected generic installer deployment stage')
if any(s['type'] == 'org.osbuild.selinux' for s in tree['stages']):
    raise SystemExit('Builder now labels the installer; review and remove this workaround')
tree['stages'].append({
    'type': 'org.osbuild.selinux',
    'options': {'file_contexts': 'etc/selinux/targeted/contexts/files/file_contexts'},
})
destination.write_text(json.dumps(manifest, indent=2) + '\n')
