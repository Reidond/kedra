//! Versioned package input selection over one already frozen Git tree.
use crate::source::{Entry, Error, blob, git};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use sysroot_catalog::language::{self, Lockfile, TargetPolicy};

const IMAGE: &str = "usr/src/kedra/image/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    schema_version: u32,
    format: String,
    entry: String,
    lock: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Frontend {
    pub schema_version: u32,
    pub format: String,
    pub entry: String,
    pub intent_sha256: String,
    pub inputs: BTreeMap<String, String>,
    pub builders: BTreeMap<String, BTreeSet<String>>,
    pub selected_packages: BTreeSet<String>,
}

pub(crate) struct Selection {
    pub packages: BTreeSet<String>,
    pub remove: BTreeSet<String>,
    pub frontend: Frontend,
}

fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn read(repo: &Path, tree: &[Entry], path: &str) -> Result<Vec<u8>, Error> {
    let entry = tree
        .iter()
        .find(|e| e.path == path)
        .ok_or_else(|| invalid("missing committed package input"))?;
    let size = git(repo, &["cat-file", "-s", &entry.blob])?;
    let size = std::str::from_utf8(&size)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .ok_or_else(|| invalid("invalid committed input size"))?;
    if size > language::MAX_INPUT {
        return Err(invalid("committed frontend input exceeds byte limit"));
    }
    let bytes = blob(repo, entry)?;
    if bytes.len() != size || !language::public_input(path, &bytes) {
        return Err(invalid("oversized or private package input"));
    }
    Ok(bytes)
}

