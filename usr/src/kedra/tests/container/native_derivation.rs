//! Closed native OS transforms executed only by the offline image-build harness.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysroot_engine::{
    NativeImplementation, NativePlan, NativeReceipt, NativeStep, VerifiedComposition,
};

use crate::composition::{
    RPM_FORMAT, checked, native_image, nonce, private_open, read_state, retire_next,
    sorted_inventory, sync_parent, write_state,
};
use crate::docker::{Build, Docker};
use crate::image::{Overlay, Request, Source};
use crate::{Result, invalid};

const RECEIPT: &str = "/usr/share/sysroot/native-receipt.json";
const IDENTITY: &str = "dev.kedra.native.identity";
const PARENT: &str = "dev.kedra.native.parent";
const TRANSACTION: &str = "dev.kedra.native.transaction";

// This code runs with the image's Python interpreter, never on the workstation.
// Only the normalized data plan is variable; neither it nor a source checkout
// supplies command bodies, Dockerfile instructions or host mount options.
const DRIVER: &str = r#"
import hashlib
import json
import os
import pathlib
import re
import shutil
import stat
import subprocess
import sys
import tempfile

RECEIPT = '/usr/share/sysroot/native-receipt.json'
BASELINE = '/usr/share/sysroot/home/default/'
SCHEMAS = '/usr/share/glib-2.0/schemas'
UNITROOT = '/etc/systemd/system'
MODULES = '/usr/lib/modules'

def fail(message):
    raise RuntimeError(message)

def run(argv):
    return subprocess.check_output(argv, stderr=sys.stderr, env={
        'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LC_ALL': 'C',
        'SYSTEMD_OFFLINE': '1', 'HOME': '/root',
    }).decode('utf-8')

