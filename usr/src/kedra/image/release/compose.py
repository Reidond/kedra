#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Build unsigned ARM composition inputs through the existing public engine APIs.

The release and disposable-fixture workflows share this unsigned producer. It
has no signing, publication, installation or candidate-root execution interface.
The release workflow validates and signs its final public-trust derivative.
"""
import argparse
import hashlib
import json
import os
import platform
import re
import signal
import stat
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
import material as m

IMAGE = 'usr/src/kedra/image/'
NATIVE_RECIPE = IMAGE + 'release/native-steps.json'
ENGINE_PLATFORM = 'aarch64-linux'
OCI_PLATFORM = 'linux/arm64'
PROGRAMS = ('sysroot', 'sysroot-helper', 'kedra-lab')
CONTEXT_FILES = frozenset(('Containerfile', 'assemble.sh', 'sysroot', 'sysroot-helper',
                           'payload.tar', 'agents.tar', 'bitwarden.tar'))
MAX_JSON = 64 * 1024**2
MAX_FILE = 8 * 1024**3
INSPECT_TIMEOUT = 300
BUILD_TIMEOUT = 2400
SOURCE_PATH = '/usr/share/sysroot/source.json'
RPM_PATH = '/usr/share/sysroot/package-material.txt'
NATIVE_PATH = '/usr/share/sysroot/native-receipt.json'
FOUNDATION_FIELDS = frozenset(('schema_version', 'target', 'source_revision',
                              'input_material_sha256', 'foundation_image', 'engine',
                              'rpm_sha256', 'source_manifest_sha256', 'retention_receipt_sha256'))
CONTRIBUTION_FIELDS = frozenset(('schema_version', 'source_revision', 'foundation_image',
                                'input_material_sha256', 'definition_sha256',
                                'catalog_pins_sha256', 'author_sha256', 'outputs'))
CONFIG_DIAGNOSTIC_KEYS = (
    'Hostname', 'Domainname', 'User', 'AttachStdin', 'AttachStdout', 'AttachStderr',
    'ExposedPorts', 'Tty', 'OpenStdin', 'StdinOnce', 'Env', 'Cmd', 'Healthcheck',
    'ArgsEscaped', 'Image', 'Volumes', 'WorkingDir', 'Entrypoint', 'NetworkDisabled',
    'MacAddress', 'OnBuild', 'Labels', 'StopSignal', 'StopTimeout', 'Shell',
)
NATIVE_STEPS = [
    {'kind': 'glib_schemas'},
    {'kind': 'systemd', 'enable': ['NetworkManager.service', 'bluetooth.service', 'greetd.service'],
     'disable': [], 'mask': ['bootc-fetch-apply-updates.service', 'bootc-fetch-apply-updates.timer'],
     'default_target': 'graphical.target'},
    {'kind': 'initial_skel'},
    {'kind': 'qemu_initramfs', 'required_modules': ['virtio_dma_buf', 'virtio_gpu', 'virtio_input']},
]


def digest(value, label='SHA-256'):
    m.require(isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value), 'Invalid ' + label)
    return value


def image_id(value):
    m.require(isinstance(value, str) and re.fullmatch('sha256:[a-f0-9]{64}', value),
              'Invalid immutable image identity')
    return value


def engine_object_id(value):
    m.require(isinstance(value, str) and re.fullmatch(r'(?:src|out)-[a-f0-9]{64}', value),
              'Invalid engine object identity')
    return value


def ordinary(path, directory=False):
    path = Path(os.path.abspath(path))
    for parent in reversed(path.parents):
        info = parent.lstat()
        m.require(stat.S_ISDIR(info.st_mode) and not stat.S_ISLNK(info.st_mode),
                  'Input parent is not an ordinary directory: ' + str(parent))
    info = path.lstat()
    if directory:
        m.require(stat.S_ISDIR(info.st_mode), 'Not an ordinary directory: ' + str(path))
    else:
        m.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1,
                  'Not a single-link regular file: ' + str(path))
    return path


def hash_file(path, limit=MAX_FILE):
    path = ordinary(path)
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    with os.fdopen(os.open(path, flags), 'rb') as stream:
        before = os.fstat(stream.fileno())
        m.require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 < before.st_size <= limit,
                  'Input is empty, oversized or aliased: ' + str(path))
        hasher = hashlib.sha256()
        total = 0
        while block := stream.read(1024**2):
            total += len(block)
            m.require(total <= limit, 'Input grew beyond its size limit: ' + str(path))
            hasher.update(block)
        result = hasher.hexdigest()
        after = os.fstat(stream.fileno())
    named = path.lstat()
    m.require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
              == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
              and (after.st_dev, after.st_ino) == (named.st_dev, named.st_ino),
              'Input changed while hashing: ' + str(path))
    return result


def read_json(path, expected=None, limit=MAX_JSON):
    path = ordinary(path)
    before = hash_file(path, limit)
    if expected is not None:
        m.require(before == digest(expected), 'Input hash differs: ' + str(path))
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK), 'rb') as stream:
        info = os.fstat(stream.fileno())
        m.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, 'JSON input changed type')
        data = stream.read(limit + 1)
    m.require(0 < len(data) <= limit, 'JSON input exceeded its size limit')
    m.require(m.sha(data) == before, 'Input changed while reading: ' + str(path))
    return m.document(data), data


def write_new(path, data, mode=0o600):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(descriptor, 'wb') as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def write_json(path, value):
    data = m.canonical(value)
    write_new(path, data)
    return m.sha(data)


def transfer_comparison(config, observed):
    """Fixed-field metadata only; never emit image config values or unknown keys."""
    exported, inspected = config.get('config'), observed.get('Config')
    exported_ids = config.get('rootfs', {}).get('diff_ids')
    inspected_ids = observed.get('RootFS', {}).get('Layers')
    report = {'os_matches': config.get('os') == 'linux',
              'architecture_matches': config.get('architecture') == 'arm64',
              'config_matches': exported == inspected, 'diff_ids_match': exported_ids == inspected_ids,
              'config_fields': []}

    def shape(value):
        kinds = {dict: 'object', list: 'array', str: 'string', bool: 'boolean',
                 int: 'number', float: 'number', type(None): 'null'}
        return {'type': kinds.get(type(value), 'unsupported'), 'is_null': value is None,
                'is_empty': value in ('', [], {}), 'is_false': value is False}

    report['exported_config_shape'] = shape(exported)
    report['inspected_config_shape'] = shape(inspected)
    if isinstance(exported, dict) and isinstance(inspected, dict):
        for key in CONFIG_DIAGNOSTIC_KEYS:
            left, right = exported.get(key), inspected.get(key)
            report['config_fields'].append({'key': key, 'exported_present': key in exported,
                'inspected_present': key in inspected, 'equal': left == right,
                'exported': shape(left), 'inspected': shape(right)})
        for side, value in (('exported', exported), ('inspected', inspected)):
            unknown = sorted(set(value) - set(CONFIG_DIAGNOSTIC_KEYS))
            report[side + '_unknown_key_count'] = len(unknown)
            report[side + '_unknown_keys_sha256'] = m.sha(m.canonical(unknown))
    for side, values in (('exported', exported_ids), ('inspected', inspected_ids)):
        report[side + '_diff_ids_count'] = len(values) if isinstance(values, list) else None
        report[side + '_diff_ids_sha256'] = (m.sha(m.canonical(values)) if isinstance(values, list)
            and len(values) <= 1024 and all(isinstance(value, str)
            and re.fullmatch('sha256:[a-f0-9]{64}', value) for value in values) else None)
    return report


def failure_diagnostic(operation, error, progress):
    """Public metadata only; exception messages, argv and log text stay private."""
    kinds = (OSError, RuntimeError, ValueError, subprocess.SubprocessError)
    category = next((kind.__name__ for kind in kinds if isinstance(error, kind)), None)
    exception_type = type(error).__name__ if error is not None else None
    if exception_type not in ('OSError', 'FileNotFoundError', 'PermissionError', 'InterruptedError',
                              'RuntimeError', 'ValueError', 'TypeError', 'KeyError', 'AttributeError',
                              'IndexError', 'TimeoutExpired', 'CalledProcessError', 'KeyboardInterrupt'):
        exception_type = None
    locations = []
    trace = error.__traceback__ if error is not None else None
    while trace is not None:
        filename = Path(trace.tb_frame.f_code.co_filename).name
        if filename in ('compose.py', 'material.py', 'candidate.py'):
            locations.append({'source': filename, 'line': trace.tb_lineno})
        trace = trace.tb_next
    logs = []
    markers = {'no_space': b'No space left on device', 'manifest_unknown': b'manifest unknown',
               'unexpected_eof': b'unexpected EOF', 'connection_reset': b'Connection reset by peer'}
    for stream, path in progress.get('logs', ()):
        item = {'stream': stream, 'available': False}
        try:
            flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
            with os.fdopen(os.open(path, flags), 'rb') as log:
                before = os.fstat(log.fileno())
                if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
                    raise ValueError('not an ordinary log')
                data = log.read(16 * 1024**2)
                after = os.fstat(log.fileno())
            item.update(available=True, size_bytes=before.st_size, hashed_bytes=len(data),
                        sha256=hashlib.sha256(data).hexdigest(),
                        complete=len(data) == before.st_size == after.st_size
                        and before.st_mtime_ns == after.st_mtime_ns,
                        observed_markers=[name for name, marker in markers.items() if marker in data])
        except (OSError, ValueError):
            pass
        logs.append(item)
    # Step names and tool basenames come only from fixed in-process call sites.
    step = progress.get('step')
    tool = progress.get('tool')
    return {'schema_version': 1, 'operation': operation,
            'last_step': step if step is None or re.fullmatch('[a-z-]{1,64}', step) else 'unknown',
            'tool': tool if tool in ('git', 'docker', 'sudo', 'skopeo', 'sysroot',
                                    'sysroot-helper', 'kedra-lab', 'uv') else None,
            'exit_code': progress.get('exit_code'), 'exception_category': category,
            'exception_type': exception_type,
            'os_errno': error.errno if isinstance(error, OSError) else None,
            'locations': locations[-4:], 'logs': logs,
            'transfer_comparison': progress.get('transfer_comparison')}


def publish_failure(path, operation, error, progress):
    if path is None:
        return
    try:
        ordinary(path.parent, directory=True)
        write_json(path, failure_diagnostic(operation, error, progress))
    except (OSError, RuntimeError, ValueError, TypeError, MemoryError):
        # Diagnostics must neither mask the original failure nor expose its text.
        print('compose: public failure diagnostic unavailable', file=sys.stderr)


def copy_pinned(source, destination, expected, mode=0o600):
    source = ordinary(source)
    m.require(hash_file(source) == digest(expected), 'Pinned input differs: ' + str(source))
    source_descriptor = os.open(source, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(source_descriptor, 'rb') as origin:
        info = os.fstat(origin.fileno())
        m.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, 'Snapshot input changed type')
        descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
        with os.fdopen(descriptor, 'wb') as target:
            total = 0
            while block := origin.read(1024**2):
                total += len(block)
                m.require(total <= MAX_FILE, 'Input grew beyond its size limit')
                target.write(block)
            target.flush()
            os.fsync(target.fileno())
    m.require(hash_file(destination) == expected and hash_file(source) == expected,
              'Pinned input changed during snapshot: ' + str(source))


def private_directory(path, create=False):
    path = Path(os.path.abspath(path))
    ordinary(path.parent, directory=True)
    if create:
        path.mkdir(mode=0o700)
    path = ordinary(path, directory=True)
    info = path.stat()
    m.require(info.st_uid == os.getuid() and stat.S_IMODE(info.st_mode) == 0o700,
              'Directory must be owner-private: ' + str(path))
    return path


def package_bytes(rows):
    return ('\n'.join('\t'.join(row) for row in rows) + '\n').encode()


def validate_inputs(value, data):
    m.require(isinstance(value, dict)
              and set(value) == {'schema_version', 'base', 'source', 'artifacts', 'recipes', 'packages'}
              and type(value['schema_version']) is int and value['schema_version'] == 1
              and m.canonical(value) == data, 'Unsupported or noncanonical resolved inputs')
    m.require(isinstance(value['base'], str)
              and re.fullmatch(r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}', value['base']),
              'Foundation must use an immutable reviewed Fedora bootc base')
    for category in ('artifacts', 'recipes'):
        m.require(isinstance(value[category], dict) and 0 < len(value[category]) <= 256,
                  'Invalid deterministic ' + category)
        for key, expected in value[category].items():
            m.require(isinstance(key, str) and 0 < len(key) <= 512 and not any(ord(c) < 32 for c in key),
                      'Invalid material input name')
            digest(expected)
    rows = value['packages']
    m.require(isinstance(rows, list) and rows and all(isinstance(row, list) and len(row) == 7
              and all(isinstance(field, str) for field in row) for row in rows), 'Invalid RPM material')
    m.require(m.packages(package_bytes(rows), 'aarch64') == rows, 'Noncanonical RPM material')
    m.require_bootc(rows, 'aarch64')


class Runner:
    def __init__(self, args, progress):
        self.progress = progress
        m.require(sys.platform == 'linux' and platform.machine() == 'aarch64'
                  and os.getuid() != 0 and os.geteuid() == os.getuid(),
                  'Run as an ordinary user on native Linux ARM; this does not install an OS')
        m.require(re.fullmatch('[a-f0-9]{40}', args.source_revision), 'Invalid committed source revision')
        self.args = args
        self.repo = ordinary(args.repo, directory=True)
        self.store = Path(os.path.abspath(args.store))
        ordinary(self.store.parent, directory=True)
        self.inputs, input_bytes = read_json(args.inputs, args.expected_inputs_sha256, m.LIMIT)
        validate_inputs(self.inputs, input_bytes)
        self.input_hash = m.sha(input_bytes)
        self.output = private_directory(args.output_dir, create=True)
        self.sequence = 0
        self.nonce = uuid.uuid4().hex
        self.programs = {}
        self.engine = None
        self.docker_socket = Path('/var/run/docker.sock').resolve(strict=True)
        m.require(stat.S_ISSOCK(self.docker_socket.stat().st_mode), 'Docker endpoint is not a local socket')
        for name in ('tools', 'home', 'logs'):
            private_directory(self.output / name, create=True)
        private_directory(self.output / 'home/.docker', create=True)
        artifacts = self.store.with_name(self.store.name + '.release-lab')
        private_directory(artifacts, create=not artifacts.exists())
        self.environment = {
            'PATH': '/usr/bin:/bin', 'HOME': str(self.output / 'home'), 'LC_ALL': 'C',
            'DOCKER_HOST': 'unix://' + str(self.docker_socket),
            'DOCKER_CONFIG': str(self.output / 'home/.docker'),
            'KEDRA_LAB_ARTIFACTS': str(artifacts),
            'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null',
            'GIT_TERMINAL_PROMPT': '0', 'GIT_OPTIONAL_LOCKS': '0',
            'BUILDX_NO_DEFAULT_ATTESTATIONS': '1',
        }
        write_new(self.output / 'resolved-inputs.json', input_bytes)
        for name in PROGRAMS:
            m.require(name in self.inputs['artifacts'], 'Missing pinned binary: ' + name)
            destination = self.output / 'tools' / name
            copy_pinned(args.binaries / name, destination, self.inputs['artifacts'][name], 0o500)
            self.programs[name] = destination
        self.check_source()
        self.check_recipes()
        self.source = self.command_json('source-plan', [self.programs['sysroot'], 'source', 'plan',
            '--repo', self.repo, '--host', args.target, '--json'])
        self.check_source_manifest(self.source)
        self.engine = self.command_json('docker-info', ['/usr/bin/docker', 'info', '--format', '{{json .}}'])
        m.require(isinstance(self.engine, dict) and self.engine.get('OSType') == 'linux'
                  and self.engine.get('Architecture') in ('aarch64', 'arm64')
                  and isinstance(self.engine.get('ID'), str) and self.engine['ID'],
                  'Docker must be an identified native Linux ARM engine')
        self.engine = self.engine['ID']

    def run(self, label, arguments, timeout=INSPECT_TIMEOUT, maximum=MAX_JSON):
        try:
            return self.execute(label, arguments, timeout, maximum)
        except Exception as error:
            # A finally block may run more commands to clean an observer. Keep
            # the first failing command's evidence before cleanup changes progress.
            if 'failure' not in self.progress:
                self.progress['failure'] = (error, dict(self.progress))
            raise

    def execute(self, label, arguments, timeout, maximum):
        self.sequence += 1
        prefix = self.output / 'logs' / f'{self.sequence:03d}-{label}'
        stdout, stderr = prefix.with_suffix('.stdout'), prefix.with_suffix('.stderr')
        program = str(arguments[0])
        self.progress.update(step=label, tool=Path(program).name, exit_code=None,
                             logs=(('stdout', stdout), ('stderr', stderr)))
        m.require(Path(program).is_absolute(), 'Executable must be an absolute pinned path')
        for name, path in self.programs.items():
            if program == str(path):
                m.require(hash_file(path) == self.inputs['artifacts'][name], 'Pinned executable changed')
        print('compose: ' + label, file=sys.stderr, flush=True)
        with stdout.open('xb') as out, stderr.open('xb') as err:
            process = subprocess.Popen([str(arg) for arg in arguments], cwd=self.repo,
                                       env=self.environment, stdin=subprocess.DEVNULL,
                                       stdout=out, stderr=err, start_new_session=True)
            deadline = time.monotonic() + timeout
            try:
                while process.poll() is None:
                    m.require(time.monotonic() < deadline, label + ' exceeded its deadline')
                    m.require(stdout.stat().st_size <= maximum and stderr.stat().st_size <= 16 * 1024**2,
                              label + ' exceeded its output limit')
                    time.sleep(0.1)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=5)
                self.progress['exit_code'] = process.returncode
        m.require(process.returncode == 0, f'{label} failed ({process.returncode}); see {stderr}')
        m.require(stdout.stat().st_size <= maximum and stderr.stat().st_size <= 16 * 1024**2,
                  label + ' exceeded its output limit')
        for name, path in self.programs.items():
            if program == str(path):
                m.require(hash_file(path) == self.inputs['artifacts'][name], 'Pinned executable changed')
        return stdout.read_bytes()

    def command_json(self, label, arguments, timeout=INSPECT_TIMEOUT):
        return m.document(self.run(label, arguments, timeout))

    def check_source(self):
        current = self.run('source-head', ['/usr/bin/git', '--no-replace-objects', '-C', self.repo,
                           'rev-parse', '--verify', 'HEAD^{commit}']).decode().strip()
        m.require(current == self.args.source_revision, 'Committed HEAD differs from selected source')

    def check_recipes(self):
        required = {IMAGE + 'Containerfile', IMAGE + 'assemble.sh', NATIVE_RECIPE,
                    IMAGE + 'release/compose.py', IMAGE + 'release/material.py'}
        m.require(required <= self.inputs['recipes'].keys(), 'Preflight lacks composition recipes')
        for name, expected in self.inputs['recipes'].items():
            path = Path(name)
            m.require(not path.is_absolute() and all(part not in ('', '.', '..', '.git') for part in name.split('/')),
                      'Recipe must name an ordinary repository file')
            actual = ordinary(self.repo / path)
            data = m.read(actual, MAX_JSON)
            committed = self.run('committed-recipe', ['/usr/bin/git', '--no-replace-objects', '-C', self.repo,
                'cat-file', 'blob', self.args.source_revision + ':' + name])
            m.require(data == committed, 'Recipe differs from committed source: ' + name)
            if name == NATIVE_RECIPE:
                steps = m.document(data)
                m.require(isinstance(steps, dict) and type(steps.get('schema_version')) is int
                          and steps == {'schema_version': 1, 'steps': NATIVE_STEPS},
                          'Unsupported native step policy')
                data = m.canonical(steps)
            m.require(m.sha(data) == expected, 'Recipe differs from preflight: ' + name)
        m.require(Path(__file__).resolve() == self.repo / (IMAGE + 'release/compose.py'),
                  'Invoke the runner from the selected checkout')

    def check_source_manifest(self, source):
        m.require(isinstance(source, dict) and source.get('schema_version') == 1
                  and source.get('source_revision') == self.args.source_revision
                  and source.get('target', {}).get('id') == 'qemu-arm64'
                  and source['target'].get('architecture') == 'aarch64'
                  and source['target'].get('fedora_release') == 44
                  and source['target'].get('image') == 'ghcr.io/reidond/kedra-qemu-arm64'
                  and source['target'].get('candidate_target') is True
                  and {key: value for key, value in source.items() if key not in ('source_revision', 'input_scope')}
                  == self.inputs['source'], 'Source manifest differs from selected material')

    def same_engine(self):
        observed = self.run('engine-identity', ['/usr/bin/docker', 'info', '--format', '{{.ID}}']).decode().strip()
        m.require(observed == self.engine, 'Docker engine identity changed')

    def inspect_image(self, reference):
        values = self.command_json('image-inspect', ['/usr/bin/docker', 'image', 'inspect', reference])
        m.require(isinstance(values, list) and len(values) == 1, 'Expected one local image')
        value = values[0]
        image_id(value.get('Id'))
        m.require(value.get('Os') == 'linux' and value.get('Architecture') == 'arm64', 'Wrong candidate platform')
        config = value.get('Config') or {}
        m.require(not config.get('Volumes') and not config.get('OnBuild'), 'Image declares volumes or ONBUILD')
        return value

    def image_files(self, image, paths, label):
        self.same_engine()
        directory = private_directory(self.output / label, create=True)
        name = 'kedra-compose-inspect-' + uuid.uuid4().hex
        container = self.run('create-observer', ['/usr/bin/docker', 'container', 'create', '--name', name,
            '--label', 'dev.kedra.release.invocation=' + self.nonce, '--network', 'none', '--read-only',
            '--user', '65534:65534', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
            '--entrypoint', '/usr/bin/false', image_id(image)]).decode().strip()
        m.require(re.fullmatch('[a-f0-9]{64}', container), 'Unexpected stopped container identity')
        try:
            for path in paths:
                destination = directory / Path(path).name
                self.run('copy-material', ['/usr/bin/docker', 'container', 'cp', container + ':' + path, destination])
                ordinary(destination)
        finally:
            self.run('remove-observer', ['/usr/bin/docker', 'container', 'rm', container])
        return directory

    def observed_material(self, image, label, native=False):
        paths = [SOURCE_PATH, RPM_PATH] + ([NATIVE_PATH] if native else [])
        directory = self.image_files(image, paths, label)
        source, source_bytes = read_json(directory / 'source.json', limit=1024**2)
        self.check_source_manifest(source)
        m.require(source == self.source, 'Image source differs from the current committed source plan')
        rows = m.packages(m.read(ordinary(directory / 'package-material.txt')), 'aarch64')
        m.require(rows == self.inputs['packages'], 'Actual image RPM material differs from preflight')
        return directory, m.sha(source_bytes), m.sha(package_bytes(rows))

    def retain(self, image):
        if not self.store.exists():
            self.command_json('store-init', [self.programs['sysroot'], 'store', 'init', '--store', self.store])
        retained = self.command_json('retain-image', [self.programs['sysroot'], 'store', 'add-image',
            '--store', self.store, '--image', image_id(image)], BUILD_TIMEOUT)
        m.require(set(retained) == {'schema', 'image', 'platform', 'sha256', 'bytes'}
                  and retained['schema'] == 1 and retained['image'] == image
                  and retained['platform'] == ENGINE_PLATFORM and type(retained['bytes']) is int
                  and 0 < retained['bytes'] <= MAX_FILE, 'Invalid complete-image retention receipt')
        digest(retained['sha256'])
        return retained

    def foundation(self):
        context = ordinary(self.args.context, directory=True)
        m.require({path.name for path in context.iterdir()} == CONTEXT_FILES, 'Unexpected foundation context entries')
        snapshot = private_directory(self.output / 'context', create=True)
        for name in sorted(CONTEXT_FILES - {'payload.tar'}):
            expected = (self.inputs['recipes'][IMAGE + name] if name in ('Containerfile', 'assemble.sh')
                        else self.inputs['artifacts'].get(name))
            copy_pinned(context / name, snapshot / name, expected, 0o500 if name in PROGRAMS else 0o600)
        self.run('source-archive', [self.programs['sysroot'], 'source', 'archive', '--repo', self.repo,
            '--host', 'qemu-arm64', '--output', snapshot / 'payload.tar'], BUILD_TIMEOUT)
        m.require(hash_file(snapshot / 'payload.tar') == hash_file(context / 'payload.tar'),
                  'Context payload differs from freshly frozen committed source')
        tag = 'localhost/kedra-compose-foundation:' + self.nonce
        write_json(self.output / 'resources.json', {'schema_version': 1, 'engine': self.engine,
                   'invocation': self.nonce, 'foundation_tag': tag, 'store': str(self.store)})
        self.run('foundation-build', ['/usr/bin/docker', 'build', '--platform', OCI_PLATFORM, '--pull',
            '--no-cache', '--iidfile', self.output / 'foundation.iid', '--tag', tag,
            '--label', 'dev.kedra.release.invocation=' + self.nonce,
            '--build-arg', 'BASE_IMAGE=' + self.inputs['base'],
            '--build-arg', 'KEDRA_ASSEMBLY_MODE=foundation', '--file', snapshot / 'Containerfile', snapshot],
            BUILD_TIMEOUT)
        image = image_id(m.read(ordinary(self.output / 'foundation.iid'), 128).decode().strip())
        observed = self.inspect_image(tag)
        m.require(observed['Id'] == image
                  and observed.get('Config', {}).get('Labels', {}).get('dev.kedra.release.invocation') == self.nonce,
                  'Foundation build result differs from its owned tag')
        retained = self.retain(image)
        retention_hash = write_json(self.output / 'retention.json', retained)
        _, source_hash, rpm_hash = self.observed_material(image, 'foundation-material')
        self.finish()
        result = {'schema_version': 1, 'target': 'qemu-arm64', 'source_revision': self.args.source_revision,
                  'input_material_sha256': self.input_hash, 'foundation_image': image, 'engine': self.engine,
                  'rpm_sha256': rpm_hash, 'source_manifest_sha256': source_hash,
                  'retention_receipt_sha256': retention_hash}
        write_json(self.output / 'foundation.json', result)
        return result

    def contribution(self, foundation):
        if self.args.definition is None:
            return None, {}
        definition, data = read_json(self.args.definition, self.args.expected_definition_sha256)
        m.require(isinstance(definition, dict), 'Contribution definition must be a typed object')
        receipt, _ = read_json(self.args.contribution_receipt, self.args.expected_contribution_receipt_sha256)
        m.require(isinstance(receipt, dict) and set(receipt) == CONTRIBUTION_FIELDS
                  and type(receipt['schema_version']) is int and receipt['schema_version'] == 1
                  and receipt['source_revision'] == self.args.source_revision
                  and receipt['foundation_image'] == foundation and receipt['input_material_sha256'] == self.input_hash
                  and receipt['definition_sha256'] == m.sha(data), 'Contribution receipt differs from selected build')
        for key, artifact in (('catalog_pins_sha256', 'catalog-pins'), ('author_sha256', 'catalog-author')):
            m.require(digest(receipt[key]) == self.inputs['artifacts'].get(artifact),
                      'Contribution differs from preflight ' + artifact)
        outputs = definition.get('outputs')
        m.require(type(definition.get('schema')) is int and definition.get('schema') == 1
                  and definition.get('platform') == ENGINE_PLATFORM
                  and definition.get('foundation') == foundation and isinstance(outputs, dict) and outputs
                  and outputs == receipt['outputs'], 'Contribution definition/foundation/output differs')
        for alias, identity in outputs.items():
            m.require(re.fullmatch('[A-Za-z0-9_-]{1,128}', alias), 'Invalid output alias')
            engine_object_id(identity)
        path = self.output / 'definition.json'
        write_new(path, data)
        return path, outputs

    def compose(self):
        foundation, _ = read_json(self.args.foundation_receipt, self.args.expected_foundation_receipt_sha256)
        m.require(isinstance(foundation, dict) and set(foundation) == FOUNDATION_FIELDS
                  and type(foundation['schema_version']) is int and foundation['schema_version'] == 1
                  and foundation['target'] == 'qemu-arm64' and foundation['source_revision'] == self.args.source_revision
                  and foundation['input_material_sha256'] == self.input_hash and foundation['engine'] == self.engine,
                  'Foundation receipt differs from selected build')
        image = image_id(foundation['foundation_image'])
        self.inspect_image(image)
        retained = self.retain(image)
        m.require(m.sha(m.canonical(retained)) == digest(foundation['retention_receipt_sha256']),
                  'Foundation retention differs from independently selected receipt')
        _, source_hash, rpm_hash = self.observed_material(image, 'foundation-material')
        m.require(source_hash == digest(foundation['source_manifest_sha256'])
                  and rpm_hash == digest(foundation['rpm_sha256']), 'Foundation material changed')
        definition, outputs = self.contribution(image)
        context = self.output / 'composition'
        arguments = [self.programs['sysroot'], 'system', 'compose', '--repo', self.repo,
                     '--target', 'qemu-arm64', '--store', self.store, '--foundation', image,
                     '--output-dir', context]
        if definition is not None:
            arguments.extend(['--definition', definition])
        composed = self.command_json('system-compose', arguments, BUILD_TIMEOUT)
        identity = digest(composed.get('identity'), 'composition identity')
        m.require(composed.get('plan', {}).get('definition', {}).get('outputs') == outputs,
                  'Composed output aliases differ from selected contribution')
        verification = private_directory(self.output / 'verification', create=True)
        self.command_json('verify-composition', [self.programs['sysroot'], 'system', 'verify',
            '--context', context, '--expected-identity', identity, '--workdir', verification],
            BUILD_TIMEOUT)
        native = self.output / 'native-definition.json'
        write_json(native, {'schema': 1, 'parent_identity': identity, 'steps': NATIVE_STEPS})
        common = ['--target', 'qemu-arm64', '--image', 'composition:' + str(context), '--overlay', 'none',
                  '--composition-identity', identity, '--native-plan', native]
        plan = self.command_json('native-plan', [self.programs['kedra-lab'], 'derive-plan', *common], BUILD_TIMEOUT)
        native_identity = digest(plan.get('identity'), 'native identity')
        write_json(self.output / 'native-plan.json', plan)
        derived = self.command_json('native-derive', [self.programs['kedra-lab'], 'derive', *common,
            '--derivation-identity', native_identity], BUILD_TIMEOUT)
        m.require(set(derived) == {'image', 'engine', 'parent_image', 'material'} and derived['engine'] == self.engine,
                  'Unexpected derived image receipt')
        generated = image_id(derived['image'])
        directory, final_source, final_rpm = self.observed_material(generated, 'native-material', native=True)
        actual, raw_receipt = read_json(directory / 'native-receipt.json')
        m.require(actual == derived['material'] and actual.get('identity') == native_identity
                  and actual.get('parent_identity') == identity and actual.get('foundation_image') == image
                  and actual.get('rpm_sha256') == rpm_hash and final_source == source_hash and final_rpm == rpm_hash,
                  'Native installed receipt/material differs from independently selected plan')
        self.retain(generated)
        transfer = self.transfer(generated)
        self.finish()
        result = {'schema_version': 1, 'target': 'qemu-arm64', 'source_revision': self.args.source_revision,
                  'input_material_sha256': self.input_hash, 'foundation_image': image, 'engine': self.engine,
                  'composition_identity': identity, 'native_identity': native_identity, 'native_image': generated,
                  'native_receipt_sha256': m.sha(raw_receipt), 'source_manifest_sha256': source_hash,
                  'rpm_sha256': rpm_hash, 'outputs': outputs, 'transfer': transfer}
        write_json(self.output / 'candidate.json', result)
        return result

    def transfer(self, image):
        self.same_engine()
        tag = 'localhost/kedra-compose:' + self.nonce
        m.require(not self.run('docker-transfer-absence', ['/usr/bin/docker', 'image', 'ls',
                  '--filter', 'reference=' + tag, '--format', '{{.ID}}']).strip(),
                  'Owned Docker transfer name already exists')
        m.require(not self.run('podman-transfer-absence', ['/usr/bin/sudo', '--non-interactive',
                  '/usr/bin/podman', 'image', 'list', '--filter', 'reference=' + tag,
                  '--format', '{{.ID}}']).strip(), 'Owned Podman transfer name already exists')
        write_json(self.output / 'resources.json', {'schema_version': 1, 'engine': self.engine,
                   'invocation': self.nonce, 'docker_transfer_tag': tag, 'podman_transfer_tag': tag})
        self.run('tag-transfer', ['/usr/bin/docker', 'image', 'tag', image, tag])
        observed = self.inspect_image(tag)
        m.require(observed['Id'] == image, 'Native transfer tag changed')
        archive = self.output / 'native-image.tar'
        self.run('export-transfer', ['/usr/bin/docker', 'image', 'save', '--output', archive, tag], BUILD_TIMEOUT)
        archive_hash = hash_file(archive)
        m.require(self.inspect_image(tag)['Id'] == image, 'Transfer image changed during export')
        source = 'docker-archive:' + str(archive) + ':' + tag
        source_config = self.run('archive-config', ['/usr/bin/skopeo', 'inspect', '--config', '--raw', source])
        source_manifest = self.run('archive-manifest', ['/usr/bin/skopeo', 'inspect', '--raw', source])
        config = m.document(source_config)
        # A diagnostic error must not change the existing strict admission result.
        try:
            self.progress['transfer_comparison'] = transfer_comparison(config, observed)
        except (TypeError, ValueError, KeyError, AttributeError, MemoryError):
            self.progress['transfer_comparison'] = {'available': False}
        m.require(config.get('os') == 'linux' and config.get('architecture') == 'arm64'
                  and config.get('config') == observed.get('Config')
                  and config.get('rootfs', {}).get('diff_ids') == observed.get('RootFS', {}).get('Layers'),
                  'Exported image differs from native filesystem lineage')
        # Loading verified local bytes is not a registry pull and installs no policy.
        self.run('load-transfer', ['/usr/bin/sudo', '--non-interactive', '/usr/bin/podman',
            'image', 'load', '--input', archive], BUILD_TIMEOUT)
        destination = 'containers-storage:' + tag
        target_config = self.run('podman-config', ['/usr/bin/sudo', '--non-interactive', '/usr/bin/skopeo',
            'inspect', '--config', '--raw', destination])
        target_manifest = self.run('podman-manifest', ['/usr/bin/sudo', '--non-interactive', '/usr/bin/skopeo',
            'inspect', '--raw', destination])
        m.require(source_config == target_config and hash_file(archive) == archive_hash,
                  'Transfer changed image config or archive bytes')
        loaded = self.command_json('podman-image', ['/usr/bin/sudo', '--non-interactive',
            '/usr/bin/podman', 'image', 'inspect', tag])
        m.require(isinstance(loaded, list) and len(loaded) == 1
                  and loaded[0].get('Os') == 'linux' and loaded[0].get('Architecture') == 'arm64',
                  'Loaded image has an unexpected platform')
        loaded_id = loaded[0].get('Id', '')
        if not loaded_id.startswith('sha256:'):
            loaded_id = 'sha256:' + loaded_id
        image_id(loaded_id)
        m.require(loaded_id == 'sha256:' + m.sha(target_config),
                  'Loaded image identity differs from its verified config digest')
        return {'archive_sha256': archive_hash, 'source_manifest_digest': 'sha256:' + m.sha(source_manifest),
                'destination_manifest_digest': 'sha256:' + m.sha(target_manifest),
                'config_digest': 'sha256:' + m.sha(target_config), 'image': loaded_id, 'reference': tag}

    def finish(self):
        self.same_engine()
        self.check_source()
        self.check_recipes()
        for name, path in self.programs.items():
            m.require(hash_file(path) == self.inputs['artifacts'][name], 'Pinned executable changed before result')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='operation', required=True)
    for operation in ('foundation', 'compose'):
        command = sub.add_parser(operation)
        command.add_argument('--target', required=True, choices=('qemu-arm64',))
        command.add_argument('--repo', required=True, type=Path)
        command.add_argument('--source-revision', required=True)
        command.add_argument('--inputs', required=True, type=Path)
        command.add_argument('--expected-inputs-sha256', required=True)
        command.add_argument('--store', required=True, type=Path)
        command.add_argument('--binaries', required=True, type=Path)
        command.add_argument('--output-dir', required=True, type=Path)
        command.add_argument('--diagnostic-output', type=Path,
                             help='New public metadata-only failure receipt; raw logs stay private')
        if operation == 'foundation':
            command.add_argument('--context', required=True, type=Path)
        else:
            command.add_argument('--foundation-receipt', required=True, type=Path)
            command.add_argument('--expected-foundation-receipt-sha256', required=True)
            command.add_argument('--definition', type=Path)
            command.add_argument('--expected-definition-sha256')
            command.add_argument('--contribution-receipt', type=Path)
            command.add_argument('--expected-contribution-receipt-sha256')
    result = parser.parse_args()
    if result.operation == 'compose':
        values = (result.definition, result.expected_definition_sha256,
                  result.contribution_receipt, result.expected_contribution_receipt_sha256)
        if any(value is not None for value in values) and not all(value is not None for value in values):
            parser.error('the definition and contribution receipt require both independently selected hashes')
    return result


def main():
    args = arguments()
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    progress = {}
    try:
        runner = Runner(args, progress)
        result = runner.foundation() if runner.args.operation == 'foundation' else runner.compose()
    except Exception as error:
        failed_error, failed_progress = progress.get('failure', (error, progress))
        publish_failure(args.diagnostic_output, args.operation, failed_error, failed_progress)
        raise
    print(json.dumps(result, sort_keys=True))


def interrupted(number, _frame):
    raise InterruptedError('Interrupted by signal ' + str(number))


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        print('compose: ' + str(error), file=sys.stderr)
        raise SystemExit(1) from error