pub(crate) fn select(
    repo: &Path,
    tree: &[Entry],
    host: &str,
    legacy: bool,
) -> Result<Option<Selection>, Error> {
    let descriptor_path = format!("{IMAGE}package-inputs.json");
    let selected = tree.iter().any(|e| e.path == descriptor_path);
    let has_language = tree
        .iter()
        .any(|e| e.path.starts_with(&format!("{IMAGE}packages/")) && e.path.ends_with(".kedra"));
    if !selected {
        if has_language {
            return Err(invalid(
                "new-language source lacks explicit package format descriptor",
            ));
        }
        return Ok(None);
    }
    if legacy
        || tree.iter().any(|e| {
            e.path == format!("{IMAGE}packages.list")
                || e.path == format!("{IMAGE}remove.list")
                || e.path.starts_with(&format!("{IMAGE}targets/"))
                    && e.path.ends_with("/packages.list")
        })
    {
        return Err(invalid(
            "source mixes authoritative language and package lists",
        ));
    }
    let descriptor_bytes = read(repo, tree, &descriptor_path)?;
    let descriptor: Descriptor = serde_json::from_slice(&descriptor_bytes)
        .map_err(|_| invalid("invalid package format descriptor"))?;
    if descriptor.schema_version != 1
        || descriptor.format != "kedra"
        || descriptor.entry != "packages/catalog.kedra"
        || descriptor.lock != "packages/packages.lock.json"
    {
        return Err(invalid(
            "unsupported package format/version/entry descriptor",
        ));
    }
    let root = format!("{IMAGE}packages/");
    let policy_path = format!("{IMAGE}package-policy.json");
    let policy_bytes = read(repo, tree, &policy_path)?;
    let policy: TargetPolicy = serde_json::from_slice(&policy_bytes)
        .map_err(|_| invalid("invalid independent package policy"))?;
    // The selected source cannot relax existing boot/session requirements.
    let required: BTreeSet<String> = [
        "niri",
        "noctalia",
        "greetd",
        "tuigreet",
        "NetworkManager",
        "pipewire",
        "wireplumber",
        "polkit",
        "gnome-keyring",
        "gnome-keyring-pam",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    if policy.namespace != "kedra"
        || policy.targets != BTreeSet::from(["desktop".into(), "qemu-arm64".into()])
        || !required.is_subset(&policy.required_packages)
    {
        return Err(invalid(
            "package policy relaxes the independent Kedra target/base contract",
        ));
    }
    let lock_path = format!("{IMAGE}{}", descriptor.lock);
    let lock_bytes = read(repo, tree, &lock_path)?;
    let lock: Lockfile =
        serde_json::from_slice(&lock_bytes).map_err(|_| invalid("invalid source pin lockfile"))?;
    let mut inputs = BTreeMap::from([
        (descriptor_path, hash(&descriptor_bytes)),
        (policy_path, hash(&policy_bytes)),
        (lock_path, hash(&lock_bytes)),
    ]);
    let entry = descriptor
        .entry
        .strip_prefix("packages/")
        .ok_or_else(|| invalid("entry outside package root"))?;
    let mut pending = vec![entry.to_owned()];
    let mut modules = BTreeMap::new();
    let mut total = descriptor_bytes.len() + policy_bytes.len() + lock_bytes.len();
    while let Some(path) = pending.pop() {
        if modules.contains_key(&path) {
            continue;
        }
        if modules.len() >= language::MAX_MODULES || !language::relative(&path) {
            return Err(invalid("package module count/path refused"));
        }
        let full = format!("{root}{path}");
        let bytes = read(repo, tree, &full)?;
        total += bytes.len();
        if total > language::MAX_TOTAL {
            return Err(invalid("committed frontend input size exceeded"));
        }
        let text = String::from_utf8(bytes).map_err(|_| invalid("package module is not UTF-8"))?;
        let module = language::parse(&path, &text).map_err(|e| Error::Invalid(e.to_string()))?;
        for import in &module.imports {
            pending.push(
                language::import_path(&path, &import.path)
                    .ok_or_else(|| invalid("unsafe committed import"))?,
            );
        }
        inputs.insert(full, hash(text.as_bytes()));
        modules.insert(path, text);
    }
    let intent = language::compile(&modules, entry, host, lock, &policy)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    if host == "desktop" && !intent.selected.is_empty() {
        return Err(invalid(
            "native x86 source package realization is not supported",
        ));
    }
    for recipe in intent.recipes.values() {
        let mut contents = vec![&recipe.build];
        if let language::SourceIntent::Files(files) = &recipe.source {
            contents.extend(files.values());
        }
        contents.extend(recipe.files.values());
        contents.extend(recipe.replace_files.values());
        contents.extend(recipe.patches.iter().map(|p| &p.content));
        for content in contents {
            if let language::Content::File { path, .. } = content {
                let full = format!("{root}{path}");
                if inputs.contains_key(&full) {
                    continue;
                }
                let bytes = read(repo, tree, &full)?;
                total += bytes.len();
                if total > language::MAX_TOTAL {
                    return Err(invalid("committed resource bytes exceeded"));
                }
                inputs.insert(full, hash(&bytes));
            }
        }
    }
    if !intent.remove.is_disjoint(&BTreeSet::from([
        "bootc".into(),
        "glibc".into(),
        "rpm".into(),
        "dnf5".into(),
        "kernel".into(),
        "shim".into(),
        "grub2".into(),
    ])) {
        return Err(invalid(
            "package removal violates the bootc foundation contract",
        ));
    }
    let bytes = serde_json::to_vec(
        &serde_json::to_value(&intent).map_err(|_| invalid("cannot encode intent"))?,
    )
    .map_err(|_| invalid("cannot encode intent"))?;
    Ok(Some(Selection {
        packages: intent.packages.clone(),
        remove: intent.remove.clone(),
        frontend: Frontend {
            schema_version: 1,
            format: "kedra".into(),
            entry: descriptor.entry,
            intent_sha256: hash(&bytes),
            inputs,
            builders: intent.builders.clone(),
            selected_packages: intent
                .selected
                .iter()
                .map(|key| intent.recipes[key].name.clone())
                .collect(),
        },
    }))
}
