"""Public image retention preflight of the Docker backend, imported by run.py.

The removed test-qemu-arm64.yml step "Check public image retention on the runner
backend", unchanged in its commands, evidence and gate: build a tiny public
arm64 image, check its identity, save it and retain it with `sysroot store
add-image`. Full candidate work starts only when that retention succeeds;
selecting the containerd image store alone is not a pass. Every outcome
leaves image-retention-{stage,preflight,cleanup}.json in the evidence directory.
"""
import contextlib
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import traceback
from pathlib import Path

PHASES = ('prepare', 'build', 'identity', 'backend', 'save', 'store-init', 'retain', 'metadata', 'result')
MARKERS = {'dockerfile_unavailable': b'failed to read dockerfile',
           'build_solve_failure': b'failed to solve', 'permission_denied': b'permission denied',
           'daemon_unavailable': b'Cannot connect to the Docker daemon',
           'archive_root_refusal': b'root identity differs or archive has multiple roots'}
LOGS = ('build.log', 'identity.stderr', 'backend.stderr', 'save.stderr', 'init.stderr',
        'retain.stdout', 'retain.stderr', 'metadata.stderr')
DRIVERS = ('overlay2', 'overlayfs', 'btrfs', 'zfs', 'vfs', 'aufs', 'devicemapper')
FIXED_ERROR = b'sysroot: corrupt engine state: image archive: root identity differs or archive has multiple roots\n'
LABEL = 'dev.kedra.retention-probe'
BACKEND = '{"version":{{json .ServerVersion}},"driver":{{json .Driver}},"status":{{json .DriverStatus}}}'
# Any failure of the probe's own Python steps; the shell step stopped on each one.
FAILURES = (OSError, ValueError, KeyError, TypeError, IndexError, AttributeError, subprocess.SubprocessError)


class Probe:
    def __init__(self, root, sysroot, evidence):
        self.root, self.sysroot, self.evidence = root, sysroot, evidence
        self.label = f'{os.environ["GITHUB_RUN_ID"]}-{os.environ["GITHUB_RUN_ATTEMPT"]}'
        self.tag = f'localhost/kedra-image-retention:{self.label}'
        self.stage, self.image, self.store_initialized = 'prepare', '', False

    def call(self, command, stdout=None, stderr=None, combined=False):
        """Exit status of one probe command; named streams go to probe files, others are inherited."""
        with contextlib.ExitStack() as files:
            out = files.enter_context(open(self.root / stdout, 'w')) if stdout else None
            err = files.enter_context(open(self.root / stderr, 'w')) if stderr else None
            return subprocess.run(command, stdin=subprocess.DEVNULL, stdout=out,
                                  stderr=subprocess.STDOUT if combined else err, check=False).returncode

    def check(self, command, **files):
        code = self.call(command, **files)
        if code:
            raise ProbeFailure(code)


class ProbeFailure(Exception):
    def __init__(self, code):
        super().__init__(f'exit {code}')
        self.code = code


def run_probe(probe):
    (probe.root / 'payload').write_text('public image retention probe\n')
    (probe.root / 'Containerfile').write_text('FROM scratch\nCOPY payload /payload\n')
    probe.stage = 'build'
    probe.check(['docker', 'build', '--platform', 'linux/arm64', '--network', 'none', '--pull=false', '--no-cache',
                 '--label', f'{LABEL}={probe.label}', '--iidfile', str(probe.root / 'image.id'),
                 '--file', str(probe.root / 'Containerfile'), '--tag', probe.tag, str(probe.root)],
                stdout='build.log', combined=True)
    probe.stage = 'identity'
    probe.image = (probe.root / 'image.id').read_text().rstrip('\n')
    if not re.fullmatch('sha256:[a-f0-9]{64}', probe.image):
        raise ProbeFailure(1)
    with open(probe.root / 'identity.stderr', 'w') as errors:
        observed = subprocess.run(['docker', 'image', 'inspect', '--format',
                                   '{{.Id}} {{.Os}} {{.Architecture}} {{index .Config.Labels "' + LABEL + '"}}',
                                   probe.tag], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=errors,
                                  text=True, check=False).stdout.rstrip('\n')
    if observed != f'{probe.image} linux arm64 {probe.label}':
        raise ProbeFailure(1)
    probe.stage = 'backend'
    probe.check(['docker', 'info', '--format', BACKEND], stdout='backend.json', stderr='backend.stderr')
    probe.stage = 'save'
    probe.check(['docker', 'image', 'save', '--output', str(probe.root / 'image.tar'), probe.image],
                stderr='save.stderr')
    probe.stage = 'store-init'
    probe.check([str(probe.sysroot), 'store', 'init', '--store', str(probe.root / 'store')],
                stdout='init.json', stderr='init.stderr')
    probe.store_initialized = True
    probe.stage = 'retain'
    retained = probe.call([str(probe.sysroot), 'store', 'add-image', '--store', str(probe.root / 'store'),
                           '--image', probe.image], stdout='retain.stdout', stderr='retain.stderr')
    probe.stage = 'metadata'
    (probe.root / 'metadata.stderr').write_text('')
    try:
        preflight(probe, retained)
    except FAILURES:
        (probe.root / 'metadata.stderr').write_text(traceback.format_exc())
        raise ProbeFailure(1) from None
    probe.stage = 'result'
    if retained:
        raise ProbeFailure(1)


