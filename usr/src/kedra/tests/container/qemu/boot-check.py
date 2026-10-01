#!/usr/bin/python3
"""Fixed native-material observer; runs only in the fixture image or VM guest."""
import hashlib
import json
import os
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

PROVENANCE = Path('/usr/share/kedra-lab/native-provenance.json')
RECEIPT = Path('/usr/share/sysroot/native-receipt.json')
SAVED = Path('/tmp/kedra-native-fixture-save')
RPM_BASE = Path('/usr/share/kedra-lab/native-base-rpms.txt')
ADJUSTMENTS = Path('/usr/share/kedra-lab/native-fixture-adjustments.json')
RESULT = Path('/run/kedra-lab/native-boot.json')
RPM_FORMAT = '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n'
PRODUCTION_TRUST = (
    '/etc/containers/policy.json', '/etc/containers/registries.d/kedra.yaml',
    '/usr/lib/bootc/install/10-kedra.toml', '/usr/lib/sysroot/trust/release.pub',
    '/usr/lib/sysroot/trust/release-policy.json', '/usr/share/sysroot/image-identity.json',
    '/usr/share/sysroot/resolved-inputs.json', '/usr/share/sysroot/source.json',
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(argv):
    result = subprocess.run(argv, check=True, capture_output=True, timeout=90,
                            env={'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LC_ALL': 'C', 'HOME': '/root'})
    require(len(result.stdout) <= 8 * 1024 * 1024, 'oversized native observation')
    return result.stdout.decode()


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def regular(path, single=False):
    path = Path(path)
    require(path.is_absolute() and '..' not in path.parts, 'unsafe material path')
    for parent in path.parents:
        require(not parent.is_symlink() and parent.is_dir(), 'aliased material parent: ' + str(parent))
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode), 'nonregular material: ' + str(path))
    require(not single or info.st_nlink == 1, 'hardlinked generated material: ' + str(path))
    return info


def read_json(path):
    require(regular(path).st_size <= 8 * 1024 * 1024, 'oversized material JSON')
    return json.loads(Path(path).read_text())


def production_trust():
    result = {}
    for path in PRODUCTION_TRUST:
        info = regular(path)
        result[path] = {'sha256': digest(path), 'bytes': info.st_size, 'mode': stat.S_IMODE(info.st_mode)}
    return result


def provenance():
    value = read_json(PROVENANCE)
    if value is None:
        return None
    require(set(value) == {'image', 'engine', 'parent_image', 'material'}, 'invalid native provenance')
    for name in ('image', 'parent_image'):
        require(re.fullmatch(r'sha256:[a-f0-9]{64}', value[name]), 'invalid native image identity')
    material = value['material']
    require(material['schema'] == 1 and material['kernels'], 'native boot material is absent')
    for name in ('identity', 'parent_identity'):
        require(re.fullmatch(r'[a-f0-9]{64}', material[name]), 'invalid native identity')
    require(read_json(RECEIPT) == material, 'native receipt changed during VM adaptation')
    return value


def inventory():
    rows = run(['/usr/bin/rpm', '-qa', '--qf', RPM_FORMAT]).splitlines()
    return '\n'.join(sorted(row for row in rows if not row.startswith('gpg-pubkey\t'))) + '\n'


def matches(path, entry, installed=False):
    path = Path(path)
    for parent in path.parents:
        require(not parent.is_symlink() and parent.is_dir(), 'aliased artifact parent')
    if entry['kind'] == 'removed_symlink':
        return not os.path.lexists(path)
    if entry['kind'] == 'symlink':
        return path.is_symlink() and os.readlink(path) == entry['target']
    require(entry['kind'] == 'regular', 'unknown native artifact')
    if not os.path.lexists(path):
        return False
    info = regular(path, single=not installed)
    return info.st_size == entry['bytes'] and stat.S_IMODE(info.st_mode) == entry['mode'] and digest(path) == entry['sha256']


def verify_material(value, installed=False):
    material = value['material']
    for artifact in material['artifacts']:
        require(matches(artifact['path'], artifact['entry'], installed), 'native artifact changed: ' + artifact['path'])
    for path, expected in material['tools'].items():
        require(digest(path) == expected, 'native tool changed: ' + path)
    for kernel in material['kernels']:
        version = kernel['version']
        require(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._+\-]*', version), 'invalid kernel version')
        root = Path('/usr/lib/modules') / version
        regular(root / 'vmlinuz')
        require(digest(root / 'vmlinuz') == kernel['kernel_sha256'], 'native kernel changed')
        listing = run(['/usr/bin/lsinitrd', str(root / 'initramfs.img')])
        require(hashlib.sha256(listing.encode()).hexdigest() == kernel['listing_sha256'], 'native initramfs listing changed')
        for path in kernel['required_modules'].values():
            require(path.startswith(str(root) + '/') and path.lstrip('/') in listing, 'native initramfs lost driver')


