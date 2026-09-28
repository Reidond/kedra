//! Home-only source payloads for the container and native VM lab transports.

use std::collections::BTreeMap;
use std::io::Read;

use base64::Engine;
use serde::Serialize;

use crate::{Result, artifact_root, builder, invalid};

#[derive(Serialize)]
struct File {
    content: String,
    mode: u32,
}

/// The guest validates this against its installed source before changing any file.
#[derive(Serialize)]
struct Request {
    schema_version: u32,
    source: serde_json::Value,
    files: BTreeMap<String, File>,
}

pub fn request(target: &str) -> Result<Vec<u8>> {
    let archive = builder::archive_worktree(target, &artifact_root().join("cache"))?;
    let mut tar = tar::Archive::new(std::fs::File::open(&archive.payload)?);
    let mut source = None;
    let mut files = BTreeMap::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        if !entry.header().entry_type().is_file() {
            return Err(invalid("source payload contains a non-regular entry"));
        }
        let home = path.strip_prefix("usr/share/sysroot/home/default/");
        if path != "usr/share/sysroot/source.json"
            && !home.is_some_and(|p| {
                p.starts_with(".config/niri/") || p.starts_with(".config/noctalia/")
            })
        {
            continue;
        }
        if entry.size() > 16 * 1024 * 1024 {
            return Err(invalid(format!("lab sync input is too large: {path}")));
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        if let Some(relative) = home {
            files.insert(
                relative.to_owned(),
                File {
                    content: base64::prelude::BASE64_STANDARD.encode(bytes),
                    mode: entry.header().mode()? & 0o777,
                },
            );
        } else {
            source = Some(
                serde_json::from_slice(&bytes)
                    .map_err(|e| invalid(format!("source manifest: {e}")))?,
            );
        }
    }
    let source = source.ok_or_else(|| invalid("source payload has no manifest"))?;
    if files.is_empty() {
        return Err(invalid(
            "source payload has no supported desktop configuration",
        ));
    }
    serde_json::to_vec(&Request {
        schema_version: 1,
        source,
        files,
    })
    .map_err(|e| invalid(format!("lab sync request: {e}")))
}
