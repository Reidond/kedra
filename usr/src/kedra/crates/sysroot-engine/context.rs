//! Offline content verification. Caller-selected identity is not release authority.
use crate::{
    Argument, BuildGraph, BuildNode, Error, Input, LOGICAL_PREFIX, MAX_JSON, ManagedSnapshot,
    ObjectReceipt, PLATFORM, ResolvedSystemFile, Result, Segment, SnapshotPurpose,
    SystemComposition, SystemContent, SystemFile, SystemFileDisposition, SystemPlan, image_archive,
    plan, system,
    tree::{self, Entry, Kind},
};
use rustix::fs::{Mode, OFlags, openat};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, Metadata, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const MAX_IMAGE: u64 = 8 * 1024 * 1024 * 1024;
const MAX_PAYLOAD: u64 = 16 * 1024 * 1024 * 1024;
const MAX_METADATA: usize = 64 * 1024 * 1024;
const RECEIPT: &str = "usr/share/sysroot/composition.json";
const NAMES: [&str; 4] = [
    "Containerfile",
    "composition.json",
    "foundation.tar",
    "payload.tar",
];

/// Owns a private copy of a context verified against an independently selected
/// identity. Keep this guard alive while consuming its paths. Foundation RPM and
/// path observations are identity-bound assertions; runtime consumers must still
/// check those assertions, inherited ONBUILD instructions, and volume settings.
pub struct VerifiedComposition {
    snapshot: ManagedSnapshot,
    composition: SystemComposition,
}
impl VerifiedComposition {
    pub fn open(input: &Path, expected_identity: &str, snapshot_parent: &Path) -> Result<Self> {
        if !plan::hex(expected_identity) {
            return Err(invalid(
                "expected identity must be 64 lowercase hexadecimal digits",
            ));
        }
        let directory = open_directory(input)?;
        let before = directory.metadata()?;
        owner_directory(&before)?;
        check_names(&directory)?;
        let snapshot = ManagedSnapshot::create(snapshot_parent, SnapshotPurpose::Composition)?;
        copy_member(
            &directory,
            snapshot.path(),
            "composition.json",
            system::MAX_COMPOSITION_JSON,
        )?;
        let composition: SystemComposition =
            serde_json::from_reader(File::open(snapshot.path().join("composition.json"))?)?;
        validate_plan(&composition, expected_identity)?;
        for (name, bound) in [("Containerfile", MAX_JSON), ("payload.tar", MAX_PAYLOAD)] {
            copy_artifact(&directory, snapshot.path(), name, bound, &composition)?;
        }
        let expected = format!("FROM {}\nADD payload.tar /\n", composition.foundation_tag);
        if fs::read(snapshot.path().join("Containerfile"))? != expected.as_bytes() {
            return Err(invalid("Containerfile differs from the static composition"));
        }
        let trees = verify_payload(&snapshot.path().join("payload.tar"), &composition)?;
        verify_files(&composition.plan, &trees)?;
        copy_artifact(
            &directory,
            snapshot.path(),
            "foundation.tar",
            MAX_IMAGE,
            &composition,
        )?;
        image_archive::verify(
            &snapshot.path().join("foundation.tar"),
            &composition.plan.foundation.receipt.image,
            &composition.plan.foundation.receipt.platform,
        )?;
        check_names(&directory)?;
        if !unchanged(&before, &directory.metadata()?) {
            return Err(invalid("context directory changed during capture"));
        }
        Ok(Self {
            snapshot,
            composition,
        })
    }
    pub fn snapshot_path(&self) -> &Path {
        self.snapshot.path()
    }
    pub fn foundation_path(&self) -> PathBuf {
        self.snapshot.path().join("foundation.tar")
    }
    pub fn payload_path(&self) -> PathBuf {
        self.snapshot.path().join("payload.tar")
    }
    pub fn containerfile_path(&self) -> PathBuf {
        self.snapshot.path().join("Containerfile")
    }
    pub fn composition(&self) -> &SystemComposition {
        &self.composition
    }
    /// Finish all consumers before explicitly retiring verified temporary inputs.
    pub fn finish(self) -> Result<()> {
        self.snapshot.finish()
    }
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(format!("composition: {}", message.into()))
}
fn open_directory(path: &Path) -> Result<File> {
    Ok(openat(
        rustix::fs::CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?
    .into())
}
fn owner_directory(metadata: &Metadata) -> Result<()> {
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o022 != 0
    {
        return Err(invalid(
            "directory must be owned and not writable by other users",
        ));
    }
    Ok(())
}
fn check_names(directory: &File) -> Result<()> {
    let mut names = BTreeSet::new();
    for entry in rustix::fs::Dir::read_from(directory)? {
        let entry = entry?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        let name = std::str::from_utf8(bytes).map_err(|_| invalid("non-UTF8 context member"))?;
        if !NAMES.contains(&name) || !names.insert(name.to_owned()) {
            return Err(invalid("unexpected context member"));
        }
    }
    if names.len() != NAMES.len() {
        return Err(invalid("missing context member"));
    }
    Ok(())
}
fn unchanged(a: &Metadata, b: &Metadata) -> bool {
    a.dev() == b.dev()
        && a.ino() == b.ino()
        && a.len() == b.len()
        && a.mode() == b.mode()
        && a.uid() == b.uid()
        && a.gid() == b.gid()
        && a.nlink() == b.nlink()
        && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec()
        && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}
fn copy_member(
    directory: &File,
    destination: &Path,
    name: &str,
    bound: u64,
) -> Result<(String, u64)> {
    let mut input: File = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?
    .into();
    let before = input.metadata()?;
    if !before.is_file()
        || before.nlink() != 1
        || before.len() > bound
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.mode() & 0o7133 != 0
    {
        return Err(invalid(format!("unsafe context file {name}")));
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination.join(name))?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > bound {
            return Err(invalid("context file exceeds limit"));
        }
        hash.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }
    if total != before.len() || !unchanged(&before, &input.metadata()?) {
        return Err(invalid("context file changed during capture"));
    }
    output.sync_all()?;
    Ok((plan::encode_hex(&hash.finalize()), total))
}
fn copy_artifact(
    directory: &File,
    destination: &Path,
    name: &str,
    bound: u64,
    composition: &SystemComposition,
) -> Result<()> {
    let actual = copy_member(directory, destination, name, bound)?;
    let expected = &composition.artifacts[name];
    if actual != (expected.sha256.clone(), expected.bytes) {
        return Err(invalid(format!("artifact hash/size differs: {name}")));
    }
    Ok(())
}