def regenerable(artifact):
    path = artifact['path']
    return artifact['entry']['kind'] == 'regular' and (
        path == '/usr/share/glib-2.0/schemas/gschemas.compiled'
        or re.fullmatch(r'/usr/lib/modules/[A-Za-z0-9._+\-]+/initramfs.img', path))


def snapshot(value):
    verify_material(value)
    rows = inventory()
    require(hashlib.sha256(rows.encode()).hexdigest() == value['material']['rpm_sha256'], 'native RPM inventory changed')
    RPM_BASE.write_text(rows)
    SAVED.mkdir(mode=0o700)
    for index, artifact in enumerate(value['material']['artifacts']):
        if regenerable(artifact):
            shutil.copyfile(artifact['path'], SAVED / str(index))


def restore(value):
    changes = []
    for index, artifact in enumerate(value['material']['artifacts']):
        if not regenerable(artifact):
            continue
        source = SAVED / str(index)
        entry = artifact['entry']
        require(regular(source, single=True).st_size == entry['bytes'] and digest(source) == entry['sha256'],
                'saved native artifact changed')
        path = Path(artifact['path'])
        if not matches(path, entry):
            changes.append({'path': str(path), 'reason': 'fixture package transaction regenerated native output',
                            'before_sha256': entry['sha256'],
                            'regenerated_sha256': digest(path) if path.exists() else None,
                            'restored_sha256': entry['sha256']})
            descriptor, temporary = tempfile.mkstemp(prefix='.kedra-native-', dir=path.parent)
            try:
                with os.fdopen(descriptor, 'wb') as stream, source.open('rb') as original:
                    shutil.copyfileobj(original, stream)
                    os.fchmod(stream.fileno(), entry['mode'])
                os.replace(temporary, path)
            finally:
                Path(temporary).unlink(missing_ok=True)
        source.unlink()
    SAVED.rmdir()
    ADJUSTMENTS.write_text(json.dumps(changes, sort_keys=True) + '\n')
    verify_material(value)


def image_observation(value, installed=False):
    verify_material(value, installed)
    baseline = RPM_BASE.read_text()
    require(hashlib.sha256(baseline.encode()).hexdigest() == value['material']['rpm_sha256'], 'native base RPM binding changed')
    require(set(baseline.splitlines()).issubset(inventory().splitlines()), 'fixture replaced a native RPM')
    return {'passed': True, 'provenance': value, 'native_receipt_sha256': digest(RECEIPT),
            'fixture_adjustments': read_json(ADJUSTMENTS), 'production_trust': production_trust()}


def boot_artifacts(value):
    version = os.uname().release
    kernels = [kernel for kernel in value['material']['kernels'] if kernel['version'] == version]
    require(len(kernels) == 1, 'running kernel is not the independently selected native kernel')
    kernel = kernels[0]
    cmdline = Path('/proc/cmdline').read_text().strip()
    options = shlex.split(cmdline)
    ostree = [option for option in options if option.startswith('ostree=')]
    require(len(ostree) == 1, 'boot has no unique OSTree deployment argument')
    require('selinux=0' not in options and 'enforcing=0' not in options, 'boot disabled SELinux')
    entries = []
    for path in Path('/boot/loader/entries').glob('*.conf'):
        fields = {}
        for line in path.read_text().splitlines():
            parts = line.strip().split(None, 1)
            if len(parts) == 2 and not parts[0].startswith('#'):
                fields.setdefault(parts[0], []).append(parts[1])
        if any(ostree[0] in shlex.split(option) for option in fields.get('options', [])):
            entries.append((path, fields))
    require(len(entries) == 1, 'cannot identify one boot loader entry for the running deployment')
    entry_path, fields = entries[0]
    selected = {}
    for name in ('linux', 'initrd'):
        paths = fields.get(name, [])
        require(len(paths) == 1 and len(shlex.split(paths[0])) == 1, 'ambiguous boot loader ' + name)
        relative = shlex.split(paths[0])[0]
        require(relative.startswith('/') and '..' not in Path(relative).parts, 'unsafe boot loader artifact')
        path = Path('/boot') / relative.lstrip('/')
        require(path.resolve().is_relative_to(Path('/boot').resolve()), 'boot artifact escapes /boot')
        require(path.is_file(), 'missing selected boot artifact')
        selected[name] = {'path': str(path), 'sha256': digest(path)}
    initramfs = '/usr/lib/modules/' + version + '/initramfs.img'
    expected = [a['entry']['sha256'] for a in value['material']['artifacts'] if a['path'] == initramfs]
    require(len(expected) == 1 and selected['initrd']['sha256'] == expected[0], 'boot loader initramfs differs from generated material')
    require(selected['linux']['sha256'] == kernel['kernel_sha256'], 'boot loader kernel differs from generated material')
    initrd_journal = run(['/usr/bin/journalctl', '-b', '--no-pager', '-o', 'short-monotonic', '-u', 'initrd-switch-root.service'])
    require('Switch' in initrd_journal or 'switch' in initrd_journal, 'initrd switch-root evidence absent')
    drivers = {}
    for driver in kernel['required_modules']:
        name = driver.replace('-', '_')
        require((Path('/sys/module') / name).is_dir(), 'required native driver is not loaded: ' + name)
        drivers[name] = True
    return {'kernel_version': version, 'cmdline': cmdline, 'ostree': ostree[0],
            'loader_entry': str(entry_path), 'selected': selected,
            'initrd_journal': initrd_journal, 'loaded_modules': drivers}


