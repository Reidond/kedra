#!/usr/bin/env python3
"""Owner tooling for the Kedra `utm` target on an Apple Silicon Mac running UTM.

check-host        read-only report: UTM, its Secure Boot firmware, Docker, free space
                  (--automation also tests utmctl's Automation access to UTM)
iso               build the local installer ISO in a disposable Linux container
create            create a UTM QEMU-backend VM bundle (UEFI Secure Boot + TPM) for it
detach-installer  remove the installer ISO from the stopped VM after installation

It only creates files under the directories you choose and invokes UTM and the
local Docker engine. It never touches the Mac's own disks or bootloader, never
needs sudo and never uploads anything. Compatible with macOS /usr/bin/python3.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import secrets
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import uuid

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
# The closed target table shared with installer/build-local.py and the release tooling.
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'build/release'))
try:
    from material import TARGETS  # noqa: E402
except (ImportError, SyntaxError) as error:
    raise SystemExit('kedra-utm: cannot load build/release/material.py with Python %s: %s'
                     % (platform.python_version(), error))

TARGET = 'utm'
SPEC = TARGETS[TARGET]
IMAGE_PATTERN = re.escape(SPEC['repository']) + r'@sha256:([a-f0-9]{64})'
BASE_PATTERN = r'quay\.io/fedora/fedora-bootc@sha256:[a-f0-9]{64}'
TOOLS = {'cp': '/bin/cp', 'defaults': '/usr/bin/defaults', 'lsof': '/usr/sbin/lsof', 'open': '/usr/bin/open',
         'osascript': '/usr/bin/osascript', 'pgrep': '/usr/bin/pgrep', 'plutil': '/usr/bin/plutil',
         'shasum': '/usr/bin/shasum', 'sysctl': '/usr/sbin/sysctl'}
UTM_ID = 'com.utmapp.UTM'
# With TPMDevice and UEFIBoot, UTM boots the secure code build; the variable
# template enrolls UTM's PK, Microsoft KEKs and the Microsoft UEFI CA 2011/2023 db.
SECURE_CODE = 'edk2-aarch64-secure-code.fd'
SECURE_VARS = 'edk2-arm-secure-vars.fd'
QCOW2_MAGIC = b'QFI\xfb'
INSTALLER_DRIVE = 'KEDRA-INSTALLER'
SYSTEM_DRIVE = 'KEDRA-SYSTEM'
SYSTEM_IMAGE = 'kedra-system.qcow2'
STORAGE_VOLUME = 'kedra-utm-podman-storage'
LABEL = 'org.kedra.utm-tool'
GIB = 1024**3
MIB = 1024**2
UNSAFE_PATH = set(':,"\n\r')
RENDERERS = {0: 'Default', 1: 'ANGLE (OpenGL)', 2: 'ANGLE (Metal)', 3: 'Apple Core OpenGL'}
VULKAN_DRIVERS = {0: 'Default (MoltenVK)', 1: 'Disabled', 2: 'MoltenVK', 3: 'KosmicKrisp'}
# Runs in the pinned Fedora container. dnf verifies Fedora's package signatures;
# the empty image is streamed to stdout, so no host directory is mounted.
QEMU_IMG = r'''set -eu
dnf -y -q --setopt=install_weak_deps=False '--setopt=*.gpgcheck=True' --repo=fedora --repo=updates install qemu-img >&2
qemu-img create -q -f qcow2 /tmp/disk.qcow2 "$1" >&2
qemu-img check -q /tmp/disk.qcow2 >&2
cat /tmp/disk.qcow2
'''
RELOAD = ['on run argv', 'tell application id "com.utmapp.UTM"',
          'reload configuration of virtual machine id (item 1 of argv)', 'end tell', 'end run']
# utmctl (UTMCtl.swift) reports failed Apple Events on stderr; -1743 is errAEEventNotPermitted.
APPLE_EVENTS_DENIED = re.compile(r'-1743|not permitted|not authori[sz]ed|SSH sessions', re.IGNORECASE)
AUTOMATION_HELP = ('macOS must allow the terminal app you run this from to control UTM: System Settings > '
                   'Privacy & Security > Automation > <terminal app> > UTM. macOS asks only once; after a '
                   'refusal run "tccutil reset AppleEvents <terminal bundle id>" (for example com.apple.Terminal) '
                   'and retry. utmctl and AppleScript do not work from SSH sessions or before logging in.')
# UTM's own defaults for a built-in terminal serial port (UTMConfigurationTerminal.swift).
BUILTIN_TERMINAL = {'ForegroundColor': '#ffffff', 'BackgroundColor': '#000000', 'Font': 'Menlo', 'FontSize': 12,
                    'CursorBlink': True}


class ToolError(Exception):
    """An operator-facing refusal or failure; existing files were not overwritten."""


def require(condition, message):
    if not condition:
        raise ToolError(message)


def run(arguments, *, timeout, capture=False, check=True, cwd=None):
    arguments = [str(item) for item in arguments]
    try:
        result = subprocess.run(arguments, stdin=subprocess.DEVNULL, cwd=cwd, timeout=timeout, check=False,
                                stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.PIPE if capture else None)
    except FileNotFoundError:
        raise ToolError('Required program not found: ' + arguments[0]) from None
    except subprocess.TimeoutExpired:
        raise ToolError('Timed out after %d s: %s' % (timeout, ' '.join(arguments[:3]))) from None
    if check and result.returncode != 0:
        detail = result.stderr.decode('utf-8', 'replace').strip()[-1500:] if capture else ''
        raise ToolError('Command failed with status %d: %s%s' % (
            result.returncode, ' '.join(arguments[:3]), '\n' + detail if detail else ''))
    return result


def regular(path):
    return path.is_file() and not path.is_symlink()


def read_bounded(path, limit=MIB):
    require(regular(path) and path.stat().st_size <= limit, 'Missing, unsafe or oversized input: ' + str(path))
    return path.read_bytes()


def load_plist(path):
    return plistlib.loads(read_bounded(path))


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(4 * MIB), b''):
            digest.update(block)
    return digest.hexdigest()


def free_bytes(path):
    while not path.exists():
        path = path.parent
    return shutil.disk_usage(str(path)).free


def sysctl(name):
    result = run([TOOLS['sysctl'], '-n', name], timeout=15, capture=True, check=False)
    text = result.stdout.decode('ascii', 'replace').strip()
    return int(text) if result.returncode == 0 and text.isdigit() else None


def version_tuple(text):
    match = re.fullmatch(r'(\d+)\.(\d+)(?:\.(\d+))?', text)
    return tuple(int(part or 0) for part in match.groups()) if match else None


def require_mac():
    require(sys.platform == 'darwin' and platform.machine() == 'arm64',
            'Run this on macOS on Apple Silicon (arm64); the utm target is aarch64')
    require(os.geteuid() != 0, 'Run as your ordinary macOS user; this tool never needs sudo')
    require(SPEC['architecture'] == 'aarch64' and SPEC['oci_architecture'] == 'arm64',
            'The shared target table no longer describes an aarch64 utm target')


def load_pins():
    pins = json.loads(read_bounded(HERE / 'inputs.json'))
    require(pins.get('schema_version') == 1 and pins.get('architecture') == SPEC['oci_architecture']
            and re.fullmatch(r'quay\.io/fedora/fedora@sha256:[a-f0-9]{64}', str(pins.get('build_container', '')))
            and version_tuple(str(pins.get('minimum_utm_version', ''))), 'Unexpected installer/utm/inputs.json pins')
    return pins


def utm_app(path, minimum):
    path = path.expanduser().absolute()
    info_path = path / 'Contents/Info.plist'
    require(regular(info_path), 'UTM not found at %s; install UTM %s or newer or pass --utm-app' % (path, minimum))
    info = load_plist(info_path)
    require(info.get('CFBundleIdentifier') == UTM_ID, str(path) + ' is not UTM')
    version = str(info.get('CFBundleShortVersionString', ''))
    found = version_tuple(version)
    require(found is not None and found >= version_tuple(minimum),
            'UTM %s is older than the qualified %s; update UTM first' % (version or 'unknown', minimum))
    firmware = path / 'Contents/Resources/qemu'
    code, variables = firmware / SECURE_CODE, firmware / SECURE_VARS
    require(regular(code) and regular(variables),
            'UTM at %s lacks its Secure Boot firmware %s and %s' % (path, SECURE_CODE, SECURE_VARS))
    with variables.open('rb') as stream:
        require(stream.read(4) == QCOW2_MAGIC, SECURE_VARS + ' is not the expected qcow2 variable store')
    return {'path': path, 'version': version, 'build': str(info.get('CFBundleVersion', '')),
            'code': code, 'vars': variables, 'utmctl': path / 'Contents/MacOS/utmctl'}


def utm_graphics():
    """Best effort: UTM keeps these app-wide settings in its sandboxed defaults."""
    domain = Path.home() / 'Library/Containers' / UTM_ID / 'Data/Library/Preferences' / UTM_ID
    values = []
    for key in ('QEMURendererBackend', 'QEMUVulkanDriver'):
        result = run([TOOLS['defaults'], 'read', domain, key], timeout=15, capture=True, check=False)
        text = result.stdout.decode('ascii', 'replace').strip()
        values.append(int(text) if result.returncode == 0 and text.isdigit() else None)
    return values


def docker_cli():
    found = shutil.which('docker')
    candidate = Path(found) if found else Path('/usr/local/bin/docker')
    require(candidate.is_absolute() and candidate.is_file() and os.access(str(candidate), os.X_OK),
            'Docker CLI not found; install and start an arm64 Docker engine (OrbStack was qualified)')
    return str(candidate)


def docker_engine(docker):
    result = run([docker, 'version', '--format', '{{.Server.Os}}/{{.Server.Arch}} {{.Server.Version}}'],
                 timeout=60, capture=True, check=False)
    require(result.returncode == 0, 'The Docker engine is not reachable; start OrbStack or another arm64 engine')
    engine, _, version = result.stdout.decode('utf-8', 'replace').strip().partition(' ')
    require(engine == 'linux/' + SPEC['oci_architecture'],
            'The Docker engine runs %s; a native linux/arm64 engine is required' % engine)
    name = run([docker, 'info', '--format', '{{.OperatingSystem}}'], timeout=60, capture=True).stdout
    return {'cli': docker, 'version': version, 'name': name.decode('utf-8', 'replace').strip()}


def installer_record(directory, image=None, iso_name=None):
    """Check build-local.py's records; ISO bytes are hashed by the caller."""
    record = json.loads(read_bounded(directory / 'installer.json'))
    require(isinstance(record, dict), 'installer.json is not an object')
    match = re.fullmatch(IMAGE_PATTERN, str(record.get('image', '')))
    require(match, 'installer.json does not describe %s media; this tool installs only the %s target'
            % (SPEC['repository'], TARGET))
    require(image is None or record['image'] == image, 'installer.json names a different image than requested')
    require(record.get('target', TARGET) == TARGET and record.get('uploaded') is False,
            'installer.json records an unexpected target or publication state')
    name = 'kedra-%s-44-%s.iso' % (TARGET, match.group(1)[:16])
    installer = record.get('installer')
    require(isinstance(installer, dict) and installer.get('filename') == name
            and isinstance(installer.get('size_bytes'), int)
            and re.fullmatch('[a-f0-9]{64}', str(installer.get('sha256', ''))),
            'installer.json has an unexpected installer identity')
    require(iso_name is None or iso_name == name, 'The ISO name does not match installer.json; expected ' + name)
    sums = read_bounded(directory / 'SHA256SUMS', 4096).decode('ascii', 'replace')
    require(sums == installer['sha256'] + '  ' + name + '\n', 'SHA256SUMS does not match installer.json')
    iso = directory / name
    require(regular(iso) and iso.stat().st_size == installer['size_bytes'],
            'The installer ISO is missing or has the wrong size: ' + str(iso))
    return record, iso


