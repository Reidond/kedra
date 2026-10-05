# Guest observer uses the installer Python; host imports bounded diagnostic helpers only.
"""Bounded fixture-only stage observations; never installer success criteria."""
import json
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
        'PROGRAM': '/tmp/program.log', 'PACKAGING': '/tmp/packaging.log', 'DBUS': '/tmp/dbus.log'}
PATTERNS = {'TRACEBACK': b'Traceback (most recent call last):',
            'KICKSTART_ERROR': b'KickstartError', 'NO_SPACE': b'No space left on device',
            'STORAGE_LAYOUT_ERROR': b'Failed to create storage layout:',
            'PAYLOAD_INSTALLATION_ERROR': b'PayloadInstallationError', 'MEMORY_ERROR': b'MemoryError'}
# Anaconda44.30 main UI and D-Bus modules use distinct log formats and files.
STAMP = rb'(?m)^[0-9]{2}:[0-9]{2}:[0-9]{2},[0-9]{3} '
TASKS = {'STORAGE': b'Create storage layout', 'MOUNT': b'Mount filesystems',
         'BOOTC_ARGS': b'Collect kernel arguments for bootc', 'BOOTC': b'Deploy bootc'}
RECORD_PATTERNS = {label: {} for label in ('ANACONDA', 'DBUS', 'STORAGE', 'PACKAGING')}
for task, name in TASKS.items():
    for state, level, prefix in (('STARTED', b'INFO', b'Task started: '), ('COMPLETED', b'DEBUG', b'Task completed: ')):
        RECORD_PATTERNS['DBUS']['TASK_' + task + '_' + state] = re.compile(
            rb'(?m)^' + level + rb':anaconda\.modules\.boss\.installation:' + re.escape(prefix + name + b' ('))
for category, literal in (('BOOTC_RUN', b'Run the bootc based installation'),
                          ('BOOTC_EXEC', b'Executing bootc install command'),
                          ('BOOTC_DEPLOY_COMPLETE', b'Bootc deploy complete')):
    RECORD_PATTERNS['PACKAGING'][category] = re.compile(
        rb'(?m)^(?:DEBUG|INFO):anaconda\.modules\.payloads\.payload\.rpm_ostree\.installation:'
        + re.escape(literal) + rb'\r?$')
for label in ('DBUS', 'STORAGE', 'PACKAGING'):
    RECORD_PATTERNS[label]['TASK_THREAD_FAILED'] = re.compile(
        rb'(?m)^ERROR:anaconda\.modules\.common\.task\.task:Thread [A-Za-z0-9_-]{1,128} has failed: ')
RECORD_PATTERNS['STORAGE']['AUTOPART_MODEL_STARTED'] = re.compile(
    rb'(?m)^DEBUG:anaconda\.modules\.storage\.partitioning\.automatic\.automatic_partitioning:'
    rb'Executing the automatic partitioning\.\r?$')
RECORD_PATTERNS['ANACONDA']['STORAGE_SPOKE_INITIALIZED'] = re.compile(
    STAMP + rb'INF lifecycle: Module initialized: StorageSpoke\r?$')
RECORD_PATTERNS['ANACONDA']['INSTALLATION_STARTED'] = re.compile(
    STAMP + rb'DBG ui\.tui\.spokes\.installation_progress: The installation has started\.\r?$')
COMMAND_REASONS = ('LAUNCH', 'TIMEOUT', 'NONZERO', 'READ_LIMIT', 'READ_ERROR', 'MALFORMED', 'CLEANUP')
TOKENS = {'OBSERVER_READY', 'OBSERVER_LIMIT', 'PRE_ENTERED', 'STORAGE_VALIDATED',
          'STORAGE_INCLUDE_WRITTEN', 'CHROOT_POST_ENTERED', 'CHROOT_POST_COMPLETE',
          'NOCHROOT_POST_ENTERED', 'NOCHROOT_POST_COMPLETE', 'TMUX_UNAVAILABLE',
          'TMUX_MAIN_DEAD', 'TMUX_MAIN_LIVE_ANACONDA', 'TMUX_MAIN_LIVE_PYTHON',
          'TMUX_MAIN_LIVE_OTHER', 'METADATA_CLEANUP_UNCERTAIN'}