fn validate_plan(composition: &SystemComposition, expected: &str) -> Result<()> {
    let plan = &composition.plan;
    if composition.schema != 1
        || plan.schema != 1
        || composition.identity != expected
        || plan.identity != expected
    {
        return Err(invalid("unsupported schema or input identity differs"));
    }
    system::validate_definition(&plan.definition)?;
    let mut canonical = plan.clone();
    canonical.identity.clear();
    canonical.definition.files.sort_by(|a, b| {
        (&a.path, a.priority, &a.provenance).cmp(&(&b.path, b.priority, &b.provenance))
    });
    canonical.definition.required_packages.sort();
    canonical.definition.removed_packages.sort();
    let mut bytes = b"sysroot-system-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(&canonical)?);
    if plan::hash(&bytes) != expected
        || serde_json::to_vec(&canonical.definition)? != serde_json::to_vec(&plan.definition)?
    {
        return Err(invalid("canonical system plan identity differs"));
    }
    let receipt = &plan.foundation.receipt;
    if receipt.schema != 1
        || receipt.platform != PLATFORM
        || receipt.image != plan.definition.foundation
        || !plan::hex(&receipt.sha256)
        || receipt.bytes > MAX_IMAGE
        || composition.foundation_tag != format!("sysroot-foundation:{}", &receipt.image[7..])
    {
        return Err(invalid("foundation receipt differs"));
    }
    if composition.artifacts.len() != 3 {
        return Err(invalid("artifact set differs"));
    }
    for (name, bound) in [
        ("Containerfile", MAX_JSON),
        ("payload.tar", MAX_PAYLOAD),
        ("foundation.tar", MAX_IMAGE),
    ] {
        let artifact = composition
            .artifacts
            .get(name)
            .ok_or_else(|| invalid("missing artifact"))?;
        if !plan::hex(&artifact.sha256) || artifact.bytes > bound {
            return Err(invalid("invalid artifact metadata"));
        }
    }
    let archive = &composition.artifacts["foundation.tar"];
    if archive.sha256 != receipt.sha256 || archive.bytes != receipt.bytes {
        return Err(invalid("foundation archive receipt differs"));
    }
    verify_observation(plan)?;
    verify_closure(plan)?;
    for file in &plan.files {
        system::validate_system_path(&file.path)?;
        if !matches!(file.mode, 0o644 | 0o755)
            || file.bytes.len() as u64 > MAX_JSON
            || file.sha256 != plan::hash(&file.bytes)
        {
            return Err(invalid("invalid resolved file"));
        }
    }
    if plan
        .files
        .windows(2)
        .any(|pair| pair[0].path >= pair[1].path)
    {
        return Err(invalid("duplicate or unordered resolved files"));
    }
    Ok(())
}
fn verify_observation(plan: &SystemPlan) -> Result<()> {
    let observation = &plan.foundation;
    let mut fields = BTreeMap::new();
    for line in observation
        .os_release
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| invalid("invalid os-release"))?;
        if fields.insert(key, value.trim_matches('"')).is_some() {
            return Err(invalid("duplicate os-release field"));
        }
    }
    if fields.get("ID") != Some(&"fedora")
        || fields.get("VERSION_ID") != Some(&"44")
        || plan::hash(observation.rpm_inventory.as_bytes()) != observation.rpm_sha256
    {
        return Err(invalid("foundation observation differs"));
    }
    let mut lines = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut packages = BTreeSet::new();
    for line in observation.rpm_inventory.lines() {
        let cols: Vec<_> = line.split('\t').collect();
        if cols.len() != 7
            || cols
                .iter()
                .any(|s| s.is_empty() || s.len() > 4096 || s.bytes().any(|b| b < 32 || b == 127))
            || cols[0] == "gpg-pubkey"
            || !cols[1].bytes().all(|b| b.is_ascii_digit())
            || !matches!(cols[4], "aarch64" | "noarch")
            || !plan::hex(cols[5])
            || !plan::hex(cols[6])
            || !identities.insert(cols[..5].join("\t"))
            || !lines.insert(line)
            || lines.len() > 100_000
        {
            return Err(invalid("invalid RPM inventory"));
        }
        packages.insert(cols[0]);
    }
    let canonical: String = lines.iter().map(|line| format!("{line}\n")).collect();
    if lines.is_empty()
        || canonical != observation.rpm_inventory
        || plan
            .definition
            .required_packages
            .iter()
            .any(|name| !packages.contains(name.as_str()))
        || plan
            .definition
            .removed_packages
            .iter()
            .any(|name| packages.contains(name.as_str()))
    {
        return Err(invalid("RPM inventory or package assertions differ"));
    }
    Ok(())
}
fn verify_closure(plan: &SystemPlan) -> Result<()> {
    if plan.objects.len() > 100_000 || plan.files.len() > 10_000 {
        return Err(invalid("plan exceeds entry limits"));
    }
    for (id, receipt) in &plan.objects {
        plan::object_id(id)?;
        if receipt.object != *id
            || receipt.schema != 1
            || !plan::hex(&receipt.tree_sha256)
            || receipt.references.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid("invalid object receipt"));
        }
        for reference in &receipt.references {
            if !plan.objects.contains_key(reference) {
                return Err(invalid("missing runtime reference"));
            }
        }
        match (&receipt.derivation, &receipt.runtime_image) {
            (None, None) if *id == format!("src-{}", receipt.tree_sha256) => {}
            (Some(spec), Some(image)) if image == &plan.definition.foundation => {
                let mut aliases = Vec::new();
                for runtime in &spec.runtime_inputs {
                    aliases.push(
                        spec.inputs
                            .iter()
                            .find(|(_, id)| *id == runtime)
                            .map(|(alias, _)| alias.clone())
                            .ok_or_else(|| invalid("undeclared runtime input"))?,
                    );
                }
                let graph = BuildGraph {
                    schema: 1,
                    nodes: BTreeMap::from([(
                        "context".into(),
                        BuildNode {
                            builder_image: spec.builder_image.clone(),
                            runtime_image: spec.runtime_image.clone(),
                            inputs: spec
                                .inputs
                                .iter()
                                .map(|(k, v)| (k.clone(), Input::Object(v.clone())))
                                .collect(),
                            argv: spec.argv.clone(),
                            env: spec.env.clone(),
                            runtime_inputs: aliases,
                            timeout_seconds: spec.timeout_seconds,
                        },
                    )]),
                };
                let rebuilt = crate::plan(&graph, "context")?;
                if spec.runtime_image != *image
                    || rebuilt.outputs["context"] != *id
                    || serde_json::to_vec(&rebuilt.specs["context"])? != serde_json::to_vec(spec)?
                {
                    return Err(invalid("output derivation differs"));
                }
            }
            _ => return Err(invalid("object kind or runtime foundation differs")),
        }
    }
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    for root in plan.definition.outputs.values() {
        if plan
            .objects
            .get(root)
            .is_none_or(|r| r.runtime_image.as_ref() != Some(&plan.definition.foundation))
        {
            return Err(invalid("selected output missing or foundation differs"));
        }
        let mut pending = vec![(root.as_str(), false)];
        while let Some((id, exit)) = pending.pop() {
            if exit {
                active.remove(id);
                visited.insert(id);
                continue;
            }
            if visited.contains(id) {
                continue;
            }
            if !active.insert(id) {
                return Err(invalid("runtime reference cycle"));
            }
            pending.push((id, true));
            for child in plan.objects[id].references.iter().rev() {
                pending.push((child, false));
            }
        }
    }
    if visited.len() != plan.objects.len() {
        return Err(invalid("extra object outside selected runtime closure"));
    }
    Ok(())
}

