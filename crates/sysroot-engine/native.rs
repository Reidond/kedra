//! Closed native transformations. Planning grants no host execution authority.
use crate::{Error, MAX_JSON, PLATFORM, Result, SystemComposition, VerifiedComposition, plan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const NATIVE_RECEIPT_PATH: &str = "/usr/share/sysroot/native-receipt.json";
const HOME: &str = "/usr/share/sysroot/home/default/";
/// A new account's applied home baseline inside `/etc/skel` (see `sysroot home`).
pub const SKEL_APPLIED: &str = ".local/state/kedra/home/applied/";
const SCHEMAS: &str = "/usr/share/glib-2.0/schemas/";
const SYSTEMD: &str = "/etc/systemd/system/";
const MODULES: &str = "/usr/lib/modules/";
const IDENTITY_DOMAIN: &[u8] = b"sysroot-native-model-v1\0";
/// Read-only BuildKit bind mounts; neither file enters an image layer.
pub const NATIVE_MOUNT: &str = "/run/sysroot-native";
pub const NATIVE_EXECUTOR: &str = "/run/sysroot-native/sysroot";
pub const NATIVE_PLAN: &str = "/run/sysroot-native/native-plan.json";
/// Written only inside the throwaway observation stage, never the derived image.
pub const NATIVE_OBSERVATION: &str = "/sysroot-native-observation.json";
const RECIPE: &str = "FROM sysroot-foundation:<parent>\nUSER 0:0\nWORKDIR /\nRUN --network=none --mount=type=bind,source=sysroot,target=/run/sysroot-native/sysroot --mount=type=bind,source=native-plan.json,target=/run/sysroot-native/native-plan.json [\"/run/sysroot-native/sysroot\", \"derive\", \"apply\", \"/run/sysroot-native/native-plan.json\"]\n";
const OBSERVATION_RECIPE: &str = "FROM sysroot-foundation:<image> AS observe\nUSER 0:0\nWORKDIR /\nRUN --network=none --mount=type=bind,source=sysroot,target=/run/sysroot-native/sysroot --mount=type=bind,source=native-plan.json,target=/run/sysroot-native/native-plan.json [\"/run/sysroot-native/sysroot\", \"derive\", \"observe\", \"/run/sysroot-native/native-plan.json\"]\nFROM scratch\nCOPY --from=observe /sysroot-native-observation.json /\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDefinition {
    pub schema: u32,
    pub parent_identity: String,
    pub steps: Vec<NativeStep>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeStep {
    GlibSchemas,
    Systemd {
        #[serde(default)]
        enable: Vec<String>,
        #[serde(default)]
        disable: Vec<String>,
        #[serde(default)]
        mask: Vec<String>,
        default_target: Option<String>,
    },
    /// Copies the committed home baseline to `/etc/skel` and records it there as
    /// a new account's applied baseline (`.local/state/kedra/home/applied`).
    InitialSkel,
    /// Regenerates every image kernel's generic (no-hostonly) initramfs with the
    /// image's own dracut configuration, adding and then requiring the listed
    /// dracut modules and kernel drivers.
    Initramfs {
        modules: Vec<String>,
        #[serde(default)]
        drivers: Vec<String>,
    },
    /// Runs each declared check on its committed home baseline file, as `nobody`
    /// with a scratch home and no network; `{file}` in an argument becomes the
    /// baseline file's path. A check produces no output and changes nothing.
    HomeChecks {
        #[serde(deserialize_with = "crate::model::unique_map")]
        checks: BTreeMap<String, Vec<String>>,
    },
}
/// Placeholder a home check argument uses for the checked file's path.
pub const CHECK_FILE: &str = "{file}";
/// Picks the declared steps out of a verified parent composition, for example
/// from a committed manifest among its files. It runs only after verification.
pub type StepSelector<'a> = &'a dyn Fn(&SystemComposition) -> Result<Vec<NativeStep>>;
/// Supplied only by the trusted Rust adapter, never deserialized from a plan.
/// `executor_sha256` names the exact sysroot binary that runs inside the build.
pub struct NativeImplementation {
    pub version: &'static str,
    pub executor_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInput {
    pub path: String,
    pub sha256: String,
    pub mode: u32,
    pub bytes: u64,
}
/// Recompute from a VerifiedComposition before execution; a deserialized plan is
/// merely data and does not establish verification or release authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePlan {
    pub schema: u32,
    pub identity: String,
    pub definition: NativeDefinition,
    pub implementation: String,
    pub driver_sha256: String,
    pub recipe_sha256: String,
    pub foundation_image: String,
    pub rpm_sha256: String,
    pub inputs: Vec<NativeInput>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeArtifact {
    pub path: String,
    pub entry: NativeArtifactKind,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeArtifactKind {
    Regular {
        sha256: String,
        bytes: u64,
        mode: u32,
    },
    Symlink {
        target: String,
    },
    RemovedSymlink {
        target: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeKernel {
    pub version: String,
    pub kernel_sha256: String,
    pub listing_sha256: String,
    /// Declared driver name to its resolved module file in this kernel.
    #[serde(deserialize_with = "crate::model::unique_map")]
    pub drivers: BTreeMap<String, String>,
}
/// Actual observed material; the deriver binds this to final image and daemon IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeReceipt {
    pub schema: u32,
    pub identity: String,
    pub parent_identity: String,
    pub implementation: String,
    pub driver_sha256: String,
    pub recipe_sha256: String,
    pub foundation_image: String,
    pub rpm_sha256: String,
    pub artifacts: Vec<NativeArtifact>,
    pub kernels: Vec<NativeKernel>,
    #[serde(deserialize_with = "crate::model::unique_map")]
    pub tools: BTreeMap<String, String>,
}

/// Binds the selected composition's declared steps to that parent.
pub fn native_definition(
    composition: &SystemComposition,
    steps: StepSelector<'_>,
) -> Result<NativeDefinition> {
    let mut definition = NativeDefinition {
        schema: 1,
        parent_identity: composition.identity.clone(),
        steps: steps(composition)?,
    };
    normalize(&mut definition)?;
    Ok(definition)
}

/// Check and order a step declaration; authoring frontends call this to refuse
/// an invalid declaration before anything is composed.
pub fn normalize_native_steps(steps: &mut [NativeStep]) -> Result<()> {
    if steps.is_empty() || steps.len() > 5 {
        return Err(invalid("unsupported step count"));
    }
    let mut kinds = BTreeSet::new();
    for step in steps.iter_mut() {
        if !kinds.insert(order(step)) {
            return Err(invalid("duplicate native step"));
        }
        match step {
            NativeStep::Systemd {
                enable,
                disable,
                mask,
                default_target,
            } => normalize_systemd(enable, disable, mask, default_target.as_deref())?,
            NativeStep::Initramfs { modules, drivers } => normalize_initramfs(modules, drivers)?,
            NativeStep::HomeChecks { checks } => normalize_checks(checks)?,
            NativeStep::GlibSchemas | NativeStep::InitialSkel => {}
        }
    }
    steps.sort_by_key(order);
    Ok(())
}

pub fn plan_native(
    parent: &VerifiedComposition,
    definition: &NativeDefinition,
    implementation: &NativeImplementation,
) -> Result<NativePlan> {
    let composition = parent.composition();
    let mut definition = definition.clone();
    normalize(&mut definition)?;
    if definition.parent_identity != composition.identity
        || composition.plan.definition.platform != PLATFORM
    {
        return Err(invalid("parent identity or native platform differs"));
    }
    if implementation.version.is_empty()
        || implementation.version.len() > 128
        || !implementation
            .version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        || !plan::hex(&implementation.executor_sha256)
    {
        return Err(invalid("invalid compiled native implementation"));
    }
    let mut inputs = Vec::new();
    for file in &composition.plan.files {
        if selected(&definition, &file.path) {
            inputs.push(NativeInput {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                mode: file.mode,
                bytes: file.bytes.len() as u64,
            });
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    if definition.steps.contains(&NativeStep::InitialSkel)
        && !inputs.iter().any(|input| input.path.starts_with(HOME))
    {
        return Err(invalid(
            "initial skeleton needs declared committed baseline files",
        ));
    }
    for step in &definition.steps {
        if let NativeStep::HomeChecks { checks } = step
            && checks.keys().any(|path| {
                !inputs
                    .iter()
                    .any(|input| input.path == format!("{HOME}{path}"))
            })
        {
            return Err(invalid("home check names no committed baseline file"));
        }
    }
    let mut result = NativePlan {
        schema: 1,
        identity: String::new(),
        definition,
        implementation: implementation.version.to_owned(),
        driver_sha256: implementation.executor_sha256.clone(),
        recipe_sha256: plan::hash(RECIPE.as_bytes()),
        foundation_image: composition.plan.foundation.receipt.image.clone(),
        rpm_sha256: composition.plan.foundation.rpm_sha256.clone(),
        inputs,
    };
    let mut identity = IDENTITY_DOMAIN.to_vec();
    identity.extend(serde_json::to_vec(&result)?);
    result.identity = plan::hash(&identity);
    Ok(result)
}

/// The deriver pins this tag to the checked immutable static image immediately
/// before its isolated, offline build; no user supplied image reference is rendered.
pub fn render_native_recipe(plan: &NativePlan, parent_image: &str) -> Result<String> {
    plan::image_id(parent_image)?;
    if plan.recipe_sha256 != plan::hash(RECIPE.as_bytes()) {
        return Err(invalid("native recipe implementation differs"));
    }
    Ok(RECIPE.replace("<parent>", &parent_image[7..]))
}
/// Readback of a built image in a throwaway stage that exports only its report.
pub(crate) fn render_observation_recipe(image: &str) -> Result<String> {
    plan::image_id(image)?;
    Ok(OBSERVATION_RECIPE.replace("<image>", &image[7..]))
}

fn selected(definition: &NativeDefinition, path: &str) -> bool {
    definition.steps.iter().any(|step| match step {
        NativeStep::GlibSchemas => path.starts_with(SCHEMAS),
        NativeStep::Systemd { .. } => {
            path.starts_with("/usr/lib/systemd/system/") || path.starts_with(SYSTEMD)
        }
        NativeStep::InitialSkel | NativeStep::HomeChecks { .. } => path.starts_with(HOME),
        NativeStep::Initramfs { .. } => {
            path.starts_with("/usr/lib/dracut/dracut.conf.d/")
                || path.starts_with("/etc/dracut.conf.d/")
                || path == "/etc/dracut.conf"
                || path == "/usr/share/sysroot/source.json"
        }
    })
}
fn normalize(definition: &mut NativeDefinition) -> Result<()> {
    if definition.schema != 1
        || !plan::hex(&definition.parent_identity)
        || serde_json::to_vec(definition)?.len() as u64 > MAX_JSON
    {
        return Err(invalid("unsupported schema, identity, or size"));
    }
    normalize_native_steps(&mut definition.steps)
}
/// Each unit takes at most one operation; every list is sorted once it is checked.
fn normalize_systemd(
    enable: &mut [String],
    disable: &mut [String],
    mask: &mut [String],
    default_target: Option<&str>,
) -> Result<()> {
    let mut units = BTreeSet::new();
    for names in [enable, disable, mask] {
        if names.len() > 64 {
            return Err(invalid("too many units"));
        }
        for name in names.iter() {
            unit(name)?;
            if !units.insert(name.clone()) {
                return Err(invalid("duplicate or conflicting unit operation"));
            }
        }
        names.sort();
    }
    if let Some(target) = default_target {
        unit(target)?;
        if !target.ends_with(".target") || units.contains(target) {
            return Err(invalid("invalid or conflicting default target"));
        }
    }
    if units.is_empty() && default_target.is_none() {
        return Err(invalid("empty systemd step"));
    }
    Ok(())
}
/// Each check names a baseline file and an argv with an absolute program.
fn normalize_checks(checks: &BTreeMap<String, Vec<String>>) -> Result<()> {
    if checks.is_empty() || checks.len() > 256 {
        return Err(invalid("invalid home check count"));
    }
    for (path, argv) in checks {
        plan::relative(path, false)?;
        let program = argv.first().ok_or_else(|| invalid("empty home check"))?;
        absolute(program)?;
        if argv.len() > 64
            || argv
                .iter()
                .any(|argument| argument.len() > 4096 || argument.bytes().any(|b| b < 32))
        {
            return Err(invalid("invalid home check argument"));
        }
    }
    Ok(())
}
/// Dracut modules are named like `rootfs-block`; kernel drivers like `virtio_gpu`.
fn normalize_initramfs(modules: &mut [String], drivers: &mut [String]) -> Result<()> {
    if modules.is_empty() || modules.len() > 64 || drivers.len() > 32 {
        return Err(invalid("invalid initramfs module or driver count"));
    }
    for (names, extra) in [(&mut *modules, b'-'), (&mut *drivers, b'_')] {
        let mut seen = BTreeSet::new();
        for name in names.iter() {
            if name.is_empty()
                || name.len() > 128
                || !name.as_bytes()[0].is_ascii_alphanumeric()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == extra)
                || !seen.insert(name.clone())
            {
                return Err(invalid("invalid or duplicate initramfs module or driver"));
            }
        }
        names.sort();
    }
    Ok(())
}
fn order(step: &NativeStep) -> u8 {
    match step {
        NativeStep::GlibSchemas => 0,
        NativeStep::Systemd { .. } => 1,
        NativeStep::InitialSkel => 2,
        NativeStep::Initramfs { .. } => 3,
        NativeStep::HomeChecks { .. } => 4,
    }
}
fn unit(name: &str) -> Result<()> {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return Err(invalid("unit requires a concrete type"));
    };
    if !matches!(
        extension,
        "service" | "socket" | "timer" | "target" | "path"
    ) || stem.is_empty()
        || name.len() > 128
        || !stem.as_bytes()[0].is_ascii_alphanumeric()
        || !stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        || stem.contains("..")
        || name == "default.target"
    {
        return Err(invalid("unsupported concrete unit name"));
    }
    Ok(())
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(format!("native derivation: {}", message.into()))
}

/// Checks the closed material contract. The deriver must additionally re-read
/// these bytes and links from the immutable final image, compare full RPM rows,
/// and bind that image to its current daemon; JSON alone is never evidence.
pub fn validate_native_receipt(expected: &NativePlan, receipt: &NativeReceipt) -> Result<()> {
    if receipt.schema != 1
        || receipt.identity != expected.identity
        || receipt.parent_identity != expected.definition.parent_identity
        || receipt.implementation != expected.implementation
        || receipt.driver_sha256 != expected.driver_sha256
        || receipt.recipe_sha256 != expected.recipe_sha256
        || receipt.foundation_image != expected.foundation_image
        || receipt.rpm_sha256 != expected.rpm_sha256
        || receipt.artifacts.len() > 20_000
        || receipt.kernels.len() > 16
        || serde_json::to_vec(receipt)?.len() as u64 > MAX_JSON
    {
        return Err(invalid("native material identity or bounds differ"));
    }
    validate_tools(expected, receipt)?;
    let artifacts = declared_artifacts(expected, receipt)?;
    for step in &expected.definition.steps {
        match step {
            NativeStep::GlibSchemas => validate_schemas(&artifacts)?,
            NativeStep::InitialSkel => validate_skeleton(&expected.inputs, &artifacts)?,
            NativeStep::Systemd {
                enable,
                mask,
                default_target,
                ..
            } => validate_systemd_links(
                enable,
                mask,
                default_target.as_deref(),
                receipt,
                &artifacts,
            )?,
            NativeStep::Initramfs { drivers, .. } => {
                validate_kernels(drivers, receipt, &artifacts)?
            }
            NativeStep::HomeChecks { .. } => {}
        }
    }
    if !expected
        .definition
        .steps
        .iter()
        .any(|s| matches!(s, NativeStep::Initramfs { .. }))
        && !receipt.kernels.is_empty()
    {
        return Err(invalid("unrequested kernel evidence"));
    }
    Ok(())
}
fn validate_tools(expected: &NativePlan, receipt: &NativeReceipt) -> Result<()> {
    let tools = native_tools(expected);
    if receipt.tools.keys().cloned().collect::<BTreeSet<_>>() != tools
        || receipt.tools.values().any(|hash| !plan::hex(hash))
    {
        return Err(invalid("unexpected or invalid installed tool inventory"));
    }
    Ok(())
}
/// Checks each output in receipt order and indexes the declared ones by path.
fn declared_artifacts<'a>(
    expected: &NativePlan,
    receipt: &'a NativeReceipt,
) -> Result<BTreeMap<&'a str, &'a NativeArtifactKind>> {
    let mut artifacts = BTreeMap::new();
    for artifact in &receipt.artifacts {
        absolute(&artifact.path)?;
        if artifacts
            .insert(artifact.path.as_str(), &artifact.entry)
            .is_some()
        {
            return Err(invalid("duplicate native output"));
        }
        if let NativeArtifactKind::Regular {
            sha256,
            bytes,
            mode,
        } = &artifact.entry
            && (!plan::hex(sha256)
                || *bytes > 1024 * 1024 * 1024
                || mode & !0o777 != 0
                || mode & 0o022 != 0)
        {
            return Err(invalid("invalid native output digest, mode, or size"));
        }
        if !allowed_artifact(expected, receipt, artifact)? {
            return Err(invalid(format!(
                "undeclared native output {}",
                artifact.path
            )));
        }
    }
    Ok(artifacts)
}
fn validate_schemas(artifacts: &BTreeMap<&str, &NativeArtifactKind>) -> Result<()> {
    if !matches!(artifacts.get("/usr/share/glib-2.0/schemas/gschemas.compiled"),
        Some(NativeArtifactKind::Regular {bytes,mode:0o644,..}) if *bytes > 0)
    {
        return Err(invalid("missing compiled GLib schemas"));
    }
    Ok(())
}
fn validate_skeleton(
    inputs: &[NativeInput],
    artifacts: &BTreeMap<&str, &NativeArtifactKind>,
) -> Result<()> {
    for input in inputs {
        if let Some(relative) = input.path.strip_prefix(HOME) {
            for destination in [
                format!("/etc/skel/{relative}"),
                format!("/etc/skel/{SKEL_APPLIED}{relative}"),
            ] {
                if !matches!(artifacts.get(destination.as_str()),
                    Some(NativeArtifactKind::Regular {sha256,bytes,mode})
                    if *sha256 == input.sha256 && *bytes == input.bytes && *mode == input.mode)
                {
                    return Err(invalid("initial skeleton differs from committed input"));
                }
            }
        }
    }
    Ok(())
}
fn validate_systemd_links(
    enable: &[String],
    mask: &[String],
    default_target: Option<&str>,
    receipt: &NativeReceipt,
    artifacts: &BTreeMap<&str, &NativeArtifactKind>,
) -> Result<()> {
    for name in enable {
        if !receipt.artifacts.iter().any(|a| {
            a.path != format!("{SYSTEMD}default.target")
                && matches!(&a.entry, NativeArtifactKind::Symlink { target }
                    if link_unit(&a.path, target).is_ok_and(|unit| unit == *name))
        }) {
            return Err(invalid("enabled unit has no recorded link"));
        }
    }
    for name in mask {
        if !matches!(artifacts.get(format!("{SYSTEMD}{name}").as_str()),
            Some(NativeArtifactKind::Symlink {target}) if target == "/dev/null")
        {
            return Err(invalid("missing unit mask"));
        }
    }
    if let Some(target) = default_target {
        let path = format!("{SYSTEMD}default.target");
        if !matches!(artifacts.get(path.as_str()), Some(NativeArtifactKind::Symlink{target:actual})
            if link_unit(&path,actual).is_ok_and(|actual| actual == target))
        {
            return Err(invalid("default target link differs"));
        }
    }
    Ok(())
}

/// Exact image executables whose resolved bytes the guest records. The executor
/// itself is bound by `driver_sha256`; it runs on the image's own C library.
pub fn native_tools(plan: &NativePlan) -> BTreeSet<String> {
    let mut tools = BTreeSet::from(["/usr/bin/rpm".to_owned()]);
    for step in &plan.definition.steps {
        match step {
            NativeStep::GlibSchemas => {
                tools.insert("/usr/bin/glib-compile-schemas".into());
            }
            NativeStep::Systemd { .. } => {
                tools.insert("/usr/bin/systemctl".into());
            }
            NativeStep::InitialSkel => {}
            NativeStep::Initramfs { .. } => {
                tools.extend(
                    ["/usr/bin/dracut", "/usr/bin/lsinitrd", "/usr/sbin/modinfo"].map(String::from),
                );
            }
            NativeStep::HomeChecks { checks } => {
                tools.extend(checks.values().filter_map(|argv| argv.first().cloned()));
            }
        }
    }
    tools
}
fn allowed_artifact(
    plan: &NativePlan,
    receipt: &NativeReceipt,
    artifact: &NativeArtifact,
) -> Result<bool> {
    for step in &plan.definition.steps {
        match step {
            NativeStep::GlibSchemas if artifact.path == format!("{SCHEMAS}gschemas.compiled") => {
                return Ok(matches!(artifact.entry, NativeArtifactKind::Regular { .. }));
            }
            NativeStep::InitialSkel => {
                if let Some(relative) = artifact.path.strip_prefix("/etc/skel/") {
                    let relative = relative.strip_prefix(SKEL_APPLIED).unwrap_or(relative);
                    return Ok(matches!(artifact.entry, NativeArtifactKind::Regular { .. })
                        && plan
                            .inputs
                            .iter()
                            .any(|input| input.path == format!("{HOME}{relative}")));
                }
            }
            NativeStep::Initramfs { .. } => {
                if receipt.kernels.iter().any(|kernel| {
                    artifact.path == format!("{MODULES}{}/initramfs.img", kernel.version)
                }) {
                    return Ok(matches!(artifact.entry, NativeArtifactKind::Regular { .. }));
                }
            }
            NativeStep::Systemd {
                enable,
                disable,
                mask,
                default_target,
            } => {
                if let Some(relative) = artifact.path.strip_prefix(SYSTEMD) {
                    let name = relative
                        .rsplit('/')
                        .next()
                        .ok_or_else(|| invalid("empty unit path"))?;
                    if relative == "default.target" {
                        return Ok(matches!((&artifact.entry,default_target),
                            (NativeArtifactKind::Symlink{target},Some(expected))
                            if link_unit(&artifact.path,target)? == *expected));
                    }
                    unit(name)?;
                    if let Some((directory, _)) = relative.rsplit_once('/') {
                        let target = directory
                            .strip_suffix(".wants")
                            .or_else(|| directory.strip_suffix(".requires"))
                            .ok_or_else(|| invalid("unapproved systemd link directory"))?;
                        unit(target)?;
                    }
                    let selected_link = |target: &str, selected: &[String]| -> Result<bool> {
                        let linked = link_unit(&artifact.path, target)?;
                        Ok(selected.contains(&linked)
                            && (linked == name
                                || (relative == name
                                    && name.rsplit('.').next() == linked.rsplit('.').next())))
                    };
                    return Ok(match &artifact.entry {
                        NativeArtifactKind::Symlink { target } if target == "/dev/null" => {
                            relative == name && mask.iter().any(|unit| unit == name)
                        }
                        NativeArtifactKind::Symlink { target } => selected_link(target, enable)?,
                        NativeArtifactKind::RemovedSymlink { target } => {
                            selected_link(target, disable)?
                        }
                        NativeArtifactKind::Regular { .. } => false,
                    });
                }
            }
            NativeStep::GlibSchemas | NativeStep::HomeChecks { .. } => {}
        }
    }
    Ok(false)
}
fn validate_kernels(
    required: &[String],
    receipt: &NativeReceipt,
    artifacts: &BTreeMap<&str, &NativeArtifactKind>,
) -> Result<()> {
    if receipt.kernels.is_empty() {
        return Err(invalid("no image-local kernel generated"));
    }
    let mut versions = BTreeSet::new();
    for kernel in &receipt.kernels {
        if kernel.version.is_empty()
            || kernel.version.len() > 128
            || !kernel
                .version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
            || kernel.version.starts_with('.')
            || kernel.version.contains("..")
            || !versions.insert(&kernel.version)
            || !plan::hex(&kernel.kernel_sha256)
            || !plan::hex(&kernel.listing_sha256)
            || kernel.drivers.keys().collect::<Vec<_>>() != required.iter().collect::<Vec<_>>()
        {
            return Err(invalid("invalid kernel/module evidence"));
        }
        let prefix = format!("{MODULES}{}/", kernel.version);
        for (name, path) in &kernel.drivers {
            absolute(path)?;
            let basename = path
                .rsplit('/')
                .next()
                .ok_or_else(|| invalid("empty module path"))?;
            if !path.starts_with(&prefix)
                || ![".ko", ".ko.xz", ".ko.zst", ".ko.gz"].iter().any(|suffix| {
                    basename
                        .strip_suffix(suffix)
                        .is_some_and(|stem| stem.replace('-', "_") == *name)
                })
            {
                return Err(invalid("required module escapes selected kernel or name"));
            }
        }
        if !matches!(artifacts.get(format!("{prefix}initramfs.img").as_str()),
            Some(NativeArtifactKind::Regular {bytes,..}) if *bytes > 0)
        {
            return Err(invalid("missing generated kernel initramfs"));
        }
    }
    Ok(())
}
fn absolute(path: &str) -> Result<()> {
    let relative = path
        .strip_prefix('/')
        .ok_or_else(|| invalid("output path is not absolute"))?;
    plan::relative(relative, false)
}
fn link_unit(path: &str, target: &str) -> Result<String> {
    if target.is_empty() || target.len() > 4096 || target.bytes().any(|b| b < 32 || b == 127) {
        return Err(invalid("unsafe unit link target"));
    }
    let mut parts: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        path.trim_start_matches('/').split('/').collect()
    };
    if !target.starts_with('/') {
        parts.pop();
    }
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(invalid("unit link traverses root"));
                }
            }
            _ => parts.push(part),
        }
    }
    let resolved = format!("/{}", parts.join("/"));
    let name = resolved
        .strip_prefix("/usr/lib/systemd/system/")
        .or_else(|| resolved.strip_prefix(SYSTEMD))
        .ok_or_else(|| invalid("unit link escapes approved native roots"))?;
    unit(name)?;
    Ok(name.to_owned())
}