def write_plist(path, value):
    with path.open('xb') as stream:
        plistlib.dump(value, stream, fmt=plistlib.FMT_XML, sort_keys=True)
    run([TOOLS['plutil'], '-lint', '-s', path], timeout=30, capture=True)
    require(load_plist(path) == value, str(path) + ' did not round-trip')


def free_serial_port(port):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        try:
            probe.bind(('127.0.0.1', port))
        except OSError:
            raise ToolError('Serial console port %d on 127.0.0.1 is in use' % port) from None
    return port


def decoded(output):
    return output.decode('utf-8', 'replace').strip()[-1500:] if output else ''


def utm_automation(app):
    """None when utmctl can script UTM from this session, else utmctl's diagnosis.

    utmctl starts UTM hidden if needed; the first call may show macOS's Automation prompt.
    """
    result = run([app['utmctl'], 'list'], timeout=120, capture=True, check=False)
    error = decoded(result.stderr)
    if result.returncode == 0 and result.stdout.startswith(b'UUID') and 'Error from event' not in error:
        return None
    return error or 'utmctl list exited with status %d' % result.returncode


def automation_refusal(detail):
    denied = APPLE_EVENTS_DENIED.search(detail)
    return 'utmctl cannot control UTM from this session%s:\n%s\n%s' % (
        ' (macOS denied Apple Events)' if denied else '', detail, AUTOMATION_HELP)


