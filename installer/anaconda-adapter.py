"""Narrow, version-guarded Fedora Anaconda 44.30-2.fc44 media adaptations."""
import hashlib
import pathlib
import py_compile
import importlib.util

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

for relative, source in updated.items():
    path = root / relative
    path.write_text(source)
    py_compile.compile(str(path), doraise=True)
    print(f'Adapted and checked Anaconda source: {relative}')