def digest(path):
    with open(path, 'rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def parents(path):
    p = pathlib.Path(path)
    for item in reversed(p.parents):
        if item.is_symlink():
            fail('native output parent is a symlink: ' + str(item))
        if item.exists() and not item.is_dir():
            fail('native output parent is not a directory: ' + str(item))

def regular(path, optional=False, single_link=False):
    parents(path)
    try:
        info = os.lstat(path)
    except FileNotFoundError:
        if optional:
            return None
        raise
    if not stat.S_ISREG(info.st_mode):
        fail('native path is not an ordinary file: ' + path)
    if single_link and info.st_nlink != 1:
        fail('native file has hardlink aliases: ' + path)
    return info

def record(path):
    info = os.lstat(path)
    if stat.S_ISLNK(info.st_mode):
        return {'kind': 'symlink', 'target': os.readlink(path)}
    if stat.S_ISREG(info.st_mode):
        return {'kind': 'regular', 'sha256': digest(path),
                'bytes': info.st_size, 'mode': stat.S_IMODE(info.st_mode)}
    if stat.S_ISDIR(info.st_mode):
        return {'kind': 'directory', 'mode': stat.S_IMODE(info.st_mode)}
    fail('unexpected image filesystem object: ' + path)

def tree():
    result = {}
    for root, directories, files in os.walk('/', followlinks=False):
        if root == '/':
            directories[:] = [d for d in directories if d not in ('proc', 'sys', 'dev')]
        for name in directories + files:
            path = os.path.join(root, name)
            if path == RECEIPT or path in ('/etc/hosts', '/etc/hostname', '/etc/resolv.conf'):
                continue
            result[path] = record(path)
    return result

def tools_for(plan):
    tools = {'/usr/bin/python3', '/usr/bin/rpm'}
    for step in plan['definition']['steps']:
        kind = step['kind']
        if kind == 'glib_schemas':
            tools.add('/usr/bin/glib-compile-schemas')
        elif kind == 'systemd':
            tools.add('/usr/bin/systemctl')
        elif kind == 'qemu_initramfs':
            with open('/usr/share/sysroot/source.json') as stream:
                source = json.load(stream)
            if source['target']['id'] != 'qemu-arm64' or source['target']['architecture'] != 'aarch64' or os.uname().machine != 'aarch64':
                fail('native initramfs requires image-local qemu-arm64 source')
            tools.update(('/usr/bin/dracut', '/usr/bin/lsinitrd', '/usr/sbin/modinfo'))
    return {path: digest(path) for path in sorted(tools)}

def rpm():
    rows = run(['/usr/bin/rpm', '-qa', '--qf', '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n']).splitlines()
    text = '\n'.join(sorted(row for row in rows if not row.startswith('gpg-pubkey\t'))) + '\n'
    return hashlib.sha256(text.encode()).hexdigest()

def unit_link(path, entry):
    if entry['kind'] != 'symlink':
        fail('systemd produced a non-symlink output: ' + path)
    target = entry['target']
    if target == '/dev/null':
        return
    resolved = os.path.normpath(os.path.join(os.path.dirname(path), target))
    if not any(resolved.startswith(prefix) for prefix in ('/usr/lib/systemd/system/', '/etc/systemd/system/')):
        fail('systemd link escapes native unit roots: ' + path)
    if not os.path.isfile(resolved):
        fail('systemd link target does not exist: ' + path)

def selected_unit_link(path, entry, selected):
    if entry['kind'] != 'symlink' or not path.startswith(UNITROOT + '/'):
        return False
    relative = path[len(UNITROOT) + 1:]
    if relative == 'default.target':
        return selected['default_target'] is not None
    parts = relative.split('/')
    name = parts[-1]
    if len(parts) == 1 and name in selected['mask']:
        return entry['target'] == '/dev/null'
    if name not in selected['enable'] or len(parts) > 2:
        return False
    if len(parts) == 2 and not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*\.(service|socket|timer|target|path)\.(wants|requires)', parts[0]):
        return False
    unit_link(path, entry)
    return os.path.basename(os.path.normpath(entry['target'])) == name

def generate_initramfs(argv):
    # Fedora bootc leaves this image-local home target absent until first boot.
    # dracut always copies /root's symlink, independent of the HOME environment.
    temporary_root = False
    if os.path.islink('/root') and not os.path.exists('/root'):
        if os.readlink('/root') != 'var/roothome':
            fail('unsupported dangling image root home')
        parents('/var/roothome')
        if os.path.lexists('/var/roothome'):
            fail('image root home target is aliased')
        os.mkdir('/var/roothome', 0o700)
        temporary_root = True
    try:
        run(argv)
    finally:
        if temporary_root:
            # Refuse unexpected contents instead of deleting generated state.
            os.rmdir('/var/roothome')

def generic_initramfs(path):
    included = set(run(['/usr/bin/lsinitrd', '-m', path]).splitlines())
    if not {'qemu', 'crypt', 'dm', 'rootfs-block'}.issubset(included):
        fail('generated initramfs lacks generic QEMU storage/crypto modules')

def build(plan):
    if rpm() != plan['rpm_sha256']:
        fail('native parent RPM inventory differs')
    for item in plan['inputs']:
        path = item['path']
        regular(path, single_link=True)
        entry = record(path)
        if any(entry[key] != item[key] for key in ('sha256', 'bytes', 'mode')):
            fail('native input differs from normalized plan: ' + path)
    before = tree()
    expected = set()
    kernels = []
    systemd = False
    for step in plan['definition']['steps']:
        kind = step['kind']
        if kind == 'glib_schemas':
            parents(SCHEMAS + '/gschemas.compiled')
            regular(SCHEMAS + '/gschemas.compiled', optional=True)
            with tempfile.TemporaryDirectory(prefix='kedra-native-schemas-') as scratch:
                copied = os.path.join(scratch, 'schemas')
                shutil.copytree(SCHEMAS, copied, symlinks=True)
                for root, directories, files in os.walk(copied):
                    for name in directories + files:
                        if os.path.islink(os.path.join(root, name)):
                            fail('GLib schema directory contains a symlink')
                run(['/usr/bin/glib-compile-schemas', '--strict', copied])
                output = SCHEMAS + '/gschemas.compiled'
                os.chmod(copied + '/gschemas.compiled', 0o644)
                os.replace(copied + '/gschemas.compiled', output)
            expected.add(output)
        elif kind == 'systemd':
            parents(UNITROOT + '/default.target')
            systemd = True
            for operation in ('disable', 'enable', 'mask'):
                for unit in step[operation]:
                    # Validate the concrete installed unit before allowing systemctl
                    # to interpret Install metadata (including Also/Alias).
                    candidates = [UNITROOT + '/' + unit, '/usr/lib/systemd/system/' + unit]
                    if not any(os.path.isfile(path) for path in candidates):
                        fail('selected native unit is absent: ' + unit)
                    run(['/usr/bin/systemctl', '--root=/', operation, '--', unit])
            if step['default_target'] is not None:
                run(['/usr/bin/systemctl', '--root=/', 'set-default', step['default_target']])
        elif kind == 'initial_skel':
            for item in plan['inputs']:
                source = item['path']
                if not source.startswith(BASELINE):
                    continue
                destination = '/etc/skel/' + source[len(BASELINE):]
                regular(destination, optional=True)
                os.makedirs(os.path.dirname(destination), mode=0o755, exist_ok=True)
                handle, temporary = tempfile.mkstemp(prefix='kedra-skel-', dir=os.path.dirname(destination))
                os.close(handle)
                shutil.copyfile(source, temporary)
                os.chmod(temporary, item['mode'])
                os.replace(temporary, destination)
                expected.add(destination)
        elif kind == 'qemu_initramfs':
            parents(MODULES + '/placeholder')
            versions = sorted(os.listdir(MODULES))
            if not versions:
                fail('image contains no native kernel')
            for version in versions:
                if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._+\-]*', version):
                    fail('invalid native kernel version')
                module_root = MODULES + '/' + version
                if os.path.islink(module_root) or not os.path.isdir(module_root):
                    fail('kernel module root is aliased')
                kernel = module_root + '/vmlinuz'
                regular(kernel)
                modules = {}
                for driver in step['required_modules']:
                    path = run(['/usr/sbin/modinfo', '-k', version, '-n', driver]).strip()
                    resolved = os.path.realpath(path)
                    if not resolved.startswith(module_root + '/'):
                        fail('required driver resolves outside image kernel: ' + driver)
                    regular(resolved)
                    modules[driver] = resolved
                output = module_root + '/initramfs.img'
                old = regular(output, optional=True)
                with tempfile.TemporaryDirectory(prefix='kedra-native-initramfs-') as scratch:
                    temporary = scratch + '/initramfs.img'
                    generate_initramfs(['/usr/bin/dracut', '--force', '--no-hostonly', '--no-hostonly-cmdline',
                         '--add', 'qemu crypt dm rootfs-block', '--add-drivers',
                         ' '.join(step['required_modules']), temporary, version])
                    os.chmod(temporary, stat.S_IMODE(old.st_mode) if old else 0o644)
                    os.replace(temporary, output)
                listing = run(['/usr/bin/lsinitrd', output])
                generic_initramfs(output)
                for driver, path in modules.items():
                    if path.lstrip('/') not in listing:
                        fail('generated initramfs lacks required module: ' + driver)
                kernels.append({'version': version, 'kernel_sha256': digest(kernel),
                                'required_modules': modules,
                                'listing_sha256': hashlib.sha256(listing.encode()).hexdigest()})
                expected.add(output)
        else:
            fail('unknown native step')
    after = tree()
    changed = {path for path in set(before) | set(after) if before.get(path) != after.get(path)}
    selected = next((s for s in plan['definition']['steps'] if s['kind'] == 'systemd'), None)
    unit_outputs = {path for path, entry in after.items() if selected and selected_unit_link(path, entry, selected)}
    artifacts = []
    for path in sorted(changed | expected):
        previous, current = before.get(path), after.get(path)
        if current and current['kind'] == 'directory':
            if previous is None and any(output.startswith(path + '/') for output in expected):
                continue
            if previous is None and current['mode'] == 0o755 and any(output.startswith(path + '/') for output in unit_outputs):
                continue
            fail('native transform changed an unrelated directory: ' + path)
        if path not in expected:
            if not systemd or not path.startswith(UNITROOT + '/'):
                fail('native transform changed an unrelated path: ' + path)
            if current:
                unit_link(path, current)
            elif previous and previous['kind'] == 'symlink':
                unit_link(path, previous)
                current = {'kind': 'removed_symlink', 'target': previous['target']}
            else:
                fail('systemd removed a non-symlink: ' + path)
        if current is None:
            fail('native transform did not generate its declared artifact: ' + path)
        if current['kind'] == 'regular':
            regular(path, single_link=True)
        artifacts.append({'path': path, 'entry': current})
    # Already-satisfied mask/default/enable links still belong in the receipt.
    if systemd:
        recorded = {a['path'] for a in artifacts}
        for path, entry in sorted(after.items()):
            if path in unit_outputs and path not in recorded:
                unit_link(path, entry)
                artifacts.append({'path': path, 'entry': entry})
    artifacts.sort(key=lambda a: a['path'])
    if rpm() != plan['rpm_sha256']:
        fail('native transform changed RPM inventory')
    receipt = {key: plan[key] for key in ('schema', 'identity', 'implementation', 'driver_sha256',
                                         'recipe_sha256', 'foundation_image', 'rpm_sha256')}
    receipt.update(parent_identity=plan['definition']['parent_identity'], artifacts=artifacts,
                   kernels=kernels, tools=tools_for(plan))
    regular(RECEIPT, optional=True, single_link=True)
    handle, temporary = tempfile.mkstemp(prefix='.native-receipt-', dir=os.path.dirname(RECEIPT))
    with os.fdopen(handle, 'w') as stream:
        json.dump(receipt, stream, sort_keys=True)
    os.chmod(temporary, 0o644)
    os.replace(temporary, RECEIPT)

