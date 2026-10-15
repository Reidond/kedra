//! Fixed-output Fedora RPM objects.
//!
//! An RPM file becomes an ordinary single-file source object whose identity is
//! known before any byte is read: file name, size and SHA-256 select the object.
//! Admission refuses any other content, so a mirror, Koji or a local copy can
//! supply the bytes interchangeably. Resolution, the only step that decides which
//! RPMs exist, lives in `rpm_resolver` and runs against a disposable container.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Durable origin of Fedora's signed RPM copies.
pub const KOJI_PACKAGES: &str = "https://kojipkgs.fedoraproject.org/packages/";
/// Largest RPM file one object admits; the store's per-file tree bound.
pub const MAX_RPM_BYTES: u64 = 256 * 1024 * 1024;
/// Upper bound on RPM files and installed rows a resolution may record.
pub const MAX_RPMS: usize = 8192;

/// One RPM file pinned by content. Rows use the seven-column inventory format:
/// name, epoch, version, release, architecture, header and payload SHA-256.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpmFile {
    pub name: String,
    pub epoch: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub size: u64,
    pub sha256: String,
    pub header_sha256: String,
    pub payload_sha256: String,
    pub source_rpm: String,
    pub repository: String,
    /// Koji's signed copy: byte-identical to what mirrors serve.
    pub durable_url: String,
    /// The mirror location the resolving run used; provenance, not identity.
    pub mirror_url: String,
    /// Fixed-output `src-` object predicted from file name, size and SHA-256.
    pub object: String,
}

/// One package set applied to its parent: the foundation over the base image, or
/// a builder role over the foundation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpmRole {
    /// Authored install requests.
    pub requests: BTreeSet<String>,
    /// Authored removals, applied after installation.
    pub remove: BTreeSet<String>,
    /// RPM file names installed on top of the parent.
    pub install: BTreeSet<String>,
    /// Complete sorted seven-column inventory after the role, without gpg-pubkey.
    pub packages: Vec<Vec<String>>,
}

/// Package sets to resolve once against one exact base image.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpmRequest {
    /// Digest-pinned image reference or local `sha256:` image identity.
    pub base: String,
    /// RPM architecture of the native resolver: `aarch64` or `x86_64`.
    pub architecture: String,
    pub repositories: Vec<String>,
    pub install: BTreeSet<String>,
    pub remove: BTreeSet<String>,
    pub builders: BTreeMap<String, BTreeSet<String>>,
}

/// Result of one resolution. Every RPM is already an admitted store object.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RpmResolution {
    pub rpms: BTreeMap<String, RpmFile>,
    pub foundation: RpmRole,
    pub builders: BTreeMap<String, RpmRole>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn token(value: &str, extra: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
}

/// Package and request names accepted by the resolver and the lock.
pub fn package_name(value: &str) -> bool {
    token(value, b"+._-") && value.as_bytes()[0].is_ascii_alphanumeric()
}

/// A digest-pinned image reference, or a local `sha256:` image identity.
pub fn pinned_image(base: &str) -> bool {
    let digest = |value: &str| value.strip_prefix("sha256:").is_some_and(crate::plan::hex);
    match base.split_once('@') {
        Some((name, value)) => {
            !name.is_empty()
                && name.len() <= 255
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"./_-:".contains(&b))
                && digest(value)
        }
        None => digest(base),
    }
}

/// RPM architectures a native target installs.
pub fn rpm_architectures(architecture: &str) -> Result<BTreeSet<&'static str>> {
    match architecture {
        "aarch64" => Ok(BTreeSet::from(["aarch64", "noarch"])),
        "x86_64" => Ok(BTreeSet::from(["x86_64", "noarch", "i686"])),
        _ => Err(invalid("RPM resolution supports only aarch64 and x86_64")),
    }
}

