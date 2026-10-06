//! Pure identity for declared ordinary source files; storage uses the same records.
use crate::{
    Error, Result,
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
    let mut entries = Vec::new();
    visit("", files, &directories, &mut entries)?;
    let mut bytes = b"sysroot-engine-tree-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(&entries)?);
    Ok(format!("src-{}", hash(&bytes)))
}

fn visit(
    prefix: &str,
    files: &BTreeMap<String, SourceFile>,
    directories: &BTreeSet<String>,
    entries: &mut Vec<Entry>,
) -> Result<()> {
    let mut names = BTreeSet::new();
    for path in files.keys().chain(directories) {
        if let Some(rest) = path.strip_prefix(prefix)
            && !rest.is_empty()
            && !rest.contains('/')
        {
            names.insert(rest.to_owned());
        }
    }
    for name in names {
        let path = format!("{prefix}{name}");
        if directories.contains(&path) {
            if path.split('/').count() > 64 {
                return Err(Error::Invalid("resource directory depth exceeded".into()));
            }
            entries.push(Entry {
                path: path.clone(),
                kind: Kind::Directory,
            });
            visit(&format!("{path}/"), files, directories, entries)?;
        } else {
            let file = &files[&path];
            entries.push(Entry {
                path,
                kind: Kind::File {
                    executable: file.executable,
                    bytes: file.bytes.len() as u64,
                    sha256: hash(&file.bytes),
                },
            });
        }
    }
    Ok(())
}
