//! Closed native transformations. Planning grants no host execution authority.
use crate::{Error, MAX_JSON, PLATFORM, Result, VerifiedComposition, plan};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
};

pub const NATIVE_RECEIPT_PATH: &str = "/usr/share/sysroot/native-receipt.json";
const HOME: &str = "/usr/share/sysroot/home/default/";
const SCHEMAS: &str = "/usr/share/glib-2.0/schemas/";
const SYSTEMD: &str = "/etc/systemd/system/";
const MODULES: &str = "/usr/lib/modules/";
const IDENTITY_DOMAIN: &[u8] = b"sysroot-native-model-v1\0";
const RECIPE: &str = "FROM sysroot-foundation:<parent>\nUSER 0:0\nWORKDIR /\nCOPY native-plan.json /tmp/kedra-native-plan.json\nCOPY native-driver.py /tmp/kedra-native-driver.py\nRUN [\"/usr/bin/python3\", \"-I\", \"/tmp/kedra-native-driver.py\", \"build\"]\nRUN [\"/usr/bin/rm\", \"--\", \"/tmp/kedra-native-plan.json\", \"/tmp/kedra-native-driver.py\"]\n";

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
    InitialSkel,
    QemuInitramfs {
        #[serde(default = "default_native_modules")]
        required_modules: Vec<String>,
    },
}
pub fn default_native_modules() -> Vec<String> {
    ["virtio_dma_buf", "virtio_gpu", "virtio_input"]
        .map(String::from)
        .to_vec()
}
/// Supplied only by the trusted Rust adapter, never deserialized from a plan.
/// The adapter must pass its reviewed, compiled-in driver and implementation version.
pub struct NativeImplementation {
    pub version: &'static str,
    pub driver: &'static str,
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
    #[serde(deserialize_with = "crate::model::unique_map")]
    pub required_modules: BTreeMap<String, String>,
}
/// Actual observed material; the harness binds this to final image and daemon IDs.
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