/// Predicted object identity of a single-file RPM object.
pub fn rpm_object(file: &str, size: u64, sha256: &str) -> Result<String> {
    if !file.ends_with(".rpm") || file.contains('/') || !token(file, b"+._-~^") {
        return Err(invalid(format!("unsafe RPM file name {file:?}")));
    }
    if size == 0 || size > MAX_RPM_BYTES || !crate::plan::hex(sha256) {
        return Err(invalid(format!("invalid RPM size or SHA-256 for {file}")));
    }
    let entries = [crate::resources::Entry {
        path: file.into(),
        kind: crate::resources::Kind::File {
            executable: false,
            bytes: size,
            sha256: sha256.into(),
        },
    }];
    Ok(format!("src-{}", crate::resources::tree_digest(&entries)?))
}

/// Split `<name>-<version>-<release>.src.rpm` into its build identity.
pub fn source_build(source_rpm: &str) -> Result<(&str, &str, &str)> {
    let stem = source_rpm
        .strip_suffix(".src.rpm")
        .ok_or_else(|| invalid(format!("unexpected source RPM {source_rpm:?}")))?;
    let mut parts = stem.rsplitn(3, '-');
    let (Some(release), Some(version), Some(name)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid(format!("unexpected source RPM {source_rpm:?}")));
    };
    if !package_name(name) || !token(version, b"._+~^") || !token(release, b"._+~^") {
        return Err(invalid(format!("unexpected source RPM {source_rpm:?}")));
    }
    Ok((name, version, release))
}

/// Koji's signed-copy URL for a build, signing key and file.
pub fn koji_url(source_rpm: &str, key: &str, arch: &str, file: &str) -> Result<String> {
    let (name, version, release) = source_build(source_rpm)?;
    if key.len() != 8
        || !key
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(invalid(
            "Koji signing key directory must be 8 lowercase hex digits",
        ));
    }
    Ok(format!(
        "{KOJI_PACKAGES}{name}/{version}/{release}/data/signed/{key}/{arch}/{file}"
    ))
}

impl RpmFile {
    /// The file name Fedora repositories and Koji use.
    pub fn file_name(&self) -> String {
        format!(
            "{}-{}-{}.{}.rpm",
            self.name, self.version, self.release, self.arch
        )
    }

    /// Seven-column installed-inventory row of this package.
    pub fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.epoch.clone(),
            self.version.clone(),
            self.release.clone(),
            self.arch.clone(),
            self.header_sha256.clone(),
            self.payload_sha256.clone(),
        ]
    }

    /// Structural checks; the content check happens on admission.
    pub fn validate(&self, file: &str, architecture: &str) -> Result<()> {
        let fields = package_name(&self.name)
            && !self.epoch.is_empty()
            && self.epoch.len() <= 10
            && self.epoch.bytes().all(|b| b.is_ascii_digit())
            && token(&self.version, b"._+~^")
            && token(&self.release, b"._+~^")
            && rpm_architectures(architecture)?.contains(self.arch.as_str())
            && crate::plan::hex(&self.header_sha256)
            && crate::plan::hex(&self.payload_sha256)
            && token(&self.repository, b"._-");
        if !fields || self.file_name() != file {
            return Err(invalid(format!("invalid locked RPM entry {file}")));
        }
        if rpm_object(file, self.size, &self.sha256)? != self.object {
            return Err(invalid(format!(
                "{file}: object differs from its fixed-output identity"
            )));
        }
        self.validate_urls(file)
    }

    fn validate_urls(&self, file: &str) -> Result<()> {
        source_build(&self.source_rpm)?;
        let durable = self
            .durable_url
            .strip_prefix(KOJI_PACKAGES)
            .is_some_and(|rest| rest.ends_with(&format!("/{}/{file}", self.arch)));
        let mirror = (self.mirror_url.starts_with("https://")
            || self.mirror_url.starts_with("http://"))
            && self.mirror_url.ends_with(&format!("/{file}"));
        let printable = |url: &str| url.len() <= 2048 && url.bytes().all(|b| b > 32 && b < 127);
        if !durable || !mirror || !printable(&self.durable_url) || !printable(&self.mirror_url) {
            return Err(invalid(format!("{file}: invalid durable or mirror URL")));
        }
        Ok(())
    }
}

