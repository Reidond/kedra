//! The producer and installed helper consume one reviewed bootc version contract.
use serde::Deserialize;

const CONTRACT: &str = include_str!("../../build/release/compatibility.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_version: u32,
    bootc_version: String,
}

/// Exact native version output accepted by this compiled helper.
/// Malformed/unknown contracts fail closed; no runtime file or environment override.
pub fn bootc_output() -> Result<String, &'static str> {
    if CONTRACT.len() > 4096 {
        return Err("embedded compatibility contract exceeds its bound");
    }
    let contract: Contract =
        serde_json::from_str(CONTRACT).map_err(|_| "invalid embedded compatibility contract")?;
    let parts: Vec<_> = contract.bootc_version.split('.').collect();
    if contract.schema_version != 1
        || parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || part.len() > 10
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || part.len() > 1 && part.starts_with('0')
                || part.parse::<u32>().is_err()
        })
    {
        return Err("unsupported embedded bootc compatibility contract");
    }
    Ok(format!("bootc {}\n", contract.bootc_version))
}
