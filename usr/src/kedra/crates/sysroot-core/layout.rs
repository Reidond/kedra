//! Committed source layouts.
//!
//! The repository root is the shared image filesystem: `etc/` and `usr/` are
//! payload, `etc/skel/` is the home baseline and `usr/src/kedra/` is the
//! development tree, which never enters the image. Package lists and target
//! overlays live under `usr/src/kedra/image/`. Commits from before this layout
//! keep `hosts/`, `packages/` and a root `home/`; both layouts stay readable,
//! selected per commit.

/// Development tree; excluded from the image payload.
pub const DEVELOPMENT: &str = "usr/src/kedra/";
/// Package lists and per-target overlays.
pub const IMAGE: &str = "usr/src/kedra/image/";
/// Top-level entries that identify a legacy-layout commit.
pub const LEGACY_ROOTS: [&str; 3] = ["hosts/", "packages/", "home/"];