def verify(plan):
    regular(RECEIPT, single_link=True)
    for item in plan['inputs']:
        regular(item['path'], single_link=True)
    with open(RECEIPT) as stream:
        receipt = json.load(stream)
    if receipt['tools'] != tools_for(plan) or rpm() != plan['rpm_sha256']:
        fail('final image native tool/RPM observation changed')
    for artifact in receipt['artifacts']:
        path, entry = artifact['path'], artifact['entry']
        parents(path)
        if entry['kind'] == 'regular':
            regular(path, single_link=True)
        if entry['kind'] == 'removed_symlink':
            if os.path.lexists(path):
                fail('removed unit link is present: ' + path)
        elif record(path) != entry:
            fail('native receipt differs from final artifact: ' + path)
    for kernel in receipt['kernels']:
        root = MODULES + '/' + kernel['version']
        if digest(root + '/vmlinuz') != kernel['kernel_sha256']:
            fail('native kernel changed')
        listing = run(['/usr/bin/lsinitrd', root + '/initramfs.img'])
        generic_initramfs(root + '/initramfs.img')
        if hashlib.sha256(listing.encode()).hexdigest() != kernel['listing_sha256']:
            fail('native initramfs listing changed')
        for driver, path in kernel['required_modules'].items():
            if path.lstrip('/') not in listing:
                fail('native initramfs module disappeared: ' + driver)
    print(json.dumps(receipt, sort_keys=True))