def with_diagnosis(message, program, detail):
    """Append a program's stderr, and the Automation fix when it reports denied Apple Events."""
    if detail:
        message += '\n%s: %s' % (program, detail)
    if APPLE_EVENTS_DENIED.search(detail):
        message += '\n' + AUTOMATION_HELP
    return message


def utm_processes():
    result = run([TOOLS['pgrep'], '-x', 'UTM'], timeout=30, capture=True, check=False)
    require(result.returncode in (0, 1), 'pgrep could not check whether UTM is running')
    return result.stdout.decode('ascii', 'replace').split()


def check_host(args):
    failures = []

    def report(level, message):
        print('%-5s %s' % (level, message))
        if level == 'fail':
            failures.append(message)

    if sys.platform != 'darwin' or platform.machine() != 'arm64':
        report('fail', 'macOS on Apple Silicon is required (found %s/%s)' % (sys.platform, platform.machine()))
        return 1
    report('ok', 'macOS %s on arm64; Python %s' % (platform.mac_ver()[0], platform.python_version()))
    hypervisor = sysctl('kern.hv_support')
    report('ok' if hypervisor == 1 else 'fail', 'Hypervisor.framework available: %s' % (hypervisor == 1))
    memory, cpus = sysctl('hw.memsize'), sysctl('hw.ncpu')
    report('info', 'Host memory %.1f GiB, %s CPUs' % ((memory or 0) / GIB, cpus))
    minimum = '5.0.6'
    try:
        pins = load_pins()
        minimum = pins['minimum_utm_version']
        report('ok', 'Pinned build container %s' % pins['build_container'])
    except (ToolError, ValueError) as error:
        pins = None
        report('fail', str(error))
    try:
        app = utm_app(args.utm_app, minimum)
        report('ok', 'UTM %s (build %s) at %s; the 5.0 series is a pre-release'
               % (app['version'], app['build'], app['path']))
        for label, path in (('firmware', app['code']), ('variable store (qcow2)', app['vars'])):
            report('ok', 'Secure Boot %s %s: %d bytes, sha256 %s'
                   % (label, path.name, path.stat().st_size, sha256(path)))
        if not os.access(str(app['utmctl']), os.X_OK):
            report('warn', 'utmctl is not executable: ' + str(app['utmctl']))
        elif args.automation:
            denied = utm_automation(app)
            report('ok' if denied is None else 'fail', 'Automation access to UTM through utmctl' if denied is None
                   else automation_refusal(denied))
        else:
            report('info', 'utmctl: %s; Automation access not tested (check-host --automation starts UTM hidden '
                   'and tests it; detach-installer needs it unless UTM is quit)' % app['utmctl'])
        renderer, vulkan = utm_graphics()
        if renderer is None and vulkan is None:
            report('info', 'UTM display settings are unreadable here (sandboxed) or unchanged; Venus needs renderer '
                   'Default or ANGLE (Metal) and a Vulkan driver other than Disabled (UTM > Settings > Display)')
        elif renderer in (1, 3) or vulkan == 1:
            report('warn', 'UTM renderer %s with Vulkan driver %s disables Venus; use renderer Default/ANGLE (Metal)'
                   % (RENDERERS.get(renderer, renderer), VULKAN_DRIVERS.get(vulkan, vulkan)))
        else:
            report('ok', 'UTM renderer %s, Vulkan driver %s'
                   % (RENDERERS.get(renderer or 0), VULKAN_DRIVERS.get(vulkan or 0)))
    except ToolError as error:
        report('fail', str(error))
    try:
        engine = docker_engine(docker_cli())
        report('ok' if engine['name'] == 'OrbStack' else 'warn', 'Docker engine %s %s (linux/arm64) via %s%s' % (
            engine['name'], engine['version'], engine['cli'],
            '' if engine['name'] == 'OrbStack' else '; only OrbStack was qualified for the ISO build'))
        if pins:
            present = run([engine['cli'], 'image', 'inspect', '--format', '{{.Id}}', pins['build_container']],
                          timeout=60, capture=True, check=False).returncode == 0
            report('info', 'Build container is ' + ('present locally' if present else 'pulled by digest on first use'))
    except ToolError as error:
        report('fail', str(error))
    directory = args.dir.expanduser().absolute()
    free = free_bytes(directory)
    report('ok' if free >= 64 * GIB else 'warn', 'Free space for %s: %.1f GiB' % (directory, free / GIB))
    unsafe = any(char in UNSAFE_PATH for char in str(ROOT))
    report('fail' if unsafe else 'ok', 'Kedra checkout %s%s' % (ROOT, ' contains : , " or newlines' if unsafe else ''))
    authority = [ROOT / SPEC['public_key'], ROOT / SPEC['key_sha256']]
    report('ok' if all(map(regular, authority)) else 'warn',
           'utm release authority files %s' % ('present' if all(map(regular, authority)) else 'missing'))
    print('%d required check(s) failed' % len(failures) if failures else 'Host is ready for the utm target tooling')
    return 1 if failures else 0


