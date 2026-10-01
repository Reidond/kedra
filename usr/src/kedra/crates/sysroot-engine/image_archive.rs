use crate::{Error, MAX_JSON, PLATFORM, Result, plan};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

const MAX_ARCHIVE: u64 = 8 * 1024 * 1024 * 1024;
const MAX_MEMBERS: usize = 4096;
const OCI_INDEX: &str = "application/vnd.oci.image.index.v1+json";
const DOCKER_INDEX: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
const OCI_MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
const DOCKER_MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json";

#[derive(Clone, Copy)]
struct Member {
    offset: u64,
    bytes: u64,
}

struct ImageArchive {
    file: File,
    members: BTreeMap<String, Member>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    media_type: String,
    digest: String,
    size: u64,
    platform: Option<ImagePlatform>,
}

#[derive(Deserialize)]
struct ImagePlatform {
    architecture: String,
    os: String,
    variant: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Index {
    schema_version: u32,
    media_type: Option<String>,
    manifests: Vec<Descriptor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    media_type: Option<String>,
    config: Descriptor,
    layers: Vec<Descriptor>,
}

#[derive(Deserialize)]
struct Configuration {
    architecture: String,
    os: String,
    variant: Option<String>,
    rootfs: RootFilesystem,
    config: Option<RuntimeConfiguration>,
}

#[derive(Deserialize)]
struct RootFilesystem {
    r#type: String,
    diff_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RuntimeConfiguration {
    #[serde(rename = "Volumes")]
    volumes: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Deserialize)]
struct DockerManifest {
    #[serde(rename = "Config")]
    config: String,
    #[serde(rename = "Layers")]
    layers: Vec<String>,
}

fn corrupt(message: impl std::fmt::Display) -> Error {
    Error::Corrupt(format!("image archive: {message}"))
}

fn blob_path(digest: &str) -> Result<String> {
    plan::image_id(digest).map_err(corrupt)?;
    Ok(format!("blobs/sha256/{}", &digest[7..]))
}

fn native(platform: &ImagePlatform) -> bool {
    platform.os == "linux"
        && platform.architecture == "arm64"
        && platform.variant.as_deref().is_none_or(|v| v == "v8")
}

fn is_index(media_type: &str) -> bool {
    matches!(media_type, OCI_INDEX | DOCKER_INDEX)
}

fn is_manifest(media_type: &str) -> bool {
    matches!(media_type, OCI_MANIFEST | DOCKER_MANIFEST)
}

/// Verify Docker's OCI save envelope and the complete selected native image graph.
/// Missing non-native branches of a multi-platform index are intentionally allowed.
pub(crate) fn verify(path: &Path, image: &str, platform: &str) -> Result<()> {
    plan::image_id(image).map_err(corrupt)?;
    if platform != PLATFORM {
        return Err(corrupt("unsupported platform"));
    }
    let mut archive = ImageArchive::read(path)?;
    let layout: BTreeMap<String, String> = archive.json("oci-layout")?;
    if layout.get("imageLayoutVersion").map(String::as_str) != Some("1.0.0") {
        return Err(corrupt("unsupported OCI layout version"));
    }
    let index: Index = archive.json("index.json")?;
    validate_index(&index)?;
    if index.manifests.len() != 1 || index.manifests[0].digest != image {
        return Err(corrupt(
            "root identity differs or archive has multiple roots",
        ));
    }
    let manifest = archive.select(
        index
            .manifests
            .into_iter()
            .next()
            .ok_or_else(|| corrupt("missing root descriptor"))?,
    )?;
    archive.validate_configuration(&manifest)?;
    let docker: Vec<DockerManifest> = archive.json("manifest.json")?;
    let expected_layers = manifest
        .layers
        .iter()
        .map(|layer| blob_path(&layer.digest))
        .collect::<Result<Vec<_>>>()?;
    if docker.len() != 1
        || docker[0].config != blob_path(&manifest.config.digest)?
        || docker[0].layers != expected_layers
    {
        return Err(corrupt(
            "Docker compatibility manifest differs from selected OCI image",
        ));
    }
    Ok(())
}

fn validate_index(index: &Index) -> Result<()> {
    if index.schema_version != 2
        || index.media_type.as_deref().is_some_and(|m| !is_index(m))
        || index.manifests.is_empty()
        || index.manifests.len() > 256
    {
        return Err(corrupt("unsupported or oversized image index"));
    }
    Ok(())
}

impl ImageArchive {
    fn read(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_ARCHIVE {
            return Err(corrupt("expected regular archive of at most 8 GiB"));
        }
        let mut archive = tar::Archive::new(file);
        archive.set_ignore_zeros(true);
        let mut members = BTreeMap::new();
        let mut names = BTreeSet::new();
        let mut buffer = [0u8; 65536];
        // Raw entries reject PAX/GNU path substitutions rather than interpreting them.
        for entry in archive.entries().map_err(corrupt)?.raw(true) {
            let mut entry = entry.map_err(corrupt)?;
            let name = std::str::from_utf8(&entry.path_bytes())
                .map_err(corrupt)?
                .to_owned();
            if names.len() >= MAX_MEMBERS || !names.insert(name.clone()) {
                return Err(corrupt("duplicate member or archive exceeds 4096 members"));
            }
            let entry_type = entry.header().entry_type();
            if entry_type.is_dir() {
                if !matches!(name.as_str(), "blobs/" | "blobs/sha256/") || entry.size() != 0 {
                    return Err(corrupt("unsupported archive directory"));
                }
                continue;
            }
            if !entry_type.is_file() {
                return Err(corrupt(
                    "archive links, special files and extensions are unsupported",
                ));
            }
            let expected_hash = if let Some(hash) = name.strip_prefix("blobs/sha256/") {
                if !plan::hex(hash) {
                    return Err(corrupt("invalid blob member name"));
                }
                Some(hash)
            } else if matches!(name.as_str(), "index.json" | "manifest.json" | "oci-layout") {
                if entry.size() > MAX_JSON {
                    return Err(corrupt("archive JSON exceeds 8 MiB"));
                }
                None
            } else {
                return Err(corrupt("unsupported archive member"));
            };
            let member = Member {
                offset: entry.raw_file_position(),
                bytes: entry.size(),
            };
            if member.bytes > MAX_ARCHIVE
                || member
                    .offset
                    .checked_add(member.bytes)
                    .is_none_or(|end| end > metadata.len())
            {
                return Err(corrupt("archive member exceeds file bounds"));
            }
            let mut hash = Sha256::new();
            let mut bytes = 0u64;
            loop {
                let read = entry.read(&mut buffer).map_err(corrupt)?;
                if read == 0 {
                    break;
                }
                hash.update(&buffer[..read]);
                bytes += read as u64;
            }
            if bytes != member.bytes
                || expected_hash
                    .is_some_and(|expected| plan::encode_hex(&hash.finalize()) != expected)
            {
                return Err(corrupt(format!("blob hash or member size differs: {name}")));
            }
            members.insert(name, member);
        }
        Ok(Self {
            file: archive.into_inner(),
            members,
        })
    }