/// Check one sorted, unique seven-column inventory.
pub fn validate_rows(rows: &[Vec<String>], architecture: &str) -> Result<()> {
    let allowed = rpm_architectures(architecture)?;
    if rows.is_empty() || rows.len() > MAX_RPMS || !rows.is_sorted() {
        return Err(invalid(
            "RPM inventory must be non-empty, bounded and sorted",
        ));
    }
    let mut identities = BTreeSet::new();
    for row in rows {
        let valid = row.len() == 7
            && package_name(&row[0])
            && row[0] != "gpg-pubkey"
            && !row[1].is_empty()
            && row[1].bytes().all(|b| b.is_ascii_digit())
            && token(&row[2], b"._+~^")
            && token(&row[3], b"._+~^")
            && allowed.contains(row[4].as_str())
            && crate::plan::hex(&row[5])
            && crate::plan::hex(&row[6]);
        if !valid || !identities.insert(&row[..5]) {
            return Err(invalid(format!("invalid or duplicate RPM row {row:?}")));
        }
    }
    Ok(())
}

#[cfg(unix)]
mod store_rpm {
    use super::*;
    use crate::{ObjectReceipt, Store, store, tree};
    use std::fs::{self, File, OpenOptions, Permissions};
    use std::io::{Read, Write};
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::Path;
    use std::process::{Command, Stdio};

    impl Store {
        /// Admit one RPM file as its fixed-output object. The file is moved when
        /// it already lives on the store's filesystem and copied otherwise; any
        /// other size or SHA-256 than `rpm` pins is refused before publication.
        pub fn import_rpm(&self, source: &Path, rpm: &RpmFile) -> Result<ObjectReceipt> {
            let file = rpm.file_name();
            rpm_object(&file, rpm.size, &rpm.sha256)?;
            let stage = self.staging()?;
            let result = (|| {
                let data = stage.join("data");
                fs::create_dir(&data)?;
                let path = data.join(&file);
                if fs::rename(source, &path).is_err() {
                    copy_bounded(source, &path, rpm.size)?;
                }
                self.admit_rpm(&stage, &path, rpm)
            })();
            tree::remove(&stage)?;
            result
        }

        /// Download a missing RPM object from its mirror, then from Koji. Both
        /// sources are untrusted until the fixed-output check passes.
        pub fn fetch_rpm(&self, rpm: &RpmFile) -> Result<(ObjectReceipt, String)> {
            if let Some(receipt) = self.present_rpm(rpm)? {
                return Ok((receipt, "store".into()));
            }
            let mut failures = Vec::new();
            for url in [&rpm.mirror_url, &rpm.durable_url] {
                match self.download_rpm(url, rpm) {
                    Ok(receipt) => return Ok((receipt, url.clone())),
                    Err(error) => failures.push(format!("{url}: {error}")),
                }
            }
            Err(invalid(format!(
                "no source supplied {}: {}",
                rpm.file_name(),
                failures.join("; ")
            )))
        }

        /// The verified object when the store already holds it.
        pub fn present_rpm(&self, rpm: &RpmFile) -> Result<Option<ObjectReceipt>> {
            if !self.object_path(&rpm.object).exists() {
                return Ok(None);
            }
            let receipt = self.verify(&rpm.object)?;
            if receipt.object != rpm.object || !receipt.references.is_empty() {
                return Err(Error::Corrupt(format!("{} object differs", rpm.object)));
            }
            Ok(Some(receipt))
        }

        /// Verify that the store holds every selected RPM object intact.
        pub fn verify_rpms(&self, rpms: &[&RpmFile]) -> Result<()> {
            for rpm in rpms {
                self.present_rpm(rpm)?.ok_or_else(|| {
                    invalid(format!("store lacks locked RPM {}", rpm.file_name()))
                })?;
            }
            Ok(())
        }

