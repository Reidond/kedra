//! Offline replay of an independently selected, verified composition context.

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use sysroot_engine::{SystemFileDisposition, VerifiedComposition};

use crate::docker::{Build, Docker, Output};
use crate::image::{Overlay, Request, Source};
use crate::{Result, invalid};

const IDENTITY: &str = "dev.kedra.composition.identity";
const FOUNDATION: &str = "dev.kedra.composition.foundation";
const PAYLOAD: &str = "dev.kedra.composition.payload";
pub(crate) const RPM_FORMAT: &str =
    "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    pub identity: String,
    pub image: String,
    pub foundation: String,
    pub engine: String,
    pub outputs: BTreeMap<String, String>,
}

impl Replay {
    pub fn run(
        &self,
        docker: &Docker,
        output: &str,
        program: &str,
        args: &[String],
    ) -> Result<Output> {
        if docker.identity()? != self.engine {
            return Err(invalid("replay engine identity changed"));
        }
        let object = self
            .outputs
            .get(output)
            .ok_or_else(|| invalid("unknown typed composition output"))?;
        if program.is_empty()
            || Path::new(program)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(invalid(
                "program must be a relative path inside the selected output",
            ));
        }
        let mut argv = vec![format!(
            "{}/{object}/{program}",
            sysroot_engine::LOGICAL_PREFIX
        )];
        argv.extend_from_slice(args);
        docker.isolated(&self.image, argv)
    }
}

pub(crate) fn checked(docker: &Docker, image: &str, argv: &[&str]) -> Result<Vec<u8>> {
    let result = docker.isolated(image, argv.iter().map(|s| (*s).to_owned()).collect())?;
    if result.exit != 0 {
        return Err(invalid(format!(
            "foundation probe failed: {}",
            result.stderr_text()
        )));
    }
    Ok(result.stdout)
}

pub(crate) fn sorted_inventory(bytes: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("RPM inventory is not UTF-8"))?;
    let mut lines: Vec<_> = text
        .lines()
        .filter(|line| !line.starts_with("gpg-pubkey\t"))
        .collect();
    lines.sort_unstable();
    Ok(format!("{}\n", lines.join("\n")))
}

pub(crate) fn native_image(
    docker: &Docker,
    image: &str,
) -> Result<testcontainers::bollard::models::ImageInspect> {
    let inspected = docker.inspect_image(image)?;
    if inspected.id.as_deref() != Some(image)
        || inspected.os.as_deref() != Some("linux")
        || inspected.architecture.as_deref() != Some("arm64")
    {
        return Err(invalid(
            "composition image ID/platform differs from native linux/arm64",
        ));
    }
    if let Some(config) = &inspected.config
        && (config.on_build.as_ref().is_some_and(|v| !v.is_empty())
            || config.volumes.as_ref().is_some_and(|v| !v.is_empty()))
    {
        return Err(invalid(
            "composition foundation/image has ONBUILD or declared volumes",
        ));
    }
    Ok(inspected)
}

