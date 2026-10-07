//! Ordinary-user immutable Linux engine with input-addressed build outputs.
#[cfg(unix)]
mod bundle;
#[cfg(unix)]
mod cache;
#[cfg(unix)]
mod context;
#[cfg(unix)]
mod executor;
#[cfg(unix)]
mod image_archive;
mod model;
#[cfg(unix)]
mod native;
mod plan;
mod resources;
pub use resources::{SourceFile, source_identity};
#[cfg(unix)]
mod profile;
#[cfg(unix)]
mod snapshot;
#[cfg(unix)]
mod store;
#[cfg(unix)]
mod system;
#[cfg(unix)]
mod tree;
#[cfg(unix)]
pub use cache::{
    CACHE_RECEIPT_PURPOSE, CachePolicy, CacheReceipt, VerifiedCacheReceipt, verify_cache_receipt,
};
#[cfg(unix)]
pub use context::VerifiedComposition;
pub use model::unique_map as deserialize_unique_map;
pub use model::*;
#[cfg(unix)]
pub use native::*;
pub use plan::{plan, read_graph};
#[cfg(unix)]
pub use snapshot::{
    ManagedSnapshot, SnapshotPurpose, SnapshotRecovery, SnapshotRefusal, recover_snapshots,
};
#[cfg(unix)]
pub use store::Store;
#[cfg(unix)]
pub use system::*;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    Corrupt(String),
    Process { code: i32, message: String },
    RecoveryRequired(String),
    Divergent(String),
}
pub type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O: {e}"),
            Self::Json(e) => write!(f, "JSON: {e}"),
            Self::Invalid(s) => write!(f, "invalid input: {s}"),
            Self::Corrupt(s) => write!(f, "corrupt engine state: {s}"),
            Self::Process { code, message } => write!(f, "process exited {code}: {message}"),
            Self::RecoveryRequired(s) => write!(f, "recovery required: {s}"),
            Self::Divergent(s) => write!(f, "independent rebuild diverged: {s}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
#[cfg(unix)]
impl From<rustix::io::Errno> for Error {
    fn from(e: rustix::io::Errno) -> Self {
        Self::Io(e.into())
    }
}
