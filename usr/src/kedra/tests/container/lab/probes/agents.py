# Runs inside the lab container as the test account with the image's own python3; uv applies to host-side scripts only.
"""Run the image's private official runtime as the generated ordinary account."""
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile

if os.geteuid() == 0:
    raise SystemExit('Agent runtime probe must run as the ordinary fixture user')
home = pathlib.Path.home()
personal = home / '.codex'
personal.mkdir(mode=0o700, exist_ok=True)
marker = personal / 'kedra-research-marker'
with marker.open('x') as output:
    output.write('synthetic personal profile marker\n')

def snapshot():
    return {str(path.relative_to(personal)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in personal.rglob('*') if path.is_file() and not path.is_symlink()}

before = snapshot()
target = json.loads(pathlib.Path('/usr/share/sysroot/source.json').read_text())['target']
# Codex is a catalog package: its private launcher, version and output come from
# the image's catalog inventory.
catalog = json.loads(pathlib.Path('/usr/share/kedra/catalog.json').read_text())
launcher = catalog['libexec']['codex']['launcher']
with tempfile.TemporaryDirectory(prefix='kedra-agent-checkout-', dir=home) as temporary:
    repo = pathlib.Path(temporary)
    # A Kedra checkout holds its system declarations; the agent edits them.
    definition = repo / 'system/catalog.kedra'
    definition.parent.mkdir(parents=True)
    definition.write_text(f'language 1;\nnamespace "kedra";\n// fixture for {target["id"]}\n')
    environment = {**os.environ, 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null'}
    for args in [['init', '-q', '--template='], ['config', 'user.name', 'Fixture'],
                 ['config', 'user.email', 'fixture@example.invalid'], ['config', 'commit.gpgsign', 'false'],
                 ['config', 'core.hooksPath', '/dev/null'], ['remote', 'add', 'origin', 'https://github.com/Reidond/kedra.git'],
                 ['add', '.'], ['commit', '-qm', 'generated fixture']]:
        subprocess.run(['git', '-C', str(repo), *args], env=environment, check=True)
    (repo / 'untracked').write_text('retain the fixture checkout\n')
    command = ['sysroot', 'codex', '--repo', str(repo), '--host', target['id']]
    plan = json.loads(subprocess.check_output([*command, '--print-plan']))
    if (plan['version'] != catalog['versions']['codex'] or plan['executable'] != launcher
            or plan['deployment_authorized']):
        raise RuntimeError('Image runtime selection mismatch')
    for args in [['--version'], ['--help'], ['login', '--help']]:
        subprocess.run([*command, '--', *args], stdout=subprocess.DEVNULL, check=True, timeout=30)
    if (repo / 'untracked').read_text() != 'retain the fixture checkout\n':
        raise RuntimeError('Agent help/version changed the checkout')
if before != snapshot():
    raise RuntimeError('Management launch changed the personal profile')
root = pathlib.Path('/usr/lib/sysroot/store') / catalog['outputs']['codex']
for relative in ['codex-path/rg', 'codex-resources/bwrap', 'codex-resources/zsh/bin/zsh']:
    subprocess.run([str(root / relative), '--version'], check=True, timeout=20)
located = subprocess.run(['bash', '-lc', 'command -v codex'], capture_output=True, check=False)
if (located.returncode == 0 or 'codex' in catalog['commands'] or pathlib.Path('/usr/bin/codex').exists()
        or pathlib.Path('/usr/local/bin/codex').exists()):
    raise RuntimeError('Bundled runtime shadows the personal command namespace')
print('KEDRA_R05_IMAGE_RUNTIME_PASS: private runtime/helpers and personal-profile retention')