#[derive(Default)]
struct ObjectTree {
    entries: Vec<Entry>,
    references: BTreeSet<String>,
    bytes: u64,
}
#[derive(Serialize)]
struct PayloadReceipt<'a> {
    schema: u32,
    identity: &'a str,
    foundation: &'a crate::FoundationObservation,
    objects: &'a BTreeMap<String, ObjectReceipt>,
}
struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn verify_payload(
    path: &Path,
    composition: &SystemComposition,
) -> Result<BTreeMap<String, ObjectTree>> {
    validate_tar_envelope(path)?;
    let plan = &composition.plan;
    let receipt = serde_json::to_vec(&PayloadReceipt {
        schema: 1,
        identity: &plan.identity,
        foundation: &plan.foundation,
        objects: &plan.objects,
    })?;
    let files: BTreeMap<_, _> = plan
        .files
        .iter()
        .filter(|f| f.disposition == SystemFileDisposition::Payload)
        .map(|f| (&f.path[1..], f))
        .collect();
    let mut expected_order: Vec<String> = files.keys().map(|p| (*p).to_owned()).collect();
    expected_order.push(RECEIPT.into());
    let mut input = tar::Archive::new(File::open(path)?);
    input.set_ignore_zeros(true);
    let mut scan = PayloadScan::new(&files, &receipt, &plan.objects);
    // Re-encoding every logical member with the exporter's exact headers also
    // checks GNU extension bytes, padding, metadata and the complete TAR ending.
    for entry in input.entries()? {
        scan.member(entry?)?;
    }
    let (trees, hash) = scan.finish()?;
    if hash != composition.artifacts["payload.tar"].sha256 {
        return Err(invalid("payload is not the canonical exported TAR"));
    }
    if trees.len() != plan.objects.len() {
        return Err(invalid("missing payload object root"));
    }
    for (id, tree) in &trees {
        verify_tree(id, tree, &plan.objects[id])?;
        expected_order.push(format!("{}/{id}", &LOGICAL_PREFIX[1..]));
        expected_order.extend(
            tree.entries
                .iter()
                .map(|entry| format!("{}/{id}/{}", &LOGICAL_PREFIX[1..], entry.path)),
        );
    }
    // A separate bounded header-only read compares export order without retaining
    // large member contents; object trees themselves retain only entry metadata.
    verify_member_order(path, expected_order)?;
    Ok(trees)
}

