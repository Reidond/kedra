//! Closed production target table. Unknown target/architecture pairs are refused;
//! adding a target is a reviewed change to the embedded table with its own key and
//! signing environment. Release tooling and the installer read the same file.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../../image/release/targets.json");

/// One independently signed and published Kedra target.
#[derive(Debug, Deserialize)]
pub struct Spec {
    #[serde(skip)]
    pub target: String,
    /// Rust, RPM, `target.toml` and signed-scope architecture name.
    pub architecture: String,
    /// OCI image configuration and bootc status architecture name.
    pub oci_architecture: String,
    /// Production signed-image repository.
    pub repository: String,
}

#[derive(Deserialize)]
struct Table {
    schema_version: u32,
    targets: BTreeMap<String, Spec>,
}

/// The embedded table; malformed or oversized input yields no enabled target.
fn targets() -> &'static [Spec] {
    static TARGETS: OnceLock<Vec<Spec>> = OnceLock::new();
    TARGETS.get_or_init(|| {
        if TABLE.len() > 16_384 {
            return Vec::new();
        }
        match serde_json::from_str::<Table>(TABLE) {
            Ok(table) if table.schema_version == 1 => table
                .targets
                .into_iter()
                .map(|(target, spec)| Spec { target, ..spec })
                .collect(),
            _ => Vec::new(),
        }
    })
}

/// The enabled target, only for its exact architecture.
pub fn enabled(target: &str, architecture: &str) -> Option<&'static Spec> {
    targets()
        .iter()
        .find(|spec| spec.target == target && spec.architecture == architecture)
}

/// OCI platform architecture of an enabled target architecture.
pub fn oci_architecture(architecture: &str) -> Option<&'static str> {
    targets()
        .iter()
        .find(|spec| spec.architecture == architecture)
        .map(|spec| spec.oci_architecture.as_str())
}
