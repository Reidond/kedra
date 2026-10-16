//! In-image native executor. It runs as root only inside the offline derive
//! build, from a read-only bind mount; the plan is data and supplies no commands.
use crate::{
    Error, MAX_JSON, NATIVE_EXECUTOR, NATIVE_OBSERVATION, NATIVE_PLAN, NATIVE_RECEIPT_PATH,
    NativeArtifact, NativeArtifactKind, NativeKernel, NativePlan, NativeReceipt, NativeStep,
    RPM_ARCHITECTURE, Result, SKEL_APPLIED, native_tools, plan, store,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, DirBuilder, File, Metadata, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const BASELINE: &str = "/usr/share/sysroot/home/default";
const SCHEMAS: &str = "/usr/share/glib-2.0/schemas";
const UNITROOT: &str = "/etc/systemd/system";
const VENDOR_UNITS: &str = "/usr/lib/systemd/system";
const MODULES: &str = "/usr/lib/modules";
const COMPOSITION: &str = "/usr/share/sysroot/composition.json";
const SOURCE: &str = "/usr/share/sysroot/source.json";
pub(crate) const RPM_FORMAT: &str =
    "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n";
const UNIT_TYPES: [&str; 5] = ["service", "socket", "timer", "target", "path"];

/// Readback exported from the throwaway observation stage.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observation {
    pub receipt: NativeReceipt,
    pub rpm_inventory: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Regular {
        sha256: String,
        bytes: u64,
        mode: u32,
    },
    Symlink {
        target: String,
    },
    Directory {
        mode: u32,
    },
}
impl Entry {
    fn artifact(&self) -> Option<NativeArtifactKind> {
        match self {
            Self::Regular {
                sha256,
                bytes,
                mode,
            } => Some(NativeArtifactKind::Regular {
                sha256: sha256.clone(),
                bytes: *bytes,
                mode: *mode,
            }),
            Self::Symlink { target } => Some(NativeArtifactKind::Symlink {
                target: target.clone(),
            }),
            Self::Directory { .. } => None,
        }
    }
}
type Tree = BTreeMap<String, Entry>;

fn fail(message: impl Into<String>) -> Error {
    Error::Invalid(format!("native executor: {}", message.into()))
}

/// Generate the declared artifacts and the receipt in the build's root filesystem.
pub fn apply(plan_path: &Path) -> Result<()> {
    let plan = build_context(plan_path)?;
    if rpm()?.1 != plan.rpm_sha256 {
        return Err(fail("native parent RPM inventory differs"));
    }
    for item in &plan.inputs {
        regular(&item.path, false, true)?;
        let actual = record(&item.path)?;
        if actual
            != (Entry::Regular {
                sha256: item.sha256.clone(),
                bytes: item.bytes,
                mode: item.mode,
            })
        {
            return Err(fail(format!(
                "native input differs from normalized plan: {}",
                item.path
            )));
        }
    }
    if plan.definition.steps.contains(&NativeStep::InitialSkel) {
        inherited_baseline(&plan)?;
    }
    let before = tree()?;
    let mut outputs = Outputs::default();
    for step in &plan.definition.steps {
        match step {
            NativeStep::GlibSchemas => outputs.expected.insert(glib_schemas()?),
            NativeStep::Systemd {
                enable,
                disable,
                mask,
                default_target,
            } => {
                systemd(enable, disable, mask, default_target.as_deref())?;
                outputs.systemd = Some(step.clone());
                true
            }
            NativeStep::InitialSkel => {
                outputs.expected.extend(initial_skel(&plan)?);
                true
            }
            NativeStep::Initramfs { modules, drivers } => {
                for (output, kernel) in initramfs(modules, drivers)? {
                    outputs.expected.insert(output);
                    outputs.kernels.push(kernel);
                }
                true
            }
        };
    }
    let after = tree()?;
    let artifacts = outputs.artifacts(&before, &after)?;
    if rpm()?.1 != plan.rpm_sha256 {
        return Err(fail("native transform changed RPM inventory"));
    }
    let receipt = NativeReceipt {
        schema: plan.schema,
        identity: plan.identity.clone(),
        parent_identity: plan.definition.parent_identity.clone(),
        implementation: plan.implementation.clone(),
        driver_sha256: plan.driver_sha256.clone(),
        recipe_sha256: plan.recipe_sha256.clone(),
        foundation_image: plan.foundation_image.clone(),
        rpm_sha256: plan.rpm_sha256.clone(),
        artifacts,
        kernels: outputs.kernels,
        tools: tools_for(&plan)?,
    };
    regular(NATIVE_RECEIPT_PATH, true, true)?;
    replace_file(
        Path::new(NATIVE_RECEIPT_PATH),
        &serde_json::to_vec(&receipt)?,
        0o644,
    )
}