if sys.argv[1] == 'build':
    with open('/tmp/kedra-native-plan.json') as stream:
        build(json.load(stream))
elif sys.argv[1] == 'verify':
    verify(json.loads(sys.argv[2]))
elif sys.argv[1] == 'baseline':
    paths = []
    for root, directories, files in os.walk(BASELINE, followlinks=False):
        for name in directories:
            if os.path.islink(os.path.join(root, name)):
                fail('inherited managed baseline directory is aliased')
        for name in files:
            path = os.path.join(root, name)
            regular(path)
            paths.append(path)
    print(json.dumps(sorted(paths)))
else:
    fail('unknown fixed native driver mode')
"#;

fn implementation() -> NativeImplementation {
    NativeImplementation {
        version: "kedra-native-v1",
        driver: DRIVER,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derived {
    pub image: String,
    pub engine: String,
    pub parent_image: String,
    pub material: NativeReceipt,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    schema: u32,
    identity: String,
    parent_image: String,
    engine: String,
    nonce: String,
    tag: String,
    image: Option<String>,
}

fn cache() -> Result<PathBuf> {
    let path = crate::artifact_root().join("native");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&path)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_dir()
        || metadata.mode() & 0o077 != 0
        || metadata.uid() != rustix::process::geteuid().as_raw()
    {
        return Err(invalid("native cache must be an owner-private directory"));
    }
    Ok(path)
}

fn verified(request: &Request, cache: &Path) -> Result<VerifiedComposition> {
    let Source::Composition(input) = &request.source else {
        return Err(invalid(
            "native derivation requires --image composition:<directory>",
        ));
    };
    if request.overlay != Overlay::None
        || request.binaries.is_some()
        || request.target.id != "qemu-arm64"
        || request.target.oci_architecture != "arm64"
    {
        return Err(invalid(
            "native derivation requires qemu-arm64 without overlays or binary overrides",
        ));
    }
    let identity = request
        .composition_identity
        .as_deref()
        .ok_or_else(|| invalid("native derivation requires independent composition identity"))?;
    VerifiedComposition::open(input, identity, cache).map_err(|error| invalid(error.to_string()))
}

/// Plans data only; does not build or execute native tools.
pub fn plan(request: &Request, specification: &Path) -> Result<NativePlan> {
    let cache = cache()?;
    let parent = verified(request, &cache)?;
    let definition =
        sysroot_engine::read_native(specification).map_err(|error| invalid(error.to_string()))?;
    sysroot_engine::plan_native(&parent, &definition, &implementation())
        .map_err(|error| invalid(error.to_string()))
}

fn image_labels(docker: &Docker, image: &str, expected: &HashMap<String, String>) -> Result<()> {
    let inspected = native_image(docker, image)?;
    let parent = expected
        .get(PARENT)
        .ok_or_else(|| invalid("native image has no selected parent"))?;
    let layers = |image: testcontainers::bollard::models::ImageInspect| -> Result<Vec<String>> {
        let root = image
            .root_fs
            .ok_or_else(|| invalid("native image has no RootFS metadata"))?;
        let layers = root
            .layers
            .ok_or_else(|| invalid("native image has no filesystem layers"))?;
        if root.typ != "layers"
            || layers.is_empty()
            || layers.iter().any(|id| {
                !id.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                })
            })
        {
            return Err(invalid("native image filesystem layer metadata is invalid"));
        }
        Ok(layers)
    };
    let parent_layers = layers(native_image(docker, parent)?)?;
    let derived_layers = layers(inspected.clone())?;
    if derived_layers.len() <= parent_layers.len() || !derived_layers.starts_with(&parent_layers) {
        return Err(invalid(
            "native image filesystem does not extend the selected immutable parent",
        ));
    }
    let actual = inspected
        .config
        .and_then(|config| config.labels)
        .unwrap_or_default();
    if !expected
        .iter()
        .all(|(key, value)| actual.get(key) == Some(value))
    {
        return Err(invalid("native image identity/provenance binding mismatch"));
    }
    Ok(())
}

