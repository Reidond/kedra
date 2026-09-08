"""Narrow, version-guarded Fedora Anaconda 44.30-2.fc44 media adaptations."""
import ast
import hashlib
import pathlib
import py_compile
import importlib.util
from types import SimpleNamespace

if not pathlib.Path('/run/.containerenv').is_file():
    raise SystemExit('Apply only inside the disposable installer image build')
spec = importlib.util.find_spec('pyanaconda')
if spec is None or not spec.submodule_search_locations or len(spec.submodule_search_locations) != 1:
    raise SystemExit('Expected one installed Anaconda package')
root = pathlib.Path(spec.submodule_search_locations[0])
patches = [
    (
        'modules/payloads/source/bootc/bootc.py',
        '8fd2f71d213e18857a212a1c4062e892e470ce77b65ab35020c48846756f99ae',
        '        :return: True or False\n        """\n        return True\n',
        '        :return: True or False\n        """\n        # Kedra: an embedded local container does not need a network.\n        ref = self.configuration.sourceImgRef\n        return not (ref and ref.startswith("containers-storage:"))\n',
    ),
    (
        'modules/users/users.py',
        'a8864d42390e9e213908dc0ca52ce370422309144646699369f299e3a5c75c19',
        '        # any root set from kickstart is fine\n        if self._rootpw_seen:\n            return True\n',
        '        # Kedra: a deliberately locked root is not an accessible admin.\n        if self._rootpw_seen and not self.root_account_locked:\n            return True\n',
    ),
]
updated = {}
for relative, expected, before, after in patches:
    path = root / relative
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected:
        raise SystemExit(f'Anaconda source changed; review adapter before rebuilding: {relative}')
    source = raw.decode()
    if source.count(before) != 1:
        raise SystemExit(f'Anaconda patch anchor is not unique: {relative}')
    updated[relative] = source.replace(before, after)

def getter(source, name):
    candidates = [node for node in ast.walk(ast.parse(source)) if isinstance(node, ast.FunctionDef) and node.name == name]
    if len(candidates) != 1:
        raise SystemExit('Expected one native property getter')
    function = candidates[0]
    function.decorator_list = []
    module = ast.fix_missing_locations(ast.Module(body=[function], type_ignores=[]))
    namespace = {}
    exec(compile(module, '<verified Anaconda property>', 'exec'), namespace)
    return namespace[name]

network = getter(updated[patches[0][0]], 'network_required')
for ref, expected in [(None, True), ('', True), ('registry:example.invalid/image', True),
                      ('oci-archive:/unqualified', True), ('containers-storage:localhost/kedra:fixture', False)]:
    actual = network(SimpleNamespace(configuration=SimpleNamespace(sourceImgRef=ref)))
    if actual is not expected:
        raise SystemExit('Native source network classification failed')
admin = getter(updated[patches[1][0]], 'check_admin_user_exists')
for seen, locked, password, users, expected in [
    (True, True, '', [], False),
    (True, False, 'synthetic', [], True),
    (False, True, '', [SimpleNamespace(lock=False, groups=['wheel'])], True),
    (True, True, '', [SimpleNamespace(lock=False, groups=['wheel'])], True),
    (True, True, '', [SimpleNamespace(lock=False, groups=[])], False),
    (True, True, '', [SimpleNamespace(lock=True, groups=['wheel'])], False),
]:
    actual = admin(SimpleNamespace(_rootpw_seen=seen, root_account_locked=locked, root_password=password, users=users))
    if actual is not expected:
        raise SystemExit('Native accessible-administrator classification failed')
for relative, source in updated.items():
    path = root / relative
    path.write_text(source)
    py_compile.compile(str(path), doraise=True)
    print(f'Adapted and checked Anaconda property: {relative}')