/// Docker loads mutable names from archive envelopes; only our checked FROM name
/// and the exact content ID may accompany the verified image.
fn check_archive_names(path: &Path, image: &str, foundation_tag: &str) -> Result<()> {
    let mut archive = tar::Archive::new(fs::File::open(path)?);
    let allowed = |name: &str| name == image || name == foundation_tag;
    for entry in archive.entries_with_seek()? {
        let mut entry = entry?;
        let name = entry.path_bytes();
        let docker = name.as_ref() == b"manifest.json";
        if !docker && name.as_ref() != b"index.json" {
            continue;
        }
        if entry.size() > 8 * 1024 * 1024 {
            return Err(invalid("foundation naming metadata exceeds 8 MiB"));
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
        if docker {
            let records = value
                .as_array()
                .ok_or_else(|| invalid("foundation Docker manifest is not an array"))?;
            for record in records {
                match record.get("RepoTags") {
                    None | Some(serde_json::Value::Null) => {}
                    Some(serde_json::Value::Array(tags))
                        if tags.iter().all(|tag| tag.as_str().is_some_and(allowed)) => {}
                    _ => {
                        return Err(invalid(
                            "foundation archive contains an unowned mutable Docker tag",
                        ));
                    }
                }
            }
        } else {
            let records =
                std::iter::once(&value).chain(value["manifests"].as_array().into_iter().flatten());
            for record in records {
                if let Some(annotations) = record.get("annotations") {
                    let annotations = annotations
                        .as_object()
                        .ok_or_else(|| invalid("invalid foundation annotations"))?;
                    for key in [
                        "io.containerd.image.name",
                        "org.opencontainers.image.ref.name",
                    ] {
                        if annotations
                            .get(key)
                            .is_some_and(|name| !name.as_str().is_some_and(allowed))
                        {
                            return Err(invalid(
                                "foundation archive contains an unowned mutable OCI name",
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

const TRANSACTION: &str = "dev.kedra.composition.transaction";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    schema: u32,
    identity: String,
    foundation: String,
    engine: String,
    payload: String,
    outputs: BTreeMap<String, String>,
    nonce: String,
    tag: String,
    image: Option<String>,
}

pub(crate) fn nonce() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn private_metadata(metadata: &fs::Metadata, links: u64) -> Result<()> {
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.nlink() != links
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(invalid(
            "composition state must be an owner-only regular file without aliases",
        ));
    }
    Ok(())
}

pub(crate) fn private_open(path: &Path, create: bool) -> Result<fs::File> {
    let file = OpenOptions::new()
        .read(true)
        .write(create)
        .create(create)
        .truncate(false)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)?;
    let metadata = file.metadata()?;
    private_metadata(&metadata, 1)?;
    let named = fs::symlink_metadata(path)?;
    if named.dev() != metadata.dev() || named.ino() != metadata.ino() {
        return Err(invalid("composition state changed while opening"));
    }
    Ok(file)
}

pub(crate) fn read_state<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let file = match private_open(path, false) {
        Ok(file) => file,
        Err(crate::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(invalid("composition state exceeds 8 MiB"));
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| invalid(format!("composition state: {error}")))
}

pub(crate) fn sync_parent(path: &Path) -> Result<()> {
    fs::File::open(
        path.parent()
            .ok_or_else(|| invalid("composition state has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

/// A killed atomic publication may leave a partial next file or a second link to
/// the completed binding. Only that checked transaction-local path is retired.
pub(crate) fn retire_next(path: &Path) -> Result<()> {
    let next = path.with_extension("next");
    let metadata = match fs::symlink_metadata(&next) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.nlink() == 2 {
        private_metadata(&metadata, 2)?;
        let bound = fs::symlink_metadata(path)?;
        if !bound.is_file() || bound.dev() != metadata.dev() || bound.ino() != metadata.ino() {
            return Err(invalid(
                "composition pending publication aliases another file",
            ));
        }
    } else {
        private_metadata(&metadata, 1)?;
    }
    fs::remove_file(next)?;
    sync_parent(path)
}

pub(crate) fn write_state(path: &Path, value: &impl Serialize, replace: bool) -> Result<()> {
    retire_next(path)?;
    if replace {
        // Never replace unknown or foreign state; callers validate its schema first.
        match private_open(path, false) {
            Ok(_) => {}
            Err(crate::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let next = path.with_extension("next");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(&next)?;
    file.write_all(&serde_json::to_vec(value).map_err(|error| invalid(error.to_string()))?)?;
    file.sync_all()?;
    if replace {
        fs::rename(&next, path)?;
    } else {
        // No-replace publication. A crash before unlink is recovered by retire_next.
        fs::hard_link(&next, path)?;
        fs::remove_file(&next)?;
    }
    sync_parent(path)
}

fn validate_image_labels(
    docker: &Docker,
    image: &str,
    labels: &HashMap<String, String>,
) -> Result<()> {
    let actual = native_image(docker, image)?
        .config
        .and_then(|config| config.labels)
        .unwrap_or_default();
    if !labels
        .iter()
        .all(|(key, value)| actual.get(key) == Some(value))
    {
        return Err(invalid(
            "composition cache image ID/provenance binding mismatch",
        ));
    }
    Ok(())
}

fn recover_or_build(
    docker: &Docker,
    verified: &VerifiedComposition,
    cache: &Path,
    key: &str,
    engine: &str,
) -> Result<Replay> {
    let composition = verified.composition();
    let identity = &composition.identity;
    let foundation = &composition.plan.foundation.receipt.image;
    let payload = &composition.artifacts["payload.tar"].sha256;
    let outputs = &composition.plan.definition.outputs;
    let tag = format!("kedra-composition:{identity}");
    let binding_path = cache.join(format!("{key}-binding.json"));
    let journal_path = cache.join(format!("{key}-transaction.json"));
    retire_next(&binding_path)?;
    retire_next(&journal_path)?;
    let bound: Option<Replay> = read_state(&binding_path)?;
    let mut pending: Option<Pending> = read_state(&journal_path)?;
    let labels = HashMap::from([
        (
            crate::docker::OWNER_LABEL.into(),
            crate::docker::OWNER.into(),
        ),
        (crate::docker::KIND_LABEL.into(), "composition".into()),
        (IDENTITY.into(), identity.clone()),
        (FOUNDATION.into(), foundation.clone()),
        (PAYLOAD.into(), payload.clone()),
    ]);
    if let Some(journal) = &pending {
        if journal.schema != 1
            || journal.identity != *identity
            || journal.foundation != *foundation
            || journal.engine != engine
            || journal.payload != *payload
            || journal.outputs != *outputs
            || journal.nonce.len() != 32
            || !journal
                .nonce
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || journal.tag != format!("kedra-composition-pending:{}", journal.nonce)
        {
            return Err(invalid(
                "composition transaction identity/provenance mismatch",
            ));
        }
        if let Some(actual) = &journal.image {
            validate_image_labels(docker, actual, &labels)?;
            if docker
                .image(&journal.tag)?
                .is_some_and(|(id, _)| id != *actual)
            {
                return Err(invalid("composition transaction tag changed"));
            }
        }
    }
    if let Some(receipt) = bound {
        if receipt.identity != *identity
            || receipt.foundation != *foundation
            || receipt.engine != engine
            || receipt.outputs != *outputs
        {
            return Err(invalid(
                "composition cache image ID/provenance binding mismatch",
            ));
        }
        validate_image_labels(docker, &receipt.image, &labels)?;
        if let Some(journal) = &pending
            && journal.image.as_deref() != Some(receipt.image.as_str())
        {
            return Err(invalid(
                "composition binding differs from interrupted transaction",
            ));
        }
        docker.publish_composition(&receipt.image, &tag)?;
        if let Some(journal) = pending {
            docker.remove_composition_pending(&journal.tag, &receipt.image, &journal.nonce)?;
            fs::remove_file(&journal_path)?;
            sync_parent(&journal_path)?;
        }
        return Ok(receipt);
    }
    // A final tag without a durable binding is never adopted, even if labels match.
    if docker.image(&tag)?.is_some() {
        return Err(invalid(
            "composition cache tag has no independently retained binding",
        ));
    }
    if pending.is_none() {
        let nonce = nonce()?;
        pending = Some(Pending {
            schema: 1,
            identity: identity.clone(),
            foundation: foundation.clone(),
            engine: engine.into(),
            payload: payload.clone(),
            outputs: outputs.clone(),
            tag: format!("kedra-composition-pending:{nonce}"),
            nonce,
            image: None,
        });
        write_state(&journal_path, &pending, false)?;
    }
    let mut pending = pending.ok_or_else(|| invalid("composition transaction missing"))?;
    let mut transaction_labels = labels;
    transaction_labels.insert(TRANSACTION.into(), pending.nonce.clone());
    if pending.image.is_none() {
        // Before a result was durably recorded, the nonce tag authorizes cleanup
        // only. Rebuild rather than treating unbound labels as output provenance.
        if let Some((actual, _)) = docker.image(&pending.tag)? {
            validate_image_labels(docker, &actual, &transaction_labels)?;
            docker.remove_composition_pending(&pending.tag, &actual, &pending.nonce)?;
        }
        docker.build_static(
            &Build {
                tag: &pending.tag,
                what: "verified static composition replay (network disabled)",
                containerfile: &verified.containerfile_path(),
                files: &[("payload.tar".into(), verified.payload_path())],
                args: &[],
                platform: "linux/arm64",
            },
            transaction_labels.clone(),
        )?;
        let actual = docker
            .image(&pending.tag)?
            .ok_or_else(|| invalid("static composition build produced no image"))?
            .0;
        validate_image_labels(docker, &actual, &transaction_labels)?;
        pending.image = Some(actual);
        write_state(&journal_path, &pending, true)?;
    }
    let actual = pending
        .image
        .as_ref()
        .ok_or_else(|| invalid("composition transaction has no built image"))?;
    validate_image_labels(docker, actual, &transaction_labels)?;
    let receipt = Replay {
        identity: identity.clone(),
        image: actual.clone(),
        foundation: foundation.clone(),
        engine: engine.into(),
        outputs: outputs.clone(),
    };
    write_state(&binding_path, &receipt, false)?;
    docker.publish_composition(actual, &tag)?;
    docker.remove_composition_pending(&pending.tag, actual, &pending.nonce)?;
    fs::remove_file(&journal_path)?;
    sync_parent(&journal_path)?;
    Ok(receipt)
}

/// Every invocation verifies a private snapshot and loads its complete foundation.
/// Only the resulting static image can subsequently enter the separate lab-tools layer.
pub fn prepare(docker: &Docker, request: &Request) -> Result<Replay> {
    let Source::Composition(input) = &request.source else {
        return Err(invalid("replay requires --image composition:<directory>"));
    };
    if request.overlay != Overlay::None || request.binaries.is_some() {
        return Err(invalid(
            "composition replay refuses working-tree overlays and binary overrides",
        ));
    }
    let identity = request.composition_identity.as_deref().ok_or_else(|| invalid("composition replay requires an independently selected --composition-identity or KEDRA_LAB_COMPOSITION_IDENTITY"))?;
    if request.target.id != "qemu-arm64"
        || request.target.oci_architecture != "arm64"
        || docker.architecture()? != "aarch64"
    {
        return Err(invalid(
            "composition replay requires the native qemu-arm64 target",
        ));
    }
    let cache = crate::artifact_root().join("composition");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&cache)?;
    let metadata = fs::symlink_metadata(&cache)?;
    if !metadata.is_dir() || metadata.mode() & 0o077 != 0 {
        return Err(invalid("composition cache must be a private directory"));
    }
    if identity.len() != 64
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid(
            "composition identity must be 64 lowercase hexadecimal characters",
        ));
    }
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(invalid("composition cache belongs to another owner"));
    }
    let engine = docker.identity()?;
    let key = format!(
        "{}-{identity}",
        crate::builder::content_key(&[engine.as_bytes()])
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
    let verified = VerifiedComposition::open(input, identity, &cache)
        .map_err(|error| invalid(error.to_string()))?;
    let composition = verified.composition();
    let foundation = &composition.plan.foundation;
    let image = &foundation.receipt.image;
    // Refuse existing conflicting FROM tags before loading, including archive-provided tags.
    if docker
        .image(&composition.foundation_tag)?
        .is_some_and(|(id, _)| id != *image)
    {
        return Err(invalid("foundation tag points at another image"));
    }
    check_archive_names(
        &verified.foundation_path(),
        image,
        &composition.foundation_tag,
    )?;
    docker.load_archive(&verified.foundation_path(), foundation.receipt.bytes)?;
    native_image(docker, image)?;
    if checked(docker, image, &["/usr/bin/cat", "/usr/lib/os-release"])?
        != foundation.os_release.as_bytes()
        || sorted_inventory(&checked(
            docker,
            image,
            &["/usr/bin/rpm", "-qa", "--qf", RPM_FORMAT],
        )?)? != foundation.rpm_inventory
    {
        return Err(invalid(
            "loaded foundation differs from the composition observation",
        ));
    }
    let script = "set -eu; for file do p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; if [ -L \"$p\" ]; then echo 'foundation path is a symlink' >&2; exit 1; fi; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; if [ -e \"$p\" ] && [ ! -d \"$p\" ]; then exit 1; fi; fi; done; case \"$file\" in /usr/lib/sysroot/store/*) [ ! -e \"$file\" ] || exit 1;; *) if [ -e \"$file\" ]; then if [ ! -f \"$file\" ] || [ -x \"$file\" ]; then echo 'foundation destination is not ordinary configuration' >&2; exit 1; fi; if [ \"$(/usr/bin/stat -c %h -- \"$file\")\" != 1 ]; then echo 'foundation destination hardlinked' >&2; exit 1; fi; fi;; esac; done";
    let paths: Vec<String> = composition
        .plan
        .files
        .iter()
        .filter(|file| file.disposition == SystemFileDisposition::Payload)
        .map(|file| file.path.clone())
        .chain(std::iter::once(
            "/usr/share/sysroot/composition.json".into(),
        ))
        .chain(
            composition
                .plan
                .objects
                .keys()
                .map(|id| format!("{}/{id}", sysroot_engine::LOGICAL_PREFIX)),
        )
        .collect();
    for chunk in paths.chunks(128) {
        let mut argv = vec!["/usr/bin/sh", "-c", script, "composition-path-check"];
        argv.extend(chunk.iter().map(String::as_str));
        checked(docker, image, &argv)?;
    }
    let passthrough = "set -eu; while [ $# -gt 0 ]; do file=$1; mode=$2; hash=$3; shift 3; p=; rest=${file#/}; while [ -n \"$rest\" ]; do part=${rest%%/*}; p=$p/$part; [ ! -L \"$p\" ] || exit 1; if [ \"$rest\" = \"$part\" ]; then rest=; else rest=${rest#*/}; [ -d \"$p\" ] || exit 1; fi; done; [ -f \"$file\" ] || { echo 'foundation passthrough file missing' >&2; exit 1; }; [ \"$(/usr/bin/stat -c %a -- \"$file\")\" = \"$mode\" ] || { echo 'foundation passthrough mode mismatch' >&2; exit 1; }; actual=$(/usr/bin/sha256sum -- \"$file\"); [ \"${actual%% *}\" = \"$hash\" ] || { echo 'foundation passthrough content mismatch' >&2; exit 1; }; done";
    for file in &composition.plan.files {
        if file.disposition == SystemFileDisposition::Foundation {
            checked(
                docker,
                image,
                &[
                    "/usr/bin/sh",
                    "-c",
                    passthrough,
                    "composition-foundation-check",
                    &file.path,
                    &format!("{:o}", file.mode),
                    &file.sha256,
                ],
            )?;
        }
    }
    if docker.pin_foundation(image)? != composition.foundation_tag {
        return Err(invalid(
            "foundation pin differs from verified Containerfile",
        ));
    }
    let replay = recover_or_build(docker, &verified, &cache, &key, &engine)?;
    verified
        .finish()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(replay)
}