        /// Copy exactly the selected RPM objects into a new directory, verifying
        /// every object tree before the directory exists and every copied byte.
        pub fn materialize_rpms(&self, rpms: &[&RpmFile], destination: &Path) -> Result<()> {
            store::safe_bind(destination)?;
            self.verify_rpms(rpms)?;
            fs::create_dir(destination)?;
            fs::set_permissions(destination, Permissions::from_mode(0o755))?;
            for rpm in rpms {
                let file = rpm.file_name();
                let target = destination.join(&file);
                copy_bounded(
                    &self.object_path(&rpm.object).join("data").join(&file),
                    &target,
                    rpm.size,
                )?;
                let (sha256, size) = store::hash_file(&target, MAX_RPM_BYTES)?;
                if sha256 != rpm.sha256 || size != rpm.size {
                    return Err(Error::Corrupt(format!("{file} changed while copied")));
                }
                fs::set_permissions(&target, Permissions::from_mode(0o644))?;
            }
            File::open(destination)?.sync_all()?;
            Ok(())
        }

        fn download_rpm(&self, url: &str, rpm: &RpmFile) -> Result<ObjectReceipt> {
            let file = rpm.file_name();
            let stage = self.staging()?;
            let result = (|| {
                let data = stage.join("data");
                fs::create_dir(&data)?;
                let path = data.join(&file);
                curl(url, &path, rpm.size)?;
                self.admit_rpm(&stage, &path, rpm)
            })();
            tree::remove(&stage)?;
            result
        }

        fn admit_rpm(&self, stage: &Path, path: &Path, rpm: &RpmFile) -> Result<ObjectReceipt> {
            fs::set_permissions(path, Permissions::from_mode(0o644))?;
            let (sha256, size) = store::hash_file(path, MAX_RPM_BYTES)?;
            if sha256 != rpm.sha256 || size != rpm.size {
                return Err(invalid(format!(
                    "{}: content SHA-256 {sha256} ({size} bytes) differs from the lock",
                    rpm.file_name()
                )));
            }
            let tree = tree::inspect(&stage.join("data"), None)?;
            let receipt = ObjectReceipt {
                schema: 1,
                object: format!("src-{}", tree.digest),
                tree_sha256: tree.digest,
                references: tree.references.into_iter().collect(),
                runtime_image: None,
                derivation: None,
            };
            if receipt.object != rpm.object || !receipt.references.is_empty() {
                return Err(Error::Corrupt(format!(
                    "{}: admitted tree differs from the fixed-output object",
                    rpm.file_name()
                )));
            }
            self.admit_object(stage, &receipt)?;
            self.pin(&receipt.object)?;
            Ok(receipt)
        }
    }

    fn copy_bounded(source: &Path, target: &Path, size: u64) -> Result<()> {
        let mut input = OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(source)?;
        if !input.metadata()?.is_file() {
            return Err(invalid("RPM source must be a regular file"));
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(target)?;
        let copied = std::io::copy(&mut Read::by_ref(&mut input).take(size + 1), &mut output)?;
        if copied != size {
            return Err(invalid("RPM size differs from the lock"));
        }
        output.flush()?;
        output.sync_all()?;
        Ok(())
    }

    fn curl(url: &str, path: &Path, size: u64) -> Result<()> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(invalid("RPM downloads use HTTP(S) only"));
        }
        let mut command = Command::new("curl");
        command
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https,http",
                "--proto-redir",
                "=https,http",
                "--max-redirs",
                "10",
                "--connect-timeout",
                "30",
                "--max-time",
                "1800",
                "--max-filesize",
                &size.to_string(),
                "--output",
            ])
            .arg(path)
            .arg("--")
            .arg(url)
            .stdin(Stdio::null());
        let result = crate::executor::command(command, 1860)?;
        if result.code != 0 {
            return Err(invalid(format!(
                "curl exited {}: {}",
                result.code,
                String::from_utf8_lossy(&result.stderr).trim()
            )));
        }
        Ok(())
    }
}
