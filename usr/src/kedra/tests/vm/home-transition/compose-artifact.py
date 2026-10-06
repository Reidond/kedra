#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Produce the signed B fixture's receipt through ordinary native ARM composition.

Actions-only, public generated inputs. The output is test evidence, not a release.
The producer and store are removed before the signed-image consumer runs.
"""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import tarfile
import tempfile

from niri_fixture import incoming

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--foundation', required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
if (os.environ.get('GITHUB_ACTIONS') != 'true'
        or os.environ.get('GITHUB_REPOSITORY') != 'Reidond/kedra'
        or os.uname().machine != 'aarch64'):
    raise SystemExit('Use only a disposable native ARM Kedra Actions runner')
repo = pathlib.Path.cwd()
tool = repo / 'target/release/sysroot'
args.output.mkdir(mode=0o700)
environment = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
environment.update({'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null'})


def run(*command, cwd=None):
    return subprocess.check_output(command, cwd=cwd, env=environment,
                                   stdin=subprocess.DEVNULL, timeout=600)


run('docker', 'pull', '--platform', 'linux/arm64', args.foundation)
foundation = json.loads(run('docker', 'image', 'inspect', args.foundation))[0]
if foundation['Architecture'] != 'arm64' or foundation['Os'] != 'linux':
    raise RuntimeError('foundation platform differs')
with tempfile.TemporaryDirectory(prefix='kedra-home-artifact-') as temporary:
    work = pathlib.Path(temporary)
    source = work / 'source'
    source.mkdir()
    image = source / 'usr/src/kedra/image'
    target = image / 'targets/qemu-arm64'
    target.mkdir(parents=True)
    (image / 'packages.list').write_text('bash\n')
    (image / 'remove.list').write_text('# none\n')
    (target / 'packages.list').write_text('# none\n')
    (target / 'target.toml').write_text("id='qemu-arm64'\narchitecture='aarch64'\n"
                                     "image='ghcr.io/reidond/kedra-qemu-arm64'\nfedora_release=44\n"
                                     "candidate_target=true\nhardware_status='synthetic'\n")
    baseline = source / 'etc/skel/.config/niri/config.kdl'
    baseline.parent.mkdir(parents=True)
    baseline.write_text(incoming(run('git', 'show', os.environ['GITHUB_SHA'] +
                                     ':etc/skel/.config/niri/config.kdl').decode()))
    run('git', 'init', '-q', '--template=', '-b', 'fixture', cwd=source)
    run('git', 'add', '.', cwd=source)
    run('git', '-c', 'user.name=Kedra fixture', '-c', 'user.email=fixture@example.invalid',
        '-c', 'commit.gpgsign=false', 'commit', '-qm', 'public signed B baseline', cwd=source)
    store = work / 'store'
    run(str(tool), 'store', 'init', '--store', str(store))
    run(str(tool), 'store', 'add-image', '--store', str(store), '--image', foundation['Id'])
    common = ('--repo', str(source), '--target', 'qemu-arm64', '--store', str(store),
              '--foundation', foundation['Id'])
    planned = json.loads(run(str(tool), 'system', 'plan', *common))
    context = work / 'context'
    composed = json.loads(run(str(tool), 'system', 'compose', *common, '--output-dir', str(context)))
    if composed['identity'] != planned['identity'] or composed['plan']['objects']:
        raise RuntimeError('planned/admitted identity or runtime outputs differ')
    with tarfile.open(context / 'payload.tar') as archive:
        def read(name):
            member = archive.getmember(name)
            if not member.isfile() or member.mode != 0o644 or member.uid != 0:
                raise RuntimeError('artifact payload type/mode/owner differs')
            return archive.extractfile(member).read()
        record = read('usr/share/sysroot/home-artifacts.json')
        installed = read('usr/share/sysroot/home/default/.config/niri/config.kdl')
    if installed != baseline.read_bytes():
        raise RuntimeError('composed baseline differs from committed public input')
    receipt = json.loads(record)['niri']
    verified = json.loads(run(str(tool), 'store', 'verify', '--store', str(store), '--object', receipt['object']))
    if verified != receipt:
        raise RuntimeError('admitted source receipt differs')
    (args.output / 'home-artifacts.json').write_bytes(record)
    (args.output / 'config.kdl').write_bytes(installed)
    (args.output / 'producer.json').write_text(json.dumps({
        'schema': 1, 'source': run('git', 'rev-parse', 'HEAD', cwd=source).decode().strip(),
        'product_source': os.environ['GITHUB_SHA'], 'foundation': foundation['Id'],
        'composition_identity': composed['identity'], 'receipt': receipt,
    }, sort_keys=True, indent=2) + '\n')
    # The successful context outlives actual producer removal and explicit GC.
    run(str(tool), 'store', 'unpin', '--store', str(store), '--object', receipt['object'])
    run(str(tool), 'store', 'gc', '--store', str(store), '--delete')
    shutil.rmtree(source)
    verification = work / 'verification'
    verification.mkdir()
    run(str(tool), 'system', 'verify', '--context', str(context), '--expected-identity',
        composed['identity'], '--workdir', str(verification))
    # Sealed engine trees are owner-controlled; restore their directories for scoped cleanup.
    for directory, _, _ in os.walk(store):
        pathlib.Path(directory).chmod(0o700)
print('PASS: native composition produced the public niri artifact; producer removed before transfer')