#[derive(Clone, Copy)]
struct MemberHeader {
    mode: u32,
    kind: tar::EntryType,
    size: u64,
}
#[derive(Default)]
struct MetadataBudget(usize);
impl MetadataBudget {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        self.0 = self.0.saturating_add(bytes);
        if self.0 > MAX_METADATA {
            return Err(invalid("payload metadata exceeds 64 MiB"));
        }
        Ok(())
    }
}
/// Verifies payload members in archive order while re-encoding them canonically
/// and collecting each declared object's tree.
struct PayloadScan<'a> {
    files: &'a BTreeMap<&'a str, &'a ResolvedSystemFile>,
    receipt: &'a [u8],
    objects: &'a BTreeMap<String, ObjectReceipt>,
    names: BTreeSet<String>,
    trees: BTreeMap<String, ObjectTree>,
    metadata: MetadataBudget,
    canonical: tar::Builder<HashWriter>,
}
impl<'a> PayloadScan<'a> {
    fn new(
        files: &'a BTreeMap<&'a str, &'a ResolvedSystemFile>,
        receipt: &'a [u8],
        objects: &'a BTreeMap<String, ObjectReceipt>,
    ) -> Self {
        Self {
            files,
            receipt,
            objects,
            names: BTreeSet::new(),
            trees: BTreeMap::new(),
            metadata: MetadataBudget::default(),
            canonical: tar::Builder::new(HashWriter(Sha256::new())),
        }
    }
    fn member(&mut self, mut entry: tar::Entry<'_, File>) -> Result<()> {
        let name = std::str::from_utf8(&entry.path_bytes())
            .map_err(|_| invalid("non-UTF8 payload path"))?
            .to_owned();
        plan::relative(&name, false)?;
        self.metadata.charge(name.len() + 256)?;
        if self.names.len() >= 1_000_000 || !self.names.insert(name.clone()) {
            return Err(invalid("duplicate or excessive payload members"));
        }
        let header = MemberHeader {
            mode: entry.header().mode()?,
            kind: entry.header().entry_type(),
            size: entry.size(),
        };
        if entry.header().uid()? != 0 || entry.header().gid()? != 0 || entry.header().mtime()? != 0
        {
            return Err(invalid("noncanonical payload ownership/time"));
        }
        if let Some(file) = self.files.get(name.as_str()) {
            verify_regular(
                &mut entry,
                &mut self.canonical,
                &name,
                file.mode,
                &file.bytes,
            )
        } else if name == RECEIPT {
            verify_regular(&mut entry, &mut self.canonical, &name, 0o644, self.receipt)
        } else {
            self.object_member(&mut entry, &name, header)
        }
    }
    fn object_member(
        &mut self,
        entry: &mut tar::Entry<'_, File>,
        name: &str,
        header: MemberHeader,
    ) -> Result<()> {
        let suffix = name
            .strip_prefix(&format!("{}/", &LOGICAL_PREFIX[1..]))
            .ok_or_else(|| invalid("unexpected payload path"))?;
        let (id, relative) = suffix.split_once('/').unwrap_or((suffix, ""));
        if !self.objects.contains_key(id) {
            return Err(invalid("undeclared payload object"));
        }
        if relative.is_empty() {
            self.object_root(id, name, header)
        } else {
            self.object_entry(entry, name, id, relative, header)
        }
    }
    fn object_root(&mut self, id: &str, name: &str, header: MemberHeader) -> Result<()> {
        if !header.kind.is_dir()
            || header.size != 0
            || header.mode != 0o555
            || self.trees.contains_key(id)
        {
            return Err(invalid("invalid object root"));
        }
        self.trees.insert(id.to_owned(), ObjectTree::default());
        self.canonical.append_data(
            &mut system::header(0o555, 0, tar::EntryType::Directory),
            name,
            std::io::empty(),
        )?;
        Ok(())
    }
    fn object_entry(
        &mut self,
        entry: &mut tar::Entry<'_, File>,
        name: &str,
        id: &str,
        relative: &str,
        header: MemberHeader,
    ) -> Result<()> {
        plan::relative(relative, false)?;
        let tree = self
            .trees
            .get_mut(id)
            .ok_or_else(|| invalid("object entry precedes root"))?;
        if tree.entries.len() >= 100_000 {
            return Err(invalid("object exceeds 100000 entries"));
        }
        tree::scan(relative.as_bytes(), &mut tree.references)?;
        let MemberHeader { mode, kind, size } = header;
        let value = if kind.is_file() && matches!(mode, 0o444 | 0o555) {
            reencode_object_file(entry, &mut self.canonical, tree, name, header)?
        } else if kind.is_dir() && mode == 0o555 && size == 0 {
            self.canonical.append_data(
                &mut system::header(mode, 0, tar::EntryType::Directory),
                name,
                std::io::empty(),
            )?;
            Kind::Directory
        } else if kind.is_symlink() && mode == 0o777 && size == 0 {
            let target = read_link_target(entry, &mut self.metadata, relative)?;
            tree::scan(target.as_bytes(), &mut tree.references)?;
            self.canonical.append_link(
                &mut system::header(mode, 0, tar::EntryType::Symlink),
                name,
                &target,
            )?;
            Kind::Symlink { target }
        } else {
            return Err(invalid("unsupported payload entry type/mode"));
        };
        tree.entries.push(Entry {
            path: relative.to_owned(),
            kind: value,
        });
        Ok(())
    }
    fn finish(self) -> Result<(BTreeMap<String, ObjectTree>, String)> {
        let mut canonical = self.canonical;
        canonical.finish()?;
        let hash = plan::encode_hex(&canonical.into_inner()?.0.finalize());
        Ok((self.trees, hash))
    }
}
fn reencode_object_file(
    entry: &mut tar::Entry<'_, File>,
    canonical: &mut tar::Builder<HashWriter>,
    tree: &mut ObjectTree,
    name: &str,
    header: MemberHeader,
) -> Result<Kind> {
    let MemberHeader { mode, size, .. } = header;
    tree.bytes = tree
        .bytes
        .checked_add(size)
        .ok_or_else(|| invalid("object size overflow"))?;
    if size > tree::MAX_FILE || tree.bytes > tree::MAX_TREE {
        return Err(invalid("object exceeds file/tree size limit"));
    }
    let mut reader = ScannedReader::new(entry);
    canonical.append_data(
        &mut system::header(mode, size, tar::EntryType::Regular),
        name,
        &mut reader,
    )?;
    let (sha256, references, bytes) = reader.finish()?;
    if bytes != size {
        return Err(invalid("truncated object file"));
    }
    tree.references.extend(references);
    if tree.references.len() > 100_000 {
        return Err(invalid("object reference limit exceeded"));
    }
    Ok(Kind::File {
        executable: mode == 0o555,
        bytes: size,
        sha256,
    })
}
fn read_link_target(
    entry: &tar::Entry<'_, File>,
    metadata: &mut MetadataBudget,
    relative: &str,
) -> Result<String> {
    let target = entry
        .link_name_bytes()
        .ok_or_else(|| invalid("symlink target missing"))?;
    let target = std::str::from_utf8(&target)
        .map_err(|_| invalid("non-UTF8 link target"))?
        .to_owned();
    metadata.charge(target.len())?;
    tree::link_target(relative, &target)?;
    Ok(target)
}
fn verify_member_order(path: &Path, expected_order: Vec<String>) -> Result<()> {
    let mut archive = tar::Archive::new(File::open(path)?);
    let mut entries = archive.entries_with_seek()?;
    for expected in expected_order {
        let actual = entries
            .next()
            .ok_or_else(|| invalid("missing expected payload member"))??;
        if actual.path_bytes().as_ref() != expected.as_bytes() {
            return Err(invalid("payload member order differs"));
        }
    }
    if entries.next().is_some() {
        return Err(invalid("extra payload member"));
    }
    Ok(())
}

