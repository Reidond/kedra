#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Prepare an isolated public installer checkout with generated fixture inputs.

Only public authority files and fixture-only installer configuration are changed
in the new checkout. Product verifier/helper and install orchestration remain the
dispatched source. Credentials/media stay private and are not evidence.
"""
import argparse
import io
import json
import re
import secrets
import subprocess
import tarfile
from pathlib import Path

from fixture import add_context_argument, load_context

ROOT = Path(__file__).resolve().parents[6]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    add_context_argument(parser)
    args = parser.parse_args()
    fixture = load_context(args.fixture_context)
    root = args.root.resolve()
    if root != Path(fixture['runner_temp']) / 'kedra-ghcr':
        parser.error('wrong fixture directory')
    private = Path(fixture['runner_temp']) / 'kedra-ghcr-private'
    checkout = private / 'installer-checkout'
    checkout.mkdir(mode=0o700)
    revision = fixture['source_revision']
    if not re.fullmatch('[a-f0-9]{40}', revision):
        parser.error('invalid dispatched revision')
    archive = subprocess.check_output(['git', '--no-replace-objects', '-C', str(ROOT),
        'archive', '--format=tar', revision], timeout=120)
    with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
        stream.extractall(checkout, filter='data')
    authority = json.loads((root / 'fixture-authority.json').read_text())
    public = root / 'context/release.pub'
    trust = checkout / 'usr/src/kedra/image/release/authority'
    (trust / 'qemu-arm64.pub').write_bytes(public.read_bytes())
    (trust / 'qemu-arm64.sha256').write_text(authority['fingerprint'] + '\n')
    passphrase = secrets.token_hex(24)
    passfile = private / 'disk-passphrase'
    passfile.write_text(passphrase + '\n')
    passfile.chmod(0o600)
    password = secrets.token_hex(24)
    hashed = subprocess.check_output(['openssl', 'passwd', '-6', '-stdin'],
                                    input=password.encode(), timeout=30).decode().strip()
    if not re.fullmatch(r'\$6\$[A-Za-z0-9./]+\$[A-Za-z0-9./]+', hashed):
        raise RuntimeError('unexpected generated account hash')
    kickstart = '''#version=DEVEL
text
lang en_US.UTF-8
keyboard us
timezone UTC --utc
network --bootproto=dhcp --device=link --activate
rootpw --lock
user --name=kedra-test --groups=wheel --password=ACCOUNT_HASH --iscrypted
shutdown
%pre --interpreter=/usr/bin/python3 --erroronfail
from pathlib import Path
import re, stat, subprocess
assert subprocess.check_output(['systemd-detect-virt','--vm'],text=True).strip() in ('qemu','kvm')
target=Path('/dev/disk/by-id/virtio-KEDRA_INSTALL_ONLY').resolve(strict=True)
keep=Path('/dev/disk/by-id/virtio-KEDRA_KEEP_DATA').resolve(strict=True)
assert target != keep and stat.S_ISBLK(target.stat().st_mode) and stat.S_ISBLK(keep.stat().st_mode)
assert re.fullmatch('[a-z][a-z0-9]*',target.name)
assert subprocess.check_output(['blockdev','--getsize64',str(target)],text=True).strip() == str(96*1024**3)
assert subprocess.check_output(['blockdev','--getsize64',str(keep)],text=True).strip() == str(16*1024**2)
Path('/tmp/kedra-fixture-storage.ks').write_text('ignoredisk --only-use='+target.name+'\\nclearpart --all --initlabel --drives='+target.name+'\\nautopart --type=btrfs --encrypted --passphrase=DISK_PASSPHRASE\\n')
%end
%include /tmp/kedra-fixture-storage.ks
%include /usr/share/anaconda/interactive-defaults.ks
%post --interpreter=/usr/bin/python3 --erroronfail
from pathlib import Path
import json
root=Path('/var/lib/kedra-ghcr-test')
root.mkdir(mode=0o700,parents=True,exist_ok=True)
(root/'fresh-install.json').write_text(json.dumps({'schema_version':1,'installer':'anaconda','target_serial':'KEDRA_INSTALL_ONLY','sentinel_serial':'KEDRA_KEEP_DATA'})+'\\n')
Path('/dev/ttyAMA0').write_text('KEDRA_FIXTURE_INSTALL_COMPLETE\\n')
%end
'''.replace('ACCOUNT_HASH', hashed).replace('DISK_PASSPHRASE', passphrase)
    media = checkout / 'usr/src/kedra/installer/media'
    # The normal builder copies a closed context. Add only generated data to
    # its fixture media input and include it in the existing initramfs build.
    # No verifier or public entrypoint gains a bypass/Kickstart argument.
    prepare = media / 'prepare.sh'
    original = prepare.read_text()
    needle = 'DRACUT_NO_XATTR=1 dracut --force --zstd --reproducible --no-hostonly --add anaconda'
    if original.count(needle) != 1:
        raise RuntimeError('public installer initramfs recipe differs from reviewed fixture adaptation')
    generated = ("# Private generated fixture Kickstart; no verifier or boot-policy changes.\n"
                 "install -m 0600 /dev/null /usr/share/anaconda/fixture.ks\n"
                 "cat > /usr/share/anaconda/fixture.ks <<'KEDRA_GENERATED_FIXTURE_KS'\n"
                 + kickstart + "KEDRA_GENERATED_FIXTURE_KS\n")
    prepare.write_text(original.replace(needle, generated + needle
        + " --install '/usr/share/anaconda/fixture.ks /usr/share/anaconda/interactive-defaults.ks'"))
    iso = media / 'iso.yaml'
    value = iso.read_text()
    old = 'console=tty0 inst.graphical quiet'
    if value.count(old) != 1:
        raise RuntimeError('public installer boot entry differs from reviewed fixture adaptation')
    iso.write_text(value.replace(old,
        'console=tty0 console=ttyAMA0,115200 inst.text inst.ks=file:/usr/share/anaconda/fixture.ks'))
    report = {'schema_version': 1, 'source_revision': revision,
              'fixture_authority': authority['fingerprint'], 'target_serial': 'KEDRA_INSTALL_ONLY',
              'sentinel_serial': 'KEDRA_KEEP_DATA', 'target_bytes': 96 * 1024**3,
              'sentinel_bytes': 16 * 1024**2, 'installer': 'anaconda', 'fresh_installation_performed': False,
              'fixture_inputs': ['authority/qemu-arm64.pub', 'authority/qemu-arm64.sha256',
                                 'media/prepare.sh generated Kickstart', 'media/iso.yaml boot arguments']}
    (root / 'installation-plan.json').write_text(json.dumps(report, indent=2) + '\n')
    print(checkout)


if __name__ == '__main__':
    main()
