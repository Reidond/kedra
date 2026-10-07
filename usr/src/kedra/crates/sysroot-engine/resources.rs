//! Pure identity for declared ordinary source files; storage uses the same records.
use crate::{
    Error, MAX_JSON, Result,
    plan::{hash, relative},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub executable: bool,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub path: String,
    pub kind: Kind,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Kind {
    Directory,
    File {
        executable: bool,
        bytes: u64,
        sha256: String,
    },
    Symlink {
        target: String,
    },
}

pub fn source_identity(files: &BTreeMap<String, SourceFile>) -> Result<String> {
    if files.is_empty()
        || files.len() > 4096
        || files.values().map(|f| f.bytes.len()).sum::<usize>() > 32 * 1024 * 1024
    {
        return Err(Error::Invalid(
            "declared resource count/bytes exceeded".into(),
        ));
    }
    let mut directories = BTreeSet::new();
    let mut folded = BTreeSet::new();
    for path in files.keys() {
        relative(path, false)?;
        if !path.is_ascii() || path.contains('\\') || !folded.insert(path.to_ascii_lowercase()) {
            return Err(Error::Invalid(
                "unsafe or case-colliding resource path".into(),
            ));
        }
        for (i, _) in path.match_indices('/') {
            let directory = &path[..i];
            if files.contains_key(directory) {
                return Err(Error::Invalid("resource file/directory collision".into()));
            }
            directories.insert(directory.to_owned());
        }
    }
    if files.len() + directories.len() > 100_000
        || files
            .keys()
            .chain(&directories)
            .map(String::len)
            .sum::<usize>()
            > MAX_JSON as usize
    {
        return Err(Error::Invalid(
            "declared resource tree metadata exceeds bound".into(),
        ));
    }
    let mut case_paths = BTreeMap::new();
    for path in files.keys().chain(&directories) {
        if case_paths
            .insert(path.to_ascii_lowercase(), path)
            .is_some_and(|previous| previous != path)
        {
            return Err(Error::Invalid("resource directory case collision".into()));
        }
        if path.split('/').count() > 64 {
            return Err(Error::Invalid("resource directory depth exceeded".into()));
        }
    }
    let mut entries: Vec<_> = directories
        .iter()
        .map(|path| Entry {
            path: path.clone(),
            kind: Kind::Directory,
        })
        .chain(files.iter().map(|(path, file)| Entry {
            path: path.clone(),
            kind: Kind::File {
                executable: file.executable,
                bytes: file.bytes.len() as u64,
                sha256: hash(&file.bytes),
            },
        }))
        .collect();
    entries.sort_by(|left, right| left.path.split('/').cmp(right.path.split('/')));
    Ok(format!("src-{}", tree_digest(&entries)?))
}

pub(crate) fn tree_digest(entries: &[Entry]) -> Result<String> {
    let mut bytes = b"sysroot-engine-tree-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(entries)?);
    Ok(hash(&bytes))
}