/// Re-read the receipt's material from a built image and export it with the RPM rows.
pub fn observe(plan_path: &Path) -> Result<()> {
    let plan = build_context(plan_path)?;
    regular(NATIVE_RECEIPT_PATH, false, true)?;
    for item in &plan.inputs {
        regular(&item.path, false, true)?;
    }
    let receipt: NativeReceipt = serde_json::from_slice(&bounded(NATIVE_RECEIPT_PATH)?)?;
    let (rpm_inventory, rpm_sha256) = rpm()?;
    if receipt.tools != tools_for(&plan)? || rpm_sha256 != plan.rpm_sha256 {
        return Err(fail("final image native tool/RPM observation changed"));
    }
    for artifact in &receipt.artifacts {
        observe_artifact(artifact)?;
    }
    let modules = initramfs_modules(&plan);
    for kernel in &receipt.kernels {
        observe_kernel(kernel, modules)?;
    }
    let observation = Observation {
        receipt,
        rpm_inventory,
    };
    replace_file(
        Path::new(NATIVE_OBSERVATION),
        &serde_json::to_vec(&observation)?,
        0o644,
    )
}

/// Refuse to touch a filesystem that is not the selected composition inside the
/// fixed build mounts: this is an accident guard for a root-only command. A
/// receipt inherited from a derived foundation is replaced, as before.
fn build_context(plan_path: &Path) -> Result<NativePlan> {
    if !rustix::process::geteuid().is_root()
        || plan_path != Path::new(NATIVE_PLAN)
        || std::env::current_exe()? != Path::new(NATIVE_EXECUTOR)
    {
        return Err(fail(
            "runs only as root inside the offline sysroot derive build",
        ));
    }
    let plan: NativePlan = serde_json::from_slice(&bounded(NATIVE_PLAN)?)?;
    let composition: serde_json::Value = serde_json::from_slice(&bounded(COMPOSITION)?)?;
    if composition["identity"].as_str() != Some(plan.definition.parent_identity.as_str()) {
        return Err(fail("image is not the selected parent composition"));
    }
    Ok(plan)
}

fn bounded(path: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(fail(format!("{path} exceeds its size limit")));
    }
    Ok(bytes)
}

fn run(argv: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .env_clear()
        .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
        .env("LC_ALL", "C")
        .env("SYSTEMD_OFFLINE", "1")
        .env("HOME", "/root")
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(fail(format!("{} failed: {}", argv[0], output.status)));
    }
    Ok(output.stdout)
}

fn text(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| fail("tool output is not UTF-8"))
}

fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect());
        }
        hasher.update(&buffer[..count]);
    }
}