pub fn read_native(path: &Path) -> Result<NativeDefinition> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(invalid("oversized declaration"));
    }
    let mut definition: NativeDefinition = serde_json::from_slice(&bytes)?;
    normalize(&mut definition)?;
    Ok(definition)
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
        return Err(invalid("parent identity or native ARM platform differs"));
    }
    if implementation.version.is_empty()
        || implementation.version.len() > 128
        || !implementation
            .version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        || implementation.driver.is_empty()
        || implementation.driver.len() > 1024 * 1024
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
    let mut result = NativePlan {
        schema: 1,
        identity: String::new(),
        definition,
        implementation: implementation.version.to_owned(),
        driver_sha256: plan::hash(implementation.driver.as_bytes()),
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

/// The harness pins this tag to the checked immutable static image immediately
/// before its isolated, offline build; no user supplied image reference is rendered.
pub fn render_native_recipe(plan: &NativePlan, parent_image: &str) -> Result<String> {
    plan::image_id(parent_image)?;
    if plan.recipe_sha256 != plan::hash(RECIPE.as_bytes()) {
        return Err(invalid("native recipe implementation differs"));
    }
    Ok(RECIPE.replace("<parent>", &parent_image[7..]))
}

fn selected(definition: &NativeDefinition, path: &str) -> bool {
    definition.steps.iter().any(|step| match step {
        NativeStep::GlibSchemas => path.starts_with(SCHEMAS),
        NativeStep::Systemd { .. } => {
            path.starts_with("/usr/lib/systemd/system/") || path.starts_with(SYSTEMD)
        }
        NativeStep::InitialSkel => path.starts_with(HOME),
        NativeStep::QemuInitramfs { .. } => {
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
        || definition.steps.is_empty()
        || definition.steps.len() > 4
        || serde_json::to_vec(definition)?.len() as u64 > MAX_JSON
    {
        return Err(invalid("unsupported schema, identity, or step count"));
    }
    let mut kinds = BTreeSet::new();
    for step in &mut definition.steps {
        if !kinds.insert(order(step)) {
            return Err(invalid("duplicate native step"));
        }
        match step {
            NativeStep::Systemd {
                enable,
                disable,
                mask,
                default_target,
            } => {
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
            }
            NativeStep::QemuInitramfs { required_modules } => {
                if required_modules.is_empty() || required_modules.len() > 32 {
                    return Err(invalid("invalid module count"));
                }
                let mut modules = BTreeSet::new();
                for module in required_modules.iter() {
                    if module.is_empty()
                        || module.len() > 128
                        || !module
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        || !modules.insert(module.clone())
                    {
                        return Err(invalid("invalid or duplicate module"));
                    }
                }
                for required in default_native_modules() {
                    if !modules.contains(&required) {
                        return Err(invalid("required QEMU graphics/input module omitted"));
                    }
                }
                required_modules.sort();
            }
            NativeStep::GlibSchemas | NativeStep::InitialSkel => {}
        }
    }
    definition.steps.sort_by_key(order);
    Ok(())
}
fn order(step: &NativeStep) -> u8 {
    match step {
        NativeStep::GlibSchemas => 0,
        NativeStep::Systemd { .. } => 1,
        NativeStep::InitialSkel => 2,
        NativeStep::QemuInitramfs { .. } => 3,
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

/// Checks the closed material contract. The harness must additionally re-read
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
    let tools = native_tools(expected);
    if receipt
        .tools
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != tools
        || receipt.tools.values().any(|hash| !plan::hex(hash))
    {
        return Err(invalid("unexpected or invalid installed tool inventory"));
    }
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
    for step in &expected.definition.steps {
        match step {
            NativeStep::GlibSchemas => {
                if !matches!(artifacts.get("/usr/share/glib-2.0/schemas/gschemas.compiled"),
                    Some(NativeArtifactKind::Regular {bytes,mode:0o644,..}) if *bytes > 0)
                {
                    return Err(invalid("missing compiled GLib schemas"));
                }
            }
            NativeStep::InitialSkel => {
                for input in &expected.inputs {
                    if let Some(relative) = input.path.strip_prefix(HOME) {
                        let destination = format!("/etc/skel/{relative}");
                        if !matches!(artifacts.get(destination.as_str()),
                            Some(NativeArtifactKind::Regular {sha256,bytes,mode})
                            if *sha256 == input.sha256 && *bytes == input.bytes && *mode == input.mode)
                        {
                            return Err(invalid("initial skeleton differs from committed input"));
                        }
                    }
                }
            }
            NativeStep::Systemd {
                enable,
                mask,
                default_target,
                ..
            } => {
                for name in enable {
                    if !receipt.artifacts.iter().any(|a| {
                        a.path.rsplit('/').next() == Some(name)
                            && matches!(a.entry, NativeArtifactKind::Symlink { .. })
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
                        if link_unit(&path,actual).is_ok_and(|actual| actual == *target))
                    {
                        return Err(invalid("default target link differs"));
                    }
                }
            }
            NativeStep::QemuInitramfs { required_modules } => {
                validate_kernels(required_modules, receipt, &artifacts)?
            }
        }
    }
    if !expected
        .definition
        .steps
        .iter()
        .any(|s| matches!(s, NativeStep::QemuInitramfs { .. }))
        && !receipt.kernels.is_empty()
    {
        return Err(invalid("unrequested kernel evidence"));
    }
    Ok(())
}

/// Exact executable inputs whose resolved bytes the guest records.
pub fn native_tools(plan: &NativePlan) -> BTreeSet<&'static str> {
    let mut tools = BTreeSet::from(["/usr/bin/python3", "/usr/bin/rpm"]);
    for step in &plan.definition.steps {
        match step {
            NativeStep::GlibSchemas => {
                tools.insert("/usr/bin/glib-compile-schemas");
            }
            NativeStep::Systemd { .. } => {
                tools.insert("/usr/bin/systemctl");
            }
            NativeStep::InitialSkel => {}
            NativeStep::QemuInitramfs { .. } => {
                tools.extend(["/usr/bin/dracut", "/usr/bin/lsinitrd", "/usr/sbin/modinfo"]);
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
                    return Ok(matches!(artifact.entry, NativeArtifactKind::Regular { .. })
                        && plan
                            .inputs
                            .iter()
                            .any(|input| input.path == format!("{HOME}{relative}")));
                }
            }
            NativeStep::QemuInitramfs { .. } => {
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
                    return Ok(match &artifact.entry {
                        NativeArtifactKind::Symlink { target } if target == "/dev/null" => {
                            relative == name && mask.iter().any(|unit| unit == name)
                        }
                        NativeArtifactKind::Symlink { target } => {
                            enable.iter().any(|unit| unit == name)
                                && link_unit(&artifact.path, target)? == name
                        }
                        NativeArtifactKind::RemovedSymlink { target } => {
                            disable.iter().any(|unit| unit == name)
                                && link_unit(&artifact.path, target)? == name
                        }
                        NativeArtifactKind::Regular { .. } => false,
                    });
                }
            }
            NativeStep::GlibSchemas => {}
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
            || kernel.required_modules.keys().collect::<Vec<_>>()
                != required.iter().collect::<Vec<_>>()
        {
            return Err(invalid("invalid kernel/module evidence"));
        }
        let prefix = format!("{MODULES}{}/", kernel.version);
        for (name, path) in &kernel.required_modules {
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
