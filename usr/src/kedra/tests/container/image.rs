//! Image under test: resolve any Kedra stage to a lab image.
//!
//! A stage is a published signed image (stable, a run tag or a digest), an
//! Actions candidate from a `-builds` repository, any image reference, or a
//! full local build of a commit or the working tree. The lab tools layer goes
//! on top, then optionally the working-tree overlay (payload + binaries).

use std::fs;
use std::path::PathBuf;

use crate::builder::{self, Stage, content_key};
use crate::docker::{Build, Docker};
use crate::{Result, invalid};

/// A signed release target from usr/src/kedra/image/release/targets.json.
#[derive(Clone, Debug)]
pub struct Target {
    pub id: String,
    pub architecture: String,
    pub oci_architecture: String,
    pub repository: String,
    pub builds: String,
}

impl Target {
    pub fn platform(&self) -> String {
        format!("linux/{}", self.oci_architecture)
    }
}

pub fn targets() -> Result<Vec<Target>> {
    let path = crate::repository_root().join("usr/src/kedra/image/release/targets.json");
    let table: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
    if table["schema_version"] != 1 {
        return Err(invalid("unsupported release target table"));
    }
    let entries = table["targets"]
        .as_object()
        .ok_or_else(|| invalid("release target table has no targets"))?;
    let text = |value: &serde_json::Value, key: &str| -> Result<String> {
        value[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| invalid(format!("release target lacks {key}")))
    };
    entries
        .iter()
        .map(|(id, value)| {
            Ok(Target {
                id: id.clone(),
                architecture: text(value, "architecture")?,
                oci_architecture: text(value, "oci_architecture")?,
                repository: text(value, "repository")?,
                builds: text(value, "builds")?,
            })
        })
        .collect()
}

/// `id`, or the target matching the container engine's native architecture.
pub fn target(docker: &Docker, id: Option<&str>) -> Result<Target> {
    let all = targets()?;
    match id {
        Some(id) => all
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| invalid(format!("unknown image target {id:?}"))),
        None => {
            let architecture = docker.architecture()?;
            all.into_iter()
                .find(|target| target.architecture == architecture)
                .ok_or_else(|| invalid(format!("no image target for {architecture}")))
        }
    }
}

/// Where the image under test comes from (`KEDRA_LAB_IMAGE` / `--image`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// `stable`: the signed production image.
    Stable,
    /// `run-<id>-<attempt>`: an immutable signed publication.
    Run(String),
    /// `sha256:<hex>`: a digest in the production repository.
    Digest(String),
    /// `builds:<tag>`: an Actions candidate in the target's builds repository.
    Builds(String),
    /// `ref:<reference>`: any local or pullable image, e.g. a CI candidate.
    Reference(String),
    /// `build` or `build:<revision>`: full local image build of the working
    /// tree or of one commit, replaying the Actions build steps.
    Build(Option<String>),
}

impl std::str::FromStr for Source {
    type Err = crate::Error;

    fn from_str(text: &str) -> Result<Self> {
        let text = text.trim();
        let hex = |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        Ok(match text {
            "" | "stable" => Source::Stable,
            "build" => Source::Build(None),
            _ if text.starts_with("run-") => Source::Run(text.to_owned()),
            _ if text.starts_with("sha256:") && hex(&text[7..]) => Source::Digest(text.to_owned()),
            _ if text.starts_with("builds:") && text.len() > 7 => {
                Source::Builds(text[7..].to_owned())
            }
            _ if text.starts_with("ref:") && text.len() > 4 => {
                Source::Reference(text[4..].to_owned())
            }
            _ if text.starts_with("build:") && text.len() > 6 => {
                Source::Build(Some(text[6..].to_owned()))
            }
            _ => {
                return Err(invalid(format!(
                    "unknown image source {text:?}; use stable, run-<id>-<attempt>, sha256:<hex>, \
                     builds:<tag>, ref:<reference>, build or build:<revision>"
                )));
            }
        })
    }
}