/// Every ancestor must be a real directory, never a symlink.
fn parents(path: &str) -> Result<()> {
    let path = Path::new(path);
    let mut ancestors: Vec<&Path> = path.ancestors().skip(1).collect();
    ancestors.reverse();
    for item in ancestors {
        match fs::symlink_metadata(item) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(fail(format!(
                    "native output parent is a symlink: {}",
                    item.display()
                )));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(fail(format!(
                    "native output parent is not a directory: {}",
                    item.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn regular(path: &str, optional: bool, single_link: bool) -> Result<Option<Metadata>> {
    parents(path)?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Err(fail(format!("native path is not an ordinary file: {path}")));
    }
    if single_link && metadata.nlink() != 1 {
        return Err(fail(format!("native file has hardlink aliases: {path}")));
    }
    Ok(Some(metadata))
}

fn record(path: &str) -> Result<Entry> {
    let metadata = fs::symlink_metadata(path)?;
    let kind = metadata.file_type();
    if kind.is_symlink() {
        let target = fs::read_link(path)?
            .into_os_string()
            .into_string()
            .map_err(|_| fail(format!("non-UTF-8 link target: {path}")))?;
        return Ok(Entry::Symlink { target });
    }
    if kind.is_file() {
        return Ok(Entry::Regular {
            sha256: digest(Path::new(path))?,
            bytes: metadata.len(),
            mode: metadata.mode() & 0o7777,
        });
    }
    if kind.is_dir() {
        return Ok(Entry::Directory {
            mode: metadata.mode() & 0o7777,
        });
    }
    Err(fail(format!("unexpected image filesystem object: {path}")))
}

/// Complete root filesystem except kernel interfaces and BuildKit-managed names.
fn tree() -> Result<Tree> {
    let mut result = Tree::new();
    let mut pending = vec![PathBuf::from("/")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let name = path
                .to_str()
                .ok_or_else(|| fail(format!("non-UTF-8 image path: {}", path.display())))?
                .to_owned();
            if matches!(name.as_str(), "/proc" | "/sys" | "/dev")
                || name == NATIVE_RECEIPT_PATH
                || matches!(
                    name.as_str(),
                    "/etc/hosts" | "/etc/hostname" | "/etc/resolv.conf"
                )
            {
                continue;
            }
            let entry = record(&name)?;
            if matches!(entry, Entry::Directory { .. }) {
                pending.push(path);
            }
            result.insert(name, entry);
        }
    }
    Ok(result)
}

fn initramfs_modules(plan: &NativePlan) -> &[String] {
    plan.definition
        .steps
        .iter()
        .find_map(|step| match step {
            NativeStep::Initramfs { modules, .. } => Some(modules.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

fn tools_for(plan: &NativePlan) -> Result<BTreeMap<String, String>> {
    if plan
        .definition
        .steps
        .iter()
        .any(|step| matches!(step, NativeStep::Initramfs { .. }))
    {
        // The deriver requires a native daemon, so this executor's architecture
        // is the image's; the kernels must belong to the image's own target.
        let source: serde_json::Value = serde_json::from_slice(&bounded(SOURCE)?)?;
        if source["target"]["architecture"].as_str() != Some(RPM_ARCHITECTURE) {
            return Err(fail(
                "native initramfs requires image-local source of the native architecture",
            ));
        }
    }
    native_tools(plan)
        .into_iter()
        .map(|path| Ok((path.to_owned(), digest(Path::new(path))?)))
        .collect()
}

/// Sorted RPM rows without signing keys, and their SHA-256.
fn rpm() -> Result<(String, String)> {
    let rows = text(run(&["/usr/bin/rpm", "-qa", "--qf", RPM_FORMAT])?)?;
    let mut lines: Vec<&str> = rows
        .lines()
        .filter(|row| !row.starts_with("gpg-pubkey\t"))
        .collect();
    lines.sort_unstable();
    let inventory = format!("{}\n", lines.join("\n"));
    let hash = plan::hash(inventory.as_bytes());
    Ok((inventory, hash))
}

fn private_scratch(prefix: &str) -> Result<PathBuf> {
    let path = PathBuf::from(format!("/tmp/{prefix}-{}", &store::nonce()?[..16]));
    DirBuilder::new().mode(0o700).create(&path)?;
    Ok(path)
}

/// Atomic same-directory replacement; the result is a single-link file.
fn replace_file(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let parent = path.parent().ok_or_else(|| fail("output has no parent"))?;
    let temporary = parent.join(format!(".sysroot-native-{}", &store::nonce()?[..16]));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    Ok(())
}

/// The composition can add baseline files but never delete foundation ones, so
/// any file here that is not a planned input is an unreconciled deletion.
fn inherited_baseline(plan: &NativePlan) -> Result<()> {
    let inputs: BTreeSet<&str> = plan.inputs.iter().map(|i| i.path.as_str()).collect();
    match fs::symlink_metadata(BASELINE) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(fail("inherited managed baseline directory is aliased"));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    let mut pending = vec![PathBuf::from(BASELINE)];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let name = path
                .to_str()
                .ok_or_else(|| fail("non-UTF-8 baseline path"))?;
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                pending.push(path.clone());
                continue;
            }
            if metadata.file_type().is_symlink() && path.is_dir() {
                return Err(fail("inherited managed baseline directory is aliased"));
            }
            regular(name, false, false)?;
            if !inputs.contains(name) {
                return Err(fail(
                    "initial skeleton refuses deletion of an inherited managed baseline file",
                ));
            }
        }
    }
    Ok(())
}

fn copy_schemas(source: &Path, destination: &Path) -> Result<()> {
    DirBuilder::new().mode(0o755).create(destination)?;
    for entry in fs::read_dir(source)? {
        let path = entry?.path();
        let target = destination.join(path.file_name().ok_or_else(|| fail("schema name"))?);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            copy_schemas(&path, &target)?;
        } else if metadata.is_file() {
            fs::copy(&path, &target)?;
        } else {
            return Err(fail("GLib schema directory contains a symlink"));
        }
    }
    Ok(())
}

fn glib_schemas() -> Result<String> {
    let output = format!("{SCHEMAS}/gschemas.compiled");
    parents(&output)?;
    regular(&output, true, false)?;
    let scratch = private_scratch("sysroot-native-schemas")?;
    let result = (|| -> Result<()> {
        let copied = scratch.join("schemas");
        copy_schemas(Path::new(SCHEMAS), &copied)?;
        let copied_name = copied.to_str().ok_or_else(|| fail("scratch path"))?;
        run(&["/usr/bin/glib-compile-schemas", "--strict", copied_name])?;
        let compiled = copied.join("gschemas.compiled");
        fs::set_permissions(&compiled, fs::Permissions::from_mode(0o644))?;
        fs::rename(compiled, &output)?;
        Ok(())
    })();
    fs::remove_dir_all(&scratch)?;
    result.map(|()| output)
}

fn systemd(
    enable: &[String],
    disable: &[String],
    mask: &[String],
    default_target: Option<&str>,
) -> Result<()> {
    parents(&format!("{UNITROOT}/default.target"))?;
    for (operation, units) in [("disable", disable), ("enable", enable), ("mask", mask)] {
        for unit in units {
            // Validate the concrete installed unit before allowing systemctl
            // to interpret Install metadata (including Also/Alias).
            if ![UNITROOT, VENDOR_UNITS]
                .iter()
                .any(|root| Path::new(&format!("{root}/{unit}")).is_file())
            {
                return Err(fail(format!("selected native unit is absent: {unit}")));
            }
            run(&["/usr/bin/systemctl", "--root=/", operation, "--", unit])?;
        }
    }
    if let Some(target) = default_target {
        run(&["/usr/bin/systemctl", "--root=/", "set-default", target])?;
    }
    Ok(())
}

/// Create a missing directory with `mode`; an existing one must be a real directory.
fn directory(path: &str, mode: u32) -> Result<()> {
    parents(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(fail(format!(
            "skeleton directory is not a directory: {path}"
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            DirBuilder::new().mode(mode).create(path)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// Baseline files go to `/etc/skel` and to the account's private applied-baseline
/// record there (owner-only state directories), so a new account's first
/// `sysroot home update` merges later image changes into its own edits.
fn initial_skel(plan: &NativePlan) -> Result<Vec<String>> {
    let applied = format!("/etc/skel/{}", SKEL_APPLIED.trim_end_matches('/'));
    let mut state = String::from("/etc/skel");
    for (part, mode) in [(".local", 0o755), ("state", 0o755)]
        .into_iter()
        .chain(["kedra", "home", "applied"].map(|part| (part, 0o700)))
    {
        state = format!("{state}/{part}");
        directory(&state, mode)?;
    }
    let mut outputs = Vec::new();
    for item in &plan.inputs {
        let Some(relative) = item.path.strip_prefix(&format!("{BASELINE}/")) else {
            continue;
        };
        let mut bytes = Vec::new();
        File::open(&item.path)?.read_to_end(&mut bytes)?;
        for root in ["/etc/skel", applied.as_str()] {
            let destination = format!("{root}/{relative}");
            regular(&destination, true, false)?;
            let parent = Path::new(&destination)
                .parent()
                .ok_or_else(|| fail("skeleton path has no parent"))?;
            DirBuilder::new()
                .recursive(true)
                .mode(0o755)
                .create(parent)?;
            replace_file(Path::new(&destination), &bytes, item.mode)?;
            outputs.push(destination);
        }
    }
    Ok(outputs)
}

fn kernel_version(version: &str) -> bool {
    let mut bytes = version.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}

fn initramfs(modules: &[String], drivers: &[String]) -> Result<Vec<(String, NativeKernel)>> {
    parents(&format!("{MODULES}/placeholder"))?;
    let mut versions = Vec::new();
    for entry in fs::read_dir(MODULES)? {
        versions.push(
            entry?
                .file_name()
                .into_string()
                .map_err(|_| fail("invalid native kernel version"))?,
        );
    }
    versions.sort();
    if versions.is_empty() {
        return Err(fail("image contains no native kernel"));
    }
    let mut kernels = Vec::new();
    for version in versions {
        if !kernel_version(&version) {
            return Err(fail("invalid native kernel version"));
        }
        kernels.push(generate_kernel(&version, modules, drivers)?);
    }
    Ok(kernels)
}

fn required_drivers(
    version: &str,
    module_root: &str,
    required: &[String],
) -> Result<BTreeMap<String, String>> {
    let mut modules = BTreeMap::new();
    for driver in required {
        let reported = text(run(&["/usr/sbin/modinfo", "-k", version, "-n", driver])?)?;
        let resolved = fs::canonicalize(reported.trim())?
            .into_os_string()
            .into_string()
            .map_err(|_| fail("non-UTF-8 module path"))?;
        if !resolved.starts_with(&format!("{module_root}/")) {
            return Err(fail(format!(
                "required driver resolves outside image kernel: {driver}"
            )));
        }
        regular(&resolved, false, false)?;
        modules.insert(driver.clone(), resolved);
    }
    Ok(modules)
}

fn generate_kernel(
    version: &str,
    modules: &[String],
    required: &[String],
) -> Result<(String, NativeKernel)> {
    let module_root = format!("{MODULES}/{version}");
    let metadata = fs::symlink_metadata(&module_root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(fail("kernel module root is aliased"));
    }
    let kernel = format!("{module_root}/vmlinuz");
    regular(&kernel, false, false)?;
    let resolved = required_drivers(version, &module_root, required)?;
    let output = format!("{module_root}/initramfs.img");
    let old = regular(&output, true, false)?;
    let scratch = private_scratch("sysroot-native-initramfs")?;
    let generated = (|| -> Result<()> {
        let temporary = scratch.join("initramfs.img");
        let temporary_name = temporary.to_str().ok_or_else(|| fail("scratch path"))?;
        let added = modules.join(" ");
        let drivers = required.join(" ");
        let mut argv = vec![
            "/usr/bin/dracut",
            "--force",
            "--no-hostonly",
            "--no-hostonly-cmdline",
            "--add",
            &added,
        ];
        if !required.is_empty() {
            argv.extend(["--add-drivers", &drivers]);
        }
        argv.extend([temporary_name, version]);
        with_root_home(|| run(&argv))?;
        let mode = old.map_or(0o644, |metadata| metadata.mode() & 0o7777);
        fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))?;
        fs::rename(&temporary, &output)?;
        Ok(())
    })();
    fs::remove_dir_all(&scratch)?;
    generated?;
    let listing = initramfs_listing(&output, modules)?;
    for (driver, path) in &resolved {
        if !listing.contains(path.trim_start_matches('/')) {
            return Err(fail(format!(
                "generated initramfs lacks required module: {driver}"
            )));
        }
    }
    let record = NativeKernel {
        version: version.to_owned(),
        kernel_sha256: digest(Path::new(&kernel))?,
        listing_sha256: plan::hash(listing.as_bytes()),
        drivers: resolved,
    };
    Ok((output, record))
}

/// Fedora bootc leaves this image-local home target absent until first boot.
/// dracut always copies /root's symlink, independent of the HOME environment.
fn with_root_home(generate: impl FnOnce() -> Result<Vec<u8>>) -> Result<()> {
    let root = Path::new("/root");
    let dangling =
        fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink()) && !root.exists();
    if dangling {
        if fs::read_link(root)? != Path::new("var/roothome") {
            return Err(fail("unsupported dangling image root home"));
        }
        parents("/var/roothome")?;
        if fs::symlink_metadata("/var/roothome").is_ok() {
            return Err(fail("image root home target is aliased"));
        }
        DirBuilder::new().mode(0o700).create("/var/roothome")?;
    }
    let result = generate();
    if dangling {
        // Refuse unexpected contents instead of deleting generated state.
        fs::remove_dir("/var/roothome")?;
    }
    result.map(drop)
}

/// The full listing (hashed into the receipt) after checking the declared
/// dracut modules are all included.
fn initramfs_listing(path: &str, modules: &[String]) -> Result<String> {
    let listed = text(run(&["/usr/bin/lsinitrd", "-m", path])?)?;
    let included: BTreeSet<&str> = listed.lines().collect();
    if let Some(missing) = modules.iter().find(|m| !included.contains(m.as_str())) {
        return Err(fail(format!(
            "generated initramfs lacks declared dracut module: {missing}"
        )));
    }
    text(run(&["/usr/bin/lsinitrd", path])?)
}

fn unit_name(name: &str) -> bool {
    let Some((stem, kind)) = name.rsplit_once('.') else {
        return false;
    };
    let mut bytes = stem.bytes();
    UNIT_TYPES.contains(&kind)
        && bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

fn unit_directory(name: &str) -> bool {
    name.strip_suffix(".wants")
        .or_else(|| name.strip_suffix(".requires"))
        .is_some_and(unit_name)
}

/// Lexical resolution of a link target relative to its directory.
fn normalized(path: &str, target: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !target.starts_with('/') {
        parts.extend(path.split('/').filter(|part| !part.is_empty()));
        parts.pop();
    }
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

fn link_target(path: &str, entry: &Entry) -> Result<String> {
    match entry {
        Entry::Symlink { target } => Ok(target.clone()),
        _ => Err(fail(format!(
            "systemd produced a non-symlink output: {path}"
        ))),
    }
}

fn unit_link(path: &str, entry: &Entry) -> Result<()> {
    let target = link_target(path, entry)?;
    if target == "/dev/null" {
        return Ok(());
    }
    let resolved = normalized(path, &target);
    if ![VENDOR_UNITS, UNITROOT]
        .iter()
        .any(|root| resolved.starts_with(&format!("{root}/")))
    {
        return Err(fail(format!(
            "systemd link escapes native unit roots: {path}"
        )));
    }
    if !Path::new(&resolved).is_file() {
        return Err(fail(format!("systemd link target does not exist: {path}")));
    }
    Ok(())
}

/// Whether a link under /etc/systemd/system is exactly one declared operation.
fn selected_unit_link(path: &str, entry: &Entry, step: &NativeStep, removed: bool) -> Result<bool> {
    let NativeStep::Systemd {
        enable,
        disable,
        mask,
        default_target,
    } = step
    else {
        return Ok(false);
    };
    let (Entry::Symlink { target }, Some(relative)) =
        (entry, path.strip_prefix(&format!("{UNITROOT}/")))
    else {
        return Ok(false);
    };
    if relative == "default.target" {
        return Ok(!removed && default_target.is_some());
    }
    let parts: Vec<&str> = relative.split('/').collect();
    let name = parts[parts.len() - 1];
    if !removed && parts.len() == 1 && mask.iter().any(|unit| unit == name) {
        return Ok(target == "/dev/null");
    }
    if parts.len() > 2
        || name.len() > 128
        || name.contains("..")
        || !unit_name(name)
        || (parts.len() == 2 && !unit_directory(parts[0]))
    {
        return Ok(false);
    }
    let resolved = normalized(path, target);
    let linked = [VENDOR_UNITS, UNITROOT]
        .iter()
        .find_map(|root| resolved.strip_prefix(&format!("{root}/")));
    let selected = if removed { disable } else { enable };
    let Some(linked) = linked.filter(|unit| selected.iter().any(|s| s == unit)) else {
        return Ok(false);
    };
    if linked != name && (parts.len() != 1 || linked.rsplit('.').next() != name.rsplit('.').next())
    {
        return Ok(false);
    }
    unit_link(path, entry)?;
    Ok(true)
}

#[derive(Default)]
struct Outputs {
    expected: BTreeSet<String>,
    kernels: Vec<NativeKernel>,
    systemd: Option<NativeStep>,
}
impl Outputs {
    fn selected(&self, path: &str, entry: &Entry, removed: bool) -> Result<bool> {
        match &self.systemd {
            Some(step) => selected_unit_link(path, entry, step, removed),
            None => Ok(false),
        }
    }

    /// Classify every changed path; anything undeclared refuses the derivation.
    fn artifacts(&self, before: &Tree, after: &Tree) -> Result<Vec<NativeArtifact>> {
        let mut unit_outputs = BTreeSet::new();
        for (path, entry) in after {
            if self.selected(path, entry, false)? {
                unit_outputs.insert(path.clone());
            }
        }
        let mut candidates: BTreeSet<&String> = self.expected.iter().collect();
        for (path, entry) in before {
            if after.get(path) != Some(entry) {
                candidates.insert(path);
            }
        }
        for (path, entry) in after {
            if before.get(path) != Some(entry) {
                candidates.insert(path);
            }
        }
        let mut artifacts = Vec::new();
        for path in candidates {
            if let Some(entry) =
                self.changed(path, before.get(path), after.get(path), &unit_outputs)?
            {
                artifacts.push(NativeArtifact {
                    path: path.clone(),
                    entry,
                });
            }
        }
        // Already-satisfied mask/default/enable links still belong in the receipt.
        if self.systemd.is_some() {
            for path in &unit_outputs {
                if !artifacts.iter().any(|artifact| artifact.path == *path) {
                    let entry = &after[path];
                    unit_link(path, entry)?;
                    if let Some(kind) = entry.artifact() {
                        artifacts.push(NativeArtifact {
                            path: path.clone(),
                            entry: kind,
                        });
                    }
                }
            }
        }
        artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(artifacts)
    }

    fn changed(
        &self,
        path: &str,
        previous: Option<&Entry>,
        current: Option<&Entry>,
        unit_outputs: &BTreeSet<String>,
    ) -> Result<Option<NativeArtifactKind>> {
        if let Some(Entry::Directory { mode }) = current {
            let prefix = format!("{path}/");
            let inside =
                |outputs: &BTreeSet<String>| outputs.iter().any(|o| o.starts_with(&prefix));
            // New parents of declared outputs are implied, never recorded.
            if previous.is_none()
                && (inside(&self.expected) || (*mode == 0o755 && inside(unit_outputs)))
            {
                return Ok(None);
            }
            return Err(fail(format!(
                "native transform changed an unrelated directory: {path}"
            )));
        }
        let kind = if self.expected.contains(path) {
            current.and_then(Entry::artifact)
        } else {
            Some(self.unit_change(path, previous, current)?)
        };
        let kind = kind.ok_or_else(|| {
            fail(format!(
                "native transform did not generate its declared artifact: {path}"
            ))
        })?;
        if matches!(kind, NativeArtifactKind::Regular { .. }) {
            regular(path, false, true)?;
        }
        Ok(Some(kind))
    }

    fn unit_change(
        &self,
        path: &str,
        previous: Option<&Entry>,
        current: Option<&Entry>,
    ) -> Result<NativeArtifactKind> {
        if self.systemd.is_none() || !path.starts_with(&format!("{UNITROOT}/")) {
            return Err(fail(format!(
                "native transform changed an unrelated path: {path}"
            )));
        }
        match (current, previous) {
            (Some(entry), _) => {
                unit_link(path, entry)?;
                if !self.selected(path, entry, false)? {
                    return Err(fail(format!(
                        "systemd changed an undeclared unit link: {path}"
                    )));
                }
                entry
                    .artifact()
                    .ok_or_else(|| fail(format!("systemd produced a directory: {path}")))
            }
            (None, Some(entry @ Entry::Symlink { target })) => {
                unit_link(path, entry)?;
                if !self.selected(path, entry, true)? {
                    return Err(fail(format!(
                        "systemd removed an undeclared unit link: {path}"
                    )));
                }
                Ok(NativeArtifactKind::RemovedSymlink {
                    target: target.clone(),
                })
            }
            _ => Err(fail(format!("systemd removed a non-symlink: {path}"))),
        }
    }
}

fn observe_artifact(artifact: &NativeArtifact) -> Result<()> {
    let path = artifact.path.as_str();
    parents(path)?;
    let expected = match &artifact.entry {
        NativeArtifactKind::RemovedSymlink { .. } => {
            if fs::symlink_metadata(path).is_ok() {
                return Err(fail(format!("removed unit link is present: {path}")));
            }
            return Ok(());
        }
        NativeArtifactKind::Regular {
            sha256,
            bytes,
            mode,
        } => {
            regular(path, false, true)?;
            Entry::Regular {
                sha256: sha256.clone(),
                bytes: *bytes,
                mode: *mode,
            }
        }
        NativeArtifactKind::Symlink { target } => Entry::Symlink {
            target: target.clone(),
        },
    };
    if record(path)? != expected {
        return Err(fail(format!(
            "native receipt differs from final artifact: {path}"
        )));
    }
    Ok(())
}

fn observe_kernel(kernel: &NativeKernel, modules: &[String]) -> Result<()> {
    if !kernel_version(&kernel.version) {
        return Err(fail("invalid native kernel version"));
    }
    let root = format!("{MODULES}/{}", kernel.version);
    if digest(Path::new(&format!("{root}/vmlinuz")))? != kernel.kernel_sha256 {
        return Err(fail("native kernel changed"));
    }
    let listing = initramfs_listing(&format!("{root}/initramfs.img"), modules)?;
    if plan::hash(listing.as_bytes()) != kernel.listing_sha256 {
        return Err(fail("native initramfs listing changed"));
    }
    for (driver, path) in &kernel.drivers {
        if !listing.contains(path.trim_start_matches('/')) {
            return Err(fail(format!(
                "native initramfs module disappeared: {driver}"
            )));
        }
    }
    Ok(())
}
