//! Retained schema-1 catalog data, never the current owner authoring source.
use crate::Catalog;

pub const JQ_SOURCE: &str = "src-5d3f1f89c8b6b206fc3c133f24d90d6225d73cd4c6274ec610c4cacf9f44e2d2";
pub const SQLITE_SOURCE: &str =
    "src-f4837e042b248ee63fd02e3f51fd6c9b9e2ffdf34bfb57450dd22a10c67f4d65";

/// Read the immutable legacy collection for callers retaining schema-1 workflows.
pub fn builtin(builder_image: &str, runtime_image: &str) -> Catalog {
    let mut catalog: Catalog = serde_json::from_str(include_str!("legacy.json"))
        .expect("compiled legacy catalog must be valid");
    for package in catalog.packages.values_mut() {
        for node in package.recipe.graph.nodes.values_mut() {
            node.builder_image = builder_image.into();
            node.runtime_image = runtime_image.into();
        }
    }
    catalog
}
