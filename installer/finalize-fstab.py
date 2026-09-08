"""Installer-only normalization of the physical-root fstab entry for bootc."""
import os
import pathlib
import re
import stat

def normalize(source):
    lines = source.splitlines(keepends=True)
    roots = [i for i, line in enumerate(lines)
             if line.strip() and not line.lstrip().startswith('#')
             and len(line.split()) >= 2 and line.split()[1] in ['/', '/sysroot']]
    if len(roots) != 1:
        raise ValueError('Expected exactly one physical-root fstab entry')
    index = roots[0]
    match = re.fullmatch(r'(\S+[ \t]+)(/|/sysroot)([ \t]+)(btrfs|ext4|xfs)([ \t]+)(\S+)([ \t]+[0-9]+[ \t]+[0-9]+[ \t]*)(\n?)', lines[index])
    if match is None:
        raise ValueError('Review unsupported physical-root fstab format')
    options = [option for option in match[6].split(',') if option not in ['ro', 'rw']]
    options.append('ro')
    lines[index] = match[1] + '/sysroot' + match[3] + match[4] + match[5] + ','.join(options) + match[7] + match[8]
    return ''.join(lines)

def main():
    # Fixed Anaconda deployment bind; no caller-controlled target or device names.
    root = pathlib.Path('/mnt/sysroot')
    if not root.is_mount() or root.resolve() == pathlib.Path('/'):
        raise RuntimeError('Expected the mounted installer deployment')
    if not (root / 'usr/share/sysroot/source.json').is_file() or not (root / 'usr/bin/bootc').is_file():
        raise RuntimeError('Expected an installed Kedra bootc payload')
    path = root / 'etc/fstab'
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != 0 or metadata.st_nlink != 1 or metadata.st_size > 65536:
        raise RuntimeError('Expected a bounded root-owned regular fstab')
    source = path.read_text()
    updated = normalize(source)
    if updated != source:
        # Preserve the native file's metadata/SELinux label. Installation is offline
        # and Anaconda's post phase serializes configuration writers.
        with path.open('w') as output:
            output.write(updated)
            output.flush()
            os.fsync(output.fileno())
    print('Kedra physical-root fstab normalization complete')

if __name__ == '__main__':
    main()
