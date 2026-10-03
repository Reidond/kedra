#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Prepare narrowly transformed private Kickstart/cases data; never build an ISO.

The generated Python snippets run in the disposable installer with its interpreter.
The module's bounded GRUB selector is used by the existing Mac fixture launcher.
"""
import argparse
import hashlib
import json
import os
import platform
import re
import socket
import stat
import subprocess
import time
from pathlib import Path

KICKSTART_ARGUMENT = 'inst.ks=hd:LABEL=KEDRA_GHCR_CASES:/fixture.ks'
ORIGINAL_ARGUMENT = 'inst.ks=file:/usr/share/anaconda/fixture.ks'
ORIGINAL_LINUX = ('linux /images/pxeboot/vmlinuz inst.stage2=hd:LABEL=KEDRA-44-Install '
                  'console=tty0 console=ttyAMA0,115200 inst.text ' + ORIGINAL_ARGUMENT)
GRUB_COMMAND = ('if search --no-floppy --set=root -l KEDRA_GHCR_CASES; '
                'then configfile /grub-marker.cfg; fi')
ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')
BROKEN_MARKER = r"Path('/dev/ttyAMA0').write_text('KEDRA_FIXTURE_INSTALL_COMPLETE\n')" + '\n'
COMPLETE = 'KEDRA_FIXTURE_INSTALL_COMPLETE'


def require(value, message):
    if not value:
        raise RuntimeError(message)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def source(path, expected=None, secret=False):
    require(path.is_absolute() and path.resolve(strict=True) == path
            and not any(character in str(path) for character in ',\r\n'), 'Aliased/unsafe input path refused')
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_uid == os.getuid(),
            'Expected an owned single-link regular input')
    if secret:
        require(stat.S_IMODE(info.st_mode) == 0o600, 'Private input must be mode 0600')
    if expected is not None:
        require(re.fullmatch('[a-f0-9]{64}', expected) and digest(path) == expected, 'Selected input hash differs')
    return path


def document(path, expected=None):
    path = source(path, expected)
    require(path.stat().st_size <= 1024 * 1024, 'Oversized instrumentation metadata')
    return json.loads(path.read_bytes())


def write(path, payload):
    with path.open('xb') as stream:
        stream.write(payload)
        stream.flush()
        os.fsync(stream.fileno())


def emit_script(marker):
    # No O_CREAT/TRUNC: even a mistaken chroot cannot create or alter a regular file.
    return f'''# Runs in the disposable installer, using its own Python interpreter.
import os, stat
device='/dev/ttyAMA0'
before=os.stat(device,follow_symlinks=False)
if not stat.S_ISCHR(before.st_mode): raise RuntimeError('serial endpoint is not a character device')
descriptor=os.open(device,os.O_WRONLY|os.O_NOFOLLOW|os.O_NOCTTY)
try:
    actual=os.fstat(descriptor)
    if not stat.S_ISCHR(actual.st_mode) or actual.st_rdev != before.st_rdev:
        raise RuntimeError('serial endpoint changed')
    payload={('\n' + marker + '\n').encode()!r}
    while payload:
        written=os.write(descriptor,payload)
        if written <= 0: raise RuntimeError('serial marker write failed')
        payload=payload[written:]
finally:
    os.close(descriptor)
'''


def corrected_kickstart(original, token):
    prefix = '#version=DEVEL\ntext\n'
    require(original.startswith(prefix), 'Original Kickstart header differs from reviewed input')
    require(not re.search(r'^\s*(?:xconfig|skipx)(?:\s|$)', original, re.MULTILINE),
            'Original Kickstart already declares graphical target policy')
    require(original.count(BROKEN_MARKER) == 1, 'Original marker recipe differs from reviewed input')
    require(original.endswith(BROKEN_MARKER + '%end\n'), 'Original completion marker is not the final post action')
    require(original.count('%post --interpreter=/usr/bin/python3 --erroronfail\n') == 1,
            'Unexpected original post-script shape')
    require(original.count('%include /usr/share/anaconda/interactive-defaults.ks\n') == 1,
            'Original public installer include is absent or duplicated')
    pre_header = '%pre --interpreter=/usr/bin/python3 --erroronfail\n'
    require(original.count(pre_header) == 1, 'Unexpected original pre-script shape')
    guard_code = f'''from pathlib import Path
locations=[value for value in Path('/proc/cmdline').read_text().split() if value.startswith('inst.ks=')]
if locations != [{KICKSTART_ARGUMENT!r}]: raise RuntimeError('external Kickstart selection differs')
'''
    guard_code += emit_script('KEDRA_EXTERNAL_KICKSTART_SELECTED_' + token)
    completion = emit_script(COMPLETE)
    compile(guard_code, '<external-kickstart-pre>', 'exec')
    compile(completion, '<external-kickstart-post>', 'exec')
    guard = pre_header + guard_code + '%end\n'
    # Anaconda must preserve the graphical target declared by native generation.
    result = original.replace(prefix, prefix + 'xconfig --startxonboot\n', 1)
    result = result.replace(BROKEN_MARKER, '')
    # The original target-chroot receipt/account/storage settings remain exact.
    result = result.replace(pre_header, guard + pre_header, 1)
    result += '%post --nochroot --interpreter=/usr/bin/python3 --erroronfail\n'
    result += completion + '%end\n'
    return result.encode()


def command(argv, cwd):
    subprocess.run(list(map(str, argv)), cwd=cwd, check=True, timeout=60,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def dump(image, member, destination, output):
    require(not destination.exists(), 'Readback path already exists')
    command(['/usr/sbin/debugfs', '-R', f'dump {member} {destination.name}', image], output)
    return source(destination, secret=True)


def prepare(args):
    require(platform.system() == 'Linux' and os.getuid() == os.geteuid() != 0,
            'Generate only as the ordinary owner in the existing Linux fixture environment')
    transfer_file = source(args.transfer / 'transfer.json', args.transfer_sha256, secret=True)
    transfer = document(transfer_file)
    require(transfer['schema'] == 2 and transfer['start'] == 'install', 'Expected exact fresh-install transfer')
    binding = document(args.binding, args.binding_sha256)
    observation = document(args.grub_observation, args.grub_observation_sha256)
    iso_hash = transfer['files']['installer.iso']['sha256']
    cases_hash = transfer['files']['cases.raw']['sha256']
    require(binding['iso_sha256'] == observation['iso_sha256'] == iso_hash
            and binding['cases']['cases_raw_sha256'] == cases_hash and observation['iso_unchanged'] is True,
            'Instrumentation observations belong to another ISO/cases transfer')
    efi = [item for item in observation['configs'] if item['iso_path'] == 'EFI/BOOT/grub.cfg']
    require(len(efi) == 1 and hashlib.sha256(efi[0]['configuration'].encode()).hexdigest() == efi[0]['sha256']
            and efi[0]['superusers_present'] is False and efi[0]['password_directive_present'] is False,
            'Unexpected original EFI GRUB selection contract')
    lines = [line.strip() for line in efi[0]['configuration'].splitlines()]
    require(lines.count(ORIGINAL_LINUX) == 1 and 'insmod ext2' in lines and 'set timeout=60' in lines
            and 'initrd /images/pxeboot/initrd.img' in lines, 'Original GRUB recipe differs')
    require(re.fullmatch('[a-f0-9]{40}', args.host_revision)
            and re.fullmatch('[a-f0-9]{64}', args.host_launcher_sha256), 'Explicit new host source identity required')
    source(args.transfer / 'cases.raw', cases_hash, secret=True)
    token = hashlib.sha256(('kedra-marker-v1:' + args.mode + ':' + args.transfer_sha256
                            + ':' + args.binding_sha256).encode()).hexdigest()[:32]
    require(args.output.is_absolute() and not args.output.exists()
            and args.output.parent.resolve(strict=True) == args.output.parent, 'Expected a new canonical output directory')
    parent_info = args.output.parent.stat()
    require(stat.S_ISDIR(parent_info.st_mode) and parent_info.st_uid == os.getuid()
            and stat.S_IMODE(parent_info.st_mode) == 0o700, 'Output parent must be owner-private mode 0700')
    args.output.mkdir(mode=0o700)
    image = args.output / 'cases-marker.raw'
    write(image, b'')
    command(['/bin/cp', '--reflink=always', '--', args.transfer / 'cases.raw', image], args.output)
    require(digest(image) == cases_hash and image.stat().st_ino != (args.transfer / 'cases.raw').stat().st_ino,
            'Cases clone is not an independent exact reflink')
    original_json = dump(image, '/cases.json', args.output / 'original-cases.json', args.output)
    require(digest(original_json) == binding['cases']['cases_json_sha256'], 'Cases JSON differs from readback')
    files = {}
    if args.mode == 'install':
        require(args.original_kickstart is not None, 'Private ISO-extracted original Kickstart required')
        original = source(args.original_kickstart, binding['original_kickstart_sha256'], secret=True)
        require(original.stat().st_size <= 65536, 'Oversized original Kickstart')
        raw = original.read_bytes()
        require(b'\r' not in raw, 'Original Kickstart must retain its exact Unix line endings')
        payload = corrected_kickstart(raw.decode('utf-8'), token)
        write(args.output / 'fixture.ks', payload)
        files['fixture.ks'] = {'sha256': digest(args.output / 'fixture.ks'), 'bytes': len(payload)}
        grub = ('search --no-floppy --set=root -l KEDRA-44-Install\n'
                + ORIGINAL_LINUX.replace(ORIGINAL_ARGUMENT, KICKSTART_ARGUMENT) + '\n'
                + 'initrd /images/pxeboot/initrd.img\nboot\n')
    else:
        grub = 'echo KEDRA_EXTERNAL_GRUB_READY_' + token + '\n'
    write(args.output / 'grub-marker.cfg', grub.encode())
    files['grub-marker.cfg'] = {'sha256': digest(args.output / 'grub-marker.cfg'), 'bytes': len(grub.encode())}
    for name in tuple(files):
        command(['/usr/sbin/debugfs', '-w', '-R', f'write {name} /{name}', image], args.output)
        observed = dump(image, '/' + name, args.output / ('readback-' + name), args.output)
        require(digest(observed) == files[name]['sha256'], 'Native filesystem write/readback differs')
        observed.unlink()
    final_json = dump(image, '/cases.json', args.output / 'final-cases.json', args.output)
    require(final_json.read_bytes() == original_json.read_bytes(), 'Existing cases JSON changed')
    final_json.unlink()
    original_json.unlink()
    require(digest(args.transfer / 'cases.raw') == cases_hash, 'Original cases image changed')
    files['cases-marker.raw'] = {'sha256': digest(image), 'bytes': image.stat().st_size}
    value = {'schema': 1, 'kind': 'kedra-external-marker-input', 'mode': args.mode,
             'parent_transfer_sha256': args.transfer_sha256, 'iso_sha256': iso_hash,
             'original_cases_sha256': cases_hash, 'cases_json_sha256': binding['cases']['cases_json_sha256'],
             'original_kickstart_sha256': binding['original_kickstart_sha256'], 'token': token,
             'binding_sha256': args.binding_sha256, 'grub_observation_sha256': args.grub_observation_sha256,
             'generator_sha256': digest(Path(__file__).resolve()), 'host_launcher_sha256': args.host_launcher_sha256,
             'host_revision': args.host_revision, 'files': files,
             'kickstart_argument': KICKSTART_ARGUMENT if args.mode == 'install' else None,
             'grub_command': GRUB_COMMAND, 'os_image_or_iso_rebuilt': False}
    write(args.output / 'marker-input.json', json.dumps(value, sort_keys=True).encode() + b'\n')
    print(json.dumps({'output': str(args.output), 'manifest_sha256': digest(args.output / 'marker-input.json'),
                      'mode': args.mode, 'token': token}))


def load_input(path, expected_hash, transfer_hash, transfer, repository):
    parent_info = path.parent.lstat()
    require(stat.S_ISDIR(parent_info.st_mode) and parent_info.st_uid == os.getuid()
            and stat.S_IMODE(parent_info.st_mode) == 0o700, 'External input directory is not private')
    value = document(source(path, expected_hash, secret=True))
    require(value['schema'] == 1 and value['kind'] == 'kedra-external-marker-input' and value['mode'] == 'install'
            and value['parent_transfer_sha256'] == transfer_hash
            and value['iso_sha256'] == transfer['files']['installer.iso']['sha256']
            and value['original_cases_sha256'] == transfer['files']['cases.raw']['sha256']
            and value['kickstart_argument'] == KICKSTART_ARGUMENT and value['grub_command'] == GRUB_COMMAND
            and re.fullmatch('[a-f0-9]{32}', value['token']) and value['os_image_or_iso_rebuilt'] is False,
            'External marker input differs from the selected transfer/recipe')
    require(set(value['files']) == {'fixture.ks', 'grub-marker.cfg', 'cases-marker.raw'}, 'Unexpected external members')
    for name, record in value['files'].items():
        member = source(path.parent / name, record['sha256'], secret=True)
        require(member.stat().st_size == record['bytes'], 'External member size differs')
    environment = {key: item for key, item in os.environ.items() if not key.startswith('GIT_')}
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null', GIT_OPTIONAL_LOCKS='0')

    def git(*arguments):
        return subprocess.check_output(['/usr/bin/git', '--no-replace-objects', '-C', str(repository),
                                        *arguments], timeout=30, env=environment)

    require(git('rev-parse', '--verify', 'HEAD^{commit}').decode().strip() == value['host_revision'],
            'Actual host revision differs from external instrumentation')
    for name, key in (('boot-macos.py', 'host_launcher_sha256'), ('marker_input.py', 'generator_sha256')):
        relative = 'usr/src/kedra/tests/vm/ghcr-update/' + name
        require(hashlib.sha256(git('show', 'HEAD:' + relative)).hexdigest() == value[key]
                and digest(repository / relative) == value[key], 'Host helper differs from committed selected source')
    return value


class GrubSelection:
    """Observe the qualified menu/empty prompt, then send one fixed command."""

    def __init__(self, qmp, name):
        self.qmp = qmp
        self.name = name
        self.started = time.monotonic()
        self.console_requested = None
        self.commands_sent = False

    def send(self, text):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(5)
            client.connect(str(self.qmp))
            stream = client.makefile('rwb')
            require('QMP' in json.loads(stream.readline(65536)), 'Unexpected QMP greeting')
            count = 0

            def call(operation, arguments=None):
                nonlocal count
                remaining = self.started + 30 - time.monotonic()
                require(remaining > 0, 'External GRUB selection exceeded 30 seconds')
                client.settimeout(min(5, remaining))
                count += 1
                payload = {'execute': operation, 'id': count}
                if arguments is not None:
                    payload['arguments'] = arguments
                stream.write(json.dumps(payload).encode() + b'\n')
                stream.flush()
                for _ in range(128):
                    remaining = self.started + 30 - time.monotonic()
                    require(remaining > 0, 'External GRUB selection exceeded 30 seconds')
                    client.settimeout(min(5, remaining))
                    result = json.loads(stream.readline(65536))
                    if result.get('id') == count:
                        require('return' in result, 'Fixed GRUB keyboard request failed')
                        return result['return']
                raise RuntimeError('QMP response bound exceeded')

            call('qmp_capabilities')
            require(call('query-name').get('name') == self.name, 'QMP belongs to another fixture')
            for character in text:
                special = {' ': ('spc', False), '-': ('minus', False), '_': ('minus', True),
                           '/': ('slash', False), '.': ('dot', False), '=': ('equal', False),
                           ';': ('semicolon', False), '\n': ('ret', False)}
                if character in special:
                    key, shifted = special[character]
                else:
                    require(character.isascii() and character.isalpha(), 'Unsupported fixed GRUB character')
                    key, shifted = character.lower(), character.isupper()
                keys = ([{'type': 'qcode', 'data': 'shift'}] if shifted else [])
                keys.append({'type': 'qcode', 'data': key})
                call('send-key', {'keys': keys, 'hold-time': 40})
                time.sleep(0.08)

    def observe(self, output):
        if self.commands_sent:
            return
        clean = ANSI.sub('', output)
        require(not any(item in clean for item in ('Linux version', 'EFI stub:', 'Starting kernel')),
                'Default kernel started before external Kickstart selection')
        require(time.monotonic() - self.started < 30, 'External GRUB selection exceeded 30 seconds')
        if self.console_requested is None:
            if 'Install Kedra (Fedora 44)' in clean and 'executed automatically' in clean:
                self.send('c')
                self.console_requested = time.monotonic()
        else:
            require(time.monotonic() - self.console_requested < 5, 'Qualified GRUB prompt did not appear')
            if 'grub>' in clean:
                require(clean.rsplit('grub>', 1)[1] == ' ', 'GRUB prompt is not empty; refusing duplicate input')
                self.send(GRUB_COMMAND + '\n')
                require(time.monotonic() - self.started < 30, 'Late external GRUB selection refused')
                self.commands_sent = True


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode', choices=('probe', 'install'), required=True)
    parser.add_argument('--transfer', type=Path, required=True)
    parser.add_argument('--transfer-sha256', required=True)
    parser.add_argument('--binding', type=Path, required=True)
    parser.add_argument('--binding-sha256', required=True)
    parser.add_argument('--grub-observation', type=Path, required=True)
    parser.add_argument('--grub-observation-sha256', required=True)
    parser.add_argument('--original-kickstart', type=Path)
    parser.add_argument('--host-revision', required=True)
    parser.add_argument('--host-launcher-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    prepare(parser.parse_args())


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit('Marker input refused: ' + str(error)) from error
