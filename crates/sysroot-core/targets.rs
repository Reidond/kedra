//! Closed production target table. Unknown target/architecture pairs are refused;
//! adding a target is a reviewed code change with its own key and signing environment.

/// One independently signed and published Kedra target.
#[derive(Debug)]
pub struct Spec {
    pub target: &'static str,
    /// Rust, RPM, `host.toml` and signed-scope architecture name.
    pub architecture: &'static str,
    /// OCI image configuration and bootc status architecture name.
    pub oci_architecture: &'static str,
    /// Production signed-image repository.
    pub repository: &'static str,
}

pub static TARGETS: [Spec; 2] = [
    Spec {
        target: "desktop",
        architecture: "x86_64",
        oci_architecture: "amd64",
        repository: "ghcr.io/reidond/kedra-desktop",
    },
    Spec {
        target: "utm",
        architecture: "aarch64",
        oci_architecture: "arm64",
        repository: "ghcr.io/reidond/kedra-utm",
    },
];

/// The enabled target, only for its exact architecture.
pub fn enabled(target: &str, architecture: &str) -> Option<&'static Spec> {
    TARGETS
        .iter()
        .find(|spec| spec.target == target && spec.architecture == architecture)
}

/// OCI platform architecture of an enabled target architecture.
pub fn oci_architecture(architecture: &str) -> Option<&'static str> {
    TARGETS
        .iter()
        .find(|spec| spec.architecture == architecture)
        .map(|spec| spec.oci_architecture)
}
