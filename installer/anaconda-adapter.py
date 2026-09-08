"""Narrow, version-guarded Fedora Anaconda 44.30-2.fc44 media adaptations."""
import ast
import hashlib
import pathlib
import py_compile
import importlib.util
from types import SimpleNamespace

if not pathlib.Path('/run/.containerenv').is_file():
    raise SystemExit('Apply only inside the disposable installer image build')
spec = importlib.util.find_spec('pyanaconda')
if spec is None or not spec.submodule_search_locations or len(spec.submodule_search_locations) != 1:
    raise SystemExit('Expected one installed Anaconda package')
root = pathlib.Path(spec.submodule_search_locations[0])
patches = [
    (
        'modules/payloads/source/bootc/bootc.py',
        '8fd2f71d213e18857a212a1c4062e892e470ce77b65ab35020c48846756f99ae',
        '        :return: True or False\n        """\n        return True\n',
        '        :return: True or False\n        """\n        # Kedra: an embedded local container does not need a network.\n        ref = self.configuration.sourceImgRef\n        return not (ref and ref.startswith("containers-storage:"))\n',
    ),
    (
        'modules/users/users.py',
        'a8864d42390e9e213908dc0ca52ce370422309144646699369f299e3a5c75c19',
        '        # any root set from kickstart is fine\n        if self._rootpw_seen:\n            return True\n',
        '        # Kedra: a deliberately locked root is not an accessible admin.\n        if self._rootpw_seen and not self.root_account_locked:\n            return True\n',
    ),
    (
        'modules/payloads/payload/rpm_ostree/installation.py',
        '614ac3f3061d959144e0a2e80919012c7254d44b1fab04daea35b2bef52f3f86',
        '''        try:
            self.report_progress(_("Deploying image..."))
            for line in execReadlines("bootc", bootc_args):
                self._parse_bootc_output(line)
        except OSError as e:
            raise PayloadInstallationError(
                "bootc installation failed: {}".format(str(e))
            ) from e
''',
        '''        self.report_progress(_("Deploying image..."))
        # Kedra: large local-store layers must use the selected disk for scratch.
        _kedra_bootc_deploy(self._physroot, bootc_args, self._parse_bootc_output)
''',
    ),
]
updated = {}
for relative, expected, before, after in patches:
    path = root / relative
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected:
        raise SystemExit(f'Anaconda source changed; review adapter before rebuilding: {relative}')
    source = raw.decode()
    if source.count(before) != 1:
        raise SystemExit(f'Anaconda patch anchor is not unique: {relative}')
    updated[relative] = source.replace(before, after)

updated[patches[2][0]] += '''

def _kedra_bootc_deploy(physroot, bootc_args, report):
    """Use only the already-selected/mounted target for transient image layers.

    Added by Kedra after the native root cleanup and deliberate storage approval.
    A self-bind keeps the scratch directory acceptable to bootc's empty-root check.
    The /var/tmp bind handles containers/image's big-file path without relying on
    TMPDIR or changing any bootc/signature argument. Mount failures are fatal.
    """
    import errno
    import tempfile

    if not os.path.ismount(physroot) or os.path.realpath(physroot) == "/":
        raise PayloadInstallationError("Kedra scratch requires a separate mounted installation root")
    scratch = tempfile.mkdtemp(prefix=".kedra-install-", dir=physroot)
    mounted = []
    try:
        safe_exec_program("mount", ["--bind", scratch, scratch])
        mounted.append(scratch)
        safe_exec_program("mount", ["--bind", scratch, "/var/tmp"])
        mounted.append("/var/tmp")
        try:
            for line in execReadlines("bootc", bootc_args):
                report(line)
        except OSError as error:
            raise PayloadInstallationError("bootc installation failed: {}".format(error)) from error
    finally:
        # Do not force-unmount or recursively erase unexpected state. If a mount
        # remains busy, leave it intact and report failure for explicit recovery.
        for mountpoint in reversed(mounted):
            safe_exec_program("umount", [mountpoint])
        try:
            os.rmdir(scratch)
        except OSError as error:
            if error.errno != errno.EROFS:
                raise
            # bootc finalizes by remounting the target read-only. Anaconda's
            # following PrepareBootcMountTargetsTask remounts this same target
            # read-write for account/configuration setup; do that for cleanup too.
            safe_exec_program("mount", ["-o", "remount,rw", physroot])
            os.rmdir(scratch)
'''

