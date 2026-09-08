//! Bounded requests shared by the ordinary CLI and installed helper.
use serde::{Deserialize, Serialize};

pub const MAX_REQUEST: usize = 524_288;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedDocument {
    pub payload: String,
    pub signature: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Enroll {
        release: SignedDocument,
        checkpoint: SignedDocument,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        installed_release: Option<SignedDocument>,
    },
    Status {},
    /// Media-only precheck of the fixed embedded payload; accepts no caller paths.
    VerifyInstaller {},
    Stage {
        release: SignedDocument,
        checkpoint: SignedDocument,
        replace_staged: Option<String>,
        resume: bool,
    },
    Rollback {
        release: SignedDocument,
        replace_staged: Option<String>,
    },
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
    if envelope.schema_version != 1 {
        return Err("unsupported helper protocol version");
    }
    Ok(envelope)
}
