//! Offline replay of an independently selected, verified composition context.
//! The static image is realized by the engine (`sysroot_engine::realize_composition`),
//! the same code `sysroot derive build` uses for its parent image.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::docker::{Docker, Output};
use crate::image::{Overlay, Request, Source};
use crate::{Result, invalid};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    pub identity: String,
    pub image: String,
    pub foundation: String,
    pub engine: String,
    pub outputs: BTreeMap<String, String>,
}

impl Replay {
    pub fn run(
        &self,
        docker: &Docker,
        output: &str,
        program: &str,
        args: &[String],
    ) -> Result<Output> {
        if docker.identity()? != self.engine {
            return Err(invalid("replay engine identity changed"));
        }
        let object = self
            .outputs
            .get(output)
            .ok_or_else(|| invalid("unknown typed composition output"))?;
        if program.is_empty()
            || Path::new(program)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(invalid(
                "program must be a relative path inside the selected output",
            ));
        }
        let mut argv = vec![format!(
            "{}/{object}/{program}",
            sysroot_engine::LOGICAL_PREFIX
        )];
        argv.extend_from_slice(args);
        docker.isolated(&self.image, argv)
    }
}

pub(crate) fn native_image(
    docker: &Docker,
    image: &str,
) -> Result<testcontainers::bollard::models::ImageInspect> {
    let inspected = docker.inspect_image(image)?;
    if inspected.id.as_deref() != Some(image)
        || inspected.os.as_deref() != Some("linux")
        || inspected.architecture.as_deref() != Some(sysroot_engine::OCI_ARCHITECTURE)
    {
        return Err(invalid(format!(
            "composition image ID/platform differs from native {}",
            sysroot_engine::OCI_PLATFORM
        )));
    }
    if let Some(config) = &inspected.config
        && (config.on_build.as_ref().is_some_and(|v| !v.is_empty())
            || config.volumes.as_ref().is_some_and(|v| !v.is_empty()))
    {
        return Err(invalid(
            "composition foundation/image has ONBUILD or declared volumes",
        ));
    }
    Ok(inspected)
}

/// Shared `sysroot derive` state under the artifact root (the release composer
/// passes the same directory, so its bindings are reused rather than rebuilt).
pub(crate) fn derive_state() -> Result<std::path::PathBuf> {
    let root = crate::artifact_root();
    fs::create_dir_all(&root)?;
    Ok(root.join("derive"))
}

pub(crate) fn nonce() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Every invocation verifies a private snapshot and loads its complete foundation.
/// Only the resulting static image can subsequently enter the separate lab-tools layer.
pub fn prepare(docker: &Docker, request: &Request) -> Result<Replay> {
    let Source::Composition(input) = &request.source else {
        return Err(invalid("replay requires --image composition:<directory>"));
    };
    if request.overlay != Overlay::None || request.binaries.is_some() {
        return Err(invalid(
            "composition replay refuses working-tree overlays and binary overrides",
        ));
    }
    let identity = request.composition_identity.as_deref().ok_or_else(|| invalid("composition replay requires an independently selected --composition-identity or KEDRA_LAB_COMPOSITION_IDENTITY"))?;
    if request.target.oci_architecture != sysroot_engine::OCI_ARCHITECTURE
        || docker.architecture()? != request.target.architecture
    {
        return Err(invalid(
            "composition replay requires the engine's native target",
        ));
    }
    let realized = sysroot_engine::realize_composition(input, identity, &derive_state()?)
        .map_err(|error| invalid(error.to_string()))?;
    if realized.engine != docker.identity()? {
        return Err(invalid(
            "composition was realized on another engine than the harness uses",
        ));
    }
    Ok(Replay {
        identity: realized.identity,
        image: realized.image,
        foundation: realized.foundation,
        engine: realized.engine,
        outputs: realized.outputs,
    })
}
