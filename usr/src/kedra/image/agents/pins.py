"""Select pinned Codex and Cosign records for an explicit target and this runner."""
import json
import pathlib
import platform

# The closed release target table (release/targets.json, also embedded by the
# Rust helper) selects target architectures; this directory only maps an
# architecture to its pinned inputs.
TABLE = json.loads((pathlib.Path(__file__).resolve().parents[1] / 'release/targets.json').read_text())
if TABLE.get('schema_version') != 1:
    raise SystemExit('Unsupported release target table')
TARGETS = TABLE['targets']

PINS = json.loads(pathlib.Path(__file__).with_name('inputs.json').read_text())
# Cosign executes on the build runner, so its pin follows the runner machine,
# never the image target. Unknown machines are refused rather than guessed.
COSIGN = {'x86_64': ('cosign_test_tool', 'cosign-linux-amd64'),
          'aarch64': ('cosign_test_tool_aarch64', 'cosign-linux-arm64')}


def codex(name, target):
    """Return one Codex record flattened for the target's musl triple."""
    if target not in TARGETS:
        raise RuntimeError(f'Unsupported target: {target!r}')
    triple = f'{TARGETS[target]["architecture"]}-unknown-linux-musl'
    record = PINS[name]
    if triple not in record['targets']:
        raise RuntimeError(f'No pinned {name} package for {triple}')
    shared = {key: value for key, value in record.items() if key != 'targets'}
    return {**shared, 'target': triple, **record['targets'][triple]}


def cosign():
    machine = platform.machine()
    if machine not in COSIGN:
        raise RuntimeError(f'No pinned Cosign build for runner architecture {machine!r}')
    key, asset = COSIGN[machine]
    return {**PINS[key], 'asset': asset}
