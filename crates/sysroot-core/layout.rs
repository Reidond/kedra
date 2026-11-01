//! Committed source layout.
//!
//! The repository root is the shared image filesystem: `etc/` and `usr/` are
//! payload, `etc/skel/` is the home baseline, and `crates/` (the Rust
//! workspace) and `usr/src/kedra/` are the development tree, which never enters
//! the image. Package inputs and target
//! overlays live under `usr/src/kedra/image/`.

/// Package lists and per-target overlays.
pub const IMAGE: &str = "usr/src/kedra/image/";
