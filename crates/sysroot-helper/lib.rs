//! Storage and the narrow protocol for image-owned management code.
//! No agent runtime or Git engine is linked into the helper.
#[cfg(target_os = "linux")]
pub mod management;
pub mod protocol;
#[cfg(target_os = "linux")]
pub mod storage;
#[cfg(target_os = "linux")]
pub mod trusted_file;
