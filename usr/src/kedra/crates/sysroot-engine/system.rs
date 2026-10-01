//! Typed native configuration and verified runtime closure export.
use crate::{
    Argument, Error, ImageReceipt, LOGICAL_PREFIX, MAX_JSON, ObjectReceipt, PLATFORM, Result,
    Segment, Store, executor, plan,
    store::{hash_file, nonce},
    tree::{self, Kind},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions, Permissions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const MAX_PAYLOAD: u64 = 16 * 1024 * 1024 * 1024;
const MAX_IMAGE: u64 = 8 * 1024 * 1024 * 1024;
const RPM_FORMAT: &str = "%{NAME}\\t%{EPOCHNUM}\\t%{VERSION}\\t%{RELEASE}\\t%{ARCH}\\t%{SHA256HEADER}\\t%{PAYLOADSHA256}\\n";
const RECEIPT_PATH: &str = "/usr/share/sysroot/composition.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemDefinition {
    pub schema: u32,
    pub platform: String,
    pub foundation: String,
    #[serde(default, deserialize_with = "crate::model::unique_map")]
    pub provenance: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "crate::model::unique_map")]
    pub outputs: BTreeMap<String, String>,
    #[serde(default)]
    pub required_packages: Vec<String>,
    #[serde(default)]
    pub removed_packages: Vec<String>,
    pub files: Vec<SystemFile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemFile {
    pub path: String,
    pub mode: u32,
    pub provenance: String,
    pub priority: i32,
    pub replaces: Option<String>,
    pub content: SystemContent,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SystemContent {
    Bytes(Vec<u8>),
    Template(Argument),
}
#[derive(Clone, Debug, Serialize)]
pub struct FoundationObservation {
    pub receipt: ImageReceipt,
    pub os_release: String,
    pub rpm_inventory: String,
    pub rpm_sha256: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResolvedSystemFile {
    pub path: String,
    pub mode: u32,
    pub provenance: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
    pub disposition: SystemFileDisposition,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemFileDisposition {
    Payload,
    Foundation,
}
#[derive(Clone, Debug, Serialize)]
pub struct SystemPlan {
    pub schema: u32,
    pub identity: String,
    pub definition: SystemDefinition,
    pub foundation: FoundationObservation,
    pub objects: BTreeMap<String, ObjectReceipt>,
    pub files: Vec<ResolvedSystemFile>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SystemArtifact {
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SystemComposition {
    pub schema: u32,
    pub identity: String,
    pub foundation_tag: String,
    pub plan: SystemPlan,
    pub artifacts: BTreeMap<String, SystemArtifact>,
}

pub fn read_system(path: &Path) -> Result<SystemDefinition> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Invalid("system definition exceeds 8 MiB".into()));
    }
    let definition = serde_json::from_slice(&bytes)?;
    validate_definition(&definition)?;
    Ok(definition)
}

impl Store {
    /// Observes the retained foundation using the private, journaled executor.
    pub fn observe_foundation(&self, foundation: &str) -> Result<FoundationObservation> {
        let receipt = self.verify_image(foundation)?;
        let os_release = self.system_command(
            foundation,
            &["/usr/bin/cat".into(), "/usr/lib/os-release".into()],
        )?;
        let mut fields = BTreeMap::new();
        for line in os_release
            .lines()
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| Error::Invalid("invalid foundation os-release".into()))?;
            if fields.insert(key, value.trim_matches('"')).is_some() {
                return Err(Error::Invalid(
                    "duplicate foundation os-release field".into(),
                ));
            }
        }
        if fields.get("ID") != Some(&"fedora") || fields.get("VERSION_ID") != Some(&"44") {
            return Err(Error::Invalid("system foundation must be Fedora 44".into()));
        }
        let inventory = self.system_command(
            foundation,
            &[
                "/usr/bin/rpm".into(),
                "-qa".into(),
                "--qf".into(),
                RPM_FORMAT.into(),
            ],
        )?;
        let mut lines = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for line in inventory.lines() {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len() != 7 {
                return Err(Error::Invalid(
                    "invalid seven-column foundation RPM inventory".into(),
                ));
            }
            if columns.first() == Some(&"gpg-pubkey") {
                continue;
            }
            if columns
                .iter()
                .any(|s| s.is_empty() || s.len() > 4096 || s.bytes().any(|b| b < 32 || b == 127))
                || !columns[1].bytes().all(|b| b.is_ascii_digit())
                || !matches!(columns[4], "aarch64" | "noarch")
                || !plan::hex(columns[5])
                || !plan::hex(columns[6])
                || !identities.insert(columns[..5].join("\t"))
                || !lines.insert(line)
                || lines.len() > 100_000
            {
                return Err(Error::Invalid(
                    "invalid seven-column foundation RPM inventory".into(),
                ));
            }
        }
        if lines.is_empty() {
            return Err(Error::Invalid("empty foundation RPM inventory".into()));
        }
        let rpm_inventory = lines
            .into_iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        Ok(FoundationObservation {
            receipt,
            os_release,
            rpm_sha256: plan::hash(rpm_inventory.as_bytes()),
            rpm_inventory,
        })
    }

    pub fn plan_system(&self, definition: &SystemDefinition) -> Result<SystemPlan> {
        validate_definition(definition)?;
        let mut definition = definition.clone();
        definition.files.sort_by(|a, b| {
            (&a.path, a.priority, &a.provenance).cmp(&(&b.path, b.priority, &b.provenance))
        });
        definition.required_packages.sort();
        definition.removed_packages.sort();
        let mut objects = BTreeMap::new();
        for id in definition.outputs.values() {
            let root = self.verify(id)?;
            if root.runtime_image.as_deref() != Some(&definition.foundation) {
                return Err(Error::Invalid(format!(
                    "output {id} has no matching runtime foundation"
                )));
            }
            for object in self.closure(id)?.objects {
                let receipt = self.verify(&object)?;
                if receipt
                    .runtime_image
                    .as_ref()
                    .is_some_and(|image| image != &definition.foundation)
                {
                    return Err(Error::Invalid(format!(
                        "mixed runtime foundation for {object}"
                    )));
                }
                objects.insert(object, receipt);
            }
        }
        let files = self.resolve_system_files(&definition, &objects)?;
        let foundation = self.observe_foundation(&definition.foundation)?;
        let installed: BTreeSet<_> = foundation
            .rpm_inventory
            .lines()
            .filter_map(|line| line.split('\t').next())
            .collect();
        for name in &definition.required_packages {
            if !installed.contains(name.as_str()) {
                return Err(Error::Invalid(format!(
                    "foundation lacks required RPM {name}"
                )));
            }
        }
        for name in &definition.removed_packages {
            if installed.contains(name.as_str()) {
                return Err(Error::Invalid(format!(
                    "foundation still contains removed RPM {name}"
                )));
            }
        }
        self.check_foundation_paths(&definition.foundation, &files, &objects)?;
        let mut result = SystemPlan {
            schema: 1,
            identity: String::new(),
            definition,
            foundation,
            objects,
            files,
        };
        let mut identity = b"sysroot-system-v1\0".to_vec();
        identity.extend(serde_json::to_vec(&result)?);
        result.identity = plan::hash(&identity);
        Ok(result)
    }

    /// Publishes an offline context; it neither builds nor installs an OS image.
    pub fn compose_system(
        &self,
        definition: &SystemDefinition,
        destination: &Path,
    ) -> Result<SystemComposition> {
        let (parent, destination) = context_destination(destination, self.path())?;
        let plan = self.plan_system(definition)?;
        let stage = parent.join(format!(".sysroot-composition-{}", nonce()?));
        fs::create_dir(&stage)?;
        fs::set_permissions(&stage, Permissions::from_mode(0o700))?;
        let result = (|| {
            let payload = stage.join("payload.tar");
            self.write_system_payload(&plan, &payload)?;
            let archive = stage.join("foundation.tar");
            fs::copy(
                self.image_path(&definition.foundation).join("image.tar"),
                &archive,
            )?;
            File::open(&archive)?.sync_all()?;
            let (sha256, bytes) = hash_file(&archive, MAX_IMAGE)?;
            if sha256 != plan.foundation.receipt.sha256 || bytes != plan.foundation.receipt.bytes {
                return Err(Error::Corrupt(
                    "foundation changed during context export".into(),
                ));
            }
            let foundation_tag = format!("sysroot-foundation:{}", &definition.foundation[7..]);
            let containerfile = format!("FROM {foundation_tag}\nADD payload.tar /\n");
            write_file(&stage.join("Containerfile"), containerfile.as_bytes())?;
            let mut artifacts = BTreeMap::new();
            for (name, limit) in [
                ("payload.tar", MAX_PAYLOAD),
                ("foundation.tar", MAX_IMAGE),
                ("Containerfile", MAX_JSON),
            ] {
                let (sha256, bytes) = hash_file(&stage.join(name), limit)?;
                artifacts.insert(name.into(), SystemArtifact { sha256, bytes });
            }
            let composition = SystemComposition {
                schema: 1,
                identity: plan.identity.clone(),
                foundation_tag,
                plan,
                artifacts,
            };
            write_file(
                &stage.join("composition.json"),
                &serde_json::to_vec_pretty(&composition)?,
            )?;
            File::open(&stage)?.sync_all()?;
            publish_context(&stage, &destination)?;
            File::open(&parent)?.sync_all()?;
            Ok(composition)
        })();
        if stage.exists() {
            tree::remove(&stage)?;
        }
        result
    }

    fn system_command(&self, image: &str, argv: &[String]) -> Result<String> {
        let result = executor::checked(executor::execute(
            self,
            executor::Execution {
                image,
                mounts: &[],
                output: None,
                argv,
                env: &BTreeMap::new(),
                seconds: 120,
            },
        )?)?;
        if result.stderr_truncated {
            return Err(Error::Invalid(
                "foundation observation stderr truncated".into(),
            ));
        }
        String::from_utf8(result.stdout)
            .map_err(|_| Error::Invalid("foundation observation is not UTF-8".into()))
    }

    fn resolve_system_files(
        &self,
        definition: &SystemDefinition,
        objects: &BTreeMap<String, ObjectReceipt>,
    ) -> Result<Vec<ResolvedSystemFile>> {
        let mut selected: BTreeMap<&str, &SystemFile> = BTreeMap::new();
        for file in &definition.files {
            match selected.get(file.path.as_str()) {
                Some(previous)
                    if file.priority > previous.priority
                        && file.replaces.as_deref() == Some(previous.provenance.as_str())
                        && file.provenance != previous.provenance => {}
                None if file.replaces.is_none() => {}
                _ => {
                    return Err(Error::Invalid(format!(
                        "ambiguous or unmatched replacement at {}",
                        file.path
                    )));
                }
            }
            selected.insert(&file.path, file);
        }
        for path in selected.keys() {
            let mut parent = Path::new(path).parent();
            while let Some(p) = parent {
                if p.to_str().is_some_and(|p| selected.contains_key(p)) {
                    return Err(Error::Invalid(format!(
                        "system file prefix collision at {path}"
                    )));
                }
                parent = p.parent();
            }
        }
        let mut files = Vec::new();
        for file in selected.values() {
            let bytes = match &file.content {
                SystemContent::Bytes(bytes) => bytes.clone(),
                SystemContent::Template(argument) => {
                    self.render_system(argument, definition)?.into_bytes()
                }
            };
            let mut refs = BTreeSet::new();
            tree::scan(&bytes, &mut refs)?;
            if refs.iter().any(|id| !objects.contains_key(id)) {
                return Err(Error::Invalid(format!(
                    "undeclared store reference in {}",
                    file.path
                )));
            }
            files.push(ResolvedSystemFile {
                path: file.path.clone(),
                mode: file.mode,
                provenance: file.provenance.clone(),
                sha256: plan::hash(&bytes),
                bytes,
                disposition: if overlay_path(&file.path) {
                    SystemFileDisposition::Payload
                } else {
                    SystemFileDisposition::Foundation
                },
            });
        }
        Ok(files)
    }

    fn render_system(&self, argument: &Argument, definition: &SystemDefinition) -> Result<String> {
        let mut result = String::new();
        for segment in &argument.0 {
            match segment {
                Segment::Literal { value } => result.push_str(value),
                Segment::Input { name, path } => {
                    plan::relative(path, true)?;
                    let id = definition.outputs.get(name).ok_or_else(|| {
                        Error::Invalid(format!("unknown system output alias {name}"))
                    })?;
                    if !path.is_empty() {
                        let entries =
                            tree::inspect(&self.object_path(id).join("data"), None)?.entries;
                        if !entries.iter().any(|entry| &entry.path == path) {
                            return Err(Error::Invalid(format!(
                                "missing typed output path {name}/{path}"
                            )));
                        }
                        let mut parent = Path::new(path).parent();
                        while let Some(p) = parent {
                            if entries.iter().any(|entry| {
                                Path::new(&entry.path) == p
                                    && !matches!(entry.kind, Kind::Directory)
                            }) {
                                return Err(Error::Invalid(
                                    "typed path traverses a symlink".into(),
                                ));
                            }
                            parent = p.parent();
                        }
                    }
                    result.push_str(&format!("{LOGICAL_PREFIX}/{id}"));
                    if !path.is_empty() {
                        result.push('/');
                        result.push_str(path);
                    }
                }
                Segment::Output { .. } => {
                    return Err(Error::Invalid(
                        "system templates have no mutable output".into(),
                    ));
                }
            }
            if result.len() as u64 > MAX_JSON {
                return Err(Error::Invalid("system template exceeds 8 MiB".into()));
            }
        }
        Ok(result)
    }

    fn check_foundation_paths(
        &self,
        foundation: &str,
        files: &[ResolvedSystemFile],
        objects: &BTreeMap<String, ObjectReceipt>,
    ) -> Result<()> {
        // Paths are positional arguments, never shell source. Checking every ancestor
        // prevents a foundation symlink from redirecting a config write into its ABI.
        let script = "set -eu; for file do p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; if [ -L \"$p\" ]; then echo 'foundation path is a symlink' >&2; exit 1; fi; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; if [ -e \"$p\" ] && [ ! -d \"$p\" ]; then exit 1; fi; fi; done; case \"$file\" in /usr/lib/sysroot/store/*) [ ! -e \"$file\" ] || exit 1;; *) if [ -e \"$file\" ]; then if [ ! -f \"$file\" ] || [ -x \"$file\" ]; then echo 'foundation destination is not ordinary configuration' >&2; exit 1; fi; if [ \"$(/usr/bin/stat -c %h -- \"$file\")\" != 1 ]; then echo 'foundation destination hardlinked' >&2; exit 1; fi; fi;; esac; done";
        let paths: Vec<String> = files
            .iter()
            .filter(|file| file.disposition == SystemFileDisposition::Payload)
            .map(|file| file.path.clone())
            .chain(std::iter::once(RECEIPT_PATH.into()))
            .chain(objects.keys().map(|id| format!("{LOGICAL_PREFIX}/{id}")))
            .collect();
        for chunk in paths.chunks(128) {
            let mut argv = vec![
                "/usr/bin/sh".into(),
                "-c".into(),
                script.into(),
                "system-path-check".into(),
            ];
            argv.extend_from_slice(chunk);
            self.system_command(foundation, &argv)?;
        }
        let passthrough = "set -eu; while [ $# -gt 0 ]; do file=$1; mode=$2; hash=$3; shift 3; p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; [ ! -L \"$p\" ] || exit 1; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; [ -d \"$p\" ] || exit 1; fi; done; [ -f \"$file\" ] || { echo 'foundation passthrough file missing' >&2; exit 1; }; [ \"$(/usr/bin/stat -c %a -- \"$file\")\" = \"$mode\" ] || { echo 'foundation passthrough mode mismatch' >&2; exit 1; }; actual=$(/usr/bin/sha256sum -- \"$file\"); [ \"${actual%% *}\" = \"$hash\" ] || { echo 'foundation passthrough content mismatch' >&2; exit 1; }; done";
        let passthrough_files: Vec<_> = files
            .iter()
            .filter(|file| file.disposition == SystemFileDisposition::Foundation)
            .collect();
        for chunk in passthrough_files.chunks(64) {
            let mut argv = vec![
                "/usr/bin/sh".into(),
                "-c".into(),
                passthrough.into(),
                "system-foundation-check".into(),
            ];
            for file in chunk {
                argv.extend([
                    file.path.clone(),
                    format!("{:o}", file.mode),
                    file.sha256.clone(),
                ]);
            }
            self.system_command(foundation, &argv)?;
        }
        Ok(())
    }

    fn write_system_payload(&self, plan: &SystemPlan, output: &Path) -> Result<()> {
        let mut archive = tar::Builder::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(output)?,
        );
        for file in plan
            .files
            .iter()
            .filter(|file| file.disposition == SystemFileDisposition::Payload)
        {
            append_file(&mut archive, &file.path[1..], file.mode, &file.bytes)?;
        }
        #[derive(Serialize)]
        struct PayloadReceipt<'a> {
            schema: u32,
            identity: &'a str,
            foundation: &'a FoundationObservation,
            objects: &'a BTreeMap<String, ObjectReceipt>,
        }
        let receipt = serde_json::to_vec(&PayloadReceipt {
            schema: 1,
            identity: &plan.identity,
            foundation: &plan.foundation,
            objects: &plan.objects,
        })?;
        append_file(&mut archive, &RECEIPT_PATH[1..], 0o644, &receipt)?;
        let mut total = 0u64;
        for (id, expected) in &plan.objects {
            let root = self.object_path(id).join("data");
            let tree = tree::inspect(&root, None)?;
            if tree.digest != expected.tree_sha256 {
                return Err(Error::Corrupt("object changed during composition".into()));
            }
            append_dir(&mut archive, &format!("{}/{id}", &LOGICAL_PREFIX[1..]))?;
            for entry in tree.entries {
                let path = format!("{}/{id}/{}", &LOGICAL_PREFIX[1..], entry.path);
                match entry.kind {
                    Kind::Directory => append_dir(&mut archive, &path)?,
                    Kind::File {
                        executable,
                        bytes,
                        sha256,
                    } => {
                        total = total
                            .checked_add(bytes)
                            .ok_or_else(|| Error::Invalid("payload size overflow".into()))?;
                        if total > MAX_PAYLOAD {
                            return Err(Error::Invalid("payload exceeds 16 GiB".into()));
                        }
                        let mut data = Vec::new();
                        File::open(root.join(&entry.path))?
                            .take(tree::MAX_FILE + 1)
                            .read_to_end(&mut data)?;
                        if data.len() as u64 != bytes || plan::hash(&data) != sha256 {
                            return Err(Error::Corrupt("object changed during export".into()));
                        }
                        append_file(
                            &mut archive,
                            &path,
                            if executable { 0o555 } else { 0o444 },
                            &data,
                        )?;
                    }
                    Kind::Symlink { target } => {
                        let mut header = header(0o777, 0, tar::EntryType::Symlink);
                        archive.append_link(&mut header, path, target)?;
                    }
                }
            }
        }
        archive.finish()?;
        archive.into_inner()?.sync_all()?;
        Ok(())
    }
}