fn validate_tar_envelope(path: &Path) -> Result<()> {
    let file = File::open(path)?;
    let length = file.metadata()?.len();
    let mut archive = tar::Archive::new(file);
    archive.set_ignore_zeros(true);
    for (count, entry) in archive.entries_with_seek()?.raw(true).enumerate() {
        let entry = entry?;
        let kind = entry.header().entry_type();
        let bound = if kind.is_gnu_longname() || kind.is_gnu_longlink() {
            4097
        } else if kind.is_file() {
            tree::MAX_FILE
        } else if kind.is_dir() || kind.is_symlink() {
            0
        } else {
            return Err(invalid("unsupported raw TAR extension or member"));
        };
        if count >= 3_000_000
            || entry.size() > bound
            || entry
                .raw_file_position()
                .checked_add(entry.size())
                .is_none_or(|end| end > length)
        {
            return Err(invalid("TAR member exceeds bounds"));
        }
    }
    Ok(())
}
fn verify_regular<R: Read>(
    entry: &mut tar::Entry<'_, R>,
    canonical: &mut tar::Builder<HashWriter>,
    name: &str,
    mode: u32,
    bytes: &[u8],
) -> Result<()> {
    if !entry.header().entry_type().is_file()
        || entry.header().mode()? != mode
        || entry.size() != bytes.len() as u64
    {
        return Err(invalid("configuration entry type/mode/size differs"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = entry.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if plan::encode_hex(&hash.finalize()) != plan::hash(bytes) {
        return Err(invalid("configuration or embedded receipt content differs"));
    }
    canonical.append_data(
        &mut system::header(mode, bytes.len() as u64, tar::EntryType::Regular),
        name,
        bytes,
    )?;
    Ok(())
}
fn verify_tree(id: &str, tree: &ObjectTree, receipt: &ObjectReceipt) -> Result<()> {
    let mut sorted: Vec<_> = tree.entries.iter().collect();
    sorted.sort_by(|a, b| a.path.split('/').cmp(b.path.split('/')));
    if sorted
        .iter()
        .zip(&tree.entries)
        .any(|(a, b)| a.path != b.path)
    {
        return Err(invalid("noncanonical object tree order"));
    }
    let paths: BTreeMap<_, _> = tree
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), &entry.kind))
        .collect();
    for entry in &tree.entries {
        let mut parent = Path::new(&entry.path).parent();
        while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
            if !matches!(
                path.to_str().and_then(|p| paths.get(p)),
                Some(Kind::Directory)
            ) {
                return Err(invalid("object parent missing or not a directory"));
            }
            parent = path.parent();
        }
    }
    tree::validate_links(&tree.entries)?;
    let mut bytes = b"sysroot-engine-tree-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(&tree.entries)?);
    if plan::hash(&bytes) != receipt.tree_sha256 {
        return Err(invalid("object tree digest differs"));
    }
    let mut references = tree.references.clone();
    if let Some(spec) = &receipt.derivation {
        references.extend(spec.runtime_inputs.iter().cloned());
        references.remove(id);
    }
    if references.into_iter().collect::<Vec<_>>() != receipt.references {
        return Err(invalid("object runtime references differ"));
    }
    Ok(())
}

