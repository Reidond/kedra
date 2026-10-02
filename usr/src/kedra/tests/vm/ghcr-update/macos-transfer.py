#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Controller-only preparation/firmware checking for an explicit private HVF copy.

Preparation draft; not executed. Linux context validation is intentionally unchanged.
"""
import argparse
import fcntl
import hashlib
import json
import os
import re
import select
import shutil
import signal
import socket
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from fixture import add_context_argument, load_context

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / 'common'))
import secure_boot as firmware

CODE = Path('/usr/share/AAVMF/AAVMF_CODE.secboot.fd')
TEMPLATE = Path('/usr/share/AAVMF/AAVMF_VARS.ms.fd')
DESCRIPTOR = Path('/usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json')
MAX_RECEIPT = 1024 * 1024
TRANSITION_PATHS = {
    'usr/src/kedra/tests/vm/ghcr-update/boot-arm64.py',
    'usr/src/kedra/tests/vm/ghcr-update/boot-macos.py',
    'usr/src/kedra/tests/vm/ghcr-update/macos-transfer.py',
    'usr/src/kedra/tests/vm/ghcr-update/run-arm64.sh',
    'worklog.md', 'usr/src/kedra/docs/STATUS.md',
    '.specs/nix-release-composition/verification.md',
}


def require(value, message):
    if not value:
        raise RuntimeError(message)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def regular(path):
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, 'Expected a single-link regular input')
    return info


def canonical_path(value):
    require(isinstance(value, (str, Path)), 'Expected a filesystem path')
    raw = os.fspath(value)
    require(isinstance(raw, str) and raw.startswith('/')
            and all(part not in ('', '.', '..') for part in raw.split('/')[1:]),
            'Expected an absolute path without aliased components')
    path = Path(raw)
    actual = path.resolve(strict=True)
    require(actual == path, 'Symlinked path components are refused')
    return actual


def directory(value):
    path = canonical_path(value)
    require(stat.S_ISDIR(path.lstat().st_mode), 'Expected a real directory')
    return path


def source_file(value, parent):
    # Resolve and establish actual ancestry before reading any input bytes.
    base = directory(parent)
    path = canonical_path(value)
    require(path != base and path.is_relative_to(base), 'Input is outside its declared directory')
    regular(path)
    return path


def new_file(parent, name):
    require(isinstance(name, str) and name not in ('', '.', '..') and '/' not in name,
            'Expected one output filename')
    return directory(parent) / name


def write_new(path, value):
    path = new_file(path.parent, path.name)
    with path.open('x') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def document(path, parent):
    path = source_file(path, parent)
    require(path.stat().st_size <= MAX_RECEIPT, 'Oversized handoff metadata')
    return json.loads(path.read_bytes())


def legacy_context(args, current):
    temporary = directory(current['runner_temp'])
    old_file = source_file(args.legacy_context, temporary)
    require(old_file.stat().st_uid == current['uid'] and stat.S_IMODE(old_file.stat().st_mode) == 0o600
            and digest(old_file) == args.legacy_context_sha256, 'Original context hash/ownership differs')
    old = document(old_file, temporary)
    require(set(old) == set(current)
            and {key: value for key, value in old.items() if key != 'fixture_revision'}
                == {key: value for key, value in current.items() if key != 'fixture_revision'}
            and re.fullmatch('[a-f0-9]{40}', old['fixture_revision'])
            and old['fixture_revision'] != current['fixture_revision'], 'Legacy/current contexts differ beyond tool revision')
    review_file = source_file(args.transition, temporary)
    require(digest(review_file) == args.transition_sha256, 'Reviewed transition digest differs')
    review = document(review_file, temporary)
    require(set(review) == {'schema', 'kind', 'original_context_sha256', 'original_fixture_revision',
                           'tool_revision', 'changes'}
            and review['schema'] == 1 and review['kind'] == 'kedra-reviewed-hvf-tool-transition'
            and review['original_context_sha256'] == args.legacy_context_sha256
            and review['original_fixture_revision'] == old['fixture_revision']
            and review['tool_revision'] == current['fixture_revision']
            and set(review['changes']) <= TRANSITION_PATHS, 'Unreviewed tool transition')
    environment = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_OPTIONAL_LOCKS='0')

    def git(*arguments):
        return subprocess.check_output(['/usr/bin/git', '--no-replace-objects', '-C', current['repository'],
                                        *arguments], env=environment, timeout=30)

    git('merge-base', '--is-ancestor', old['fixture_revision'], current['fixture_revision'])
    changed = set(git('diff', '--no-renames', '--no-ext-diff', '--no-textconv', '--name-only', '-z',
                      old['fixture_revision'], current['fixture_revision'], '--')
                  .decode().rstrip('\0').split('\0'))
    require(changed == set(review['changes']) and changed <= TRANSITION_PATHS, 'Commit changes exceed reviewed bridge')
    git('diff', '--no-renames', '--no-ext-diff', '--no-textconv', '--quiet', 'HEAD', '--')
    for name, expected in review['changes'].items():
        require(set(expected) == {'before', 'after'} and expected['after'] is not None, 'Invalid reviewed file entry')
        for label, revision in (('before', old['fixture_revision']), ('after', current['fixture_revision'])):
            tree = git('ls-tree', '-z', revision, '--', name)
            if not tree:
                require(label == 'before' and expected[label] is None, 'Unexpected absent reviewed file')
                continue
            mode, kind, _object = tree.split(b'\t', 1)[0].decode().split()
            require(kind == 'blob' and mode in ('100644', '100755'), 'Unsupported transition artifact')
            content_hash = hashlib.sha256(git('show', revision + ':' + name)).hexdigest()
            require(expected[label] == {'sha256': content_hash, 'mode': mode}, 'Reviewed source bytes/mode differ')
        actual = source_file(Path(current['repository']) / name, Path(current['repository']))
        require(digest(actual) == expected['after']['sha256'], 'Working source differs from reviewed bridge')
    return old, {'original_context_sha256': args.legacy_context_sha256,
                 'current_context_sha256': digest(args.fixture_context),
                 'original_fixture_revision': old['fixture_revision'], 'tool_revision': current['fixture_revision'],
                 'transition_sha256': args.transition_sha256}


def process_identity(pid):
    require(type(pid) is int and pid > 1, 'Invalid selected process')
    process = Path('/proc') / str(pid)
    fields = (process / 'stat').read_text().rsplit(')', 1)[1].split()
    uid = next(line.split()[1] for line in (process / 'status').read_text().splitlines() if line.startswith('Uid:'))
    identity = {'pid': pid, 'start_ticks': fields[19], 'uid': int(uid),
                'argv_sha256': hashlib.sha256((process / 'cmdline').read_bytes()).hexdigest(),
                'exe': str((process / 'exe').resolve(strict=True))}
    return identity, fields[0]


def inspect(argv):
    result = subprocess.run(argv, check=True, capture_output=True, timeout=60)
    values = json.loads(result.stdout)
    require(isinstance(values, list) and len(values) == 1, 'Expected one exact owned resource')
    return values[0]


def resources(value):
    controller = inspect(['/usr/bin/docker', 'inspect', value['controller']])
    require(controller['Id'] == value['controller'] and controller['State']['Running']
            and controller['Config']['Hostname'] == socket.gethostname()
            and controller['Config']['Labels'].get('dev.kedra.lab.owner') == 'kedra-release-fixture'
            and controller['Config']['Labels'].get('dev.kedra.lab.kind') == 'release-controller',
            'Selected controller differs from this fixture')
    registry = inspect(['/usr/bin/sudo', '--non-interactive', '/usr/bin/podman', 'inspect', value['registry']])
    require(registry['Id'] == value['registry'] and registry['Name'].lstrip('/') == 'kedra-ghcr-registry'
            and registry['Config']['Labels'].get('dev.kedra.lab.owner') == 'kedra-release-fixture',
            'Selected registry identity changed')
    require(process_identity(value['control']['pid'])[0] == value['control'], 'Control process identity changed')
    # A stopped registry is valid while the unchanged guest exercises offline refusal.
    if value.get('outer') is not None:
        actual, state = process_identity(value['outer']['pid'])
        require(actual == value['outer'] and state == 'T', 'Legacy outer shell is no longer exactly held')


def observed_boundary(args, fixture, root, private):
    boundary_file = source_file(args.boundary, private)
    info = boundary_file.stat()
    require(info.st_uid == fixture['uid'] and stat.S_IMODE(info.st_mode) == 0o600
            and digest(boundary_file) == args.boundary_sha256, 'Observed boundary identity changed')
    value = document(boundary_file, private)
    require(set(value) == {'schema', 'kind', 'provenance', 'observation', 'source_revision', 'fixture_revision',
                           'controller', 'registry', 'outer', 'control', 'iso_sha256', 'cases_sha256', 'sentinel_sha256'}
            and value['schema'] == 1 and value['kind'] == 'kedra-observed-install-boundary'
            and value['provenance'] == 'operator-observed-existing-tools'
            and value['observation'] in ('completed', 'deadline')
            and value['source_revision'] == fixture['source_revision']
            and value['fixture_revision'] == fixture['fixture_revision']
            and args.start == ('A' if value['observation'] == 'completed' else 'install'),
            'Observed boundary does not select this exact fixture/start')
    for key in ('outer', 'control'):
        require(value[key]['uid'] == fixture['uid'], 'Boundary process has another owner')
    resources(value)
    outer = value['outer']
    command = (Path('/proc') / str(outer['pid']) / 'cmdline').read_bytes().split(b'\0')
    require(Path(outer['exe']).name == 'bash'
            and any(item in (str(HERE / 'run-arm64.sh').encode(),
                             b'usr/src/kedra/tests/vm/ghcr-update/run-arm64.sh') for item in command)
            and (Path('/proc') / str(outer['pid']) / 'cwd').resolve() == Path(fixture['repository']),
            'Held process is not this original fixture shell')
    for option, expected in {
        b'--runner-temp': fixture['runner_temp'], b'--evidence': fixture['evidence'],
        b'--source-revision': fixture['source_revision'], b'--fixture-revision': fixture['fixture_revision'],
    }.items():
        require(command.count(option) == 1 and command.index(option) + 1 < len(command)
                and command[command.index(option) + 1] == expected.encode(),
                'Held shell arguments select another fixture')
    selected = document(root / 'disks.json', root)
    require(value['iso_sha256'] == selected['iso_sha256']
            and value['sentinel_sha256'] == selected['sentinel_sha256']
            and value['cases_sha256'] == digest(source_file(root / 'cases.raw', root)),
            'Observed inputs differ from retained fixture')
    return value


def finish_probe(path, expected_hash, fixture, outer):
    evidence = directory(fixture['evidence'])
    path = source_file(path, evidence)
    require(digest(path) == expected_hash, 'Selected Bash observation changed')
    value = document(path, evidence)
    executable = value['shell_executable']
    require(value['schema_version'] == 1 and value['outcome'] == 'pass'
            and value['parent_wait_status'] == -15 and value['exit_trap_original_status'] == '0'
            and value['pending_SIGTERM_observed'] is True and value['state_before_continue'] == 'T'
            and value['next_command_executed'] is False and value['sentinel_unchanged'] is True
            and executable['matches_held_fixture_shell'] is True and executable['path'] == outer['exe']
            and digest(Path(outer['exe'])) == executable['sha256'], 'Bash finish observation is not applicable')
    return {'path': str(path), 'sha256': expected_hash}


def trust():
    expected = firmware.trust(TEMPLATE)
    require({'PK', 'KEK', 'db', 'SecureBootEnable'} <= expected.keys()
            and expected['SecureBootEnable'][1] == b'\x01'
            and expected['PK'][1] and expected['KEK'][1]
            and firmware.MICROSOFT_2023 in firmware.certificates(expected['db'][1]),
            'Required Microsoft Secure Boot authority is absent')
    descriptor = json.loads(DESCRIPTOR.read_text())
    require(descriptor['mapping']['nvram-template']['filename'] == str(TEMPLATE)
            and Path(descriptor['mapping']['executable']['filename']).resolve() == CODE.resolve()
            and {'enrolled-keys', 'secure-boot'} <= set(descriptor['features'])
            and any(item.get('architecture') == 'aarch64' for item in descriptor['targets']),
            'Unexpected ARM firmware descriptor')
    return expected


def export(args, fixture, root, private):
    require(fixture['mode'] == 'local', 'HVF handoff is local-fixture only')
    boundary = None
    if args.operation == 'adopt':
        boundary = observed_boundary(args, fixture, root, private)
        args.controller, args.registry = boundary['controller'], boundary['registry']
        control, outer = boundary['control'], boundary['outer']
        probe = finish_probe(args.finish_probe, args.finish_probe_sha256, fixture, outer)
    else:
        control, _ = process_identity(args.control_pid)
        require(control['uid'] == fixture['uid'], 'Control process has another owner')
        outer, probe = None, None
        if args.start == 'install':
            timeout = document(root / 'install-timeout.json', root)
            require(timeout == {'schema_version': 1, 'phase': 'install', 'reason': 'phase_deadline',
                                'source_revision': fixture['source_revision'],
                                'fixture_revision': fixture['fixture_revision'], 'qemu_reaped': True},
                    'Fresh handoff requires the actual stopped-install deadline receipt')
    require(isinstance(args.controller, str) and re.fullmatch('[a-f0-9]{64}', args.controller),
            'An exact selected controller ID is required')
    require(isinstance(args.registry, str) and re.fullmatch('[a-f0-9]{64}', args.registry),
            'An exact selected registry ID is required')
    identity = {'controller': args.controller, 'registry': args.registry, 'control': control, 'outer': outer}
    resources(identity)
    expires_at = int(time.time()) + (28800 if args.start == 'install' else 19800)
    require(not (root / 'hvf-owner.json').exists(), 'Fixture already transferred; do not transfer twice')
    # Existing run-arm64 orchestration must first be stopped at a reviewed phase
    # boundary. The patched Linux launcher shares our held lock for future starts.
    commands = subprocess.check_output(['ps', '-eo', 'comm='], timeout=30).decode().splitlines()
    require(not any(Path(name.strip()).name.startswith('qemu-system') for name in commands),
            'A controller QEMU writer is still present')
    require(not (Path(fixture['evidence']) / 'cleanup.json').exists(), 'Original fixture already entered cleanup')
    selected = json.loads(source_file(root / 'disks.json', root).read_text())
    iso = source_file(selected['iso'], private)
    sentinel = source_file(root / 'sentinel.raw', root)
    variables = source_file(root / 'AAVMF_VARS.fd', root)
    require(digest(iso) == selected['iso_sha256']
            and digest(sentinel) == selected['sentinel_sha256'], 'Fixture inputs changed')
    require(sentinel.stat().st_size == 16 * 1024**2, 'Unexpected sentinel size')
    expected = trust()
    require(firmware.trust(variables) == expected, 'Installed firmware authority changed')
    if args.start == 'A':
        installed = json.loads(source_file(root / 'install-result.json', root).read_text())
        require(installed['phase'] == 'install' and installed['outcome'] == 'pass'
                and not (root / 'A.serial.log').exists(), 'A requires a completed, unstarted installed fixture')
        disk = source_file(root / 'installed.qcow2', root)
        subprocess.run(['qemu-img', 'check', disk], check=True,
                       timeout=300, stdout=subprocess.DEVNULL)
    else:
        require(not (root / 'install-result.json').exists(), 'Fresh retry must not relabel a completed target')
    # A validated adoption now owns failure cleanup; before this point rejection
    # must not signal any existing process or discard the original evidence.
    args.adopted = {'outer': outer, 'finish_probe': probe} if outer is not None else None
    output = private / ('hvf-export-' + os.urandom(12).hex())
    output.mkdir(mode=0o700)
    output = directory(output)
    files = {}

    def copy(name, source, parent):
        source = source_file(source, parent)
        before = digest(source)
        target = new_file(output, name)
        if name in ('installer.iso', 'disk.qcow2'):
            with target.open('xb'):
                pass
            subprocess.run(['/bin/cp', '--reflink=always', '--', source, target], check=True, timeout=300)
            require((source.stat().st_dev, source.stat().st_ino) != (target.stat().st_dev, target.stat().st_ino),
                    'Reflink target aliases the source inode')
            with target.open('rb') as writer:
                os.fsync(writer.fileno())
        else:
            with source.open('rb') as reader, target.open('xb') as writer:
                shutil.copyfileobj(reader, writer, 1024 * 1024)
                writer.flush()
                os.fsync(writer.fileno())
        require(digest(source_file(target, output)) == before == digest(source_file(source, parent)),
                'Input changed while transferring')
        files[name] = {'sha256': before, 'bytes': target.stat().st_size, 'mode': '0600'}

    for name, (source, parent) in {
        'installer.iso': (iso, private), 'cases.raw': (root / 'cases.raw', root),
        'sentinel.raw': (sentinel, root), 'code.fd': (CODE.resolve(strict=True), CODE.parent),
        'template.fd': (TEMPLATE.resolve(strict=True), TEMPLATE.parent),
        'firmware.json': (DESCRIPTOR, DESCRIPTOR.parent),
        'disk-passphrase': (private / 'disk-passphrase', private),
    }.items():
        copy(name, source, parent)
    if args.start == 'A':
        copy('disk.qcow2', disk, root)
        copy('vars.fd', variables, root)
    else:
        blank = new_file(output, 'blank.qcow2')
        subprocess.run(['qemu-img', 'create', '-f', 'qcow2', blank, '96G'],
                       check=True, timeout=120, stdout=subprocess.DEVNULL)
        copy('disk.qcow2', blank, output)
        source_file(blank, output).unlink()
        copy('vars.fd', TEMPLATE.resolve(strict=True), TEMPLATE.parent)
    # Separate negative-case variables: only the exact existing fixture's one
    # enforcement bit changes; neither enrolled keys nor guest policy changes.
    copy('insecure-vars.fd', TEMPLATE.resolve(strict=True), TEMPLATE.parent)
    insecure_variables = source_file(output / 'insecure-vars.fd', output)
    subprocess.run(['virt-fw-vars', '--inplace', insecure_variables, '--set-false', 'SecureBootEnable'],
                   check=True, timeout=120, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    insecure = dict(expected)
    insecure['SecureBootEnable'] = (expected['SecureBootEnable'][0], b'\x00')
    insecure_variables = source_file(insecure_variables, output)
    require(firmware.trust(insecure_variables) == insecure, 'Insecure fixture changed other authority')
    files['insecure-vars.fd']['sha256'] = digest(insecure_variables)
    files['insecure-vars.fd']['bytes'] = insecure_variables.stat().st_size
    manifest = {'schema': 2, 'kind': 'kedra-private-hvf-transfer', 'fixture_revision': fixture['fixture_revision'],
                'source_revision': fixture['source_revision'], 'controller': args.controller,
                'uid': fixture['uid'], 'context': str(args.fixture_context), 'start': args.start,
                'helper': str(Path(__file__).resolve()), 'helper_sha256': digest(Path(__file__)),
                'code_sha256': digest(CODE), 'template_sha256': digest(TEMPLATE), 'files': files,
                'registry': args.registry, 'control': control, 'outer': outer, 'finish_probe': probe,
                'boundary_sha256': args.boundary_sha256 if boundary else None,
                'boundary_provenance': boundary['provenance'] if boundary else 'current-launcher-receipt',
                'context_admission': args.context_admission,
                'incomplete_original_target_excluded': args.start == 'install',
                'expires_at': expires_at, 'export_free_bytes': shutil.disk_usage(private).free}
    write_new(output / 'transfer.json', manifest)
    resources(identity)
    write_new(root / 'hvf-owner.json', {'schema': 2, 'transfer': str(output),
                                      'manifest_sha256': digest(output / 'transfer.json')})
    for export_directory in (output, root):
        descriptor = os.open(export_directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    transfer_hash = digest(output / 'transfer.json')
    print(json.dumps({'export': str(output), 'manifest_sha256': transfer_hash, 'expires_at': expires_at}), flush=True)
    return manifest, transfer_hash


def transferred(root, private, expected_hash):
    owner = document(root / 'hvf-owner.json', root)
    require(owner['schema'] == 2 and owner['manifest_sha256'] == expected_hash, 'Wrong retained transfer')
    output = canonical_path(owner['transfer'])
    require(output.is_relative_to(private), 'Retained export escaped private fixture')
    filename = source_file(output / 'transfer.json', output)
    require(digest(filename) == expected_hash, 'Retained manifest changed')
    manifest = document(filename, output)
    require(manifest['schema'] == 2, 'Unknown retained protocol')
    return manifest


def finish_receipt(payload, manifest, expected_hash):
    value = json.loads(payload)
    require(set(value) == {'schema', 'manifest_sha256', 'source_revision', 'fixture_revision',
                          'controller', 'registry', 'outcome', 'reports', 'final_files', 'runtime_receipt_sha256',
                          'context_admission', 'transport', 'connector_sha256'}
            and value['schema'] == 1 and value['manifest_sha256'] == expected_hash
            and value['source_revision'] == manifest['source_revision']
            and value['fixture_revision'] == manifest['fixture_revision']
            and value['controller'] == manifest['controller'] and value['registry'] == manifest['registry']
            and value['context_admission'] == manifest['context_admission']
            and value['transport'] == 'guestfwd-unix-v1'
            and re.fullmatch('[a-f0-9]{64}', value['connector_sha256'])
            and value['outcome'] in ('pass', 'abort'), 'Finish receipt belongs to another fixture')
    require(time.time() < manifest['expires_at'], 'Retained handoff deadline expired')
    if value['outcome'] == 'abort':
        require(value['reports'] == {} and value['final_files'] == {}, 'Abort cannot claim passing phases')
        return value
    phases = ('refuse-insecure', 'install', 'A', 'B', 'ROLLBACK') if manifest['start'] == 'install' else ('A', 'B', 'ROLLBACK')
    require(set(value['reports']) == set(phases), 'Finish lacks the exact required phases')
    for phase_name in phases:
        report = value['reports'][phase_name]
        negative = phase_name == 'refuse-insecure'
        media = phase_name in ('refuse-insecure', 'install')
        require(report['phase'] == phase_name and report['outcome'] == ('refused' if negative else 'pass')
                and report['accelerator'] == 'hvf' and report['transfer_sha256'] == expected_hash
                and report['transport'] == value['transport'] and report['connector_sha256'] == value['connector_sha256']
                and report['sentinel_unchanged'] is True and report['iso_unchanged'] is True
                and report['firmware_trust_unchanged'] is True and report['iso_attached'] is media
                and report['luks_unlock_observed'] is (not media)
                and report['stop'] == ('qmp-quit-after-verifier-refusal' if negative else 'guest-poweroff'),
                'Incomplete or failed HVF phase')
    files = value['final_files']
    require(set(files) == {'disk.qcow2', 'vars.fd', 'installer.iso', 'sentinel.raw'}
            and all(isinstance(item, str) and re.fullmatch('[a-f0-9]{64}', item) for item in files.values())
            and files['installer.iso'] == manifest['files']['installer.iso']['sha256']
            and files['sentinel.raw'] == manifest['files']['sentinel.raw']['sha256'], 'Final immutable inputs differ')
    require(re.fullmatch('[a-f0-9]{64}', value['runtime_receipt_sha256']), 'Runtime identity is absent')
    return value


def terminate_legacy(manifest, fixture, root):
    outer = manifest['outer']
    probe = manifest['finish_probe']
    finish_probe(probe['path'], probe['sha256'], fixture, outer)
    actual, state = process_identity(outer['pid'])
    require(actual == outer and state == 'T', 'Refusing to signal a changed/unheld outer shell')
    descriptor = os.pidfd_open(outer['pid'], 0)
    try:
        actual, state = process_identity(outer['pid'])
        require(actual == outer and state == 'T', 'Outer shell changed before pidfd finish')
        signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        signal.pidfd_send_signal(descriptor, signal.SIGCONT)
        poller = select.poll()
        poller.register(descriptor, select.POLLIN)
        require(bool(poller.poll(600000)), 'Original fixture cleanup did not exit within 600 seconds')
    finally:
        os.close(descriptor)
    evidence = directory(fixture['evidence'])
    cleanup_file = source_file(evidence / 'cleanup.json', evidence)
    cleanup = document(cleanup_file, evidence)
    require(cleanup['cleanup_failed'] is False and cleanup['private_inputs_removed'] is True
            and cleanup['registry'] == 'removed', 'Original fixture cleanup failed')
    return {'method': 'owned-pidfd-SIGTERM-then-SIGCONT', 'pidfd_exit_observed': True,
            'cleanup_sha256': digest(cleanup_file), 'cleanup_original_exit_code': cleanup['original_exit_code'],
            'actual_parent_wait_status': None, 'parent_wait_observation_required': True,
            'probe_sha256': probe['sha256'], 'fixture_outcome_independent_of_cleanup_exit': True}


def finish(payload, expected_hash, fixture, root, private):
    request_hash = hashlib.sha256(payload).hexdigest()
    completed = root / 'hvf-finish.json'
    if completed.exists():
        value = document(completed, root)
        require(value['manifest_sha256'] == expected_hash and value['request_sha256'] == request_hash,
                'Conflicting finish retry')
        return value
    manifest = transferred(root, private, expected_hash)
    require(manifest['source_revision'] == fixture['source_revision']
            and manifest['context_admission']['tool_revision'] == fixture['fixture_revision']
            and digest(Path(manifest['context'])) == manifest['context_admission']['current_context_sha256'],
            'Finish current-tools context changed after adoption')
    receipt = finish_receipt(payload, manifest, expected_hash)
    resources(manifest)
    pending = root / 'hvf-finishing.json'
    require(not pending.exists(), 'Finish already in progress; inspect exact owned cleanup')
    write_new(pending, {'schema': 1, 'manifest_sha256': expected_hash, 'request_sha256': request_hash,
                        'deadline': int(time.time()) + 660})
    legacy = terminate_legacy(manifest, fixture, root) if manifest['outer'] is not None else None
    result = {'schema': 1, 'manifest_sha256': expected_hash, 'request_sha256': request_hash,
              'source_revision': manifest['source_revision'], 'fixture_revision': manifest['fixture_revision'],
              'controller': manifest['controller'], 'registry': manifest['registry'],
              'outcome': receipt['outcome'], 'hvf_receipt': receipt, 'legacy_termination': legacy,
              'context_admission': manifest['context_admission']}
    write_new(completed, result)
    return result


def await_finish(manifest, expected_hash, fixture, root):
    deadline = time.monotonic() + max(0, manifest['expires_at'] - time.time())
    next_liveness = 0
    try:
        while time.monotonic() < deadline:
            completed = root / 'hvf-finish.json'
            if completed.exists():
                result = document(completed, root)
                require(result['manifest_sha256'] == expected_hash, 'Finish is for another handoff')
                require(result['outcome'] == 'pass', 'Owner aborted HVF qualification')
                evidence = directory(fixture['evidence'])
                write_new(evidence / 'hvf-finish.json', result)
                for name, report in result['hvf_receipt']['reports'].items():
                    write_new(evidence / ('hvf-' + name + '-result.json'), report)
                print(json.dumps({'manifest_sha256': expected_hash, 'outcome': 'pass'}), flush=True)
                return
            pending = root / 'hvf-finishing.json'
            if pending.exists():
                value = document(pending, root)
                require(value['manifest_sha256'] == expected_hash and time.time() < value['deadline'],
                        'Explicit finish did not complete within its bound')
            elif time.monotonic() >= next_liveness:
                try:
                    resources(manifest)
                except (OSError, RuntimeError, subprocess.SubprocessError):
                    if not pending.exists():
                        raise
                next_liveness = time.monotonic() + 10
            time.sleep(2)
        raise RuntimeError('Retained HVF handoff timed out')
    except BaseException:
        # For a newly started run-arm64, our failure returns to its unchanged
        # EXIT trap. An adopted held shell needs the explicitly qualified finish.
        if manifest['outer'] is not None and not (root / 'hvf-finishing.json').exists():
            termination = terminate_legacy(manifest, fixture, root)
            write_new(root / 'hvf-aborted.json', {'schema': 1, 'manifest_sha256': expected_hash,
                                               'outcome': 'aborted', 'legacy_termination': termination})
        raise


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    add_context_argument(parser)
    parser.add_argument('--operation', choices=('handoff', 'adopt', 'finish', 'verify-vars'), required=True)
    parser.add_argument('--controller')
    parser.add_argument('--registry')
    parser.add_argument('--control-pid', type=int)
    parser.add_argument('--boundary', type=Path)
    parser.add_argument('--boundary-sha256')
    parser.add_argument('--finish-probe', type=Path)
    parser.add_argument('--finish-probe-sha256')
    parser.add_argument('--manifest-sha256')
    parser.add_argument('--legacy-context', type=Path)
    parser.add_argument('--legacy-context-sha256')
    parser.add_argument('--transition', type=Path)
    parser.add_argument('--transition-sha256')
    parser.add_argument('--start', choices=('install', 'A'), default='A')
    parser.add_argument('--insecure', action='store_true')
    args = parser.parse_args()
    args.adopted = None
    require(args.operation != 'adopt' or args.legacy_context is not None,
            'Adoption requires explicit original/current context transition evidence')
    require(args.fixture_context is not None, 'Explicit closed Linux fixture context required')
    fixture = load_context(args.fixture_context)
    args.context_admission = {'original_context_sha256': digest(args.fixture_context),
                              'current_context_sha256': digest(args.fixture_context),
                              'original_fixture_revision': fixture['fixture_revision'],
                              'tool_revision': fixture['fixture_revision'], 'transition_sha256': None}
    if args.legacy_context is not None:
        require(args.operation == 'adopt', 'Legacy transition is explicit adoption only')
        fixture, args.context_admission = legacy_context(args, fixture)
    root = directory(Path(fixture['runner_temp']) / 'kedra-ghcr')
    private = root.parent / 'kedra-ghcr-private'
    descriptor = os.open(root / 'host-launch.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'r+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if args.operation == 'finish':
            payload = sys.stdin.buffer.read(MAX_RECEIPT + 1)
            require(0 < len(payload) <= MAX_RECEIPT, 'Oversized finish receipt')
            print(json.dumps(finish(payload, args.manifest_sha256, fixture, root, private)), flush=True)
            return
        private = directory(private)
        if args.operation in ('handoff', 'adopt'):
            try:
                manifest, transfer_hash = export(args, fixture, root, private)
            except BaseException:
                if args.adopted is not None:
                    termination = terminate_legacy(args.adopted, fixture, root)
                    write_new(root / 'hvf-adoption-failed.json', {'schema': 1, 'outcome': 'failed',
                                                                'legacy_termination': termination})
                raise
        else:
            expected = trust()
            if args.insecure:
                expected['SecureBootEnable'] = (expected['SecureBootEnable'][0], b'\x00')
            payload = sys.stdin.buffer.read(64 * 1024**2 + 1)
            require(0 < len(payload) <= 64 * 1024**2, 'Unexpected firmware size')
            with tempfile.TemporaryDirectory(dir=private, prefix='hvf-vars-') as temporary:
                path = new_file(Path(temporary), 'vars.fd')
                path.write_bytes(payload)
                require(firmware.trust(source_file(path, Path(temporary))) == expected,
                        'Transferred firmware authority changed')
            print(json.dumps({'trust_unchanged': True, 'code_sha256': digest(CODE),
                              'template_sha256': digest(TEMPLATE)}))
            return
    # Do not hold the launch lock while the separate finish command needs it.
    await_finish(manifest, transfer_hash, fixture, root)


if __name__ == '__main__':
    def interrupt(number, _frame):
        raise InterruptedError('Interrupted retained handoff: ' + str(number))

    signal.signal(signal.SIGTERM, interrupt)
    signal.signal(signal.SIGINT, interrupt)
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit('HVF transfer refused: ' + str(error)) from error
