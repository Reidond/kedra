"""Exercise the installer fstab transformation without touching a mounted root."""
import pathlib
import runpy

module = runpy.run_path(str(pathlib.Path(__file__).with_name('finalize-fstab.py')))
normalize = module['normalize']
deployment_path = module['deployment_path']
original = '# native comment\nUUID=root / btrfs subvol=root,compress=zstd:1,rw 0 0\nUUID=home /home btrfs subvol=home 0 0\n'
expected = '# native comment\nUUID=root /sysroot btrfs subvol=root,compress=zstd:1,ro 0 0\nUUID=home /home btrfs subvol=home 0 0\n'
assert normalize(original) == expected
assert normalize(expected) == expected
assert normalize('UUID=root\t/\text4\tdefaults\t1\t1') == 'UUID=root\t/sysroot\text4\tdefaults,ro\t1\t1'
for source in ['', 'UUID=x / btrfs defaults 0 0\nUUID=y /sysroot btrfs defaults 0 0\n',
               'UUID=x / unknown defaults 0 0\n', 'UUID=x / btrfs defaults\n']:
    try:
        normalize(source)
        raise AssertionError('Unexpected root layout was accepted')
    except ValueError:
        pass
physical = pathlib.Path('/synthetic-physical')
valid = physical / 'ostree/deploy/default/deploy' / ('a' * 64 + '.0')
assert deployment_path(physical, str(valid) + '\n') == valid
for reported in ['/etc', 'relative', str(physical / 'ostree/deploy/other/deploy' / valid.name), str(valid) + '/extra']:
    try:
        deployment_path(physical, reported)
        raise AssertionError('Unexpected deployment path was accepted')
    except RuntimeError:
        pass
print('PASS: seven installer fstab and five native deployment-path cases')
