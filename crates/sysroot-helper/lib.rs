//! Linux storage primitives shared by image-owned management code.
//! No CLI, agent runtime, Git engine, network or privileged operation is exposed.
#[cfg(target_os = "linux")]
pub mod storage;
