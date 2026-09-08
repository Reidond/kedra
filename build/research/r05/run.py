"""Native launcher probes under a generated user home and no network namespace."""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--sysroot', type=pathlib.Path, required=True)
parser.add_argument('--packages', type=pathlib.Path, required=True)
parser.add_argument('--work', type=pathlib.Path, required=True)
parser.add_argument('--evidence', type=pathlib.Path, required=True)
args = parser.parse_args()
if os.geteuid() == 0:
    raise SystemExit('Probe must run as an ordinary user')
args.work.mkdir(mode=0o700)
home = args.work / 'home'
home.mkdir(mode=0o700)
personal = home / '.codex'
personal.mkdir(mode=0o700)
(personal / 'config.toml').write_text('# synthetic personal preferences; preserve exactly\n')
def snapshot(directory):
    return {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in directory.rglob('*') if p.is_file() and not p.is_symlink()}
repo = args.work / 'checkout'
repo.mkdir()
repo.joinpath('hosts/desktop').mkdir(parents=True)
shutil.copyfile('hosts/desktop/host.toml', repo / 'hosts/desktop/host.toml')
environment = {'HOME': str(home), 'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'TERM': 'xterm-256color'}
for command in [['init', '-q'], ['config', 'user.name', 'Fixture'],
                ['config', 'user.email', 'fixture@example.invalid'],
                ['config', 'commit.gpgsign', 'false'], ['config', 'core.hooksPath', '/dev/null'],
                ['remote', 'add', 'origin', 'https://github.com/Reidond/kedra.git'],
                ['add', '.'], ['commit', '-qm', 'fixture']]:
    subprocess.run(['git', '-C', str(repo), *command], env=environment, check=True)
repo.joinpath('untracked').write_text('must remain untouched\n')
before = subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain=v1'], env=environment)
results = []
for runtime in ['bundled', 'user']:
    for scope in ['management', 'personal']:
        personal_before = snapshot(personal)
        command = [str(args.sysroot), 'codex', '--repo', str(repo), '--runtime', runtime, '--config-scope', scope]
        if runtime == 'user':
            command.extend(['--executable', str(args.packages / 'personal-codex/bin/codex')])
        plan = json.loads(subprocess.check_output([*command, '--print-plan'], env=environment))
        expected_version = '0.153.4' if runtime == 'bundled' else '0.153.3'
        if plan['version'] != expected_version or plan['deployment_authorized']:
            raise RuntimeError('Wrong native runtime selected')
        for upstream in [['--version'], ['--help'], ['login', '--help']]:
            output = subprocess.run([*command, '--', *upstream], env=environment, capture_output=True, timeout=30)
            if output.returncode != 0:
                raise RuntimeError(f'Native invocation failed: {runtime}/{scope}/{upstream}')
        results.append({'runtime': runtime, 'scope': scope, 'version': plan['version'],
                        'version_help_login_help': 'pass', 'deployment_authorized': False})
        if scope == 'management' and personal_before != snapshot(personal):
            raise RuntimeError('Management launch changed personal profile files')
after = subprocess.check_output(['git', '-C', str(repo), 'status', '--porcelain=v1'], env=environment)
if before != after or repo.joinpath('untracked').read_text() != 'must remain untouched\n':
    raise RuntimeError('Checkout changed during read-only native probes')
if (personal / 'config.toml').read_text() != '# synthetic personal preferences; preserve exactly\n':
    raise RuntimeError('Personal configuration changed')
args.evidence.mkdir(parents=True, exist_ok=True)
(args.evidence / 'native-results.json').write_text(json.dumps(results, indent=2) + '\n')
print('PASS: native Codex 0.153.4/0.153.3 runtime and config selection; unchanged checkout/personal profile')