for command in (*UNITS, 'TMUX'):
    TOKENS.update('CAPTURE_' + command + '_' + reason for reason in COMMAND_REASONS)
for label in UNITS:
    TOKENS.update('UNIT_' + label + '_' + name for name in ('UNAVAILABLE', 'STARTED', 'EXITED'))
    TOKENS.update('UNIT_' + label + '_STATE_' + value.upper() for value in (*STATES, 'other'))
    TOKENS.update('UNIT_' + label + '_RESULT_' + value.upper().replace('-', '_') for value in (*RESULTS, 'other'))
    TOKENS.update('UNIT_' + label + '_STATUS_' + str(value) for value in range(256))
for label in LOGS:
    TOKENS.update(('LOG_' + label + '_PRESENT', 'LOG_' + label + '_TAIL_LIMIT'))
    TOKENS.update('LOG_' + label + '_' + name for name in PATTERNS)
for label, patterns in RECORD_PATTERNS.items():
    TOKENS.update('LOG_' + label + '_' + name for name in patterns)


HEALTH_PREFIX = 'KEDRA_INSTALL_HEALTH '
HEALTH_INTERVAL = 30
HEALTH_SAMPLES = 240
MAX_NUMBER = 2**63 - 1
GUEST_METRICS = ('elapsed_ms', 'mem_available_kib', 'swap_free_kib', 'pgmajfault',
                 'oom_kill', 'iowait_ticks', 'root_available_bytes')


def number(value):
    return value if type(value) is int and 0 <= value <= MAX_NUMBER else None


def bounded_read(path, limit=16384, first_line=False):
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, 'rb') as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                return None
            data = stream.readline(limit + 1) if first_line else stream.read(limit + 1)
        return data.decode('ascii') if len(data) <= limit else None
    except (OSError, UnicodeError):
        return None


def counters(path, keys, kib=False):
    result = dict.fromkeys(keys)
    data = bounded_read(path)
    for line in (data or '').splitlines():
        parts = line.split()
        if (len(parts) == (3 if kib else 2) and parts[0].rstrip(':') in result
                and (not kib or parts[2] == 'kB') and re.fullmatch('[0-9]{1,20}', parts[1])):
            result[parts[0].rstrip(':')] = number(int(parts[1]))
    return result


def available_bytes(path):
    try:
        value = os.statvfs(path)
        return number(value.f_bavail * value.f_frsize)
    except OSError:
        return None


def guest_health(sequence, started):
    memory = counters('/proc/meminfo', ('MemAvailable', 'SwapFree'), kib=True)
    virtual = counters('/proc/vmstat', ('pgmajfault', 'oom_kill'))
    cpu = (bounded_read('/proc/stat', 512, first_line=True) or '').split()
    iowait = number(int(cpu[5])) if len(cpu) > 5 and cpu[0] == 'cpu' and cpu[5].isdigit() else None
    return {'seq': sequence, 'elapsed_ms': int((time.monotonic() - started) * 1000),
            'mem_available_kib': memory['MemAvailable'], 'swap_free_kib': memory['SwapFree'],
            'pgmajfault': virtual['pgmajfault'], 'oom_kill': virtual['oom_kill'],
            'iowait_ticks': iowait, 'root_available_bytes': available_bytes('/')}


def send_health(frame):
    payload = ('\n' + HEALTH_PREFIX + json.dumps(frame, separators=(',', ':')) + '\n').encode()
    if len(payload) > 512:
        return
    try:
        descriptor = os.open('/dev/ttyAMA0', os.O_WRONLY | os.O_NOFOLLOW | os.O_NOCTTY | os.O_NONBLOCK)
        try:
            if stat.S_ISCHR(os.fstat(descriptor).st_mode):
                os.write(descriptor, payload)
        finally:
            os.close(descriptor)
    except OSError:
        pass


def summarize(summary, values):
    for key, value in values.items():
        if number(value) is None:
            missing = summary.setdefault('unavailable_counts', {})
            missing[key] = missing.get(key, 0) + 1
            continue
        metrics = summary.setdefault('metrics', {})
        if key not in metrics:
            metrics[key] = {'first': value, 'last': value, 'min': value, 'max': value, 'count': 1}
        else:
            item = metrics[key]
            item.update(last=value, min=min(item['min'], value), max=max(item['max'], value), count=item['count'] + 1)