def build_iso(args):
    require_mac()
    require(re.fullmatch(IMAGE_PATTERN, args.image),
            'An exact reviewed %s@sha256:... digest is required, not a mutable tag' % SPEC['repository'])
    require(args.base_image is None or re.fullmatch(BASE_PATTERN, args.base_image),
            'Invalid reviewed Fedora base digest')
    pins = load_pins()
    engine = docker_engine(docker_cli())
    docker = engine['cli']
    if engine['name'] != 'OrbStack':
        print('kedra-utm: warning: only OrbStack was qualified for this build; continuing with ' + engine['name'],
              file=sys.stderr)
    require(regular(ROOT / 'installer/build-local.py') and regular(HERE / 'build-in-container.sh'),
            'Run this from a complete trusted Kedra checkout')
    public, recorded = ROOT / SPEC['public_key'], ROOT / SPEC['key_sha256']
    require(regular(public) and regular(recorded), 'The checkout lacks the utm release authority files')
    fingerprint = read_bounded(recorded, 65).decode('ascii', 'replace').strip()
    require(re.fullmatch('[a-f0-9]{64}', fingerprint), 'Malformed ' + SPEC['key_sha256'])
    output = args.output.expanduser().absolute()
    require(not os.path.lexists(str(output)) and output.parent.is_dir(),
            '--output must name a new directory under an existing parent; nothing is overwritten')
    require(not any(char in UNSAFE_PATH for char in str(ROOT) + str(output.parent.resolve()) + output.name),
            'The checkout and output paths must not contain : , " or newlines (container mount syntax)')
    require(free_bytes(output.parent) >= 8 * GIB, 'Less than 8 GiB free for the ISO under ' + str(output.parent))
    running = run([docker, 'ps', '--filter', 'label=%s=iso' % LABEL, '--format', '{{.Names}}'],
                  timeout=60, capture=True).stdout.decode('utf-8', 'replace').split()
    require(not running, 'Another kedra-utm ISO build is running: ' + ', '.join(running))
    print('kedra-utm: utm public key SPKI SHA-256 %s; confirm it independently' % fingerprint, file=sys.stderr)
    token = secrets.token_hex(6)
    container, volume = 'kedra-utm-iso-' + token, 'kedra-utm-iso-out-' + token
    run([docker, 'volume', 'create', '--label', LABEL + '=iso-output', volume], timeout=120, capture=True)
    created = verified = False
    try:
        output.mkdir(mode=0o755)
        created = True
        # A fresh privileged container per run (no stale Podman locks); rootful
        # Podman state persists only in the named storage volume.
        run([docker, 'run', '--rm', '--name', container, '--label', LABEL + '=iso', '--platform', 'linux/arm64',
             '--pull', 'missing', '--privileged', '--cgroupns', 'private',
             '--mount', 'type=volume,source=%s,target=/var/lib/containers' % STORAGE_VOLUME,
             '--mount', 'type=volume,source=%s,target=/var/tmp/kedra-out' % volume,
             '--mount', 'type=bind,source=%s,target=/kedra,readonly' % ROOT,
             '--mount', 'type=bind,source=%s,target=/export' % output.resolve(),
             pins['build_container'], '/bin/bash', '/kedra/installer/utm/build-in-container.sh',
             args.image, args.base_image or ''], timeout=4 * 3600)
        record, iso = installer_record(output, image=args.image)
        names = sorted(entry.name for entry in output.iterdir())
        require(names == sorted([iso.name, 'installer.json', 'SHA256SUMS']),
                'Unexpected output files: ' + ', '.join(names))
        run([TOOLS['shasum'], '-a', '256', '-c', 'SHA256SUMS'], timeout=1800, cwd=str(output))
        verified = True
    finally:
        run([docker, 'rm', '--force', container], timeout=300, capture=True, check=False)
        if verified:
            run([docker, 'volume', 'rm', volume], timeout=300, capture=True, check=False)
        else:
            print('kedra-utm: build scratch retained in Docker volume %s (remove: docker volume rm %s)'
                  % (volume, volume), file=sys.stderr)
            if created and not any(output.iterdir()):
                output.rmdir()
            elif created:
                print('kedra-utm: unverified output retained for inspection in ' + str(output), file=sys.stderr)
    print('Verified %s (%d bytes) built from %s' % (iso, record['installer']['size_bytes'], record['image']))
    print('Next: python3 %s create --iso %s' % (shlex.quote(sys.argv[0]), shlex.quote(str(iso))))
    return 0