def boot_observation(value):
    result = image_observation(value, installed=True)
    result['boot_id'] = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
    result['boot_artifacts'] = boot_artifacts(value)
    require(run(['/usr/bin/mokutil', '--sb-state']).strip() == 'SecureBoot enabled', 'Secure Boot disabled')
    variables = Path('/sys/firmware/efi/efivars')
    for name, expected in (('SecureBoot', 1), ('SetupMode', 0)):
        data = (variables / (name + '-8be4df61-93ca-11d2-aa0d-00e098032b8c')).read_bytes()
        require(len(data) == 5 and data[4] == expected, 'unexpected EFI ' + name)
    lockdown = Path('/sys/kernel/security/lockdown').read_text().strip()
    require('[integrity]' in lockdown or '[confidentiality]' in lockdown, 'kernel lockdown disabled')
    kernel_log = run(['/usr/bin/journalctl', '-k', '-b', '--no-pager', '-o', 'cat', '--grep', 'secureboot: Secure boot enabled'])
    require('secureboot: Secure boot enabled' in kernel_log.splitlines(), 'kernel Secure Boot evidence absent')
    firmware = run(['/usr/sbin/efibootmgr', '-v'])
    current = re.search(r'^BootCurrent: ([0-9A-Fa-f]{4})$', firmware, re.MULTILINE)
    require(current is not None, 'firmware BootCurrent absent')
    require(re.search(r'^Boot' + current[1] + r'[* ].*shimaa64\.efi', firmware, re.MULTILINE | re.IGNORECASE),
            'firmware did not select the Fedora shim entry')
    require(run(['/usr/sbin/getenforce']).strip() == 'Enforcing', 'SELinux is not enforcing')
    selinux = run(['/usr/sbin/sestatus'])
    require(re.search(r'^Loaded policy name:\s+targeted$', selinux, re.MULTILINE), 'unexpected SELinux policy')
    status = json.loads(run(['/usr/bin/bootc', 'status', '--json']))
    booted = status['status']['booted']['image']
    require(booted['architecture'] == 'arm64', 'wrong booted architecture')
    require(re.fullmatch(r'localhost/kedra-qemu-fixture/[a-f0-9]{32}@sha256:[a-f0-9]{64}', booted['image']['image']),
            'wrong booted signed-fixture reference')
    require(status['spec']['image']['signature'] == 'containerPolicy', 'bootc signature-policy enforcement absent')
    require(status['status']['staged'] is None and status['status']['rollback'] is None, 'unexpected additional deployment')
    failed = run(['/usr/bin/systemctl', 'list-units', '--state=failed', '--no-legend', '--plain', '--no-pager']).strip()
    require(not failed, 'failed system units: ' + failed)
    avcs = subprocess.run([
        '/usr/bin/journalctl', '-b', '--no-pager', '-o', 'cat', '--grep', 'avc: +denied',
    ], capture_output=True, check=False, timeout=90,
        env={'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LC_ALL': 'C', 'HOME': '/root'})
    require(len(avcs.stdout) + len(avcs.stderr) <= 8 * 1024 * 1024, 'oversized AVC observation')
    # No matches is a normal journalctl result; denials remain evidence to review.
    result['avc_observation'] = {
        'exit': avcs.returncode, 'stdout': avcs.stdout.decode(), 'stderr': avcs.stderr.decode(),
        'complete': avcs.returncode == 0 or avcs.returncode == 1 and not avcs.stdout and not avcs.stderr,
    }
    result.update(secure_boot=True, lockdown=lockdown, firmware=firmware, selinux=selinux, bootc=status,
                  release_trust='disposable fixture signature admission; no production release authority')
    return result


def main():
    require(len(sys.argv) == 2 and sys.argv[1] in ('snapshot', 'restore', 'image', 'boot', 'trust'), 'unknown fixed observation')
    if sys.argv[1] == 'trust':
        print(json.dumps(production_trust(), sort_keys=True))
        return
    value = provenance()
    if value is None:
        return
    operation = sys.argv[1]
    if operation == 'snapshot':
        snapshot(value)
    elif operation == 'restore':
        restore(value)
    elif operation == 'image':
        print(json.dumps(image_observation(value), sort_keys=True))
    else:
        RESULT.parent.mkdir(mode=0o755, exist_ok=True)
        try:
            result = boot_observation(value)
        except Exception as error:
            RESULT.write_text(json.dumps({'passed': False, 'error': str(error)}) + '\n')
            raise
        RESULT.write_text(json.dumps(result, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
