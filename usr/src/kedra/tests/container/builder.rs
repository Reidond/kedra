//! Disposable Linux builder: Linux `sysroot` binaries and deterministic source
//! payloads for any repository stage, without touching the checkout.
//!
//! Source snapshots are fresh throwaway Git repositories. The real
//! `sysroot source archive` then applies the production layout rules, target
//! overlays and private-key refusal to the snapshot's committed tree.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use sha2::{Digest, Sha256};
use testcontainers::core::Mount;
use testcontainers::runners::SyncRunner;
use testcontainers::{GenericImage, ImageExt};

use crate::docker::{self, Docker, Exec};
use crate::{Error, Result, invalid};

/// Which tree the payload comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Tracked and untracked, non-ignored files of the working tree.
    Worktree,
    /// An exact commit (or anything `git rev-parse` resolves to one).
    Revision(String),
}

/// Linux binaries and a payload archive produced for one target.
pub struct Outputs {
    pub sysroot: PathBuf,
    pub helper: PathBuf,
    pub payload: PathBuf,
    /// Commit of the throwaway snapshot; stable for identical trees.
    pub snapshot_commit: String,
    /// Commit the snapshot was taken from, when known.
    pub source_commit: Option<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn file_sha256(path: &Path) -> Result<String> {
    Ok(sha256_hex(&fs::read(path)?))
}

/// Short content key for image tags and cache entries.
pub fn content_key(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize()[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Git with inherited `GIT_*` state removed, like `sysroot` itself runs it.
fn git(repo: &Path) -> Command {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(repo)
        .stdin(Stdio::null());
    command
}

fn git_output(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = git(repo).args(args).output()?;
    if !output.status.success() {
        return Err(Error::Command {
            what: format!("git {}", args.join(" ")),
            exit: i64::from(output.status.code().unwrap_or(-1)),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(output.stdout)
}

fn copy_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if metadata.file_type().is_symlink() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(fs::read_link(source)?, destination)?;
        #[cfg(not(unix))]
        return Err(invalid("symbolic links need a Unix host"));
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
    }
    Ok(())
}

/// Materialize `stage` of `repo` as a new single-commit repository at `destination`.
pub fn snapshot(
    repo: &Path,
    stage: &Stage,
    destination: &Path,
) -> Result<(String, Option<String>)> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    let source_commit = match stage {
        Stage::Worktree => {
            let listed = git_output(
                repo,
                &[
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                ],
            )?;
            for raw in listed
                .split(|byte| *byte == 0)
                .filter(|raw| !raw.is_empty())
            {
                let relative = std::str::from_utf8(raw)
                    .map_err(|_| invalid("non-UTF-8 path in the working tree"))?;
                let source = repo.join(relative);
                // Deleted-but-tracked files simply stay absent from the snapshot.
                if fs::symlink_metadata(&source).is_ok() {
                    copy_entry(&source, &destination.join(relative))?;
                }
            }
            let head = git_output(repo, &["rev-parse", "--verify", "HEAD^{commit}"])?;
            Some(String::from_utf8_lossy(&head).trim().to_owned())
        }
        Stage::Revision(revision) => {
            let resolved = git_output(
                repo,
                &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
            )?;
            let commit = String::from_utf8_lossy(&resolved).trim().to_owned();
            let mut child = git(repo)
                .args(["archive", "--format=tar", &commit])
                .stdout(Stdio::piped())
                .spawn()?;
            let mut archive = Vec::new();
            if let Some(mut stdout) = child.stdout.take() {
                stdout.read_to_end(&mut archive)?;
            }
            if !child.wait()?.success() {
                return Err(invalid(format!("git archive {commit} failed")));
            }
            tar::Archive::new(archive.as_slice()).unpack(destination)?;
            Some(commit)
        }
    };
    // Fixed identity and dates: identical trees give identical commits.
    let init = |args: &[&str]| -> Result<Vec<u8>> {
        let output = git(destination)
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=snapshot",
            ])
            .args(args)
            .env("GIT_AUTHOR_NAME", "kedra-lab")
            .env("GIT_AUTHOR_EMAIL", "kedra-lab@example.invalid")
            .env("GIT_AUTHOR_DATE", "1970-01-01T00:00:00Z")
            .env("GIT_COMMITTER_NAME", "kedra-lab")
            .env("GIT_COMMITTER_EMAIL", "kedra-lab@example.invalid")
            .env("GIT_COMMITTER_DATE", "1970-01-01T00:00:00Z")
            .output()?;
        if !output.status.success() {
            return Err(Error::Command {
                what: format!("snapshot git {}", args.join(" ")),
                exit: i64::from(output.status.code().unwrap_or(-1)),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(output.stdout)
    };
    init(&["init", "-q"])?;
    init(&["add", "-A"])?;
    init(&["commit", "-q", "--no-verify", "-m", "kedra-lab snapshot"])?;
    let commit = init(&["rev-parse", "HEAD"])?;
    Ok((
        String::from_utf8_lossy(&commit).trim().to_owned(),
        source_commit,
    ))
}

fn oci_architecture(architecture: &str) -> &str {
    match architecture {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    }
}

/// Build (or reuse) the pinned builder image.
fn builder_image(docker: &Docker, platform: &str) -> Result<GenericImage> {
    let containerfile = crate::harness_dir().join("lab/builder.Containerfile");
    let key = content_key(&[&fs::read(&containerfile)?, platform.as_bytes()]);
    docker.build(&docker::Build {
        tag: &format!("kedra-lab-builder:{key}"),
        what: "lab builder",
        containerfile: &containerfile,
        files: &[],
        args: &[],
        platform,
    })?;
    Ok(GenericImage::new("kedra-lab-builder", &key))
}

/// Build Linux binaries (unless `prebuilt` supplies them) and archive `stage`
/// for `target` inside one disposable builder container.
pub fn prepare(
    docker: &Docker,
    target: &str,
    architecture: &str,
    stage: &Stage,
    prebuilt: Option<&Path>,
    cache: &Path,
) -> Result<Outputs> {
    let repo = crate::repository_root();
    let snapshot_dir = cache.join(format!("snapshot-{}", crate::execution_id()));
    let (snapshot_commit, source_commit) = snapshot(&repo, stage, &snapshot_dir)?;
    // Keep the snapshot's history: images built from it record this commit.
    let bundles = cache.join("bundles");
    fs::create_dir_all(&bundles)?;
    let bundle = bundles.join(format!("{snapshot_commit}.bundle"));
    if !bundle.is_file() {
        git_output(
            &snapshot_dir,
            &["bundle", "create", &bundle.to_string_lossy(), "HEAD"],
        )?;
    }
    let outputs = cache.join(architecture);
    fs::create_dir_all(&outputs)?;

    eprintln!("kedra-lab: preparing Linux sysroot binaries and the {target} payload");
    let request = builder_image(docker, &format!("linux/{}", oci_architecture(architecture)))?
        .with_cmd(["sleep", "infinity"])
        .with_label(docker::OWNER_LABEL, docker::OWNER)
        .with_label(docker::KIND_LABEL, "builder")
        .with_env_var("CARGO_TARGET_DIR", "/target")
        .with_env_var("RUSTUP_TOOLCHAIN", "1.98.1")
        .with_mount(Mount::volume_mount(
            "kedra-lab-cargo-registry",
            "/usr/local/cargo/registry",
        ))
        .with_mount(Mount::volume_mount(
            format!("kedra-lab-target-{architecture}"),
            "/target",
        ))
        .with_mount(
            Mount::bind_mount(snapshot_dir.to_string_lossy(), "/snapshot")
                .with_access_mode(testcontainers::core::AccessMode::ReadOnly),
        );
    let container = request
        .with_startup_timeout(Duration::from_secs(120))
        .start()?;
    let id = container.id().to_owned();

    let sysroot = outputs.join("sysroot");
    let helper = outputs.join("sysroot-helper");
    // Prebuilt binaries never enter the shared Cargo target volume.
    let mut archiver = "/target/release/sysroot";
    match prebuilt {
        Some(directory) => {
            fs::copy(directory.join("sysroot"), &sysroot)?;
            fs::copy(directory.join("sysroot-helper"), &helper)?;
            archiver = "/usr/local/bin/kedra-lab-sysroot";
            docker.write_file(&id, archiver, &fs::read(&sysroot)?, "root", "0755")?;
        }
        None => {
            let mut build = Exec::new([
                "cargo",
                "build",
                "--release",
                "--locked",
                "-p",
                "sysroot",
                "-p",
                "sysroot-helper",
            ])
            .timeout(Duration::from_secs(30 * 60));
            // Build the stage itself: the snapshot is the working tree or the commit.
            build.workdir = Some("/snapshot".into());
            let output = docker.run(&id, &build)?;
            let log = output.stderr_text();
            if let Some(last) = log.lines().rev().find(|line| !line.trim().is_empty()) {
                eprintln!("kedra-lab: {}", last.trim());
            }
            fs::write(&sysroot, docker.read_file(&id, "/target/release/sysroot")?)?;
            fs::write(
                &helper,
                docker.read_file(&id, "/target/release/sysroot-helper")?,
            )?;
        }
    }

    let payload = outputs.join(format!("payload-{target}-{snapshot_commit}.tar"));
    docker.run(
        &id,
        &Exec::new([
            archiver,
            "source",
            "archive",
            "--repo",
            "/snapshot",
            "--host",
            target,
            "--output",
            "/tmp/payload.tar",
        ]),
    )?;
    fs::write(&payload, docker.read_file(&id, "/tmp/payload.tar")?)?;
    container.rm()?;
    fs::remove_dir_all(&snapshot_dir)?;
    Ok(Outputs {
        sysroot,
        helper,
        payload,
        snapshot_commit,
        source_commit,
    })
}