def observe_health(text, progress, elapsed_ms):
    summary = progress.setdefault('guest_health', {'sample_count': 0, 'last_sequence': 0})
    for match in re.finditer(r'(?:^|\n)KEDRA_INSTALL_HEALTH (\{[^\r\n]{1,489}\})\r?(?=\n|$)', text):
        try:
            frame = json.loads(match[1])
        except (ValueError, TypeError):
            continue
        if not isinstance(frame, dict) or set(frame) != {'seq', *GUEST_METRICS}:
            continue
        sequence = frame['seq']
        if number(sequence) is None or not summary['last_sequence'] < sequence <= HEALTH_SAMPLES:
            continue
        if number(frame['elapsed_ms']) is None or frame['elapsed_ms'] > 7200000:
            continue
        if any(value is not None and number(value) is None for key, value in frame.items() if key != 'seq'):
            continue
        # Only a new valid sequence advances liveness; rescanned old serial cannot.
        summary.setdefault('first_seen_host_elapsed_ms', elapsed_ms)
        summary['last_seen_host_elapsed_ms'] = elapsed_ms
        summary['sequence_gaps'] = summary.get('sequence_gaps', 0) + sequence - summary['last_sequence'] - 1
        summary['last_sequence'] = sequence
        summary['sample_count'] += 1
        summarize(summary, {key: frame[key] for key in GUEST_METRICS})


def sample_host_health(process, root, progress, elapsed_ms):
    summary = progress.setdefault('host_qemu_health', {'sample_count': 0})
    if process.poll() is not None or summary['sample_count'] >= HEALTH_SAMPLES:
        return
    if elapsed_ms < progress.get('next_host_health_ms', 0):
        return
    progress['next_host_health_ms'] = elapsed_ms + HEALTH_INTERVAL * 1000
    summary.setdefault('first_sample_host_elapsed_ms', elapsed_ms)
    summary['last_sample_host_elapsed_ms'] = elapsed_ms
    summary['sample_count'] += 1
    values = dict.fromkeys(('qemu_user_ticks', 'qemu_system_ticks', 'qemu_rss_bytes',
                            'qemu_read_bytes', 'qemu_write_bytes'))
    raw = bounded_read('/proc/' + str(process.pid) + '/stat', 4096)
    try:
        fields = raw.rsplit(') ', 1)[1].split() if raw else []
        if raw and raw.split(' ', 1)[0] == str(process.pid) and len(fields) >= 22:
            start_ticks = int(fields[19])
            if number(start_ticks) is not None and start_ticks > 0:
                progress.setdefault('qemu_start_ticks', start_ticks)
            if start_ticks > 0 and start_ticks == progress.get('qemu_start_ticks') and process.poll() is None:
                values.update(qemu_user_ticks=number(int(fields[11])), qemu_system_ticks=number(int(fields[12])),
                              qemu_rss_bytes=number(int(fields[21]) * os.sysconf('SC_PAGE_SIZE')))
                io = counters('/proc/' + str(process.pid) + '/io', ('read_bytes', 'write_bytes'))
                values.update(qemu_read_bytes=io['read_bytes'], qemu_write_bytes=io['write_bytes'])
                ticks = os.sysconf('SC_CLK_TCK')
                summary['clock_ticks_per_second'] = ticks if 0 < ticks <= 1000000 else None
    except (ValueError, IndexError, OSError):
        pass
    memory = counters('/proc/meminfo', ('MemAvailable', 'SwapFree'), kib=True)
    values.update(host_mem_available_kib=memory['MemAvailable'], host_swap_free_kib=memory['SwapFree'],
                  host_available_bytes=available_bytes(root))
    summarize(summary, values)


