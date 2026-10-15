//! Bounded requests shared by the ordinary CLI and installed helper.
use serde::{Deserialize, Serialize};

pub const MAX_REQUEST: usize = 524_288;
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    ChannelCheck {},
    ChannelStatus {},
    ChannelEnroll {
        expected_digest: Option<String>,
    },
    ChannelStage {
        expected_digest: Option<String>,
        replace_staged: Option<String>,
        resume: bool,
    },
    ChannelRollback {
        replace_staged: Option<String>,
    },
    /// Media-only precheck of the fixed embedded payload; accepts no caller paths.
    VerifyInstaller {},
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema_version: u32,
    pub request: Request,
}
pub fn decode(bytes: &[u8]) -> Result<Envelope, &'static str> {
    if bytes.len() > MAX_REQUEST {
        return Err("helper request exceeds the protocol bound");
    }
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|_| "invalid helper protocol")?;
    // The installer media request keeps the version its fixed request file records.
    let installer = matches!(envelope.request, Request::VerifyInstaller {});
    if envelope.schema_version != if installer { 1 } else { 2 } {
        return Err("unsupported helper protocol version");
    }
    Ok(envelope)
}
