//! The producer and installed helper share one minimum bootc version.
use serde::Deserialize;

const CONTRACT: &str = include_str!("../../image/release/compatibility.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_version: u32,
    minimum_bootc_version: String,
}

/// A plain `MAJOR.MINOR.PATCH` of decimal parts without leading zeros.
fn version(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split('.').map(|part| {
        let decimal = !part.is_empty()
            && part.len() <= 10
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && !(part.len() > 1 && part.starts_with('0'));
        decimal.then(|| part.parse::<u32>().ok()).flatten()
    });
    let triplet = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(triplet)
}

/// Accept `bootc --version` output at or above the embedded minimum.
/// Malformed contracts or output fail closed; no runtime file or environment override.
pub fn require_bootc(output: &[u8]) -> Result<(), String> {
    if CONTRACT.len() > 4096 {
        return Err("embedded compatibility contract exceeds its bound".into());
    }
    let contract: Contract = serde_json::from_str(CONTRACT)
        .map_err(|_| "invalid embedded compatibility contract".to_owned())?;
    let minimum = version(&contract.minimum_bootc_version)
        .filter(|_| contract.schema_version == 2)
        .ok_or("unsupported embedded bootc compatibility contract")?;
    let installed = std::str::from_utf8(output)
        .ok()
        .and_then(|text| text.strip_prefix("bootc "))
        .and_then(|text| text.strip_suffix('\n'))
        .and_then(version)
        .ok_or("unrecognized bootc version output")?;
    if installed < minimum {
        return Err(format!(
            "installed bootc {}.{}.{} is older than the required {}",
            installed.0, installed.1, installed.2, contract.minimum_bootc_version
        ));
    }
    Ok(())
}