# The bootc task currently omits the separate mounts which its OSTree sibling
# binds after /var. Without these, useradd writes behind a later /home mount.
before_mounts = '''        # Create /var subdirectories (roothome and home) after bind mount
        self._fill_var_subdirectories()

        # Make sure /boot is accessible during %post scripts
'''
after_mounts = '''        # Create /var subdirectories (roothome and home) after bind mount
        self._fill_var_subdirectories()

        # Kedra: preserve selected separate filesystems while creating accounts.
        for mount in sorted(mount_points, key=len):
            if mount in ('/', '/var', '/dev', '/proc', '/run', '/sys'):
                continue
            self._setup_internal_bindmount(mount, recurse=False)

        # Make sure /boot is accessible during %post scripts
'''
if updated[patches[2][0]].count(before_mounts) != 1:
    raise SystemExit('Expected one native bootc mount-preparation anchor')
updated[patches[2][0]] = updated[patches[2][0]].replace(before_mounts, after_mounts)

before_cleanup = '''            # Try to unmount if it's a mount point, use lazy unmount for busy mounts
            if os.path.ismount(path):
                # Skip if /boot
                if path == self._physroot + "/boot":
                    log.debug("Bootc workaround: skip unmounting /boot")
                    continue
                safe_exec_program("umount", ["-l", path])
'''
after_cleanup = '''            # Kedra: bootc 1.16.10 accepts mounted children. Preserve the
            # selected /home and /var filesystems for later native bind setup.
            if os.path.ismount(path):
                continue
'''
if updated[patches[2][0]].count(before_cleanup) != 1:
    raise SystemExit('Expected one native physical-root cleanup anchor')
updated[patches[2][0]] = updated[patches[2][0]].replace(before_cleanup, after_cleanup)
before_root = '        log.debug("Bootc workaround: prepare clean root partition for bootc install")\n'
after_root = '''        if not os.path.ismount(self._physroot) or os.path.realpath(self._physroot) == "/":
            raise PayloadInstallationError("Kedra requires a separate mounted physical root")
        log.debug("Bootc workaround: prepare clean root partition for bootc install")
'''
if updated[patches[2][0]].count(before_root) != 1:
    raise SystemExit('Expected one native physical-root guard anchor')
updated[patches[2][0]] = updated[patches[2][0]].replace(before_root, after_root)

def getter(source, name):
    candidates = [node for node in ast.walk(ast.parse(source)) if isinstance(node, ast.FunctionDef) and node.name == name]
    if len(candidates) != 1:
        raise SystemExit('Expected one native property getter')
    function = candidates[0]
    function.decorator_list = []
    module = ast.fix_missing_locations(ast.Module(body=[function], type_ignores=[]))
    namespace = {}
    exec(compile(module, '<verified Anaconda property>', 'exec'), namespace)
    return namespace[name]

network = getter(updated[patches[0][0]], 'network_required')
for ref, expected in [(None, True), ('', True), ('registry:example.invalid/image', True),
                      ('oci-archive:/unqualified', True), ('containers-storage:localhost/kedra:fixture', False)]:
    actual = network(SimpleNamespace(configuration=SimpleNamespace(sourceImgRef=ref)))
    if actual is not expected:
        raise SystemExit('Native source network classification failed')