def capture(argv):
    """Fixed metadata only; fixed failure reasons and bounded owned-child cleanup."""
    try:
        child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.DEVNULL, env={'PATH': '/usr/sbin:/usr/bin', 'LC_ALL': 'C'})
    except OSError:
        return None, 'LAUNCH'
    deadline = time.monotonic() + 3
    data = bytearray()
    outcome = None, 'READ_LIMIT'
    try:
        while len(data) <= 2048:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([child.stdout], [], [], remaining)[0]:
                outcome = None, 'TIMEOUT'
                break
            block = os.read(child.stdout.fileno(), 2049 - len(data))
            if not block:
                remaining = deadline - time.monotonic()
                if remaining > 0:
                    outcome = (bytes(data), None) if child.wait(timeout=remaining) == 0 else (None, 'NONZERO')
                else:
                    outcome = None, 'TIMEOUT'
                break
            data.extend(block)
    except subprocess.TimeoutExpired:
        outcome = None, 'TIMEOUT'
    except OSError:
        outcome = None, 'READ_ERROR'
    finally:
        try:
            child.stdout.close()
        except OSError:
            outcome = None, 'CLEANUP'
        try:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=3)
        except (OSError, subprocess.SubprocessError):
            outcome = None, 'CLEANUP'
    return outcome


def observe_commands(emit):
    for label, unit in UNITS.items():
        raw, reason = capture(['/usr/bin/systemctl', 'show', '--property=ActiveState,Result,ExecMainStatus,ExecMainStartTimestampMonotonic,ExecMainExitTimestampMonotonic', unit])
        if reason:
            emit('CAPTURE_' + label + '_' + reason)
            if reason == 'CLEANUP':
                return False
        try:
            values = dict(line.split('=', 1) for line in raw.decode('ascii').splitlines()) if raw else {}
            state = values.get('ActiveState', '')
            result = values.get('Result', '')
            status = values.get('ExecMainStatus', '')
            started = values.get('ExecMainStartTimestampMonotonic', '')
            exited = values.get('ExecMainExitTimestampMonotonic', '')
            if not values:
                emit('UNIT_' + label + '_UNAVAILABLE')
                if reason is None:
                    emit('CAPTURE_' + label + '_MALFORMED')
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
            emit('CAPTURE_' + label + '_MALFORMED')
    pane, reason = capture(['/usr/bin/tmux', 'display-message', '-p', '-t', 'anaconda:main.0',
                    '#{pane_dead}|#{pane_current_command}'])
    if reason:
        emit('CAPTURE_TMUX_' + reason)
        if reason == 'CLEANUP':
            return False
    if pane is None or not re.fullmatch(rb'[01]\|[A-Za-z0-9_.+-]{0,64}\n', pane):
        emit('TMUX_UNAVAILABLE')
        if pane is not None:
            emit('CAPTURE_TMUX_MALFORMED')
    elif pane.startswith(b'1|'):
        emit('TMUX_MAIN_DEAD')
    elif pane == b'0|anaconda\n':
        emit('TMUX_MAIN_LIVE_ANACONDA')
    elif pane in (b'0|python3\n', b'0|python\n'):
        emit('TMUX_MAIN_LIVE_PYTHON')
    else:
        emit('TMUX_MAIN_LIVE_OTHER')
    return True


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
    started = time.monotonic()
    deadline = started + 7200
    sequence = 0
    next_health = started
    metadata_enabled = True
    while time.monotonic() < deadline:
        now = time.monotonic()
        if now >= next_health and sequence < HEALTH_SAMPLES:
            sequence += 1
            next_health = now + HEALTH_INTERVAL
            send_health(guest_health(sequence, started))
        # Heartbeats precede metadata work and outlive stage-token saturation.
        if metadata_enabled and len(seen) < 128:
            metadata_enabled = observe_commands(emit)
            if not metadata_enabled:
                emit('METADATA_CLEANUP_UNCERTAIN')
        for label, path in LOGS.items():
            try:
                descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                with os.fdopen(descriptor, 'rb') as stream:
                    info = os.fstat(stream.fileno())
                    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                        continue
                    if info.st_size:
                        emit('LOG_' + label + '_PRESENT')
                    if info.st_size > 65536:
                        emit('LOG_' + label + '_TAIL_LIMIT')
                    stream.seek(max(0, info.st_size - 65536))
                    data = stream.read(65536)
                for name, pattern in RECORD_PATTERNS.get(label, {}).items():
                    if pattern.search(data):
                        emit('LOG_' + label + '_' + name)
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