def bounded(path):
    with path.open('rb') as stream:
        data = stream.read(65537)
    return {'bytes': path.stat().st_size, 'hashed_bytes': min(len(data), 65536),
            'sha256': hashlib.sha256(data[:65536]).hexdigest(), 'complete': len(data) <= 65536,
            'exact_root_identity_refusal': data == FIXED_ERROR}


def backend_report(root):
    backend = json.loads((root / 'backend.json').read_bytes())
    version = backend.get('version', '')
    driver = backend.get('driver')
    return {'docker_version': version if re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][a-zA-Z0-9.-]+)?', version) else None,
            'storage_driver': driver if driver in DRIVERS else None,
            'containerd_driver_marker_observed': any(row == ['driver-type', 'io.containerd.snapshotter.v1']
                                                     for row in backend.get('status') or [])}


def preflight(probe, retained):
    report = {'schema_version': 1, 'selected_image': (probe.root / 'image.id').read_text().strip(),
              'public_add_image_exit': retained}
    source = os.environ['GITHUB_SHA']
    if re.fullmatch('[a-f0-9]{40}', source) is None:
        raise ValueError('invalid source identity')
    report['source_revision'] = source
    with probe.sysroot.open('rb') as program:
        report['sysroot_sha256'] = hashlib.file_digest(program, 'sha256').hexdigest()
    report.update(backend_report(probe.root))
    for name in ('retain.stdout', 'retain.stderr'):
        report[name] = bounded(probe.root / name)
    try:
        report.update(archive_metadata(probe.root / 'image.tar'))
        report['archive_metadata'] = 'observed'
    except (OSError, ValueError, KeyError, TypeError, tarfile.TarError):
        report['archive_metadata'] = 'unavailable_or_unsupported'
    (probe.evidence / 'image-retention-preflight.json').write_text(json.dumps(report, sort_keys=True) + '\n')


def digest(value):
    if not isinstance(value, str) or re.fullmatch('sha256:[a-f0-9]{64}', value) is None:
        raise ValueError('unsupported digest')
    return value


class Archive:
    def __init__(self, archive):
        self.archive, self.members = archive, {}
        for member in archive:
            if len(self.members) >= 256 or member.name in self.members:
                raise ValueError('duplicate or oversized archive')
            self.members[member.name] = member

    def document(self, name):
        member = self.members[name]
        if not member.isfile() or not 0 < member.size <= 65536:
            raise ValueError('unsupported metadata member')
        stream = self.archive.extractfile(member)
        if stream is None:
            raise ValueError('missing metadata member')
        with stream:
            data = stream.read(65537)
        if len(data) != member.size:
            raise ValueError('metadata size differs')
        return json.loads(data), hashlib.sha256(data).hexdigest()


def docker_manifests(archive):
    compatibility, compatibility_hash = archive.document('manifest.json')
    if not isinstance(compatibility, list) or len(compatibility) > 16:
        raise ValueError('unsupported Docker manifest count')
    configs = []
    for item in compatibility:
        name = item['Config']
        if not isinstance(name, str) or re.fullmatch(r'(?:blobs/sha256/[a-f0-9]{64}|[a-f0-9]{64}\.json)', name) is None:
            raise ValueError('unsupported Docker config name')
        _, config_hash = archive.document(name)
        configs.append('sha256:' + config_hash)
    return {'docker_manifest_count': len(compatibility), 'docker_config_digests': configs,
            'docker_manifest_sha256': compatibility_hash}


def manifest_graph(archive, roots):
    pending, manifests, seen = list(roots), [], set()
    while pending:
        current = pending.pop()
        if current in seen or len(seen) >= 16:
            raise ValueError('duplicate or oversized metadata graph')
        seen.add(current)
        value, actual = archive.document('blobs/sha256/' + current[7:])
        if current != 'sha256:' + actual:
            raise ValueError('metadata digest differs')
        if 'manifests' in value:
            children = value['manifests']
            if not isinstance(children, list) or len(children) > 16:
                raise ValueError('unsupported child count')
            pending.extend(digest(child['digest']) for child in children)
        else:
            config = digest(value['config']['digest'])
            _, actual_config = archive.document('blobs/sha256/' + config[7:])
            if config != 'sha256:' + actual_config:
                raise ValueError('config digest differs')
            manifests.append({'manifest': current, 'config': config})
    return manifests