fn native_environment(docker: &Docker, image: &str) -> Result<()> {
    let config = native_image(docker, image)?.config;
    for variable in config.and_then(|config| config.env).unwrap_or_default() {
        if let Some((key, value)) = variable.split_once('=')
            && !value.is_empty()
            && matches!(
                key,
                "LD_PRELOAD" | "LD_LIBRARY_PATH" | "PYTHONHOME" | "PYTHONPATH" | "BASH_ENV" | "ENV"
            )
        {
            return Err(invalid(
                "native parent inherits an executable environment override",
            ));
        }
    }
    Ok(())
}

fn material(
    docker: &Docker,
    image: &str,
    plan: &NativePlan,
    inventory: &str,
) -> Result<NativeReceipt> {
    let data = serde_json::to_string(plan).map_err(|error| invalid(error.to_string()))?;
    // Validate the bounded receipt before it selects paths for guest readback.
    let bytes = checked(docker, image, &["/usr/bin/cat", RECEIPT])?;
    let expected: NativeReceipt =
        serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    sysroot_engine::validate_native_receipt(plan, &expected)
        .map_err(|error| invalid(error.to_string()))?;
    let output = docker.isolated_native(
        image,
        ["/usr/bin/python3", "-I", "-c", DRIVER, "verify", &data]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    )?;
    if output.exit != 0 {
        return Err(invalid(format!(
            "native final-image readback failed: {}",
            output.stderr_text()
        )));
    }
    let bytes = output.stdout;
    let actual: NativeReceipt =
        serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    if actual != expected
        || sorted_inventory(&checked(
            docker,
            image,
            &["/usr/bin/rpm", "-qa", "--qf", RPM_FORMAT],
        )?)? != inventory
    {
        return Err(invalid("native final artifact or RPM inventory differs"));
    }
    Ok(actual)
}

