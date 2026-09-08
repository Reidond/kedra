"""Boot extracted ISO kernel/initramfs with the unchanged ISO stage2, no disks."""
import argparse
import pathlib
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--iso', required=True, type=pathlib.Path)
parser.add_argument('--kernel-dir', required=True, type=pathlib.Path)
parser.add_argument('--work', required=True, type=pathlib.Path)
args = parser.parse_args()
args.work.mkdir(parents=True, exist_ok=True)
events = args.work / 'events.log'
command = [
    'qemu-system-x86_64', '-machine', 'q35,accel=kvm', '-cpu', 'host',
    '-smp', '2', '-m', '6144', '-device', 'virtio-rng-pci',
    '-kernel', str(args.kernel_dir / 'vmlinuz'),
    '-initrd', str(args.kernel_dir / 'initrd.img'),
    '-append', 'inst.stage2=hd:LABEL=KEDRA-44-Install console=ttyS0,115200 console=tty0 inst.graphical kedra.research=1',
    '-drive', f'file={args.iso},media=cdrom,readonly=on',
    '-display', 'none', '-vga', 'std', '-nic', 'none', '-monitor', 'none',
    '-serial', f'file:{args.work / "serial.log"}', '-serial', f'file:{events}',
]
with (args.work / 'qemu.log').open('w') as log:
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    try:
        deadline = time.monotonic() + 180
        while process.poll() is None and time.monotonic() < deadline:
            if events.exists() and 'KEDRA_INSTALLER_ANACONDA_STARTED' in events.read_text(errors='replace'):
                if 'KEDRA_INSTALLER_SIGNED_PAYLOAD_PASS' not in events.read_text(errors='replace'):
                    raise RuntimeError('Anaconda started without successful embedded-payload verification')
                print('PASS: offline signed payload verification and Anaconda startup; no disks attached')
                break
            time.sleep(1)
        else:
            raise SystemExit('Anaconda startup smoke did not pass')
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
