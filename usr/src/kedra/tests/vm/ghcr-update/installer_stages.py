# Runs in the disposable installer with its own Python interpreter; host imports only constants.
"""Bounded fixture-only stage observations; never installer success criteria."""
import os
import re
import select
import stat
import subprocess
import time

PREFIX = 'KEDRA_INSTALL_STAGE_'
UNITS = {'PRE': 'anaconda-pre.service', 'VERIFY': 'kedra-installer-verify.service',
         'WRAPPER': 'anaconda.service', 'DIRECT': 'anaconda-direct.service'}
STATES = ('inactive', 'activating', 'active', 'deactivating', 'failed', 'reloading')
RESULTS = ('success', 'resources', 'timeout', 'exit-code', 'signal', 'core-dump',
           'watchdog', 'start-limit-hit', 'exec-condition', 'protocol')
LOGS = {'ANACONDA': '/tmp/anaconda.log', 'STORAGE': '/tmp/storage.log',
        'PROGRAM': '/tmp/program.log', 'PACKAGING': '/tmp/packaging.log'}
PATTERNS = {'TRACEBACK': b'Traceback (most recent call last):',
            'KICKSTART_ERROR': b'KickstartError', 'NO_SPACE': b'No space left on device'}
TOKENS = {'OBSERVER_READY', 'OBSERVER_LIMIT', 'PRE_ENTERED', 'STORAGE_VALIDATED',
          'STORAGE_INCLUDE_WRITTEN', 'CHROOT_POST_ENTERED', 'CHROOT_POST_COMPLETE',
          'NOCHROOT_POST_ENTERED', 'NOCHROOT_POST_COMPLETE', 'TMUX_UNAVAILABLE',
          'TMUX_MAIN_DEAD', 'TMUX_MAIN_LIVE_ANACONDA', 'TMUX_MAIN_LIVE_PYTHON',
          'TMUX_MAIN_LIVE_OTHER'}
for label in UNITS:
    TOKENS.update('UNIT_' + label + '_' + name for name in ('UNAVAILABLE', 'STARTED', 'EXITED'))
    TOKENS.update('UNIT_' + label + '_STATE_' + value.upper() for value in (*STATES, 'other'))
    TOKENS.update('UNIT_' + label + '_RESULT_' + value.upper().replace('-', '_') for value in (*RESULTS, 'other'))
    TOKENS.update('UNIT_' + label + '_STATUS_' + str(value) for value in range(256))
for label in LOGS:
    TOKENS.add('LOG_' + label + '_PRESENT')
    TOKENS.update('LOG_' + label + '_' + name for name in PATTERNS)


def capture(argv):
    """Fixed metadata commands only: 2 KiB output, three seconds, owned child cleanup."""
    try:
        child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.DEVNULL, env={'PATH': '/usr/sbin:/usr/bin', 'LC_ALL': 'C'})
    except OSError:
        return None
    deadline = time.monotonic() + 3
    data = bytearray()
    try:
        while len(data) <= 2048:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([child.stdout], [], [], remaining)[0]:
                return None
            block = os.read(child.stdout.fileno(), 2049 - len(data))
            if not block:
                remaining = deadline - time.monotonic()
                if remaining > 0 and child.wait(timeout=remaining) == 0:
                    return bytes(data)
                return None
            data.extend(block)
        return None
    except (OSError, subprocess.SubprocessError):
        return None
    finally:
        child.stdout.close()
        if child.poll() is None:
            child.kill()
            child.wait(timeout=3)


def main():
    seen = set()

    def emit(token):
        if token not in TOKENS or token in seen or len(seen) >= 128:
            return
        if len(seen) == 127:
            token = 'OBSERVER_LIMIT'
        try:
            descriptor = os.open('/dev/ttyAMA0', os.O_WRONLY | os.O_NOFOLLOW | os.O_NOCTTY | os.O_NONBLOCK)
            try:
                if stat.S_ISCHR(os.fstat(descriptor).st_mode):
                    payload = ('\n' + PREFIX + token + '\n').encode()
                    if os.write(descriptor, payload) == len(payload):
                        seen.add(token)
            finally:
                os.close(descriptor)
        except OSError:
            return

    emit('OBSERVER_READY')
    deadline = time.monotonic() + 7200
    while time.monotonic() < deadline and len(seen) < 128:
        for label, unit in UNITS.items():
            raw = capture(['/usr/bin/systemctl', 'show', '--property=ActiveState,Result,ExecMainStatus,ExecMainStartTimestampMonotonic,ExecMainExitTimestampMonotonic', unit])
            try:
                values = dict(line.split('=', 1) for line in raw.decode('ascii').splitlines()) if raw else {}
                state = values.get('ActiveState', '')
                result = values.get('Result', '')
                status = values.get('ExecMainStatus', '')
                started = values.get('ExecMainStartTimestampMonotonic', '')
                exited = values.get('ExecMainExitTimestampMonotonic', '')
                if not values:
                    emit('UNIT_' + label + '_UNAVAILABLE')
                    continue
                emit('UNIT_' + label + '_STATE_' + (state if state in STATES else 'other').upper())
                if re.fullmatch('[0-9]{1,20}', started) and int(started) > 0:
                    emit('UNIT_' + label + '_STARTED')
                if re.fullmatch('[0-9]{1,20}', exited) and int(exited) > 0:
                    emit('UNIT_' + label + '_EXITED')
                    emit('UNIT_' + label + '_RESULT_' + (result if result in RESULTS else 'other').upper().replace('-', '_'))
                    if re.fullmatch('[0-9]{1,3}', status) and int(status) < 256:
                        emit('UNIT_' + label + '_STATUS_' + str(int(status)))
            except (UnicodeError, ValueError):
                emit('UNIT_' + label + '_UNAVAILABLE')
        pane = capture(['/usr/bin/tmux', 'display-message', '-p', '-t', 'anaconda:main.0',
                        '#{pane_dead}|#{pane_current_command}'])
        if pane is None or not re.fullmatch(rb'[01]\|[A-Za-z0-9_.+-]{0,64}\n', pane):
            emit('TMUX_UNAVAILABLE')
        elif pane.startswith(b'1|'):
            emit('TMUX_MAIN_DEAD')
        elif pane == b'0|anaconda\n':
            emit('TMUX_MAIN_LIVE_ANACONDA')
        elif pane in (b'0|python3\n', b'0|python\n'):
            emit('TMUX_MAIN_LIVE_PYTHON')
        else:
            emit('TMUX_MAIN_LIVE_OTHER')
        for label, path in LOGS.items():
            try:
                descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                with os.fdopen(descriptor, 'rb') as stream:
                    info = os.fstat(stream.fileno())
                    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                        continue
                    if info.st_size:
                        emit('LOG_' + label + '_PRESENT')
                    stream.seek(max(0, info.st_size - 65536))
                    data = stream.read(65536)
                for name, pattern in PATTERNS.items():
                    if pattern in data:
                        emit('LOG_' + label + '_' + name)
            except OSError:
                continue
        time.sleep(20)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError):
        # Observer failure must not print private exceptions or change installation.
        pass