NETWORK_MODES = {'emulated': 'Emulated', 'shared': 'Shared'}


def vm_configuration(args, vm_uuid, mac, port, iso_name, notes):
    drive = {'InterfaceVersion': 1, 'ReadOnly': False}
    # The PL011 carries the firmware, GRUB and a login console. UTM's built-in
    # terminal keeps it inside UTM; a TCP server is reachable by every local
    # account, app and container, so it is only added on explicit request.
    serial = ({'Mode': 'TcpServer', 'Target': 'Auto', 'TcpPort': port, 'WaitForConnection': False,
               'RemoteConnectionAllowed': False} if port else
              {'Mode': 'Terminal', 'Target': 'Auto', 'Terminal': dict(BUILTIN_TERMINAL)})
    return {
        'Backend': 'QEMU', 'ConfigurationVersion': 4,
        'Information': {'Name': args.name, 'UUID': vm_uuid, 'Icon': 'linux', 'IconCustom': False, 'Notes': notes},
        'System': {'Architecture': 'aarch64', 'Target': 'virt', 'CPU': 'default', 'CPUFlagsAdd': [],
                   'CPUFlagsRemove': [], 'CPUCount': args.cpus, 'ForceMulticore': False,
                   'MemorySize': args.memory_mib, 'JITCacheSize': 0},
        'QEMU': {'DebugLog': False, 'UEFIBoot': True, 'RNGDevice': True, 'BalloonDevice': True, 'TPMDevice': True,
                 'Hypervisor': True, 'TSO': False, 'RTCLocalTime': False, 'PS2Controller': False,
                 'AdditionalArguments': []},
        'Input': {'UsbBusSupport': '3.0', 'UsbSharing': False, 'MaximumUsbShare': 3},
        'Sharing': {'DirectoryShareMode': 'None', 'DirectoryShareReadOnly': False, 'ClipboardSharing': True},
        'Display': [{'Hardware': 'virtio-gpu-gl-pci', 'DynamicResolution': True, 'NativeResolution': False,
                     'UpscalingFilter': 'Linear', 'DownscalingFilter': 'Linear'}],
        # Drive order is boot order: the installer CD first, then the system disk
        # (virtio serial KEDRASYSTEM, /dev/disk/by-id/virtio-KEDRASYSTEM).
        'Drive': [dict(drive, Identifier=INSTALLER_DRIVE, ImageName=iso_name, ImageType='CD', Interface='USB',
                       ReadOnly=True),
                  dict(drive, Identifier=SYSTEM_DRIVE, ImageName=SYSTEM_IMAGE, ImageType='Disk', Interface='VirtIO')],
        # Emulated VLAN is QEMU's own DHCP/NAT. Shared (macOS vmnet) needs the host's
        # bootpd, which gave a 2026-09-26 install no IPv4 lease behind VPN software.
        'Network': [{'Mode': NETWORK_MODES[args.network], 'Hardware': 'virtio-net-pci', 'MacAddress': mac,
                     'IsolateFromHost': False,
                     'PortForward': []}],
        'Serial': [serial],
        'Sound': [{'Hardware': 'intel-hda'}],
    }


