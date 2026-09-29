//! Screenshot receipts describe the image and home state that was actually captured.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::docker::Docker;
use crate::environment::Environment;
use crate::session::Session;
use crate::{Result, builder, invalid};

pub fn destination(label: &str) -> Result<PathBuf> {
    if label.is_empty()
        || label.len() > 64
        || !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid(
            "screenshot label must contain 1–64 letters, digits, '-' or '_'",
        ));
    }
    Ok(crate::artifact_root()
        .join("shots")
        .join(format!("{label}-{}.png", crate::execution_id())))
}

pub fn container_receipt(
    docker: &Docker,
    environment: &Environment,
    session: &Session,
    path: &Path,
    elapsed: Duration,
) -> Result<PathBuf> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err(invalid("capture did not produce a PNG with an IHDR"));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| invalid("PNG width"))?);
    let height = u32::from_be_bytes(
        bytes[20..24]
            .try_into()
            .map_err(|_| invalid("PNG height"))?,
    );
    if width == 0 || height == 0 {
        return Err(invalid("capture has an empty display"));
    }
    let installed: serde_json::Value = serde_json::from_slice(
        &docker.read_file(&environment.id, "/usr/share/sysroot/source.json")?,
    )
    .map_err(|e| invalid(format!("capture source: {e}")))?;
    let target = installed["target"]["id"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid("capture image has no target identity"))?;
    let source = installed["source_revision"]
        .as_str()
        .filter(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| invalid("capture image has no source revision"))?;
    let receipt_path = format!(
        "{}/.local/state/kedra-lab/sync/receipt.json",
        session.user.home
    );
    let present = environment.exec(docker, &session.exec(["test", "-e", &receipt_path]))?;
    let synced: Option<serde_json::Value> = if present.exit == 0 {
        Some(
            serde_json::from_slice(&docker.read_file(&environment.id, &receipt_path)?)
                .map_err(|e| invalid(format!("capture home receipt: {e}")))?,
        )
    } else if present.exit == 1 {
        None
    } else {
        return Err(invalid(
            "could not inspect the captured home's source receipt",
        ));
    };
    let home_source = synced
        .as_ref()
        .map(|receipt| {
            if receipt["schema_version"] != 1 {
                return Err(invalid("unknown home sync receipt"));
            }
            receipt["source"]["source_revision"]
                .as_str()
                .filter(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(|| invalid("home sync receipt has no source revision"))
        })
        .transpose()?;
    let value = serde_json::json!({
        "schema_version": 1,
        "captured_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),
        "backend": "container",
        "environment": environment.name,
        "container_id": environment.id,
        "image": environment.image,
        "target": target,
        "installed_source": source,
        "home_source": home_source,
        "renderer_mode": "software (nested container profile)",
        "gpu_qualified": false,
        "display": {"width": width, "height": height, "scale": session.display.scale},
        "capture_ms": elapsed.as_millis(),
        "timing_scope": "compositor capture and PNG transfer; excludes metadata collection",
        "png_sha256": builder::file_sha256(path)?,
    });
    let receipt = path.with_extension("json");
    std::fs::write(
        &receipt,
        serde_json::to_vec_pretty(&value).map_err(|e| invalid(e.to_string()))?,
    )?;
    Ok(receipt)
}