fn validate_definition(definition: &SystemDefinition) -> Result<()> {
    if definition.schema != 1
        || definition.platform != PLATFORM
        || serde_json::to_vec(definition)?.len() as u64 > MAX_JSON
    {
        return Err(Error::Invalid(
            "unsupported or oversized system definition".into(),
        ));
    }
    plan::image_id(&definition.foundation)?;
    if definition.files.len() > 10_000
        || definition.outputs.len() > 1000
        || definition.provenance.len() > 1000
    {
        return Err(Error::Invalid(
            "system definition entry limit exceeded".into(),
        ));
    }
    for (key, value) in &definition.provenance {
        text(key)?;
        text(value)?;
    }
    for (name, id) in &definition.outputs {
        plan::name(name)?;
        plan::object_id(id)?;
    }
    let mut packages = BTreeSet::new();
    for name in definition
        .required_packages
        .iter()
        .chain(&definition.removed_packages)
    {
        if name.is_empty()
            || name.len() > 256
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-+._".contains(&b))
            || !packages.insert(name)
        {
            return Err(Error::Invalid(
                "invalid, duplicate or conflicting package policy".into(),
            ));
        }
    }
    for file in &definition.files {
        validate_system_path(&file.path)?;
        if !matches!(file.mode, 0o644 | 0o755) {
            return Err(Error::Invalid(
                "system file mode must be 0644 or 0755".into(),
            ));
        }
        text(&file.provenance)?;
        if let Some(replaces) = &file.replaces {
            text(replaces)?;
        }
        if let SystemContent::Template(argument) = &file.content {
            if argument.0.len() > 4096 {
                return Err(Error::Invalid("too many template segments".into()));
            }
            for segment in &argument.0 {
                match segment {
                    Segment::Input { name, path } => {
                        plan::name(name)?;
                        plan::relative(path, true)?;
                        if !definition.outputs.contains_key(name) {
                            return Err(Error::Invalid(format!(
                                "unknown system output alias {name}"
                            )));
                        }
                    }
                    Segment::Output { .. } => {
                        return Err(Error::Invalid(
                            "system templates cannot reference a mutable output".into(),
                        ));
                    }
                    Segment::Literal { .. } => {}
                }
            }
        }
    }
    Ok(())
}
fn text(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 4096 || s.bytes().any(|b| b < 32 || b == 127) {
        return Err(Error::Invalid("invalid provenance text".into()));
    }
    Ok(())
}
/// Checks namespace safety. Non-configuration paths additionally require exact
/// foundation equality and never enter the emitted overlay.
pub fn validate_system_path(path: &str) -> Result<()> {
    let relative = path
        .strip_prefix('/')
        .ok_or_else(|| Error::Invalid("system file path must be absolute".into()))?;
    plan::relative(relative, false)?;
    if !(path.starts_with("/etc/") || path.starts_with("/usr/"))
        || path == "/usr/lib/sysroot"
        || path.starts_with("/usr/lib/sysroot/")
        || RECEIPT_PATH.starts_with(&format!("{path}/"))
        || LOGICAL_PREFIX.starts_with(&format!("{path}/"))
        || path == RECEIPT_PATH
        || path.starts_with(&format!("{RECEIPT_PATH}/"))
        || matches!(
            path,
            "/etc/ld.so.preload" | "/etc/ld.so.conf" | "/etc/ld.so.conf.d"
        )
        || path.starts_with("/etc/ld.so.conf.d/")
    {
        return Err(Error::Invalid(format!(
            "unsafe or reserved system configuration path {path}"
        )));
    }
    Ok(())
}
fn overlay_path(path: &str) -> bool {
    [
        "/etc/",
        "/usr/share/",
        "/usr/lib/systemd/",
        "/usr/lib/environment.d/",
        "/usr/lib/tmpfiles.d/",
        "/usr/lib/sysusers.d/",
        "/usr/lib/udev/rules.d/",
        "/usr/lib/sysctl.d/",
        "/usr/lib/bootc/kargs.d/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}
fn header(mode: u32, bytes: u64, kind: tar::EntryType) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_mode(mode);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_size(bytes);
    header.set_entry_type(kind);
    header.set_cksum();
    header
}
fn append_file(archive: &mut tar::Builder<File>, path: &str, mode: u32, data: &[u8]) -> Result<()> {
    archive.append_data(
        &mut header(mode, data.len() as u64, tar::EntryType::Regular),
        path,
        data,
    )?;
    Ok(())
}
fn append_dir(archive: &mut tar::Builder<File>, path: &str) -> Result<()> {
    archive.append_data(
        &mut header(0o555, 0, tar::EntryType::Directory),
        path,
        std::io::empty(),
    )?;
    Ok(())
}
fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn context_destination(destination: &Path, store: &Path) -> Result<(PathBuf, PathBuf)> {
    let parent = fs::canonicalize(
        destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    )?;
    let metadata = fs::metadata(&parent)?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o022 != 0 {
        return Err(Error::Invalid(
            "context parent must be owned and not writable by other users".into(),
        ));
    }
    let name = destination
        .file_name()
        .ok_or_else(|| Error::Invalid("context destination has no name".into()))?;
    let destination = parent.join(name);
    if destination.starts_with(store) {
        return Err(Error::Invalid(
            "context must be outside the engine store".into(),
        ));
    }
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err(Error::Invalid("context destination already exists".into())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok((parent, destination))
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn publish_context(stage: &Path, destination: &Path) -> Result<()> {
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        stage,
        rustix::fs::CWD,
        destination,
        rustix::fs::RenameFlags::NOREPLACE,
    )?;
    Ok(())
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn publish_context(_stage: &Path, _destination: &Path) -> Result<()> {
    Err(Error::Invalid(
        "atomic context publication requires Linux or macOS".into(),
    ))
}