def create_disk(docker, pins, destination, size):
    result = run([docker, 'run', '--rm', '--label', LABEL + '=qemu-img', '--platform', 'linux/arm64',
                  '--pull', 'missing', pins['build_container'], 'sh', '-euc', QEMU_IMG, 'kedra-qemu-img', str(size)],
                 timeout=900, capture=True)
    image = result.stdout
    require(len(image) <= 16 * MIB and image[:4] == QCOW2_MAGIC and int.from_bytes(image[4:8], 'big') == 3
            and int.from_bytes(image[24:32], 'big') == size, 'qemu-img did not produce the expected empty qcow2 image')
    with destination.open('xb') as stream:
        stream.write(image)
    os.chmod(str(destination), 0o644)


def create(args):
    require_mac()
    pins = load_pins()
    app = utm_app(args.utm_app, pins['minimum_utm_version'])
    docker = docker_engine(docker_cli())['cli']
    require(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9 ._-]{0,62}', args.name),
            'Use a simple VM name (letters, digits, space . _ -)')
    memory, cpus = sysctl('hw.memsize') or 0, sysctl('hw.ncpu') or 0
    require(4096 <= args.memory_mib <= memory // MIB - 4096,
            '--memory-mib must be between 4096 and the host memory minus 4096 MiB')
    require(2 <= args.cpus <= cpus, '--cpus must be between 2 and %d' % cpus)
    require(64 <= args.disk_gib <= 4096, '--disk-gib must be between 64 and 4096')
    require(args.serial_port is None or 1024 <= args.serial_port <= 65535, '--serial-port must be 1024-65535')
    iso = args.iso.expanduser().absolute()
    require(regular(iso), '--iso must name a regular local ISO file, not a link')
    record, _ = installer_record(iso.parent, iso_name=iso.name)
    directory = args.dir.expanduser().absolute()
    if not os.path.lexists(str(directory)):
        require(directory.parent.is_dir(), 'The parent of --dir does not exist: ' + str(directory.parent))
        directory.mkdir(mode=0o755)
    require(directory.is_dir() and not directory.is_symlink(), '--dir must be a directory, not a link')
    bundle = directory / (args.name + '.utm')
    require(not os.path.lexists(str(bundle)), 'Refusing to overwrite existing ' + str(bundle))
    if free_bytes(directory) < args.disk_gib * GIB:
        print('kedra-utm: warning: less free space than the %d GiB disk can grow to' % args.disk_gib,
              file=sys.stderr)
    port = free_serial_port(args.serial_port) if args.serial_port else None
    console = ('nc 127.0.0.1 %d' % port) if port else 'UTM\'s built-in terminal window'
    if port:
        print('kedra-utm: warning: the TCP serial console on 127.0.0.1:%d has no authentication. Any local '
              'account, app or container that reaches it has the VM\'s console: firmware setup (Secure Boot can '
              'be disabled), GRUB command-line editing (a root shell once the disk is unlocked) and a login '
              'prompt' % port, file=sys.stderr)
    octets = bytearray(secrets.token_bytes(6))
    octets[0] = (octets[0] & 0xFC) | 0x02  # locally administered unicast, as UTM generates
    mac = ':'.join('%02X' % octet for octet in octets)
    vm_uuid = str(uuid.uuid4()).upper()
    bundle.mkdir(mode=0o755)
    try:
        data = bundle / 'Data'
        data.mkdir(mode=0o755)
        # Pre-seed the Microsoft-keyed store: UTM preloads keys only from its GUI TPM
        # toggle or at a first start, and any earlier save would create empty vars.
        variables = data / 'efi_vars.fd'
        with app['vars'].open('rb') as source, variables.open('xb') as target:
            shutil.copyfileobj(source, target)
        os.chmod(str(variables), 0o644)
        require(sha256(variables) == sha256(app['vars']), 'efi_vars.fd differs from UTM\'s template')
        create_disk(docker, pins, data / SYSTEM_IMAGE, args.disk_gib * GIB)
        media = data / iso.name
        run([TOOLS['cp'], '-c', '--', iso, media], timeout=1800)  # APFS clone when on the same volume
        require(regular(media) and sha256(media) == record['installer']['sha256'],
                'The installer ISO does not match installer.json/SHA256SUMS; rebuild or recopy it')
        with media.open('rb') as stream:
            stream.seek(0x8001)
            require(stream.read(5) == b'CD001', str(iso) + ' is not an ISO 9660 image')
        os.chmod(str(media), 0o444)
        notes = ('Kedra %s target (aarch64). Installer %s from %s. Serial console: %s. '
                 'Created by installer/utm/kedra-utm.py.' % (TARGET, iso.name, record['image'], console))
        write_plist(bundle / 'config.plist', vm_configuration(args, vm_uuid, mac, port, iso.name, notes))
    except BaseException:
        shutil.rmtree(str(bundle))  # only the bundle this run created
        raise
    if not args.no_register:
        registered = run([TOOLS['open'], '-a', app['path'], bundle], timeout=60, capture=True, check=False)
        require(registered.returncode == 0, 'Created %s, but UTM did not open it; open it from UTM (File > Open)'
                % bundle)
    print('Created %s (UUID %s)' % (bundle, vm_uuid))
    print('  Secure Boot: UEFI + TPM, Data/efi_vars.fd from UTM %s %s' % (app['version'], SECURE_VARS))
    print('  Installer: Data/%s (USB CD, first boot device) from %s' % (iso.name, record['image']))
    print('  System disk: Data/%s, %d GiB, /dev/disk/by-id/virtio-KEDRASYSTEM' % (SYSTEM_IMAGE, args.disk_gib))
    print('  Network: %s, MAC %s; serial console: %s' % (args.network, mac, console))
    print('  UTM: ' + ('not registered (--no-register)' if args.no_register else 'registered; start it from UTM or '
                        'with: %s start %s' % (shlex.quote(str(app['utmctl'])), vm_uuid)))
    print('After installing, shut the VM down and run: python3 %s detach-installer --bundle %s'
          % (shlex.quote(sys.argv[0]), shlex.quote(str(bundle))))
    return 0