/// Whether the working tree is layered over the image (`KEDRA_LAB_OVERLAY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    None,
    Worktree,
}

impl std::str::FromStr for Overlay {
    type Err = crate::Error;

    fn from_str(text: &str) -> Result<Self> {
        match text.trim() {
            "none" => Ok(Overlay::None),
            "worktree" => Ok(Overlay::Worktree),
            other => Err(invalid(format!(
                "unknown overlay {other:?}; use worktree or none"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub target: Target,
    pub source: Source,
    pub overlay: Overlay,
    /// Directory with prebuilt Linux `sysroot` and `sysroot-helper`.
    pub binaries: Option<PathBuf>,
}

impl Request {
    /// Settings from `KEDRA_LAB_*` variables; the default is the working tree
    /// over the stable image of the engine's native target.
    pub fn from_env(docker: &Docker) -> Result<Self> {
        let variable = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        let target = target(docker, variable("KEDRA_LAB_TARGET").as_deref())?;
        let source: Source = variable("KEDRA_LAB_IMAGE").unwrap_or_default().parse()?;
        let overlay = match variable("KEDRA_LAB_OVERLAY") {
            Some(value) => value.parse()?,
            None if matches!(source, Source::Build(_)) => Overlay::None,
            None => Overlay::Worktree,
        };
        Ok(Request {
            target,
            source,
            overlay,
            binaries: variable("KEDRA_LAB_BINARIES").map(PathBuf::from),
        })
    }
}

/// A ready lab image and how it was made.
#[derive(Clone, Debug, serde::Serialize)]
pub struct LabImage {
    /// `kedra-lab:<key>` or `kedra-lab-tools:<key>`.
    pub name: String,
    pub tag: String,
    pub target: String,
    pub source: String,
    /// Immutable base reference (`repository@sha256:…` or a local image ID).
    pub base: String,
    pub overlay: Option<OverlayInfo>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OverlayInfo {
    pub snapshot_commit: String,
    pub source_commit: Option<String>,
    pub payload_sha256: String,
    /// Image-build inputs that differ from the base's source revision and
    /// therefore are not reflected by the overlay.
    pub unapplied_build_inputs: Vec<String>,
}

impl LabImage {
    pub fn reference(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}

/// Pull a tag and pin it to its repository digest.
fn pinned(docker: &Docker, repository: &str, reference: &str, platform: &str) -> Result<String> {
    docker.pull(reference, platform)?;
    let (id, digests) = docker
        .image(reference)?
        .ok_or_else(|| invalid(format!("{reference} is missing after pulling")))?;
    Ok(digests
        .into_iter()
        .find(|digest| digest.starts_with(&format!("{repository}@")))
        .unwrap_or(id))
}

fn base(docker: &Docker, request: &Request) -> Result<(String, String)> {
    let target = &request.target;
    let platform = target.platform();
    Ok(match &request.source {
        Source::Stable => {
            let reference = format!("{}:stable", target.repository);
            (
                pinned(docker, &target.repository, &reference, &platform)?,
                reference,
            )
        }
        Source::Run(tag) => {
            let reference = format!("{}:{tag}", target.repository);
            (
                pinned(docker, &target.repository, &reference, &platform)?,
                reference,
            )
        }
        Source::Digest(digest) => {
            let reference = format!("{}@{digest}", target.repository);
            docker.pull(&reference, &platform)?;
            (reference.clone(), reference)
        }
        Source::Builds(tag) => {
            let reference = format!("{}:{tag}", target.builds);
            (
                pinned(docker, &target.builds, &reference, &platform)?,
                reference,
            )
        }
        Source::Reference(reference) => {
            if docker.image(reference)?.is_none() {
                docker.pull(reference, &platform)?;
            }
            let (id, _) = docker
                .image(reference)?
                .ok_or_else(|| invalid(format!("{reference} is not available")))?;
            (id, reference.clone())
        }
        Source::Build(revision) => {
            let built = crate::image::local_build(docker, request, revision.as_deref())?;
            (
                built.clone(),
                format!(
                    "local build of {}",
                    revision.as_deref().unwrap_or("the working tree")
                ),
            )
        }
    })
}

fn lab_file(name: &str) -> PathBuf {
    crate::harness_dir().join("lab").join(name)
}

/// Image-build inputs changed between two commits (overlay blind spots).
fn unapplied_build_inputs(from: &str, to: Option<&str>) -> Vec<String> {
    let repo = crate::repository_root();
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(&repo)
        .args(["diff", "--name-only", from]);
    if let Some(to) = to {
        command.arg(to);
    }
    command.args([
        "--",
        "usr/src/kedra/image/Containerfile",
        "usr/src/kedra/image/assemble.sh",
        "usr/src/kedra/image/inputs.json",
        "usr/src/kedra/image/agents",
        "usr/src/kedra/image/bitwarden",
        "usr/src/kedra/image/release",
    ]);
    match command.output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect(),
        _ => vec![format!(
            "unknown: base revision {from} is not in this checkout"
        )],
    }
}

fn base_source_revision(docker: &Docker, image: &str) -> Option<String> {
    use testcontainers::ImageExt;
    use testcontainers::runners::SyncRunner;
    let (name, tag) = image.rsplit_once(':')?;
    let container = testcontainers::GenericImage::new(name, tag)
        .with_cmd(["sleep", "300"])
        .with_label(crate::docker::OWNER_LABEL, crate::docker::OWNER)
        .with_label(crate::docker::KIND_LABEL, "inspect")
        .start()
        .ok()?;
    let manifest = docker
        .read_file(container.id(), "/usr/share/sysroot/source.json")
        .ok();
    let _ = container.rm();
    let value: serde_json::Value = serde_json::from_slice(&manifest?).ok()?;
    value["source_revision"].as_str().map(str::to_owned)
}

/// Resolve, build and cache the lab image for `request`.
pub fn prepare(docker: &Docker, request: &Request) -> Result<LabImage> {
    let (base, described) = base(docker, request)?;
    let lab_inputs = [
        "tools.Containerfile",
        "kedra-lab-host",
        "kedra-lab-host.service",
        "niri-nested.conf",
        "container-skip.conf",
        "rtkit-container.conf",
        "journald-container.conf",
    ];
    let test_profile =
        crate::repository_root().join("usr/src/kedra/tests/common/test-profile.toml");
    let probes_dir = lab_file("probes");
    let mut probes: Vec<PathBuf> = fs::read_dir(&probes_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    probes.sort();
    let mut key_parts: Vec<Vec<u8>> = vec![base.as_bytes().to_vec()];
    for name in lab_inputs {
        key_parts.push(fs::read(lab_file(name))?);
    }
    key_parts.push(fs::read(&test_profile)?);
    for probe in &probes {
        key_parts.push(
            probe
                .file_name()
                .unwrap_or_default()
                .as_encoded_bytes()
                .to_vec(),
        );
        key_parts.push(fs::read(probe)?);
    }
    let parts: Vec<&[u8]> = key_parts.iter().map(Vec::as_slice).collect();
    let tools_tag = content_key(&parts);
    let mut files: Vec<(String, PathBuf)> = lab_inputs[1..]
        .iter()
        .map(|name| ((*name).to_owned(), lab_file(name)))
        .collect();
    files.push(("test-profile.toml".into(), test_profile));
    for probe in &probes {
        let name = probe
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        files.push((format!("probes/{name}"), probe.clone()));
    }
    docker.build(&Build {
        tag: &format!("kedra-lab-tools:{tools_tag}"),
        what: "lab tools layer; the first build per base image takes a few minutes",
        containerfile: &lab_file("tools.Containerfile"),
        files: &files,
        args: &[("BASE", base.clone())],
        platform: &request.target.platform(),
    })?;
    let tools_reference = format!("kedra-lab-tools:{tools_tag}");

    if request.overlay == Overlay::None {
        return Ok(LabImage {
            name: "kedra-lab-tools".into(),
            tag: tools_tag,
            target: request.target.id.clone(),
            source: described,
            base,
            overlay: None,
        });
    }

    let cache = crate::artifact_root().join("cache");
    let outputs = builder::prepare(
        docker,
        &request.target.id,
        &request.target.architecture,
        &Stage::Worktree,
        request.binaries.as_deref(),
        &cache,
    )?;
    let payload_sha256 = builder::file_sha256(&outputs.payload)?;
    let overlay_key = content_key(&[
        tools_reference.as_bytes(),
        payload_sha256.as_bytes(),
        builder::file_sha256(&outputs.sysroot)?.as_bytes(),
        builder::file_sha256(&outputs.helper)?.as_bytes(),
        &fs::read(lab_file("overlay.Containerfile"))?,
        &fs::read(lab_file("overlay-apply.sh"))?,
    ]);
    docker.build(&Build {
        tag: &format!("kedra-lab:{overlay_key}"),
        what: "working-tree overlay",
        containerfile: &lab_file("overlay.Containerfile"),
        files: &[
            ("sysroot".into(), outputs.sysroot.clone()),
            ("sysroot-helper".into(), outputs.helper.clone()),
            ("payload.tar".into(), outputs.payload.clone()),
            ("overlay-apply.sh".into(), lab_file("overlay-apply.sh")),
        ],
        args: &[("BASE", tools_reference.clone())],
        platform: &request.target.platform(),
    })?;
    let unapplied = match base_source_revision(docker, &tools_reference) {
        Some(revision) => unapplied_build_inputs(&revision, None),
        None => vec!["unknown: the base image records no source revision".into()],
    };
    for path in &unapplied {
        eprintln!("kedra-lab: warning: overlay does not apply image-build input {path}");
    }
    Ok(LabImage {
        name: "kedra-lab".into(),
        tag: overlay_key,
        target: request.target.id.clone(),
        source: format!("{described} + working tree"),
        base,
        overlay: Some(OverlayInfo {
            snapshot_commit: outputs.snapshot_commit,
            source_commit: outputs.source_commit,
            payload_sha256,
            unapplied_build_inputs: unapplied,
        }),
    })
}

/// Full local image build of the working tree or `revision`; see build.rs.
pub fn local_build(docker: &Docker, request: &Request, revision: Option<&str>) -> Result<String> {
    crate::localbuild::local(docker, &request.target, revision)
}

/// Git bundle of the source revision an image records in
/// /usr/share/sysroot/source.json: the snapshot bundle for overlays, or one
/// made from this checkout for published and CI images.
pub fn source_bundle(docker: &Docker, container: &str) -> Result<PathBuf> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&docker.read_file(container, "/usr/share/sysroot/source.json")?)
            .map_err(|error| invalid(format!("unreadable source manifest: {error}")))?;
    let revision = manifest["source_revision"]
        .as_str()
        .filter(|value| value.len() >= 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| invalid("the image records no source revision"))?;
    let bundles = crate::artifact_root().join("cache/bundles");
    let path = bundles.join(format!("{revision}.bundle"));
    if path.is_file() {
        return Ok(path);
    }
    fs::create_dir_all(&bundles)?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(crate::repository_root())
        .args(["bundle", "create"])
        .arg(&path)
        .arg(revision)
        .stdin(std::process::Stdio::null())
        .output()?;
    if !output.status.success() {
        let _ = fs::remove_file(&path);
        return Err(invalid(format!(
            "source revision {revision} of the image is not in this checkout; fetch it first ({})",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(path)
}