    fn json<T: serde::de::DeserializeOwned>(&mut self, name: &str) -> Result<T> {
        let member = self
            .members
            .get(name)
            .ok_or_else(|| corrupt(format!("missing {name}")))?;
        if member.bytes > MAX_JSON {
            return Err(corrupt("selected JSON blob exceeds 8 MiB"));
        }
        self.file.seek(SeekFrom::Start(member.offset))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut self.file)
            .take(member.bytes)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != member.bytes {
            return Err(corrupt("truncated JSON blob"));
        }
        serde_json::from_slice(&bytes).map_err(corrupt)
    }

    fn descriptor(&self, descriptor: &Descriptor) -> Result<String> {
        let path = blob_path(&descriptor.digest)?;
        if self
            .members
            .get(&path)
            .is_none_or(|member| member.bytes != descriptor.size)
        {
            return Err(corrupt(format!(
                "missing blob or descriptor size differs: {path}"
            )));
        }
        Ok(path)
    }

    fn select(&mut self, mut descriptor: Descriptor) -> Result<Manifest> {
        let mut visited = BTreeSet::new();
        for _ in 0..8 {
            if !visited.insert(descriptor.digest.clone())
                || descriptor.platform.as_ref().is_some_and(|p| !native(p))
            {
                return Err(corrupt(
                    "cyclic image graph or unsupported selected platform",
                ));
            }
            let path = self.descriptor(&descriptor)?;
            if is_manifest(&descriptor.media_type) {
                let manifest: Manifest = self.json(&path)?;
                if manifest.schema_version != 2
                    || manifest
                        .media_type
                        .as_ref()
                        .is_some_and(|m| m != &descriptor.media_type)
                    || manifest.layers.len() > 256
                {
                    return Err(corrupt("unsupported image manifest"));
                }
                return Ok(manifest);
            }
            if !is_index(&descriptor.media_type) {
                return Err(corrupt("unsupported selected descriptor media type"));
            }
            let index: Index = self.json(&path)?;
            validate_index(&index)?;
            if index
                .media_type
                .as_ref()
                .is_some_and(|m| m != &descriptor.media_type)
            {
                return Err(corrupt("index media type differs from its descriptor"));
            }
            let mut selected = None;
            for child in index.manifests {
                let child_path = blob_path(&child.digest)?;
                if self.members.contains_key(&child_path) {
                    self.descriptor(&child)?;
                }
                let candidate = child
                    .platform
                    .as_ref()
                    .map_or_else(|| is_index(&child.media_type), native);
                if candidate && selected.replace(child).is_some() {
                    return Err(corrupt("ambiguous native platform selection"));
                }
            }
            descriptor = selected.ok_or_else(|| corrupt("native platform missing from index"))?;
        }
        Err(corrupt("image index depth exceeds 8"))
    }

