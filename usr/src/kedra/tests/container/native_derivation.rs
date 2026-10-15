//! Native images come from the public `sysroot derive build` command; the
//! harness only selects inputs and checks the result against its own engine.
//! The steps are the ones the composition's committed source declares.

use std::process::Command;

use crate::docker::Docker;
use crate::image::{Overlay, Request, Source};
use crate::{Result, invalid};

/// `sysroot derive build` output: image, engine, parent image and material.
pub use sysroot_engine::DerivedImage as Derived;

/// Native Linux `sysroot` binary that runs the derivation; it is also the
/// executor admitted into the offline build, so it must match the plan's.
fn sysroot() -> Result<std::path::PathBuf> {
    std::env::var_os("KEDRA_LAB_SYSROOT")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .ok_or_else(|| {
            invalid("native derivation needs KEDRA_LAB_SYSROOT: a native Linux sysroot binary")
        })
}

/// Derive through the public command with the selected identities.
/// State is shared with the release composer through the artifact root.
pub fn prepare(docker: &Docker, request: &Request, expected_identity: &str) -> Result<Derived> {
    let Source::Composition(context) = &request.source else {
        return Err(invalid(
            "native derivation requires --image composition:<directory>",
        ));
    };
    if request.overlay != Overlay::None
        || request.binaries.is_some()
        || request.target.architecture != docker.architecture()?
    {
        return Err(invalid(
            "native derivation requires the engine's native target without overlays or binary overrides",
        ));
    }
    let composition = request
        .composition_identity
        .as_deref()
        .ok_or_else(|| invalid("native derivation requires independent composition identity"))?;
    eprintln!("kedra-lab: deriving native image {expected_identity} with sysroot");
    let output = Command::new(sysroot()?)
        .args(["derive", "build", "--context"])
        .arg(context)
        .args(["--expected-identity", composition])
        .args(["--expected-derivation", expected_identity, "--state"])
        .arg(crate::composition::derive_state()?)
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(invalid(format!(
            "sysroot derive build failed ({})",
            output.status
        )));
    }
    let derived: Derived =
        serde_json::from_slice(&output.stdout).map_err(|error| invalid(error.to_string()))?;
    if derived.engine != docker.identity()?
        || derived.material.identity != expected_identity
        || derived.material.parent_identity != composition
        || docker.image(&derived.image)?.map(|(id, _)| id).as_deref() != Some(&derived.image)
    {
        return Err(invalid(
            "sysroot derived on another engine or returned unselected material",
        ));
    }
    Ok(derived)
}
