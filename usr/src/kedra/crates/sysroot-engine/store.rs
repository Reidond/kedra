use crate::{
    executor,
    plan::{hex, image_id, object_id},
    tree, *,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions, Permissions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
const DIRECTORIES: [&str; 6] = [
    "objects",
    "images",
    "transactions",
    "profiles",
    "quarantine",
    "roots",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: u32,
    token: String,
    prefix: String,
    uid: u32,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Roots {
    pub schema: u32,
    pub objects: BTreeSet<String>,
    pub images: BTreeSet<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GcJournal {
    schema: u32,
    token: String,
    stage: String,
    objects: Vec<String>,
    images: Vec<String>,
}
pub struct Store {
    path: PathBuf,
    pub(crate) token: String,
    _lock: File,
    connection: std::sync::OnceLock<executor::Connection>,
    _not_shared: std::marker::PhantomData<std::cell::Cell<()>>,
}
impl Store {
    pub(crate) fn connection(&self) -> Result<executor::Connection> {
        if let Some(connection) = self.connection.get() {
            return Ok(connection.clone());
        }
        let connection = executor::Connection::resolve()?;
        let _ = self.connection.set(connection.clone());
        Ok(connection)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn create(path: &Path) -> Result<Self> {
        if path.exists() {
            return Self::open(path);
        }
        if rustix::process::geteuid().is_root() {
            return Err(Error::Invalid(
                "owner-private engine stores cannot be root-owned".into(),
            ));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = fs::canonicalize(parent)?;
        let leaf = path
            .file_name()
            .ok_or_else(|| Error::Invalid("store path has no name".into()))?;
        let path = parent.join(leaf);
        safe_bind(&path)?;
        fs::create_dir(&path)?;
        fs::set_permissions(&path, Permissions::from_mode(0o700))?;
        for dir in DIRECTORIES {
            fs::create_dir(path.join(dir))?;
            fs::set_permissions(path.join(dir), Permissions::from_mode(0o700))?;
        }
        let marker = Marker {
            schema: 1,
            token: nonce()?,
            prefix: LOGICAL_PREFIX.into(),
            uid: rustix::process::geteuid().as_raw(),
        };
        write_json_new(&path.join("marker.json"), &marker)?;
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path.join("lock"))?;
        lock.sync_all()?;
        write_json_new(
            &path.join("roots/state.json"),
            &Roots {
                schema: 1,
                ..Roots::default()
            },
        )?;
        File::open(&path)?.sync_all()?;
        Self::open(&path)
    }
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_inner(path, false)
    }
    fn open_inner(path: &Path, recover: bool) -> Result<Self> {
        private(path, true)?;
        let path = fs::canonicalize(path)?;
        safe_bind(&path)?;
        for dir in DIRECTORIES {
            private(&path.join(dir), true)?;
        }
        private(&path.join("marker.json"), false)?;
        private(&path.join("lock"), false)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path.join("lock"))?;
        lock.lock()?;
        let marker: Marker = read_json(&path.join("marker.json"))?;
        if marker.schema != 1
            || marker.prefix != LOGICAL_PREFIX
            || marker.uid != rustix::process::geteuid().as_raw()
            || !hex(&marker.token)
        {
            return Err(Error::Invalid(
                "unknown store marker/ownership/schema".into(),
            ));
        }
        let store = Self {
            path,
            token: marker.token,
            _lock: lock,
            connection: std::sync::OnceLock::new(),
            _not_shared: std::marker::PhantomData,
        };
        store.roots()?;
        if !recover
            && fs::read_dir(store.path.join("transactions"))?
                .next()
                .is_some()
        {
            return Err(Error::RecoveryRequired(
                "unfinished transaction; use store recover".into(),
            ));
        }
        Ok(store)
    }
    pub(crate) fn roots(&self) -> Result<Roots> {
        private(&self.path.join("roots/state.json"), false)?;
        let roots: Roots = read_json(&self.path.join("roots/state.json"))?;
        if roots.schema != 1 {
            return Err(Error::Corrupt("roots schema differs".into()));
        }
        for id in &roots.objects {
            object_id(id)?;
        }
        for id in &roots.images {
            image_id(id)?;
        }
        Ok(roots)
    }
    pub(crate) fn save_roots(&self, roots: &Roots) -> Result<()> {
        atomic_json(&self.path.join("roots/state.json"), roots)
    }
    pub(crate) fn object_path(&self, id: &str) -> PathBuf {
        self.path.join("objects").join(id)
    }
    pub(crate) fn image_path(&self, id: &str) -> PathBuf {
        self.path
            .join("images")
            .join(id.strip_prefix("sha256:").unwrap_or(id))
    }
    pub(crate) fn staging(&self) -> Result<PathBuf> {
        let path = self
            .path
            .join("transactions")
            .join(format!("stage-{}", nonce()?));
        fs::create_dir(&path)?;
        fs::set_permissions(&path, Permissions::from_mode(0o700))?;
        File::open(self.path.join("transactions"))?.sync_all()?;
        Ok(path)
    }
    pub fn import_source(&self, source: &Path) -> Result<ObjectReceipt> {
        let stage = self.staging()?;
        let result = (|| {
            fs::create_dir(stage.join("data"))?;
            let tree = tree::inspect(source, Some(&stage.join("data")))?;
            let object = format!("src-{}", tree.digest);
            for id in &tree.references {
                self.verify(id)?;
            }
            let receipt = ObjectReceipt {
                schema: 1,
                object: object.clone(),
                tree_sha256: tree.digest,
                references: tree.references.into_iter().collect(),
                runtime_image: None,
                derivation: None,
            };
            self.admit_object(&stage, &receipt)?;
            self.pin(&object)?;
            Ok(receipt)
        })();
        if stage.exists() {
            tree::remove(&stage)?;
        }
        result
    }
    pub(crate) fn admit_object(&self, stage: &Path, receipt: &ObjectReceipt) -> Result<()> {
        object_id(&receipt.object)?;
        let tree = tree::inspect(&stage.join("data"), None)?;
        if tree.digest != receipt.tree_sha256 {
            return Err(Error::Corrupt("admission tree mismatch".into()));
        }
        self.validate_receipt(receipt, &tree)?;
        let dest = self.object_path(&receipt.object);
        if dest.exists() {
            let existing = self.verify(&receipt.object)?;
            if serde_json::to_vec(&existing)? != serde_json::to_vec(receipt)? {
                return Err(Error::Divergent(receipt.object.clone()));
            }
            return Ok(());
        }
        write_json_new(&stage.join("receipt.json"), receipt)?;
        tree::seal(stage)?;
        publish_directory(stage, &dest)?;
        File::open(self.path.join("objects"))?.sync_all()?;
        Ok(())
    }
    pub(crate) fn validate_receipt(
        &self,
        receipt: &ObjectReceipt,
        tree: &tree::Tree,
    ) -> Result<()> {
        if receipt.schema != 1 || !hex(&receipt.tree_sha256) {
            return Err(Error::Corrupt("invalid object receipt schema/hash".into()));
        }
        let refs: BTreeSet<_> = receipt.references.iter().cloned().collect();
        if refs.len() != receipt.references.len() {
            return Err(Error::Corrupt("duplicate references".into()));
        }
        for id in &refs {
            object_id(id)?;
        }
        match (&receipt.derivation, &receipt.runtime_image) {
            (None, None) => {
                if receipt.object != format!("src-{}", tree.digest) || refs != tree.references {
                    return Err(Error::Corrupt(
                        "source receipt identity/references differ".into(),
                    ));
                }
            }
            (Some(spec), Some(image)) => {
                image_id(image)?;
                if spec.runtime_image != *image
                    || plan::derivation_id(spec)? != receipt.object
                    || spec.schema != 1
                    || spec.platform != PLATFORM
                    || spec.prefix != LOGICAL_PREFIX
                {
                    return Err(Error::Corrupt("output derivation identity differs".into()));
                }
                let mut expected = tree.references.clone();
                expected.extend(spec.runtime_inputs.iter().cloned());
                expected.remove(&receipt.object);
                if expected != refs {
                    return Err(Error::Corrupt("output runtime references differ".into()));
                }
            }
            _ => return Err(Error::Corrupt("object kind/runtime mismatch".into())),
        }
        Ok(())
    }
    pub fn verify(&self, id: &str) -> Result<ObjectReceipt> {
        let mut visiting = BTreeSet::new();
        let mut verified = BTreeMap::new();
        let receipt = self.verify_inner(id, &mut visiting, &mut verified)?;
        let foundations: BTreeSet<_> = verified
            .values()
            .filter_map(|value| value.runtime_image.as_ref())
            .collect();
        if foundations.len() > 1 {
            return Err(Error::Corrupt(
                "transitive runtime image foundations differ".into(),
            ));
        }
        Ok(receipt)
    }
    fn verify_inner(
        &self,
        id: &str,
        visiting: &mut BTreeSet<String>,
        verified: &mut BTreeMap<String, ObjectReceipt>,
    ) -> Result<ObjectReceipt> {
        object_id(id)?;
        if let Some(receipt) = verified.get(id) {
            return Ok(receipt.clone());
        }
        if !visiting.insert(id.into()) {
            return Err(Error::Corrupt(format!("runtime reference cycle at {id}")));
        }
        let path = self.object_path(id);
        owned_node(&path, true)?;
        owned_node(&path.join("receipt.json"), false)?;
        owned_node(&path.join("data"), true)?;
        let receipt: ObjectReceipt = read_json(&path.join("receipt.json"))?;
        if receipt.object != id {
            return Err(Error::Corrupt("receipt object identity differs".into()));
        }
        let tree = tree::inspect(&path.join("data"), None)?;
        if tree.digest != receipt.tree_sha256 {
            return Err(Error::Corrupt(format!("tree digest differs for {id}")));
        }
        self.validate_receipt(&receipt, &tree)?;
        if let Some(image) = &receipt.runtime_image {
            self.verify_image(image)?;
        }
        for reference in &receipt.references {
            if reference == id {
                return Err(Error::Corrupt(
                    "explicit self reference is noncanonical".into(),
                ));
            }
            let dep = self.verify_inner(reference, visiting, verified)?;
            if let (Some(a), Some(b)) = (&receipt.runtime_image, &dep.runtime_image)
                && a != b
            {
                return Err(Error::Corrupt("runtime image foundation differs".into()));
            }
        }
        visiting.remove(id);
        verified.insert(id.into(), receipt.clone());
        Ok(receipt)
    }
    pub fn import_image(&self, id: &str) -> Result<ImageReceipt> {
        image_id(id)?;
        let connection = self.connection()?;
        executor::daemon(&connection)?;
        executor::inspect_image(&connection, id)?;
        let existing = self.image_path(id);
        if existing.exists() {
            let r = self.verify_image(id)?;
            let mut roots = self.roots()?;
            roots.images.insert(id.into());
            self.save_roots(&roots)?;
            return Ok(r);
        }
        let stage = self.staging()?;
        let result = (|| {
            let mut cmd = connection.command();
            cmd.args(["image", "save", "--output"])
                .arg(stage.join("image.tar"))
                .arg(id);
            executor::checked(executor::command(cmd, 300)?)?;
            let (sha256, bytes) = hash_file(&stage.join("image.tar"), 8 * 1024 * 1024 * 1024)?;
            let receipt = ImageReceipt {
                schema: 1,
                image: id.into(),
                platform: PLATFORM.into(),
                sha256,
                bytes,
            };
            self.admit_image(&stage, &receipt)?;
            let mut roots = self.roots()?;
            roots.images.insert(id.into());
            self.save_roots(&roots)?;
            Ok(receipt)
        })();
        if stage.exists() {
            tree::remove(&stage)?;
        }
        result
    }
    pub(crate) fn admit_image(&self, stage: &Path, receipt: &ImageReceipt) -> Result<()> {
        image_id(&receipt.image)?;
        let (hash, bytes) = hash_file(&stage.join("image.tar"), 8 * 1024 * 1024 * 1024)?;
        if receipt.schema != 1
            || receipt.platform != PLATFORM
            || hash != receipt.sha256
            || bytes != receipt.bytes
        {
            return Err(Error::Corrupt("image archive receipt mismatch".into()));
        }
        crate::image_archive::verify(&stage.join("image.tar"), &receipt.image, &receipt.platform)?;
        let dest = self.image_path(&receipt.image);
        if dest.exists() {
            let old = self.verify_image(&receipt.image)?;
            if old.sha256 != receipt.sha256 {
                return Err(Error::Corrupt("conflicting image archive".into()));
            }
            return Ok(());
        }
        write_json_new(&stage.join("receipt.json"), receipt)?;
        tree::seal(stage)?;
        publish_directory(stage, &dest)?;
        File::open(self.path.join("images"))?.sync_all()?;
        Ok(())
    }
    pub(crate) fn verify_image(&self, id: &str) -> Result<ImageReceipt> {
        image_id(id)?;
        let path = self.image_path(id);
        owned_node(&path, true)?;
        owned_node(&path.join("receipt.json"), false)?;
        owned_node(&path.join("image.tar"), false)?;
        let receipt: ImageReceipt = read_json(&path.join("receipt.json"))?;
        let (sha, bytes) = hash_file(&path.join("image.tar"), 8 * 1024 * 1024 * 1024)?;
        if receipt.schema != 1
            || receipt.image != id
            || receipt.platform != PLATFORM
            || receipt.sha256 != sha
            || receipt.bytes != bytes
        {
            return Err(Error::Corrupt(format!("invalid image evidence {id}")));
        }
        crate::image_archive::verify(&path.join("image.tar"), &receipt.image, &receipt.platform)?;
        Ok(receipt)
    }
    pub fn closure(&self, id: &str) -> Result<Closure> {
        self.verify(id)?;
        let mut objects = BTreeSet::new();
        let mut images = BTreeSet::new();
        self.collect(id, &mut objects, &mut images)?;
        Ok(Closure {
            objects: objects.into_iter().collect(),
            images: images.into_iter().collect(),
        })
    }
    fn collect(
        &self,
        id: &str,
        objects: &mut BTreeSet<String>,
        images: &mut BTreeSet<String>,
    ) -> Result<()> {
        if !objects.insert(id.into()) {
            return Ok(());
        }
        let r: ObjectReceipt = read_json(&self.object_path(id).join("receipt.json"))?;
        if let Some(image) = r.runtime_image {
            images.insert(image);
        }
        for reference in r.references {
            self.collect(&reference, objects, images)?;
        }
        Ok(())
    }
    pub fn pin(&self, id: &str) -> Result<()> {
        self.verify(id)?;
        let mut roots = self.roots()?;
        roots.objects.insert(id.into());
        self.save_roots(&roots)
    }
    pub fn unpin(&self, id: &str) -> Result<()> {
        object_id(id)?;
        let mut roots = self.roots()?;
        roots.objects.remove(id);
        self.save_roots(&roots)
    }
    pub fn unpin_image(&self, id: &str) -> Result<()> {
        image_id(id)?;
        let mut roots = self.roots()?;
        roots.images.remove(id);
        self.save_roots(&roots)
    }
    pub fn build(&self, graph: &BuildGraph, root: &str, rebuild: bool) -> Result<BuildResult> {
        let plan = plan(graph, root)?;
        let mut result = BuildResult {
            outputs: plan.outputs.clone(),
            built: vec![],
            reused: vec![],
            reproduced: vec![],
        };
        for name in &plan.order {
            let id = &plan.outputs[name];
            let spec = &plan.specs[name];
            let existing = if self.object_path(id).exists() {
                Some(self.verify(id)?)
            } else {
                None
            };
            if existing.is_some() && !rebuild {
                self.pin(id)?;
                result.reused.push(name.clone());
                continue;
            }
            self.verify_image(&spec.runtime_image)?;
            self.verify_image(&spec.builder_image)?;
            let mut mounts = BTreeSet::new();
            for input in spec.inputs.values() {
                mounts.extend(self.closure(input)?.objects);
            }
            for reference in &spec.runtime_inputs {
                let closure = self.closure(reference)?;
                if closure
                    .images
                    .iter()
                    .any(|image| image != &spec.runtime_image)
                {
                    return Err(Error::Invalid(
                        "declared runtime closure images differ".into(),
                    ));
                }
            }
            let stage = self.staging()?;
            fs::create_dir(stage.join("data"))?;
            fs::set_permissions(stage.join("data"), Permissions::from_mode(0o755))?;
            let operation = (|| {
                let argv: Vec<_> = spec
                    .argv
                    .iter()
                    .map(|arg| plan::render(arg, spec, id))
                    .collect::<Result<_>>()?;
                let env = spec
                    .env
                    .iter()
                    .map(|(key, arg)| Ok((key.clone(), plan::render(arg, spec, id)?)))
                    .collect::<Result<_>>()?;
                let run = executor::execute(
                    self,
                    executor::Execution {
                        image: &spec.builder_image,
                        mounts: &mounts.iter().cloned().collect::<Vec<_>>(),
                        output: Some((id, &stage.join("data"))),
                        argv: &argv,
                        env: &env,
                        seconds: spec.timeout_seconds,
                    },
                )?;
                if run.code != 0 {
                    return Err(Error::Process {
                        code: run.code,
                        message: String::from_utf8_lossy(&run.stderr).into_owned(),
                    });
                }
                let tree = tree::inspect(&stage.join("data"), None)?;
                let mut refs = tree.references;
                refs.extend(spec.runtime_inputs.iter().cloned());
                refs.remove(id);
                if refs.iter().any(|reference| !mounts.contains(reference)) {
                    return Err(Error::Invalid(
                        "output references undeclared or missing object".into(),
                    ));
                }
                for reference in &refs {
                    let closure = self.closure(reference)?;
                    if closure
                        .images
                        .iter()
                        .any(|image| image != &spec.runtime_image)
                    {
                        return Err(Error::Invalid(
                            "detected runtime closure images differ".into(),
                        ));
                    }
                }
                let receipt = ObjectReceipt {
                    schema: 1,
                    object: id.clone(),
                    tree_sha256: tree.digest,
                    references: refs.into_iter().collect(),
                    runtime_image: Some(spec.runtime_image.clone()),
                    derivation: Some(spec.clone()),
                };
                if let Some(old) = &existing {
                    if old.tree_sha256 != receipt.tree_sha256
                        || old.references != receipt.references
                    {
                        write_json_new(&stage.join("receipt.json"), &receipt)?;
                        let dest = self
                            .path
                            .join("quarantine")
                            .join(format!("{id}-{}", nonce()?));
                        fs::rename(&stage, &dest)?;
                        File::open(self.path.join("quarantine"))?.sync_all()?;
                        return Err(Error::Divergent(format!(
                            "{id}; alternate preserved at {}",
                            dest.display()
                        )));
                    }
                    result.reproduced.push(name.clone());
                } else {
                    self.admit_object(&stage, &receipt)?;
                    result.built.push(name.clone());
                }
                self.pin(id)?;
                Ok(())
            })();
            if stage.exists() && !self.path.join("transactions/execution.json").exists() {
                tree::remove(&stage)?;
            }
            operation?;
        }
        Ok(result)
    }
    pub fn run_output(
        &self,
        id: &str,
        program: &str,
        args: &[String],
        seconds: u64,
    ) -> Result<RunResult> {
        plan::relative(program, false)?;
        let receipt = self.verify(id)?;
        let image = receipt
            .runtime_image
            .ok_or_else(|| Error::Invalid("source has no runtime foundation".into()))?;
        let closure = self.closure(id)?;
        if closure.images.iter().any(|i| i != &image) {
            return Err(Error::Corrupt("closure runtime images differ".into()));
        }
        let mut argv = vec![format!("{LOGICAL_PREFIX}/{id}/{program}")];
        argv.extend_from_slice(args);
        validate_run_args(&argv)?;
        executor::execute(
            self,
            executor::Execution {
                image: &image,
                mounts: &closure.objects,
                output: None,
                argv: &argv,
                env: &BTreeMap::new(),
                seconds,
            },
        )
    }
    fn retained(&self) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
        let roots = self.roots()?;
        let mut keep = BTreeSet::new();
        let mut images = roots.images;
        for id in &roots.objects {
            let c = self.closure(id)?;
            keep.extend(c.objects);
            images.extend(c.images);
        }
        for name in list_names(&self.path.join("profiles"))? {
            let profile = self.profile_list(&name)?;
            for generation in profile.generations {
                let c = self.closure(&generation.object)?;
                keep.extend(c.objects);
                images.extend(c.images);
            }
        }
        for image in &images {
            self.verify_image(image)?;
        }
        Ok((keep, images))
    }
    pub fn gc(&self, delete: bool) -> Result<GcResult> {
        let (keep, images) = self.retained()?;
        let mut doomed = BTreeSet::new();
        for id in list_names(&self.path.join("objects"))? {
            self.verify(&id)?;
            if !keep.contains(&id) {
                doomed.insert(id);
            }
        }
        let mut image_doomed = Vec::new();
        for digest in list_names(&self.path.join("images"))? {
            let id = format!("sha256:{digest}");
            self.verify_image(&id)?;
            if !images.contains(&id) {
                image_doomed.push(id);
            }
        }
        let mut ordered = Vec::new();
        while !doomed.is_empty() {
            let mut referenced = BTreeSet::new();
            for id in &doomed {
                let receipt: ObjectReceipt = read_json(&self.object_path(id).join("receipt.json"))?;
                referenced.extend(receipt.references);
            }
            let batch: Vec<_> = doomed.difference(&referenced).cloned().collect();
            if batch.is_empty() {
                return Err(Error::Corrupt("GC reference cycle".into()));
            }
            for id in batch {
                doomed.remove(&id);
                ordered.push(id);
            }
        }
        if delete && (!ordered.is_empty() || !image_doomed.is_empty()) {
            let stage = self.staging()?;
            for name in ["objects", "images"] {
                fs::create_dir(stage.join(name))?;
                fs::set_permissions(stage.join(name), Permissions::from_mode(0o700))?;
            }
            File::open(&stage)?.sync_all()?;
            let journal = GcJournal {
                schema: 1,
                token: self.token.clone(),
                stage: stage
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::Invalid("stage name missing".into()))?
                    .into(),
                objects: ordered.clone(),
                images: image_doomed.clone(),
            };
            write_json_new(&self.path.join("transactions/gc.json"), &journal)?;
            self.finish_gc(&journal)?;
        }
        Ok(GcResult {
            objects: ordered,
            images: image_doomed,
            deleted: delete,
        })
    }
    fn recover_gc(&self) -> Result<()> {
        let path = self.path.join("transactions/gc.json");
        if !exists_nofollow(&path)? {
            return Ok(());
        }
        private(&path, false)?;
        let journal: GcJournal = read_json(&path)?;
        self.finish_gc(&journal)
    }
    fn finish_gc(&self, journal: &GcJournal) -> Result<()> {
        if journal.schema != 1 || journal.token != self.token {
            return Err(Error::RecoveryRequired(
                "GC journal identity differs".into(),
            ));
        }
        stage_name(&journal.stage)?;
        let mut positions = BTreeMap::new();
        for (position, id) in journal.objects.iter().enumerate() {
            object_id(id)?;
            if positions.insert(id.clone(), position).is_some() {
                return Err(Error::RecoveryRequired("duplicate GC object".into()));
            }
        }
        let mut images = BTreeSet::new();
        for image in &journal.images {
            image_id(image)?;
            if !images.insert(image.clone()) {
                return Err(Error::RecoveryRequired("duplicate GC image".into()));
            }
        }
        let (retained, retained_images) = self.retained()?;
        if retained.iter().any(|id| positions.contains_key(id))
            || !images.is_disjoint(&retained_images)
        {
            return Err(Error::RecoveryRequired(
                "GC journal includes retained artifacts".into(),
            ));
        }
        // Every live referrer must be retired before its dependencies. Previously
        // retired directories can be partly deleted and are intentionally not read.
        for id in list_names(&self.path.join("objects"))? {
            let receipt = self.verify(&id)?;
            for reference in &receipt.references {
                if let Some(dependency_position) = positions.get(reference)
                    && positions
                        .get(&id)
                        .is_none_or(|position| position >= dependency_position)
                {
                    return Err(Error::RecoveryRequired(
                        "GC journal has unsafe dependency order".into(),
                    ));
                }
            }
            if receipt
                .runtime_image
                .as_ref()
                .is_some_and(|image| images.contains(image))
                && !positions.contains_key(&id)
            {
                return Err(Error::RecoveryRequired(
                    "GC journal includes an image used by a surviving object".into(),
                ));
            }
        }
        let stage = self.path.join("transactions").join(&journal.stage);
        if exists_nofollow(&stage)? {
            private(&stage, true)?;
            for name in list_names(&stage)? {
                if name != "objects" && name != "images" {
                    return Err(Error::RecoveryRequired("unknown GC staging member".into()));
                }
                private(&stage.join(&name), true)?;
                for id in list_names(&stage.join(&name))? {
                    let declared = if name == "objects" {
                        positions.contains_key(&id)
                    } else {
                        images.contains(&format!("sha256:{id}"))
                    };
                    if !declared {
                        return Err(Error::RecoveryRequired("undeclared GC tombstone".into()));
                    }
                    owned_node(&stage.join(&name).join(id), true)?;
                }
            }
            for id in &journal.objects {
                retire(&self.object_path(id), &stage.join("objects").join(id))?;
            }
            for image in &journal.images {
                retire(
                    &self.image_path(image),
                    &stage.join("images").join(&image[7..]),
                )?;
            }
            tree::remove(&stage)?;
            File::open(self.path.join("transactions"))?.sync_all()?;
        } else if journal
            .objects
            .iter()
            .any(|id| self.object_path(id).exists())
            || journal.images.iter().any(|id| self.image_path(id).exists())
        {
            return Err(Error::RecoveryRequired(
                "GC staging missing before retirement completed".into(),
            ));
        }
        fs::remove_file(self.path.join("transactions/gc.json"))?;
        File::open(self.path.join("transactions"))?.sync_all()?;
        Ok(())
    }
    pub fn recover(path: &Path) -> Result<Recovery> {
        let store = Self::open_inner(path, true)?;
        let mut recovered = Vec::new();
        let execution = store.path.join("transactions/execution.json");
        if execution.exists() {
            private(&execution, false)?;
            let journal: executor::Journal = read_json(&execution)?;
            executor::cleanup(&store, &journal)?;
            if !journal.staging.is_empty() {
                stage_name(&journal.staging)?;
                tree::remove(&store.path.join("transactions").join(&journal.staging))?;
            }
            fs::remove_file(execution)?;
            recovered.push(journal.container);
        }
        store.recover_import()?;
        store.recover_gc()?;
        for name in list_names(&store.path.join("transactions"))? {
            stage_name(&name)?;
            let path = store.path.join("transactions").join(&name);
            owned_node(&path, true)?;
            tree::remove(&path)?;
            recovered.push(name);
        }
        File::open(store.path.join("transactions"))?.sync_all()?;
        Ok(Recovery { recovered })
    }
}
pub(crate) fn validate_run_args(args: &[String]) -> Result<()> {
    if args.len() > 256 || args.iter().any(|s| s.len() > 32768 || s.contains('\0')) {
        return Err(Error::Invalid("command arguments exceed limits".into()));
    }
    Ok(())
}
pub(crate) fn nonce() -> Result<String> {
    let mut bytes = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(plan::hash(&bytes))
}
pub(crate) fn safe_bind(path: &Path) -> Result<()> {
    let value = path
        .to_str()
        .ok_or_else(|| Error::Invalid("non-UTF8 store path".into()))?;
    if value
        .bytes()
        .any(|b| b < 32 || b == 127 || b == b',' || b == b':')
    {
        return Err(Error::Invalid("unsafe Docker bind path".into()));
    }
    Ok(())
}
pub(crate) fn owned_node(path: &Path, dir: bool) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if m.uid() != rustix::process::geteuid().as_raw()
        || m.file_type().is_symlink()
        || (dir && !m.is_dir())
        || (!dir && (!m.is_file() || m.nlink() != 1))
        || m.mode() & 0o022 != 0
    {
        return Err(Error::Corrupt(format!(
            "unsafe owned path {}",
            path.display()
        )));
    }
    Ok(())
}
pub(crate) fn private(path: &Path, dir: bool) -> Result<()> {
    owned_node(path, dir)?;
    if fs::symlink_metadata(path)?.mode() & 0o077 != 0 {
        return Err(Error::Invalid(format!(
            "store management path is not owner-private: {}",
            path.display()
        )));
    }
    Ok(())
}
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)?;
    Read::by_ref(&mut f)
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Corrupt("JSON exceeds 8 MiB".into()));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
pub(crate) fn write_json_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    File::open(
        path.parent()
            .ok_or_else(|| Error::Invalid("missing parent".into()))?,
    )?
    .sync_all()?;
    Ok(())
}
pub(crate) fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("missing parent".into()))?;
    let temporary = parent.join(format!(".pending-{}", nonce()?));
    write_json_new(&temporary, value)?;
    fs::rename(&temporary, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub(crate) fn list_names(path: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        out.push(
            entry
                .file_name()
                .into_string()
                .map_err(|_| Error::Corrupt("non-UTF8 management name".into()))?,
        );
    }
    out.sort();
    Ok(out)
}
pub(crate) fn hash_file(path: &Path, limit: u64) -> Result<(String, u64)> {
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)?;
    if !f.metadata()?.is_file() {
        return Err(Error::Invalid("expected regular file".into()));
    }
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let count = f.read(&mut buf)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > limit {
            return Err(Error::Invalid(format!("file exceeds {limit} byte bound")));
        }
        hash.update(&buf[..count]);
    }
    Ok((crate::plan::encode_hex(&hash.finalize()), total))
}
pub(crate) fn stage_name(name: &str) -> Result<()> {
    if !name.strip_prefix("stage-").is_some_and(hex) {
        return Err(Error::RecoveryRequired(format!(
            "unknown transaction staging name {name}"
        )));
    }
    Ok(())
}
fn publish_directory(stage: &Path, destination: &Path) -> Result<()> {
    // Darwin needs write permission on the moved directory to update its parent;
    // its data and receipt have already been sealed, and the store lock excludes readers.
    fs::set_permissions(stage, Permissions::from_mode(0o700))?;
    fs::rename(stage, destination)?;
    fs::set_permissions(destination, Permissions::from_mode(0o555))?;
    File::open(destination)?.sync_all()?;
    Ok(())
}
fn exists_nofollow(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn retire(live: &Path, tombstone: &Path) -> Result<()> {
    let present = exists_nofollow(live)?;
    let retired = exists_nofollow(tombstone)?;
    if present && retired {
        return Err(Error::RecoveryRequired(
            "both live artifact and GC tombstone exist".into(),
        ));
    }
    if present {
        owned_node(live, true)?;
        fs::set_permissions(live, Permissions::from_mode(0o700))?;
        fs::rename(live, tombstone)?;
        File::open(
            live.parent()
                .ok_or_else(|| Error::Invalid("missing artifact parent".into()))?,
        )?
        .sync_all()?;
        File::open(
            tombstone
                .parent()
                .ok_or_else(|| Error::Invalid("missing tombstone parent".into()))?,
        )?
        .sync_all()?;
    }
    if present || retired {
        owned_node(tombstone, true)?;
        tree::remove(tombstone)?;
        File::open(
            tombstone
                .parent()
                .ok_or_else(|| Error::Invalid("missing tombstone parent".into()))?,
        )?
        .sync_all()?;
    }
    Ok(())
}
