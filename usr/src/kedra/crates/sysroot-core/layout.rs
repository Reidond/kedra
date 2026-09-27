//! Committed source layouts.
//!
//! The repository root is the shared image filesystem: `etc/` and `usr/` are
//! payload, `etc/skel/` is the home baseline and `usr/src/kedra/` is the
//! development tree, which never enters the image. Package lists and target
//! overlays live under `usr/src/kedra/image/`. Commits from before this layout
//! keep `hosts/`, `packages/` and a root `home/`; installed baselines and review
//! state can name them, so both layouts stay readable, selected per commit.

/// Development tree; excluded from the image payload.
pub const DEVELOPMENT: &str = "usr/src/kedra/";
/// Package lists and per-target overlays.
pub const IMAGE: &str = "usr/src/kedra/image/";
/// Top-level entries that identify a legacy-layout commit.
pub const LEGACY_ROOTS: [&str; 3] = ["hosts/", "packages/", "home/"];

/// Which overlay provides a home-baseline file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Shared,
    Target,
}

/// Overlay and home-relative path of a committed home-baseline source path in
/// either layout. Provenance compares this, not the raw path, so a file keeps
/// its identity when the repository layout changes.
pub fn home_source<'a>(target: &str, source_path: &'a str) -> Option<(Layer, &'a str)> {
    let shared = source_path
        .strip_prefix("etc/skel/")
        .or_else(|| source_path.strip_prefix("home/"))
        .map(|relative| (Layer::Shared, relative));
    shared
        .or_else(|| {
            [
                format!("{IMAGE}targets/{target}/etc/skel/"),
                format!("hosts/{target}/home/"),
            ]
            .iter()
            .find_map(|prefix| source_path.strip_prefix(prefix.as_str()))
            .map(|relative| (Layer::Target, relative))
        })
        .filter(|(_, relative)| !relative.is_empty())
}

/// Whether two committed source paths name the same home-baseline file.
pub fn same_home_source(target: &str, left: &str, right: &str) -> bool {
    home_source(target, left).is_some_and(|source| home_source(target, right) == Some(source))
}
