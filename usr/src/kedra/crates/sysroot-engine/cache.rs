//! Binary cache: realized closures in an OCI registry, admitted only when every
//! object equals a record the consumer already trusts.
//!
//! The registry is never trusted. A consumer reads object receipts from the
//! installed `/usr/share/sysroot/composition.json` of a signature-verified image
//! (in a release, the previous stable image; on a machine, its booted image),
//! derives the closure's cache tag from those receipts, and admits a pulled
//! closure only if every staged object, whose tree hash the store recomputes,
//! equals its record. Anything else is rebuilt. Transport is `skopeo` with a
//! private policy and home, so no user or system container configuration applies.
use crate::{
    Error, MAX_JSON, OCI_ARCHITECTURE, ObjectReceipt, Result, Store, executor,
    plan::{self, object_id},
    store::{hash_file, write_json_new},
    system::MAX_COMPOSITION_JSON,
    tree,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Tags are `closure-` plus the SHA-256 of the closure's canonical receipts.
pub const CACHE_TAG_PREFIX: &str = "closure-";
const MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
const CONFIG: &str = "application/vnd.oci.image.config.v1+json";
const LAYER: &str = "application/vnd.oci.image.layer.v1.tar";
const MAX_BUNDLE: u64 = 16 * 1024 * 1024 * 1024;
const TRANSFER_SECONDS: u64 = 1800;
const INSPECT_SECONDS: u64 = 120;

/// Object receipts read from a signature-verified image's installed composition
/// receipt. Only their content hashes and recipes are used.
pub struct TrustedRecords {
    objects: BTreeMap<String, ObjectReceipt>,
}

#[derive(Deserialize)]
struct InstalledComposition {
    schema: u32,
    #[serde(deserialize_with = "crate::model::unique_map")]
    objects: BTreeMap<String, ObjectReceipt>,
}

impl TrustedRecords {
    /// Read `/usr/share/sysroot/composition.json` (schema 2) or a copy of it.
    pub fn read(path: &Path) -> Result<Self> {
        let bytes = bounded_file(path, MAX_COMPOSITION_JSON)?;
        let composition: InstalledComposition = serde_json::from_slice(&bytes).map_err(|e| {
            invalid(format!(
                "trusted records are not a composition receipt: {e}"
            ))
        })?;
        if composition.schema != 2 {
            return Err(invalid(
                "trusted records need a schema 2 composition receipt",
            ));
        }
        for (id, receipt) in &composition.objects {
            object_id(id)?;
            if receipt.object != *id || receipt.schema != 1 || !plan::hex(&receipt.tree_sha256) {
                return Err(invalid(format!("trusted record {id} is malformed")));
            }
        }
        Ok(Self {
            objects: composition.objects,
        })
    }

    pub fn contains(&self, id: &str) -> bool {
        self.objects.contains_key(id)
    }

    /// The trusted receipts of `root`'s complete runtime closure.
    fn closure(&self, root: &str) -> Result<BTreeMap<String, ObjectReceipt>> {
        let mut closure = BTreeMap::new();
        let mut pending = vec![root.to_owned()];
        while let Some(id) = pending.pop() {
            if closure.contains_key(&id) {
                continue;
            }
            let receipt = self
                .objects
                .get(&id)
                .ok_or_else(|| invalid(format!("no trusted record for {id}")))?;
            pending.extend(receipt.references.iter().cloned());
            closure.insert(id, receipt.clone());
        }
        Ok(closure)
    }
}

/// Cache tag of a closure: its receipts decide it, so different realizations of
/// one derivation never share a tag.
pub fn closure_tag(objects: &BTreeMap<String, ObjectReceipt>) -> Result<String> {
    let mut bytes = b"sysroot-engine-closure-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(objects)?);
    Ok(format!("{CACHE_TAG_PREFIX}{}", plan::hash(&bytes)))
}

/// An OCI repository holding closures. Plain HTTP is for local test registries:
/// it changes no trust (records decide admission) but refuses credentials.
pub struct BinaryCache {
    repository: String,
    authfile: Option<PathBuf>,
    plain_http: bool,
    source: Option<String>,
}

impl BinaryCache {
    pub fn new(repository: &str, authfile: Option<PathBuf>, plain_http: bool) -> Result<Self> {
        let valid = repository.len() <= 255
            && repository.split_once('/').is_some_and(|(host, path)| {
                !host.is_empty()
                    && host
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".-:".contains(&b))
                    && !path.is_empty()
                    && path.split('/').all(|part| {
                        !part.is_empty()
                            && part.bytes().all(|b| {
                                b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)
                            })
                    })
            });
        if !valid {
            return Err(invalid(
                "cache repository must be HOST[:PORT]/lowercase/path without tag or digest",
            ));
        }
        if plain_http && authfile.is_some() {
            return Err(invalid("credentials are never sent to a plain HTTP cache"));
        }
        Ok(Self {
            repository: repository.into(),
            authfile,
            plain_http,
            source: None,
        })
    }

    /// Source repository URL annotated on pushed closures (GHCR links packages by it).
    pub fn with_source(mut self, source: Option<String>) -> Result<Self> {
        if source.as_deref().is_some_and(|url| {
            !url.starts_with("https://") || url.len() > 512 || url.contains(char::is_whitespace)
        }) {
            return Err(invalid("source must be an https URL"));
        }
        self.source = source;
        Ok(self)
    }

    fn reference(&self, tag: &str) -> String {
        format!("docker://{}:{tag}", self.repository)
    }

    /// skopeo with this store's private home and a policy that accepts exactly
    /// the cache repository and local layouts: signatures play no part here.
    fn skopeo(&self, work: &Path) -> Result<Command> {
        let home = work.join("home");
        fs::create_dir_all(&home)?;
        let policy = work.join("policy.json");
        if !policy.exists() {
            let accept = serde_json::json!([{"type": "insecureAcceptAnything"}]);
            write_json_new(
                &policy,
                &serde_json::json!({
                    "default": [{"type": "reject"}],
                    "transports": {
                        "docker": {self.repository.clone(): accept},
                        "dir": {"": accept},
                    },
                }),
            )?;
        }
        let mut command = Command::new("skopeo");
        command
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("HOME", &home)
            .env("TMPDIR", work)
            .arg("--policy")
            .arg(&policy);
        Ok(command)
    }

    fn transport_flags(&self, command: &mut Command, side: &str) {
        if let Some(authfile) = &self.authfile {
            command.arg(format!("--{side}authfile")).arg(authfile);
        }
        if self.plain_http {
            command.arg(format!("--{side}tls-verify=false"));
        }
    }

    /// The raw manifest of `tag`, or `None` when the repository lacks it.
    fn manifest(&self, work: &Path, tag: &str) -> Result<Option<Vec<u8>>> {
        let mut command = self.skopeo(work)?;
        command.args(["inspect", "--raw", "--retry-times", "2"]);
        self.transport_flags(&mut command, "");
        command.arg(self.reference(tag));
        let result = executor::command(command, INSPECT_SECONDS)?;
        let stderr = String::from_utf8_lossy(&result.stderr);
        if result.code != 0 {
            if stderr.contains("manifest unknown") || stderr.contains("not found") {
                return Ok(None);
            }
            return Err(Error::Unavailable(format!(
                "binary cache {} is unavailable: {}",
                self.repository,
                stderr.trim()
            )));
        }
        if result.stdout_truncated || result.stdout.len() as u64 > MAX_JSON {
            return Err(invalid("cache manifest exceeds its bound"));
        }
        Ok(Some(result.stdout))
    }

    fn copy(&self, work: &Path, from: &str, to: &str, push: bool) -> Result<()> {
        let mut command = self.skopeo(work)?;
        command.args(["copy", "--quiet", "--retry-times", "2"]);
        if push {
            command.arg("--preserve-digests");
        }
        self.transport_flags(&mut command, if push { "dest-" } else { "src-" });
        command.args([from, to]);
        let result = executor::command(command, TRANSFER_SECONDS)?;
        if result.code != 0 {
            return Err(Error::Unavailable(format!(
                "binary cache {} transfer failed: {}",
                self.repository,
                String::from_utf8_lossy(&result.stderr).trim()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct PushResult {
    pub object: String,
    pub tag: String,
    pub pushed: bool,
}

#[derive(Debug, Serialize)]
pub struct SubstituteResult {
    pub object: String,
    pub tag: String,
    pub objects: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Descriptor {
    #[serde(rename = "mediaType")]
    media_type: String,
    digest: String,
    size: u64,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: Descriptor,
    layers: Vec<Descriptor>,
    #[serde(default)]
    annotations: BTreeMap<String, String>,
}

impl Store {
    /// Push `id`'s verified closure as one OCI artifact under its receipt tag.
    /// A tag already present is left as it is: content decides the tag.
    pub fn push(&self, cache: &BinaryCache, id: &str) -> Result<PushResult> {
        let receipts = self.closure_receipts(id)?;
        let tag = closure_tag(&receipts)?;
        let stage = self.staging()?;
        let result = (|| -> Result<bool> {
            if cache.manifest(&stage, &tag)?.is_some() {
                return Ok(false);
            }
            let layout = stage.join("layout");
            fs::create_dir(&layout)?;
            let bundle = stage.join("bundle.tar");
            let exported = self.export(id, &bundle)?;
            fs::rename(&bundle, layout.join(&exported.sha256))?;
            let layer = format!("sha256:{}", exported.sha256);
            let config = serde_json::to_vec(&serde_json::json!({
                "architecture": OCI_ARCHITECTURE,
                "os": "linux",
                "rootfs": {"type": "layers", "diff_ids": [layer]},
            }))?;
            let config_sha256 = plan::hash(&config);
            write_new(&layout.join(&config_sha256), &config)?;
            let mut annotations = BTreeMap::from([
                ("dev.kedra.engine.closure".to_owned(), tag.clone()),
                ("dev.kedra.engine.object".to_owned(), id.to_owned()),
            ]);
            if let Some(source) = &cache.source {
                annotations.insert("org.opencontainers.image.source".into(), source.clone());
            }
            let manifest = Manifest {
                schema_version: 2,
                media_type: MANIFEST.into(),
                config: Descriptor {
                    media_type: CONFIG.into(),
                    digest: format!("sha256:{config_sha256}"),
                    size: config.len() as u64,
                },
                layers: vec![Descriptor {
                    media_type: LAYER.into(),
                    digest: layer,
                    size: exported.bytes,
                }],
                annotations,
            };
            write_new(
                &layout.join("manifest.json"),
                &serde_json::to_vec(&manifest)?,
            )?;
            write_new(
                &layout.join("version"),
                b"Directory Transport Version: 1.1\n",
            )?;
            cache.copy(
                &stage,
                &format!("dir:{}", layout.display()),
                &cache.reference(&tag),
                true,
            )?;
            Ok(true)
        })();
        tree::remove(&stage)?;
        Ok(PushResult {
            object: id.into(),
            tag,
            pushed: result?,
        })
    }

    /// Admit `id`'s closure from the cache. Refused unless the records hold the
    /// complete closure and every pulled object equals its record; a closure the
    /// store already holds needs no transfer.
    pub fn substitute(
        &self,
        cache: &BinaryCache,
        records: &TrustedRecords,
        id: &str,
    ) -> Result<SubstituteResult> {
        let trusted = records.closure(id)?;
        let tag = closure_tag(&trusted)?;
        let mut missing = Vec::new();
        for (object, receipt) in &trusted {
            if !self.object_path(object).exists() {
                missing.push(object.clone());
            } else if serde_json::to_vec(&self.verify(object)?)? != serde_json::to_vec(receipt)? {
                return Err(invalid(format!(
                    "stored {object} differs from its trusted record"
                )));
            }
        }
        if missing.is_empty() {
            return Ok(SubstituteResult {
                object: id.into(),
                tag,
                objects: missing,
            });
        }
        let stage = self.staging()?;
        let result = (|| -> Result<()> {
            let manifest = cache
                .manifest(&stage, &tag)?
                .ok_or_else(|| Error::Unavailable(format!("binary cache lacks {tag}")))?;
            let layer = single_layer(&manifest)?;
            let layout = stage.join("layout");
            cache.copy(
                &stage,
                &cache.reference(&tag),
                &format!("dir:{}", layout.display()),
                false,
            )?;
            let digest = &layer.digest[7..];
            let bundle = layout.join(digest);
            let (sha256, bytes) = hash_file(&bundle, MAX_BUNDLE)?;
            if sha256 != digest || bytes != layer.size {
                return Err(Error::Corrupt(
                    "pulled closure differs from its manifest".into(),
                ));
            }
            fs::rename(&bundle, stage.join("closure.tar"))?;
            tree::remove(&layout)?;
            Ok(())
        })();
        if let Err(error) = result {
            tree::remove(&stage)?;
            return Err(error);
        }
        let admitted = self.import_trusted(
            &stage.join("closure.tar"),
            &TrustedClosure {
                root: id.into(),
                objects: trusted,
            },
        );
        tree::remove(&stage)?;
        admitted?;
        Ok(SubstituteResult {
            object: id.into(),
            tag,
            objects: missing,
        })
    }

    fn closure_receipts(&self, id: &str) -> Result<BTreeMap<String, ObjectReceipt>> {
        self.closure(id)?
            .objects
            .into_iter()
            .map(|object| Ok((object.clone(), self.verify(&object)?)))
            .collect()
    }
}

/// Substitution for `Store::build_with`: where closures come from and which
/// records admit them.
#[derive(Clone, Copy)]
pub struct Substitution<'a> {
    pub cache: &'a BinaryCache,
    pub records: &'a TrustedRecords,
}

/// The receipts a substituted bundle must hold exactly.
pub(crate) struct TrustedClosure {
    pub root: String,
    pub objects: BTreeMap<String, ObjectReceipt>,
}

impl TrustedClosure {
    /// Every staged object's tree hash was recomputed from its staged bytes, so
    /// receipt equality is content equality with the signed record.
    pub(crate) fn admit(
        &self,
        roots: &[String],
        staged: &BTreeMap<String, &ObjectReceipt>,
    ) -> Result<()> {
        if roots != [self.root.clone()]
            || staged.keys().collect::<BTreeSet<_>>()
                != self.objects.keys().collect::<BTreeSet<_>>()
        {
            return Err(invalid(
                "substituted closure differs from the trusted records",
            ));
        }
        for (id, receipt) in staged {
            if serde_json::to_vec(receipt)? != serde_json::to_vec(&self.objects[id])? {
                return Err(invalid(format!(
                    "substituted {id} differs from its trusted record"
                )));
            }
        }
        Ok(())
    }
}

fn single_layer(bytes: &[u8]) -> Result<Descriptor> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|_| invalid("cache manifest is not an OCI manifest"))?;
    let mut layers = manifest.layers.into_iter();
    match (layers.next(), layers.next()) {
        (Some(layer), None)
            if manifest.schema_version == 2
                && manifest.media_type == MANIFEST
                && layer.media_type == LAYER
                && layer.size <= MAX_BUNDLE
                && layer.digest.strip_prefix("sha256:").is_some_and(plan::hex) =>
        {
            Ok(layer)
        }
        _ => Err(invalid("cache artifact must hold one bounded tar layer")),
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// A regular file read without following links or blocking on a FIFO.
fn bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let input: File = rustix::fs::openat(
        rustix::fs::CWD,
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?
    .into();
    let metadata = input.metadata()?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(invalid("trusted records must be a bounded regular file"));
    }
    let mut bytes = Vec::new();
    input.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("trusted records exceed their bound"));
    }
    Ok(bytes)
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(format!("binary cache: {}", message.into()))
}