admin = getter(updated[patches[1][0]], 'check_admin_user_exists')
for seen, locked, password, users, expected in [
    (True, True, '', [], False),
    (True, False, 'synthetic', [], True),
    (False, True, '', [SimpleNamespace(lock=False, groups=['wheel'])], True),
    (True, True, '', [SimpleNamespace(lock=False, groups=['wheel'])], True),
    (True, True, '', [SimpleNamespace(lock=False, groups=[])], False),
    (True, True, '', [SimpleNamespace(lock=True, groups=['wheel'])], False),
]:
    actual = admin(SimpleNamespace(_rootpw_seen=seen, root_account_locked=locked, root_password=password, users=users))
    if actual is not expected:
        raise SystemExit('Native accessible-administrator classification failed')

# Exercise the exact injected function with inert mount/process substitutes.
# These checks never mount anything in the image builder or its host.
from unittest.mock import patch
deploy = getter(updated[patches[2][0]], '_kedra_bootc_deploy')
for failure in [None, 'first-bind', 'second-bind', 'bootc', 'unmount', 'read-only', 'remount']:
    calls = []
    scratch = '/synthetic-target/.kedra-install-fixture'
    arguments = ['install', 'to-filesystem', '--source-imgref=containers-storage:fixture', '/synthetic-target']
    def execute(command, args):
        calls.append((command, tuple(args)))
        if (failure == 'first-bind' and len(calls) == 1
                or failure == 'second-bind' and len(calls) == 2
                or failure == 'unmount' and command == 'umount'
                or failure == 'remount' and command == 'mount' and args[0] == '-o'):
            raise RuntimeError('synthetic mount failure')
    def remove(path):
        calls.append(('rmdir', (path,)))
        if failure in ['read-only', 'remount'] and sum(command == 'rmdir' for command, _ in calls) == 1:
            raise OSError(30, 'synthetic read-only target')
    def lines(command, args):
        if command != 'bootc' or args != arguments:
            raise SystemExit('Native bootc arguments changed')
        calls.append(('bootc', tuple(args)))
        if failure == 'bootc':
            raise OSError('synthetic deployment failure')
        return ['synthetic progress']
    deploy.__globals__.update(
        os=SimpleNamespace(path=SimpleNamespace(ismount=lambda p: True, realpath=lambda p: p),
                           rmdir=remove),
        safe_exec_program=execute, execReadlines=lines, PayloadInstallationError=RuntimeError,
    )
    with patch('tempfile.mkdtemp', return_value=scratch):
        try:
            deploy('/synthetic-target', arguments, lambda line: None)
            if failure not in [None, 'read-only']:
                raise SystemExit('Expected scratch/deployment failure')
        except RuntimeError:
            if failure in [None, 'read-only']:
                raise
    expected_cleanup = [('umount', ('/var/tmp',)), ('umount', (scratch,)), ('rmdir', (scratch,))]
    if failure == 'first-bind':
        expected_cleanup = [('rmdir', (scratch,))]
    elif failure == 'second-bind':
        expected_cleanup = [('umount', (scratch,)), ('rmdir', (scratch,))]
    elif failure == 'unmount':
        expected_cleanup = [('umount', ('/var/tmp',))]
    elif failure in ['read-only', 'remount']:
        expected_cleanup += [('mount', ('-o', 'remount,rw', '/synthetic-target'))]
        if failure == 'read-only':
            expected_cleanup += [('rmdir', (scratch,))]
    if calls[-len(expected_cleanup):] != expected_cleanup:
        raise SystemExit('Scratch cleanup order changed')
for mounted, root_path in [(False, '/synthetic-target'), (True, '/')]:
    deploy.__globals__['os'].path = SimpleNamespace(ismount=lambda p: mounted, realpath=lambda p: root_path)
    with patch('tempfile.mkdtemp', side_effect=AssertionError('must reject before creating scratch')):
        try:
            deploy(root_path, [], lambda line: None)
            raise SystemExit('Unsafe scratch root was accepted')
        except RuntimeError:
            pass