    fn validate_configuration(&mut self, manifest: &Manifest) -> Result<()> {
        if !matches!(
            manifest.config.media_type.as_str(),
            "application/vnd.oci.image.config.v1+json"
                | "application/vnd.docker.container.image.v1+json"
        ) {
            return Err(corrupt("unsupported image configuration media type"));
        }
        let config_path = self.descriptor(&manifest.config)?;
        let configuration: Configuration = self.json(&config_path)?;
        if !native(&ImagePlatform {
            architecture: configuration.architecture,
            os: configuration.os,
            variant: configuration.variant,
        }) || configuration.rootfs.r#type != "layers"
            || configuration.rootfs.diff_ids.len() != manifest.layers.len()
            || configuration
                .config
                .and_then(|c| c.volumes)
                .is_some_and(|v| !v.is_empty())
        {
            return Err(corrupt(
                "configuration platform, rootfs or volumes unsupported",
            ));
        }
        for (layer, diff_id) in manifest.layers.iter().zip(configuration.rootfs.diff_ids) {
            plan::image_id(&diff_id).map_err(corrupt)?;
            self.descriptor(layer)?;
            match layer.media_type.as_str() {
                "application/vnd.oci.image.layer.v1.tar"
                | "application/vnd.docker.image.rootfs.diff.tar"
                    if layer.digest == diff_id => {}
                "application/vnd.oci.image.layer.v1.tar+gzip"
                | "application/vnd.oci.image.layer.v1.tar+zstd"
                | "application/vnd.docker.image.rootfs.diff.tar.gzip" => {}
                _ => {
                    return Err(corrupt(
                        "unsupported layer media type or uncompressed digest differs",
                    ));
                }
            }
        }
        Ok(())
    }
}
