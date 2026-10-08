use crate::{
    plan::{hex, image_id, object_id, relative},
    store::{hash_file, private, read_json, write_json_new},
    tree::{self, Entry, Kind},
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions, Permissions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_BUNDLE: u64 = 16 * 1024 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema: u32,
    roots: Vec<String>,
    #[serde(deserialize_with = "crate::model::unique_map")]
    objects: BTreeMap<String, BundledObject>,
    #[serde(deserialize_with = "crate::model::unique_map")]
    images: BTreeMap<String, ImageReceipt>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundledObject {
    receipt: ObjectReceipt,
    entries: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportJournal {
    schema: u32,
    token: String,
    stage: String,
    objects: Vec<String>,
    images: Vec<String>,
    roots: Vec<String>,
}
impl Store {
    pub fn export(&self, id: &str, output: &Path) -> Result<ExportResult> {
        let closure = self.closure(id)?;
        let mut manifest = Bundle {
            schema: 1,
            roots: vec![id.into()],
            objects: BTreeMap::new(),
            images: BTreeMap::new(),
        };
        for object in &closure.objects {
            let receipt = self.verify(object)?;
            let entries = tree::inspect(&self.object_path(object).join("data"), None)?.entries;
            manifest
                .objects
                .insert(object.clone(), BundledObject { receipt, entries });
        }
        for image in &closure.images {
            manifest
                .images
                .insert(image.clone(), self.verify_image(image)?);
        }
        let bytes = serde_json::to_vec(&manifest)?;
        if bytes.len() as u64 > MAX_JSON {
            return Err(Error::Invalid("bundle manifest exceeds 8 MiB".into()));
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(output)?;
        let mut tar = tar::Builder::new(file);
        append(
            &mut tar,
            "manifest.json",
            bytes.len() as u64,
            bytes.as_slice(),
        )?;
        for (id, object) in &manifest.objects {
            for entry in &object.entries {
                if let Kind::File { bytes, .. } = entry.kind {
                    append(
                        &mut tar,
                        &format!("objects/{id}/files/{}", entry.path),
                        bytes,
                        File::open(self.object_path(id).join("data").join(&entry.path))?,
                    )?;
                }
            }
        }
        for (id, image) in &manifest.images {
            append(
                &mut tar,
                &format!("images/{}.tar", &id[7..]),
                image.bytes,
                File::open(self.image_path(id).join("image.tar"))?,
            )?;
        }
        tar.finish()?;
        tar.into_inner()?.sync_all()?;
        let (sha256, bytes) = hash_file(output, MAX_BUNDLE)?;
        Ok(ExportResult { sha256, bytes })
    }
    pub fn import(&self, bundle: &Path, expected_sha256: &str) -> Result<ImportResult> {
        self.import_checked(bundle, expected_sha256, None)
    }
    /// Admit an authorized binary closure after independently resolving its recipe.
    pub fn substitute(
        &self,
        bundle: &Path,
        authorization: &VerifiedCacheReceipt,
    ) -> Result<ImportResult> {
        self.import_checked(
            bundle,
            &authorization.receipt().bundle_sha256,
            Some(authorization),
        )
    }
    fn import_checked(
        &self,
        bundle: &Path,
        expected_sha256: &str,
        authorization: Option<&VerifiedCacheReceipt>,
    ) -> Result<ImportResult> {
        if !hex(expected_sha256) {
            return Err(Error::Invalid(
                "expected bundle SHA256 must be 64 lowercase hex digits".into(),
            ));
        }
        let stage = self.staging()?;
        let result = (|| {
            let archive = stage.join("bundle.tar");
            let input: File = rustix::fs::openat(
                rustix::fs::CWD,
                bundle,
                rustix::fs::OFlags::RDONLY
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )?
            .into();
            if !input.metadata()?.is_file() {
                return Err(Error::Invalid("bundle must be a regular file".into()));
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&archive)?;
            let copied = std::io::copy(&mut input.take(MAX_BUNDLE + 1), &mut output)?;
            if copied > MAX_BUNDLE {
                return Err(Error::Invalid("bundle exceeds 16 GiB".into()));
            }
            output.sync_all()?;
            let (actual, bytes) = hash_file(&archive, MAX_BUNDLE)?;
            if actual != expected_sha256 {
                return Err(Error::Corrupt("bundle SHA256 differs from expected".into()));
            }
            self.import_staged(&archive, &stage, &actual, bytes, authorization)
        })();
        if stage.exists() && !self.path().join("transactions/import.json").exists() {
            tree::remove(&stage)?;
        }
        result
    }
    fn import_staged(
        &self,
        bundle: &Path,
        stage: &Path,
        sha256: &str,
        bundle_bytes: u64,
        authorization: Option<&VerifiedCacheReceipt>,
    ) -> Result<ImportResult> {
        let mut archive = tar::Archive::new(File::open(bundle)?);
        let mut members = archive.entries()?;
        let (manifest, manifest_bytes) = read_manifest(&mut members)?;
        let allowlist = validate_manifest(&manifest)?;
        create_staged_directories(stage, &manifest)?;
        unpack_members(members, stage, &allowlist, manifest_bytes)?;
        for (id, object) in &manifest.objects {
            self.verify_staged_object(stage, id, object)?;
        }
        for (id, receipt) in &manifest.images {
            self.verify_staged_image(stage, id, receipt)?;
        }
        let order = dependency_order(&manifest)?;
        if let Some(authorization) = authorization {
            authorize_bundle(authorization, &manifest, sha256, bundle_bytes)?;
        }
        let journal = ImportJournal {
            schema: 1,
            token: self.token.clone(),
            stage: stage
                .file_name()
                .and_then(|p| p.to_str())
                .ok_or_else(|| Error::Invalid("stage name unavailable".into()))?
                .into(),
            objects: order.clone(),
            images: manifest.images.keys().cloned().collect(),
            roots: manifest.roots.clone(),
        };
        self.publish_import_journal(&journal)?;
        for (id, receipt) in &manifest.images {
            self.admit_image(&stage.join("images").join(&id[7..]), receipt)?;
        }
        for id in order {
            self.admit_object(
                &stage.join("objects").join(&id),
                &manifest.objects[&id].receipt,
            )?;
        }
        for root in &manifest.roots {
            self.pin(root)?;
        }
        fs::remove_file(self.path().join("transactions/import.json"))?;
        File::open(self.path().join("transactions"))?.sync_all()?;
        if stage.exists() {
            tree::remove(stage)?;
        }
        Ok(ImportResult {
            roots: manifest.roots,
        })
    }
    fn verify_staged_object(&self, stage: &Path, id: &str, object: &BundledObject) -> Result<()> {
        let data = stage.join("objects").join(id).join("data");
        restore_modes_and_links(&data, &object.entries)?;
        let tree = tree::inspect(&data, None)?;
        if tree.digest != object.receipt.tree_sha256
            || serde_json::to_vec(&tree.entries)? != serde_json::to_vec(&object.entries)?
        {
            return Err(Error::Corrupt(format!("bundle tree differs for {id}")));
        }
        self.validate_receipt(&object.receipt, &tree)?;
        if self.object_path(id).exists() {
            let existing = self.verify(id)?;
            if serde_json::to_vec(&existing)? != serde_json::to_vec(&object.receipt)? {
                return Err(Error::Corrupt("bundle conflicts with stored output".into()));
            }
        }
        Ok(())
    }
    fn verify_staged_image(&self, stage: &Path, id: &str, receipt: &ImageReceipt) -> Result<()> {
        let (sha, bytes) = hash_file(
            &stage.join("images").join(&id[7..]).join("image.tar"),
            8 * 1024 * 1024 * 1024,
        )?;
        if sha != receipt.sha256 || bytes != receipt.bytes {
            return Err(Error::Corrupt("bundle image archive differs".into()));
        }
        crate::image_archive::verify(
            &stage.join("images").join(&id[7..]).join("image.tar"),
            &receipt.image,
            &receipt.platform,
        )?;
        if self.image_path(id).exists() {
            let old = self.verify_image(id)?;
            if old.sha256 != receipt.sha256 {
                return Err(Error::Corrupt(
                    "bundle conflicts with stored image archive".into(),
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn recover_import(&self) -> Result<()> {
        self.retire_import_next()?;
        let path = self.path().join("transactions/import.json");
        if !path.exists() {
            return Ok(());
        }
        private(&path, false)?;
        let journal: ImportJournal = read_json(&path)?;
        if journal.schema != 1 || journal.token != self.token {
            return Err(Error::RecoveryRequired(
                "import journal identity differs".into(),
            ));
        }
        crate::store::stage_name(&journal.stage)?;
        let mut roots = self.roots()?;
        for image in &journal.images {
            image_id(image)?;
            if self.image_path(image).exists() {
                self.verify_image(image)?;
                roots.images.insert(image.clone());
            }
        }
        for object in &journal.objects {
            object_id(object)?;
            if self.object_path(object).exists() {
                self.verify(object)?;
                roots.objects.insert(object.clone());
            }
        }
        self.save_roots(&roots)?;
        tree::remove(&self.path().join("transactions").join(&journal.stage))?;
        fs::remove_file(path)?;
        Ok(())
    }
    fn publish_import_journal(&self, journal: &ImportJournal) -> Result<()> {
        let directory = self.path().join("transactions");
        let pending = directory.join("import.next");
        let destination = directory.join("import.json");
        write_json_new(&pending, journal)?;
        // A complete fsynced record is linked without replacement. Admission
        // starts only after the extra link is retired and the directory synced.
        fs::hard_link(&pending, &destination)?;
        File::open(&directory)?.sync_all()?;
        fs::remove_file(&pending)?;
        File::open(directory)?.sync_all()?;
        Ok(())
    }
    fn retire_import_next(&self) -> Result<()> {
        let directory = self.path().join("transactions");
        let pending = directory.join("import.next");
        let metadata = match fs::symlink_metadata(&pending) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o600
        {
            return Err(Error::RecoveryRequired(
                "unsafe import publication temporary".into(),
            ));
        }
        let destination = directory.join("import.json");
        match metadata.nlink() {
            1 => match fs::symlink_metadata(&destination) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
                Ok(_) => {
                    return Err(Error::RecoveryRequired(
                        "import temporary conflicts with journal".into(),
                    ));
                }
            },
            2 => {
                let published = fs::symlink_metadata(&destination)?;
                if !published.is_file()
                    || published.dev() != metadata.dev()
                    || published.ino() != metadata.ino()
                {
                    return Err(Error::RecoveryRequired(
                        "import temporary aliases another file".into(),
                    ));
                }
                let journal: ImportJournal = read_json(&destination)?;
                if journal.schema != 1 || journal.token != self.token {
                    return Err(Error::RecoveryRequired(
                        "import journal identity differs".into(),
                    ));
                }
                crate::store::stage_name(&journal.stage)?;
            }
            _ => {
                return Err(Error::RecoveryRequired(
                    "import temporary has unexpected aliases".into(),
                ));
            }
        }
        // A lone partial temporary cannot have authorized admission. Its exact
        // reserved name lives under the locked private store transaction root.
        fs::remove_file(pending)?;
        File::open(directory)?.sync_all()?;
        Ok(())
    }
}
fn append<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    size: u64,
    data: impl Read,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(0o600);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    tar.append_data(&mut header, path, data)?;
    Ok(())
}
fn read_manifest(members: &mut tar::Entries<'_, File>) -> Result<(Bundle, u64)> {
    let mut first = members
        .next()
        .ok_or_else(|| Error::Invalid("empty bundle".into()))??;
    if first.path()?.as_ref() != Path::new("manifest.json")
        || !first.header().entry_type().is_file()
        || first.size() > MAX_JSON
    {
        return Err(Error::Invalid(
            "bundle must begin with bounded regular manifest.json".into(),
        ));
    }
    let mut bytes = Vec::new();
    first.read_to_end(&mut bytes)?;
    let manifest: Bundle = serde_json::from_slice(&bytes)?;
    Ok((manifest, bytes.len() as u64))
}
fn create_staged_directories(stage: &Path, manifest: &Bundle) -> Result<()> {
    fs::create_dir(stage.join("objects"))?;
    fs::create_dir(stage.join("images"))?;
    for (id, object) in &manifest.objects {
        let data = stage.join("objects").join(id).join("data");
        fs::create_dir_all(&data)?;
        for entry in &object.entries {
            if matches!(entry.kind, Kind::Directory) {
                fs::create_dir(data.join(&entry.path))?;
                fs::set_permissions(data.join(&entry.path), Permissions::from_mode(0o755))?;
            }
        }
    }
    for id in manifest.images.keys() {
        fs::create_dir(stage.join("images").join(&id[7..]))?;
    }
    Ok(())
}
fn unpack_members(
    members: tar::Entries<'_, File>,
    stage: &Path,
    allowlist: &BTreeMap<String, (String, u64)>,
    manifest_bytes: u64,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut total = manifest_bytes;
    for member in members {
        let mut member = member?;
        if !member.header().entry_type().is_file() {
            return Err(Error::Invalid(
                "archive links, devices and non-files are forbidden".into(),
            ));
        }
        let path = member
            .path()?
            .to_str()
            .ok_or_else(|| Error::Invalid("non-UTF8 bundle member".into()))?
            .to_owned();
        relative(&path, false)?;
        if !seen.insert(path.clone()) {
            return Err(Error::Invalid(format!("duplicate bundle member {path}")));
        }
        let (dest, expected) = allowlist
            .get(&path)
            .ok_or_else(|| Error::Invalid(format!("unknown bundle member {path}")))?;
        if member.size() != *expected {
            return Err(Error::Corrupt("bundle member size differs".into()));
        }
        total = total
            .checked_add(member.size())
            .ok_or_else(|| Error::Invalid("bundle size overflow".into()))?;
        if total > MAX_BUNDLE {
            return Err(Error::Invalid("bundle exceeds 16 GiB".into()));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(stage.join(dest))?;
        std::io::copy(&mut member, &mut file)?;
        file.sync_all()?;
    }
    if seen.len() != allowlist.len() {
        return Err(Error::Invalid("bundle is missing declared members".into()));
    }
    Ok(())
}
fn restore_modes_and_links(data: &Path, entries: &[Entry]) -> Result<()> {
    for entry in entries {
        match &entry.kind {
            Kind::File { executable, .. } => fs::set_permissions(
                data.join(&entry.path),
                Permissions::from_mode(if *executable { 0o755 } else { 0o644 }),
            )?,
            Kind::Symlink { target } => std::os::unix::fs::symlink(target, data.join(&entry.path))?,
            Kind::Directory => {}
        }
    }
    Ok(())
}
fn authorize_bundle(
    authorization: &VerifiedCacheReceipt,
    manifest: &Bundle,
    sha256: &str,
    bundle_bytes: u64,
) -> Result<()> {
    let root = manifest
        .objects
        .get(&authorization.receipt().root.object)
        .ok_or_else(|| Error::Invalid("authorized bundle root is absent".into()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Invalid("system clock precedes Unix epoch".into()))?
        .as_secs();
    authorization.validate_bundle(&manifest.roots, &root.receipt, sha256, bundle_bytes, now)
}
fn validate_manifest(manifest: &Bundle) -> Result<BTreeMap<String, (String, u64)>> {
    if manifest.schema != 1
        || manifest.roots.is_empty()
        || manifest.roots.len() > 256
        || manifest.objects.len() > 256
        || manifest.images.len() > 256
    {
        return Err(Error::Invalid("unsupported bundle schema/count".into()));
    }
    let mut allow = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for root in &manifest.roots {
        object_id(root)?;
        if !roots.insert(root) || !manifest.objects.contains_key(root) {
            return Err(Error::Invalid("duplicate or missing bundle root".into()));
        }
    }
    for (id, object) in &manifest.objects {
        object_id(id)?;
        if object.receipt.object != *id || object.entries.len() > 100000 {
            return Err(Error::Invalid(
                "bundle object identity/count mismatch".into(),
            ));
        }
        let mut entries: BTreeMap<&str, &Kind> = BTreeMap::new();
        let mut total = 0u64;
        for entry in &object.entries {
            relative(&entry.path, false)?;
            if entries.insert(&entry.path, &entry.kind).is_some() {
                return Err(Error::Invalid("duplicate tree path".into()));
            }
            if let Some((parent, _)) = entry.path.rsplit_once('/')
                && !matches!(entries.get(parent), Some(Kind::Directory))
            {
                return Err(Error::Invalid(
                    "tree parent is missing or not a directory".into(),
                ));
            }
            match &entry.kind {
                Kind::Directory => {}
                Kind::Symlink { target } => tree::link_target(&entry.path, target)?,
                Kind::File { bytes, sha256, .. } => {
                    total = total
                        .checked_add(*bytes)
                        .ok_or_else(|| Error::Invalid("tree size overflow".into()))?;
                    if *bytes > tree::MAX_FILE || total > tree::MAX_TREE || !hex(sha256) {
                        return Err(Error::Invalid("tree file limit/hash invalid".into()));
                    }
                    allow.insert(
                        format!("objects/{id}/files/{}", entry.path),
                        (format!("objects/{id}/data/{}", entry.path), *bytes),
                    );
                }
            }
        }
        tree::validate_links(&object.entries)?;
        for reference in &object.receipt.references {
            if !manifest.objects.contains_key(reference) {
                return Err(Error::Invalid("bundle missing runtime reference".into()));
            }
        }
        if let Some(image) = &object.receipt.runtime_image
            && !manifest.images.contains_key(image)
        {
            return Err(Error::Invalid("bundle missing runtime image".into()));
        }
    }
    for (id, image) in &manifest.images {
        image_id(id)?;
        if image.image != *id
            || image.schema != 1
            || image.platform != PLATFORM
            || image.bytes > 8 * 1024 * 1024 * 1024
            || !hex(&image.sha256)
        {
            return Err(Error::Invalid("bundle image identity/limit invalid".into()));
        }
        allow.insert(
            format!("images/{}.tar", &id[7..]),
            (format!("images/{}/image.tar", &id[7..]), image.bytes),
        );
    }
    dependency_order(manifest)?;
    Ok(allow)
}
fn dependency_order(manifest: &Bundle) -> Result<Vec<String>> {
    fn visit(
        id: &str,
        m: &Bundle,
        active: &mut BTreeSet<String>,
        done: &mut BTreeMap<String, BTreeSet<String>>,
        order: &mut Vec<String>,
    ) -> Result<BTreeSet<String>> {
        if let Some(images) = done.get(id) {
            return Ok(images.clone());
        }
        if !active.insert(id.into()) {
            return Err(Error::Invalid("bundle reference cycle".into()));
        }
        let object = m
            .objects
            .get(id)
            .ok_or_else(|| Error::Invalid("bundle missing referenced object".into()))?;
        let mut images = BTreeSet::new();
        if let Some(image) = &object.receipt.runtime_image {
            images.insert(image.clone());
        }
        for dep in &object.receipt.references {
            images.extend(visit(dep, m, active, done, order)?);
        }
        if images.len() > 1 {
            return Err(Error::Invalid("bundle runtime foundations differ".into()));
        }
        active.remove(id);
        done.insert(id.into(), images.clone());
        order.push(id.into());
        Ok(images)
    }
    let mut order = Vec::new();
    let mut done = BTreeMap::new();
    let mut used_images = BTreeSet::new();
    for root in &manifest.roots {
        used_images.extend(visit(
            root,
            manifest,
            &mut BTreeSet::new(),
            &mut done,
            &mut order,
        )?);
    }
    if done.len() != manifest.objects.len() || used_images.len() != manifest.images.len() {
        return Err(Error::Invalid(
            "bundle contains unreferenced members".into(),
        ));
    }
    Ok(order)
}