struct ScannedReader<R> {
    input: R,
    hash: Sha256,
    references: BTreeSet<String>,
    bytes: u64,
    prefix: Vec<u8>,
    matched: usize,
    token: Option<Vec<u8>>,
}
impl<R: Read> ScannedReader<R> {
    fn new(input: R) -> Self {
        Self {
            input,
            hash: Sha256::new(),
            references: BTreeSet::new(),
            bytes: 0,
            prefix: format!("{LOGICAL_PREFIX}/").into_bytes(),
            matched: 0,
            token: None,
        }
    }
    fn finish_token(&mut self) -> Result<()> {
        if let Some(token) = self.token.take() {
            let id = std::str::from_utf8(&token).map_err(|_| invalid("invalid store reference"))?;
            plan::object_id(id)?;
            self.references.insert(id.into());
            if self.references.len() > 100_000 {
                return Err(invalid("file reference limit exceeded"));
            }
        }
        Ok(())
    }
    fn scan(&mut self, bytes: &[u8]) -> Result<()> {
        for &byte in bytes {
            if let Some(token) = &mut self.token {
                if byte.is_ascii_alphanumeric() || byte == b'-' {
                    if token.len() >= 68 {
                        return Err(invalid("oversized store reference"));
                    }
                    token.push(byte);
                    continue;
                }
                self.finish_token()?;
            }
            if byte == self.prefix[self.matched] {
                self.matched += 1;
                if self.matched == self.prefix.len() {
                    self.matched = 0;
                    self.token = Some(Vec::new());
                }
            } else {
                self.matched = usize::from(byte == self.prefix[0]);
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<(String, BTreeSet<String>, u64)> {
        self.finish_token()?;
        Ok((
            plan::encode_hex(&self.hash.finalize()),
            self.references,
            self.bytes,
        ))
    }
}
impl<R: Read> Read for ScannedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.input.read(buffer)?;
        self.scan(&buffer[..count]).map_err(std::io::Error::other)?;
        self.hash.update(&buffer[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
}

fn verify_files(plan: &SystemPlan, trees: &BTreeMap<String, ObjectTree>) -> Result<()> {
    let mut selected: BTreeMap<&str, &SystemFile> = BTreeMap::new();
    for file in &plan.definition.files {
        match selected.get(file.path.as_str()) {
            Some(previous)
                if file.priority > previous.priority
                    && file.replaces.as_deref() == Some(previous.provenance.as_str())
                    && file.provenance != previous.provenance => {}
            None if file.replaces.is_none() => {}
            _ => return Err(invalid("ambiguous file contribution")),
        }
        selected.insert(&file.path, file);
    }
    if selected.len() != plan.files.len() {
        return Err(invalid("resolved file count differs"));
    }
    for ((path, source), resolved) in selected.iter().zip(&plan.files) {
        let mut parent = Path::new(path).parent();
        while let Some(p) = parent {
            if p.to_str().is_some_and(|p| selected.contains_key(p)) {
                return Err(invalid("configuration prefix collision"));
            }
            parent = p.parent();
        }
        let bytes = match &source.content {
            SystemContent::Bytes(bytes) => bytes.clone(),
            SystemContent::Template(argument) => render(argument, plan, trees)?.into_bytes(),
        };
        let mut references = BTreeSet::new();
        tree::scan(&bytes, &mut references)?;
        if references.iter().any(|id| !plan.objects.contains_key(id)) {
            return Err(invalid("undeclared configuration store reference"));
        }
        let disposition = if system::overlay_path(path) {
            SystemFileDisposition::Payload
        } else {
            SystemFileDisposition::Foundation
        };
        if resolved.path != *path
            || resolved.mode != source.mode
            || resolved.provenance != source.provenance
            || resolved.bytes != bytes
            || resolved.sha256 != plan::hash(&bytes)
            || resolved.disposition != disposition
        {
            return Err(invalid("resolved configuration differs from definition"));
        }
    }
    Ok(())
}
fn render(
    argument: &Argument,
    plan: &SystemPlan,
    trees: &BTreeMap<String, ObjectTree>,
) -> Result<String> {
    let mut result = String::new();
    for segment in &argument.0 {
        match segment {
            Segment::Literal { value } => result.push_str(value),
            Segment::Input { name, path } => {
                let id = plan
                    .definition
                    .outputs
                    .get(name)
                    .ok_or_else(|| invalid("unknown template output"))?;
                let tree = trees
                    .get(id)
                    .ok_or_else(|| invalid("template object missing"))?;
                if !path.is_empty() {
                    if !tree.entries.iter().any(|entry| entry.path == *path) {
                        return Err(invalid("typed template output path missing"));
                    }
                    let mut parent = Path::new(path).parent();
                    while let Some(p) = parent {
                        if tree.entries.iter().any(|entry| {
                            Path::new(&entry.path) == p && !matches!(entry.kind, Kind::Directory)
                        }) {
                            return Err(invalid("template traverses a symlink"));
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
            Segment::Output { .. } => return Err(invalid("mutable system template output")),
        }
        if result.len() as u64 > MAX_JSON {
            return Err(invalid("template exceeds 8 MiB"));
        }
    }
    Ok(result)
}
