"""Serial completion markers written by the disposable installer's Kickstart.

The generated Python snippets run in the disposable installer with its own
interpreter; prepare-install-arm64.py embeds them and boot-arm64.py waits for
the markers on the VM's serial log.
"""

COMPLETE = 'KEDRA_FIXTURE_INSTALL_COMPLETE'


def emit_script(marker, *, nonblocking=False):
    # No O_CREAT/TRUNC: even a mistaken chroot cannot create or alter a regular file.
    flags = 'os.O_WRONLY|os.O_NOFOLLOW|os.O_NOCTTY' + ('|os.O_NONBLOCK' if nonblocking else '')
    return f'''# Runs in the disposable installer, using its own Python interpreter.
import os, stat
device='/dev/ttyAMA0'
before=os.stat(device,follow_symlinks=False)
if not stat.S_ISCHR(before.st_mode): raise RuntimeError('serial endpoint is not a character device')
descriptor=os.open(device,{flags})
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