def archive_metadata(path):
    if not 0 < path.stat().st_size <= 16 * 1024**2:
        raise ValueError('oversized probe archive')
    with tarfile.open(path, mode='r:') as opened:
        archive = Archive(opened)
        report = docker_manifests(archive)
        index, index_hash = archive.document('index.json')
        roots = index['manifests']
        if not isinstance(roots, list) or len(roots) > 16:
            raise ValueError('unsupported root count')
        report.update(root_count=len(roots), root_digests=[digest(item['digest']) for item in roots],
                      index_sha256=index_hash)
        report['manifest_configs'] = manifest_graph(archive, report['root_digests'])
    return report


def log_record(root, name):
    item = {'name': name, 'available': False}
    try:
        descriptor = os.open(root / name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, 'rb') as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != os.getuid():
                raise ValueError('unsupported private log')
            data = stream.read(65536)
        item.update(available=True, bytes=info.st_size, hashed_bytes=len(data),
                    sha256=hashlib.sha256(data).hexdigest(), complete=len(data) == info.st_size,
                    observed_markers=[key for key, marker in MARKERS.items() if marker in data])
    except (OSError, ValueError):
        pass
    return item


def stage_report(probe, result):
    report = {'schema_version': 1, 'stage': probe.stage if probe.stage in PHASES else 'unknown',
              'original_exit': result, 'logs': [log_record(probe.root, name) for name in LOGS]}
    (probe.evidence / 'image-retention-stage.json').write_text(json.dumps(report, sort_keys=True) + '\n')


def remove_store(probe):
    """Exit statuses of unpinning and collecting the probe store, and whether collection ran."""
    if not probe.store_initialized:
        return 0, 0, False
    store = ['--store', str(probe.root / 'store')]
    unpinned = probe.call([str(probe.sysroot), 'store', 'unpin-image', *store, '--image', probe.image],
                          stdout='unpin.stdout', stderr='unpin.stderr')
    if unpinned:
        return unpinned, 1, False
    collected = probe.call([str(probe.sysroot), 'store', 'gc', *store, '--delete'],
                           stdout='gc.stdout', stderr='gc.stderr')
    return 0, collected, True


def remove_image(probe):
    if not probe.image:
        return 0
    current = subprocess.run(['docker', 'image', 'inspect', '--format', '{{.Id}}', probe.tag], stdin=subprocess.DEVNULL,
                             capture_output=True, text=True, check=False).stdout.rstrip('\n')
    if current != probe.image:
        return 1
    return subprocess.run(['docker', 'image', 'rm', probe.tag], stdin=subprocess.DEVNULL,
                          stdout=subprocess.DEVNULL, check=False).returncode


def cleanup(probe, result):
    """The removed step's EXIT trap: report, remove what the probe created and keep the first failure."""
    try:
        stage_report(probe, result)
        diagnostic = 0
    except FAILURES:
        traceback.print_exc()
        diagnostic = 1
    unpinned, collected, collection = remove_store(probe)
    removed = remove_image(probe)
    try:
        shutil.rmtree(probe.root)
        work = 0
    except OSError:
        work = 1
    receipt = {'schema_version': 1, 'original_exit': result, 'diagnostic_exit': diagnostic,
               'store_unpin_attempted': probe.store_initialized, 'store_unpin_exit': unpinned,
               'store_gc_attempted': collection, 'store_gc_exit': collected,
               'image_remove_attempted': bool(probe.image), 'image_remove_exit': removed, 'work_remove_exit': work}
    try:
        (probe.evidence / 'image-retention-cleanup.json').write_text(json.dumps(receipt, separators=(',', ':')) + '\n')
    except OSError:
        result = result or 1
    if result == 0 and any((diagnostic, unpinned, collected, removed, work)):
        result = 1
    return result


def check(evidence, sysroot):
    """Run the preflight; its exit status is 0 only when retention and cleanup both succeed."""
    previous = os.umask(0o077)
    try:
        probe = Probe(Path(tempfile.mkdtemp(prefix='kedra-image-retention.', dir=os.environ['RUNNER_TEMP'])),
                      Path(sysroot), Path(evidence))
        result = 0
        try:
            run_probe(probe)
        except ProbeFailure as failure:
            result = failure.code
        except FAILURES:
            traceback.print_exc(file=sys.stderr)
            result = 1
        return cleanup(probe, result)
    finally:
        os.umask(previous)
