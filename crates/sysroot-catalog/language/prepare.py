# Executes inside the exact admitted Linux builder, using its Python interpreter.
# This fixed driver is an engine build input, never a host-side recipe evaluator.
import json
import os
import re
import shutil
import stat
import subprocess
import sys
from pathlib import Path


def require(ok):
    if not ok:
        raise RuntimeError('Unsafe or inexact source preparation')


def relative(value):
    require(isinstance(value, str) and value.isascii() and 0 < len(value) <= 4096
            and '\\' not in value and ':' not in value
            and all(part not in ('', '.', '..', '.git') for part in value.split('/'))
            and not any(ord(c) < 32 or ord(c) == 127 for c in value))
    return value


def ordinary(root, name, exists=True):
    parts = relative(name).split('/')
    parent = root
    for part in parts[:-1]:
        parent /= part
        require(not parent.is_symlink())
        if parent.exists():
            require(parent.is_dir())
        elif not exists:
            parent.mkdir(mode=0o755)
        else:
            require(False)
    path = parent / parts[-1]
    require(not path.is_symlink())
    if exists:
        info = path.lstat()
        require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1)
    else:
        require(not path.exists())
    return path


def patch(root, resources, item):
    data = ordinary(resources, item['file']).read_bytes()
    require(len(data) <= 8 * 1024**2 and b'\0' not in data)
    lines = data.decode('utf-8').splitlines()
    index = 0
    touched = set()
    while index < len(lines):
        require(lines[index].startswith('--- '))
        require(index + 1 < len(lines) and lines[index + 1].startswith('+++ '))
        paths = []
        for line in lines[index:index + 2]:
            header = line[4:]
            require('\t' not in header and ' ' not in header)
            components = relative(header).split('/')
            require(type(item['strip']) is int and 0 <= item['strip'] < len(components))
            paths.append('/'.join(components[item['strip']:]))
        require(paths[0] == paths[1] and paths[0] not in touched)
        ordinary(root, paths[0])
        touched.add(paths[0])
        index += 2
        hunks = 0
        while index < len(lines) and not lines[index].startswith('--- '):
            require(re.fullmatch(r'@@ -[0-9]+(?:,[0-9]+)? \+[0-9]+(?:,[0-9]+)? @@.*', lines[index]))
            hunks += 1
            index += 1
            while index < len(lines) and not lines[index].startswith(('@@ ', '--- ')):
                require(lines[index].startswith((' ', '+', '-', '\\ No newline at end of file')))
                index += 1
        require(hunks > 0)
    require(touched)
    process = subprocess.run(['/usr/bin/patch', '--batch', '--forward', '--fuzz=0',
                              '--no-backup-if-mismatch', '-p' + str(item['strip']),
                              '--input', str(resources / item['file'])],
                             cwd=root, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT, timeout=120, check=False,
                             env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C'})
    require(process.returncode == 0
            and re.search(rb'^Hunk #[0-9]+ .*\b(?:offset|fuzz)\b', process.stdout,
                          re.MULTILINE | re.IGNORECASE) is None)
    for name in touched:
        ordinary(root, name)


def main():
    source, output, resources = map(Path, sys.argv[1:4])
    require(len(sys.argv) == 4 and source.is_dir() and output.is_dir() and resources.is_dir())
    manifest = json.loads(ordinary(resources, 'prepare.json').read_bytes())
    require(set(manifest) == {'patches', 'add', 'replace'} and len(manifest['patches']) <= 256)
    for member in source.iterdir():
        destination = output / member.name
        require(not destination.exists() and not destination.is_symlink())
        if member.is_symlink():
            destination.symlink_to(os.readlink(member))
        elif member.is_dir():
            shutil.copytree(member, destination, symlinks=True)
        else:
            require(member.is_file() and member.stat().st_nlink == 1)
            shutil.copy2(member, destination)
    for directory, names, files in os.walk(output, followlinks=False):
        Path(directory).chmod(0o755)
        for name in files:
            path = Path(directory) / name
            if not path.is_symlink():
                path.chmod(0o755 if path.stat().st_mode & 0o111 else 0o644)
    for item in manifest['patches']:
        require(set(item) == {'strip', 'file'})
        patch(output, resources, item)
    require(not set(manifest['add']).intersection(manifest['replace']))
    for operation in ('add', 'replace'):
        for name, item in sorted(manifest[operation].items()):
            require(set(item) == {'file', 'mode'} and item['mode'] in (0o644, 0o755))
            destination = ordinary(output, name, exists=operation == 'replace')
            content = ordinary(resources, item['file']).read_bytes()
            if operation == 'replace':
                destination.unlink()
            with destination.open('xb') as stream:
                stream.write(content)
            destination.chmod(item['mode'])


if __name__ == '__main__':
    main()