def open_holders(paths):
    existing = [str(path) for path in paths if path.exists()]
    if not existing:
        return []
    # lsof exits 1 when a named file is not open; every process record is a holder.
    result = run([TOOLS['lsof'], '-F', 'pc', '--'] + existing, timeout=60, capture=True, check=False)
    require(result.returncode in (0, 1), 'lsof could not check whether the VM files are in use')
    holders, pid = [], ''
    for line in result.stdout.decode('utf-8', 'replace').splitlines():
        if line.startswith('p'):
            pid = line[1:]
            holders.append('pid ' + pid)
        elif line.startswith('c') and pid:
            holders[-1] = '%s (pid %s)' % (line[1:], pid)
    return holders


def detach_installer(args):
    require_mac()
    bundle = args.bundle.expanduser().absolute()
    require(bundle.suffix == '.utm' and bundle.is_dir() and not bundle.is_symlink(),
            '--bundle must be a .utm directory')
    configuration_path = bundle / 'config.plist'
    configuration = load_plist(configuration_path)
    require(isinstance(configuration, dict) and configuration.get('Backend') == 'QEMU'
            and configuration.get('ConfigurationVersion') == 4 and isinstance(configuration.get('Information'), dict)
            and isinstance(configuration.get('Drive'), list)
            and all(isinstance(drive, dict) for drive in configuration['Drive']),
            'Not a UTM QEMU configuration (version 4)')
    vm_uuid = str(uuid.UUID(str(configuration['Information'].get('UUID')))).upper()
    drives = configuration['Drive']
    installers = [drive for drive in drives if drive.get('Identifier') == INSTALLER_DRIVE]
    if not installers:
        print('No Kedra installer drive is attached to ' + str(bundle))
        return 0
    require(len(installers) == 1, 'More than one Kedra installer drive; edit the VM in UTM instead')
    installer = installers[0]
    image_name = installer.get('ImageName')
    require(installer.get('ImageType') == 'CD' and isinstance(image_name, str)
            and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*\.iso', image_name), 'Unexpected installer drive entry')
    data = bundle / 'Data'
    media = data / image_name
    require(not media.is_symlink() and (media.is_file() or not media.exists()),
            'Unexpected installer file ' + str(media))
    remaining = [drive for drive in drives if drive is not installer]
    require(any(drive.get('Identifier') == SYSTEM_DRIVE for drive in remaining),
            'The Kedra system disk is not configured; refusing to leave the VM without it')
    app = None
    if args.utm_quit:
        # UTM reads config.plist again when it starts, so no reload is needed.
        running = utm_processes()
        require(not running, 'UTM is running (pid %s); quit UTM first, or omit --utm-quit' % ', '.join(running))
    elif not args.unregistered:
        app = utm_app(args.utm_app, load_pins()['minimum_utm_version'])
        status = run([app['utmctl'], 'status', vm_uuid], timeout=120, capture=True, check=False)
        state = status.stdout.decode('utf-8', 'replace').strip()
        if status.returncode != 0 or not state:
            denied = utm_automation(app)
            if denied is not None:
                raise ToolError(automation_refusal(denied) + ' Alternatively quit UTM and rerun with --utm-quit.')
        require(status.returncode == 0 and state == 'stopped', with_diagnosis(
            'The VM must be stopped and registered in UTM (utmctl status: %s)' % (state or 'unavailable'),
            'utmctl', decoded(status.stderr)))
    holders = open_holders([media, data / SYSTEM_IMAGE, data / 'efi_vars.fd', data / 'tpmdata'])
    require(not holders, 'VM files are still open by: %s; stop the VM first' % ', '.join(sorted(set(holders))))
    system = data / SYSTEM_IMAGE
    if regular(system) and system.stat().st_size < 64 * MIB:
        print('kedra-utm: warning: the system disk looks unwritten; was the installation completed?',
              file=sys.stderr)
    replacement = bundle / '.config.plist.kedra-new'
    try:
        write_plist(replacement, dict(configuration, Drive=remaining))
        os.replace(str(replacement), str(configuration_path))
    finally:
        if os.path.lexists(str(replacement)):
            replacement.unlink()
    if media.exists():
        media.unlink()
    if app:
        arguments = [TOOLS['osascript']]
        for line in RELOAD:
            arguments += ['-e', line]
        reloaded = run(arguments + [vm_uuid], timeout=120, capture=True, check=False)
        require(reloaded.returncode == 0, with_diagnosis(
            'Detached on disk, but UTM did not reload the configuration; quit and reopen UTM before starting the VM',
            'osascript', decoded(reloaded.stderr)))
    offline = '--utm-quit' if args.utm_quit else '--unregistered'
    print('Detached %s from %s%s' % (image_name, bundle, '' if app else ' (UTM not contacted: %s)' % offline))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    utm = argparse.ArgumentParser(add_help=False)
    utm.add_argument('--utm-app', type=Path, default=Path('/Applications/UTM.app'), help='UTM.app to use')
    host = commands.add_parser('check-host', parents=[utm], help='Read-only host readiness report')
    host.add_argument('--dir', type=Path, default=Path('~/VMs'), help='Where VM bundles will live')
    host.add_argument('--automation', action='store_true',
                      help='Also test Automation access to UTM with utmctl list (starts UTM hidden)')
    iso = commands.add_parser('iso', help='Build the utm installer ISO in a disposable container')
    iso.add_argument('--image', required=True, help='Reviewed %s@sha256:...' % SPEC['repository'])
    iso.add_argument('--output', required=True, type=Path, help='New local directory for the ISO and its records')
    iso.add_argument('--base-image', help='Explicit reviewed Fedora base digest (legacy payloads only)')
    new = commands.add_parser('create', parents=[utm], help='Create a UTM VM bundle for the installer ISO')
    new.add_argument('--iso', required=True, type=Path, help='ISO produced by the iso command (with its records)')
    new.add_argument('--name', default='Kedra', help='VM name; the bundle is <dir>/<name>.utm')
    new.add_argument('--disk-gib', type=int, default=96)
    new.add_argument('--memory-mib', type=int, default=8192)
    new.add_argument('--cpus', type=int, default=6)
    new.add_argument('--dir', type=Path, default=Path('~/VMs'), help='Directory for the new bundle')
    new.add_argument('--serial-port', type=int,
                     help='Expose the serial console as an unauthenticated TCP server on this 127.0.0.1 port '
                          '(default: UTM\'s built-in terminal only)')
    new.add_argument('--network', choices=sorted(NETWORK_MODES), default='emulated',
                     help='emulated (default): UTM Emulated VLAN, QEMU DHCP/NAT without host access; '
                          'shared: macOS vmnet Shared Network with host access, needs the host DHCP service')
    new.add_argument('--no-register', action='store_true', help='Do not open the bundle in UTM')
    detach = commands.add_parser('detach-installer', parents=[utm], help='Remove the installer from a stopped VM')
    detach.add_argument('--bundle', required=True, type=Path)
    offline = detach.add_mutually_exclusive_group()
    offline.add_argument('--unregistered', action='store_true',
                         help='The bundle was created with --no-register and never opened in UTM: skip utmctl and '
                              'the reload (the open-file check still applies)')
    offline.add_argument('--utm-quit', action='store_true',
                         help='UTM has been quit (for example without Automation access or over SSH): refuse if a '
                              'UTM process runs, then skip utmctl and the reload (the open-file check still applies)')
    args = parser.parse_args()
    handlers = {'check-host': check_host, 'iso': build_iso, 'create': create, 'detach-installer': detach_installer}
    return handlers[args.command](args)


def interrupted(signum, frame):
    raise KeyboardInterrupt


if __name__ == '__main__':
    # Termination runs the same cleanup as Ctrl-C (build container, partial bundle).
    signal.signal(signal.SIGTERM, interrupted)
    if signal.getsignal(signal.SIGHUP) is not signal.SIG_IGN:
        signal.signal(signal.SIGHUP, interrupted)
    try:
        sys.exit(main())
    except (ToolError, OSError, ValueError, KeyError, TypeError) as error:
        print('kedra-utm: ' + str(error), file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        print('kedra-utm: interrupted', file=sys.stderr)
        sys.exit(130)
