//! Composition-owned public baseline identity, never installed-source authority.
use serde::{Deserialize, Serialize};
use sysroot_engine::{
    Error, ObjectReceipt, Result, Store, single_file_source_receipt, verify_single_file_source,
};

pub const NIRI_DESTINATION: &str = "usr/share/sysroot/home/default/.config/niri/config.kdl";
pub const RECORD_PATH: &str = "/usr/share/sysroot/home-artifacts.json";
pub const RECORD_LIMIT: usize = 4096;
pub const NIRI_LIMIT: usize = 131_072;
const MEMBER: &str = "config.kdl";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    niri: ObjectReceipt,
}

pub fn niri_content(bytes: &[u8]) -> Result<&str> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::Invalid("managed text must be UTF-8".into()))?;
    if bytes.len() > NIRI_LIMIT
        || text.lines().count() > 8192
        || text.contains(['\0', '\r'])
        || (!text.is_empty() && !text.ends_with('\n'))
    {
        return Err(Error::Invalid(
            "managed text must be bounded LF text ending with a newline".into(),
        ));
    }
    Ok(text)
}

pub fn produce(bytes: &[u8], store: Option<&Store>) -> Result<(Vec<u8>, Vec<u8>)> {
    niri_content(bytes)?;
    let expected = single_file_source_receipt(MEMBER, bytes)?;
    let (receipt, stored) = match store {
        Some(store) => store.import_source_file(MEMBER, bytes)?,
        None => (expected, bytes.to_vec()),
    };
    verify_single_file_source(&receipt, MEMBER, bytes)?;
    let mut record = serde_json::to_vec(&Record {
        schema: 1,
        niri: receipt,
    })?;
    record.push(b'\n');
    if record.len() > RECORD_LIMIT {
        return Err(Error::Invalid(
            "home artifact record exceeds 4096 bytes".into(),
        ));
    }
    verify(&record, &stored)?;
    Ok((record, stored))
}

pub fn verify(record: &[u8], bytes: &[u8]) -> Result<()> {
    if record.len() > RECORD_LIMIT {
        return Err(Error::Invalid(
            "home artifact record exceeds 4096 bytes".into(),
        ));
    }
    let record: Record = serde_json::from_slice(record)?;
    if record.schema != 1 {
        return Err(Error::Invalid(
            "unsupported home artifact record schema".into(),
        ));
    }
    verify_single_file_source(&record.niri, MEMBER, bytes)
}