fn initial_baseline(docker: &Docker, plan: &NativePlan) -> Result<()> {
    if !plan.definition.steps.contains(&NativeStep::InitialSkel) {
        return Ok(());
    }
    let bytes = checked(
        docker,
        &plan.foundation_image,
        &["/usr/bin/python3", "-I", "-c", DRIVER, "baseline"],
    )?;
    let inherited: Vec<String> =
        serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
    if inherited
        .iter()
        .any(|path| !plan.inputs.iter().any(|input| input.path == *path))
    {
        return Err(invalid(
            "initial skeleton refuses deletion of an inherited managed baseline file",
        ));
    }
    Ok(())
}

struct Context(PathBuf);
impl Drop for Context {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn context(cache: &Path, plan: &NativePlan, parent_image: &str) -> Result<Context> {
    let context = Context(cache.join(format!(".context-{}", nonce()?)));
    fs::DirBuilder::new().mode(0o700).create(&context.0)?;
    let recipe = sysroot_engine::render_native_recipe(plan, parent_image)
        .map_err(|error| invalid(error.to_string()))?;
    fs::write(context.0.join("Containerfile"), recipe)?;
    fs::write(context.0.join("native-driver.py"), DRIVER)?;
    fs::write(
        context.0.join("native-plan.json"),
        serde_json::to_vec(plan).map_err(|error| invalid(error.to_string()))?,
    )?;
    Ok(context)
}

/// Execute the selected plan through the harness's offline BuildKit path.
/// The caller supplies neither a command body nor a build context.
pub fn prepare(
    docker: &Docker,
    request: &Request,
    specification: &Path,
    expected_identity: &str,
) -> Result<Derived> {
    let cache = cache()?;
    let verified = verified(request, &cache)?;
    let definition =
        sysroot_engine::read_native(specification).map_err(|error| invalid(error.to_string()))?;
    let plan = sysroot_engine::plan_native(&verified, &definition, &implementation())
        .map_err(|error| invalid(error.to_string()))?;
    if plan.identity != expected_identity {
        return Err(invalid(
            "native derivation identity differs from independent selection",
        ));
    }
    let parent = crate::composition::prepare(docker, request)?;
    native_environment(docker, &parent.image)?;
    native_environment(docker, &plan.foundation_image)?;
    initial_baseline(docker, &plan)?;
    let engine = docker.identity()?;
    if engine != parent.engine {
        return Err(invalid("native derivation daemon changed"));
    }
    let key = format!(
        "{}-{}",
        crate::builder::content_key(&[engine.as_bytes()]),
        plan.identity
    );
    let lock = private_open(&cache.join(format!("{key}.lock")), true)?;
    loop {
        crate::cancel::check()?;
        match lock.try_lock() {
            Ok(()) => break,
            Err(fs::TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(100))
            }
            Err(fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
    let tag = format!("kedra-native:{}", plan.identity);
    let binding_path = cache.join(format!("{key}-binding.json"));
    let journal_path = cache.join(format!("{key}-transaction.json"));
    retire_next(&binding_path)?;
    retire_next(&journal_path)?;
    let bound: Option<Derived> = read_state(&binding_path)?;
    let mut pending: Option<Pending> = read_state(&journal_path)?;
    let labels = HashMap::from([
        (
            crate::docker::OWNER_LABEL.into(),
            crate::docker::OWNER.into(),
        ),
        (crate::docker::KIND_LABEL.into(), "native".into()),
        (IDENTITY.into(), plan.identity.clone()),
        (PARENT.into(), parent.image.clone()),
    ]);
    let inventory = &verified.composition().plan.foundation.rpm_inventory;
    if let Some(journal) = &pending {
        if journal.schema != 1
            || journal.identity != plan.identity
            || journal.parent_image != parent.image
            || journal.engine != engine
            || journal.nonce.len() != 32
            || !journal
                .nonce
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            || journal.tag != format!("kedra-native-pending:{}", journal.nonce)
        {
            return Err(invalid("native transaction identity/provenance mismatch"));
        }
        if let Some(actual) = &journal.image {
            image_labels(docker, actual, &labels)?;
            if docker
                .image(&journal.tag)?
                .is_some_and(|(id, _)| id != *actual)
            {
                return Err(invalid("native transaction tag changed"));
            }
        }
    }
    if let Some(receipt) = bound {
        if receipt.engine != engine || receipt.parent_image != parent.image {
            return Err(invalid("native cache image/daemon binding mismatch"));
        }
        image_labels(docker, &receipt.image, &labels)?;
        let actual = material(docker, &receipt.image, &plan, inventory)?;
        if receipt.material != actual
            || pending
                .as_ref()
                .is_some_and(|journal| journal.image.as_deref() != Some(receipt.image.as_str()))
        {
            return Err(invalid(
                "native cache receipt differs from final image or transaction",
            ));
        }
        docker.publish_native(&receipt.image, &tag)?;
        if let Some(journal) = pending {
            docker.remove_native_pending(&journal.tag, &receipt.image, &journal.nonce)?;
            fs::remove_file(&journal_path)?;
            sync_parent(&journal_path)?;
        }
        return Ok(receipt);
    }
    if docker.image(&tag)?.is_some() {
        return Err(invalid(
            "native cache tag has no independently retained binding",
        ));
    }
    if pending.is_none() {
        let nonce = nonce()?;
        pending = Some(Pending {
            schema: 1,
            identity: plan.identity.clone(),
            parent_image: parent.image.clone(),
            engine: engine.clone(),
            tag: format!("kedra-native-pending:{nonce}"),
            nonce,
            image: None,
        });
        write_state(&journal_path, &pending, false)?;
    }
    let mut pending = pending.ok_or_else(|| invalid("native transaction missing"))?;
    let mut transaction_labels = labels;
    transaction_labels.insert(TRANSACTION.into(), pending.nonce.clone());
    if pending.image.is_none() {
        if let Some((actual, _)) = docker.image(&pending.tag)? {
            image_labels(docker, &actual, &transaction_labels)?;
            docker.remove_native_pending(&pending.tag, &actual, &pending.nonce)?;
        }
        docker.pin_foundation(&parent.image)?;
        let context = context(&cache, &plan, &parent.image)?;
        docker.build_static(
            &Build {
                tag: &pending.tag,
                what: "closed native OS derivation (network disabled)",
                containerfile: &context.0.join("Containerfile"),
                files: &[
                    (
                        "native-plan.json".into(),
                        context.0.join("native-plan.json"),
                    ),
                    (
                        "native-driver.py".into(),
                        context.0.join("native-driver.py"),
                    ),
                ],
                args: &[],
                platform: "linux/arm64",
            },
            transaction_labels.clone(),
        )?;
        let actual = docker
            .image(&pending.tag)?
            .ok_or_else(|| invalid("native derivation produced no image"))?
            .0;
        image_labels(docker, &actual, &transaction_labels)?;
        pending.image = Some(actual);
        write_state(&journal_path, &pending, true)?;
    }
    let image = pending
        .image
        .as_ref()
        .ok_or_else(|| invalid("native transaction has no image"))?;
    image_labels(docker, image, &transaction_labels)?;
    let receipt = Derived {
        image: image.clone(),
        engine,
        parent_image: parent.image,
        material: material(docker, image, &plan, inventory)?,
    };
    write_state(&binding_path, &receipt, false)?;
    docker.publish_native(image, &tag)?;
    docker.remove_native_pending(&pending.tag, image, &pending.nonce)?;
    fs::remove_file(&journal_path)?;
    sync_parent(&journal_path)?;
    Ok(receipt)
}
