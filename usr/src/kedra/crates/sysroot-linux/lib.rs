//! Linux primitives shared by the ordinary CLI and the installed helper:
//! owner-checked storage, trusted reads, journalable replacement and firmware
//! observations. Privileged management code stays in the helper binary.
#[cfg(target_os = "linux")]
pub mod firmware;
#[cfg(target_os = "linux")]
pub mod replace_file;
#[cfg(target_os = "linux")]
pub mod storage;
#[cfg(target_os = "linux")]
pub mod trusted_file;