native_class = next(node for node in ast.parse(updated[patches[2][0]]).body
                    if isinstance(node, ast.ClassDef) and node.name == 'PrepareBootcMountTargetsTask')
native_run = next(node for node in native_class.body if isinstance(node, ast.FunctionDef) and node.name == 'run')
module = ast.fix_missing_locations(ast.Module(body=[native_run], type_ignores=[]))
for points in [{'/': 'root', '/home': 'home', '/boot': 'boot', '/boot/efi': 'efi'},
               {'/': 'root', '/var': 'var', '/var/home': 'home', '/proc': 'api', '/srv': 'data'},
               {'/': 'root'}]:
    events = []
    namespace = {'STORAGE': SimpleNamespace(get_proxy=lambda _: SimpleNamespace(GetMountPoints=lambda: points)),
                 'DEVICE_TREE': 'fixture', 'safe_exec_program': lambda *args: events.append(('remount', args))}
    exec(compile(module, '<verified bootc mount preparation>', 'exec'), namespace)
    fixture = SimpleNamespace(_sysroot='/fixture-deploy', _physroot='/fixture-root', _internal_mounts=[],
        _handle_api_mount_points=lambda: events.append(('api',)),
        _handle_var_mount_point=lambda points: events.append(('var',)),
        _fill_var_subdirectories=lambda: events.append(('fill-var',)),
        _setup_internal_bindmount=lambda mount, recurse: events.append(('bind', mount, recurse)),
        _handle_boot_if_not_mount_point=lambda: events.append(('boot',)))
    namespace['run'](fixture)
    actual = [event[1] for event in events if event[0] == 'bind']
    expected = [point for point in sorted(points, key=len) if point not in ['/', '/var', '/dev', '/proc', '/run', '/sys']]
    if actual != expected or any(event[2] is not False for event in events if event[0] == 'bind'):
        raise SystemExit('Separate filesystem binding failed')
    if actual and events.index(('fill-var',)) > events.index(('bind', actual[0], False)):
        raise SystemExit('Separate home must be bound after persistent /var')
clean = getter(updated[patches[2][0]], '_clean_physroot')
for bad_root, unsupported, mounted in [(False, False, True), (True, False, True), (False, True, True), (False, False, False)]:
    calls = []
    points = {'/': 'root', '/boot': 'boot', '/home': 'home', '/var': 'var'}
    if unsupported:
        points['/unsupported'] = 'unsupported'
    clean.__globals__.update(
        os=SimpleNamespace(path=SimpleNamespace(ismount=lambda p: mounted and p != '/fixture/remove-me',
            realpath=lambda p: '/' if bad_root else p, join=lambda p, q: p + '/' + q),
            listdir=lambda p: ['boot', 'home', 'var', 'remove-me']),
        STORAGE=SimpleNamespace(get_proxy=lambda _: SimpleNamespace(GetMountPoints=lambda: points)),
        DEVICE_TREE='fixture', PayloadInstallationError=RuntimeError,
        log=SimpleNamespace(debug=lambda *args: None),
        safe_exec_program=lambda command, args: calls.append((command, args)), _=lambda value: value,
    )
    try:
        clean(SimpleNamespace(_physroot='/fixture'))
        if bad_root or unsupported or not mounted:
            raise SystemExit('Unsafe or unsupported physical-root cleanup accepted')
    except RuntimeError:
        if not (bad_root or unsupported or not mounted):
            raise
    expected = [] if bad_root or unsupported or not mounted else [('rm', ['-rf', '/fixture/remove-me'])]
    if calls != expected:
        raise SystemExit('Native cleanup touched a selected mounted filesystem')

for relative, source in updated.items():
    path = root / relative
    path.write_text(source)
    py_compile.compile(str(path), doraise=True)
    print(f'Adapted and checked Anaconda source: {relative}')
