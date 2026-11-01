//! Committed source layout.
//!
//! `system/` holds the `.kedra` modules that declare every target: packages,
//! image files, the home baseline and image steps. `crates/` (the Rust
//! workspace) and `usr/src/kedra/` are the development tree; nothing else is
//! committed, and nothing enters an image except what `system/` declares.

/// Independent package policy, image build and release tooling.
pub const IMAGE: &str = "usr/src/kedra/image/";
