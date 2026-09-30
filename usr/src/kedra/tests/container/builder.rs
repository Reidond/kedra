//! Disposable Linux builder: Linux `sysroot` binaries and deterministic source
//! payloads for any repository stage, without touching the checkout.
//!
//! Source snapshots are fresh throwaway Git repositories. The real
//! `sysroot source archive` then applies the production layout rules, target
//! overlays and private-key refusal to the snapshot's committed tree.

use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
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

fn cache_object(cache: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let digest = sha256_hex(bytes);
    let directory = cache.join("objects").join(&digest);
    fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    if path.is_file() {
        if file_sha256(&path)? != digest {
            return Err(invalid(format!(
                "cached build object changed: {}",
                path.display()
            )));
        }
    } else {
        let pending = directory.join(format!("{name}-{}.pending", crate::execution_id()));
        fs::write(&pending, bytes)?;
        fs::rename(pending, &path)?;
    }
    Ok(path)
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

/// Prepared host archiver and its exact source inputs. Home sync needs no Linux build.
fn archiver_directory(cache: &Path) -> Result<PathBuf> {
    let repo = crate::repository_root();
    let listed = git_listing_output(
        &repo,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "usr/src/kedra/crates",
            "usr/src/kedra/image/release/targets.json",
            "usr/src/kedra/image/release/compatibility.json",
        ],
    )?;
    let mut names: Vec<&[u8]> = listed
        .split(|byte| *byte == 0)
        .filter(|v| !v.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut parts = vec![
        std::env::consts::OS.as_bytes().to_vec(),
        std::env::consts::ARCH.as_bytes().to_vec(),
    ];
    for name in names {
        let name =
            std::str::from_utf8(name).map_err(|_| invalid("non-UTF-8 archiver source path"))?;
        let path = repo.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                parts.push(name.as_bytes().to_vec());
                parts.push(fs::read(path)?);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(invalid(format!(
                    "archiver input is not a regular file: {name}"
                )));
            }
            Err(error) => return Err(error.into()),
        }
    }
    let parts: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Ok(cache.join("host-archiver").join(content_key(&parts)))
}

fn prepared_archiver(directory: &Path) -> Result<PathBuf> {
    let binary = directory.join("sysroot");
    let digest = directory.join("sha256");
    if !binary.is_file() || !digest.is_file() {
        return Err(invalid(
            "host source archiver is not prepared for these inputs; run kedra-lab prepare-sync",
        ));
    }
    if file_sha256(&binary)? != fs::read_to_string(digest)?.trim() {
        return Err(invalid(
            "prepared source archiver hash changed; run kedra-lab prepare-sync",
        ));
    }
    Ok(binary)
}

/// Explicit cold preparation; subsequent config-only syncs never invoke Cargo.
pub fn prepare_archiver(cache: &Path) -> Result<PathBuf> {
    fs::create_dir_all(cache)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join("host-archiver.lock"))?;
    lock.lock()?;
    let directory = archiver_directory(cache)?;
    if let Ok(binary) = prepared_archiver(&directory) {
        return Ok(binary);
    }
    eprintln!("kedra-lab: preparing the host source archiver (once per Rust source revision)");
    let repo = crate::repository_root();
    let target = cache.join("host-build");
    let status = Command::new("cargo")
        .current_dir(&repo)
        .env("RUSTUP_TOOLCHAIN", "1.98.1")
        .env("CARGO_TARGET_DIR", &target)
        .args(["build", "--release", "--locked", "-p", "sysroot"])
        .stdin(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(invalid("host source archiver build failed"));
    }
    if archiver_directory(cache)? != directory {
        return Err(invalid(
            "archiver source changed during compilation; prepare again",
        ));
    }
    fs::create_dir_all(&directory)?;
    let pending = directory.join("sysroot.pending");
    fs::copy(target.join("release/sysroot"), &pending)?;
    let digest = file_sha256(&pending)?;
    fs::rename(&pending, directory.join("sysroot"))?;
    fs::write(directory.join("sha256.pending"), digest)?;
    fs::rename(directory.join("sha256.pending"), directory.join("sha256"))?;
    prepared_archiver(&directory)
}

pub struct SourceArchive {
    pub payload: PathBuf,
    pub snapshot_commit: String,
    pub source_commit: Option<String>,
}

fn worktree_paths(repo: &Path) -> Result<Vec<u8>> {
    let listed = git_listing_output(
        repo,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    #[cfg(unix)]
    let owner = fs::symlink_metadata(repo)?.uid();
    let mut existing = Vec::with_capacity(listed.len());
    for raw in listed
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
    {
        let relative =
            std::str::from_utf8(raw).map_err(|_| invalid("non-UTF-8 path in the working tree"))?;
        match fs::symlink_metadata(repo.join(relative)) {
            Ok(metadata) if metadata.is_file() => {
                #[cfg(unix)]
                if metadata.nlink() != 1 || metadata.uid() != owner {
                    return Err(invalid(format!(
                        "working tree input has unsafe ownership or link count: {relative}"
                    )));
                }
                existing.extend_from_slice(raw);
                existing.push(0);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                existing.extend_from_slice(raw);
                existing.push(0);
            }
            Ok(_) => {
                return Err(invalid(format!(
                    "working tree input is not a regular file or symlink: {relative}"
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(existing)
}

/// Build the same deterministic single-commit snapshot as `snapshot`, using a
/// private bare repository and index instead of copying the whole checkout.
/// This is only for the host-side home hot path; Linux/full image builds still
/// receive an ordinary materialized checkout.
fn snapshot_worktree_bare(repo: &Path, destination: &Path) -> Result<(String, Option<String>)> {
    if destination.exists() {
        return Err(invalid("private home sync snapshot path already exists"));
    }
    fs::create_dir(destination)?;
    #[cfg(unix)]
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700))?;
    let private = fs::symlink_metadata(destination)?;
    if !private.is_dir() || private.file_type().is_symlink() {
        return Err(invalid("private home sync snapshot is not a directory"));
    }
    #[cfg(unix)]
    if private.uid() != fs::symlink_metadata(repo)?.uid() || private.mode() & 0o777 != 0o700 {
        return Err(invalid(
            "private home sync snapshot has unsafe ownership or permissions",
        ));
    }
    let paths = worktree_paths(repo)?;
    let head = git_output(repo, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let source_commit = String::from_utf8_lossy(&head).trim().to_owned();
    let initialized = git(destination).args(["init", "-q", "--bare"]).output()?;
    if !initialized.status.success() {
        return Err(Error::Command {
            what: "private snapshot git init".into(),
            exit: i64::from(initialized.status.code().unwrap_or(-1)),
            stderr: String::from_utf8_lossy(&initialized.stderr).into_owned(),
        });
    }
    let private_git = |index: &Path| {
        let mut command = git(repo);
        command
            .arg("--git-dir")
            .arg(destination)
            .env("GIT_WORK_TREE", repo)
            .env("GIT_INDEX_FILE", index);
        command
    };
    let checked = |command: &mut Command, operation: &str| -> Result<Vec<u8>> {
        let output = command.output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Error::Command {
                what: operation.into(),
                exit: i64::from(output.status.code().unwrap_or(-1)),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    };
    let build_tree = |index: &Path, paths: &[u8]| -> Result<String> {
        checked(
            private_git(index).args(["read-tree", "--empty"]),
            "private snapshot read-tree",
        )?;
        let mut add = private_git(index);
        add.args(["add", "--pathspec-from-file=-", "--pathspec-file-nul"])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = add.spawn()?;
        child
            .stdin
            .take()
            .ok_or_else(|| invalid("private snapshot Git has no stdin"))?
            .write_all(paths)?;
        let added = child.wait_with_output()?;
        if !added.status.success() {
            return Err(Error::Command {
                what: "private snapshot git add".into(),
                exit: i64::from(added.status.code().unwrap_or(-1)),
                stderr: String::from_utf8_lossy(&added.stderr).into_owned(),
            });
        }
        let tree = checked(
            private_git(index).args(["write-tree"]),
            "private snapshot write-tree",
        )?;
        Ok(String::from_utf8_lossy(&tree).trim().to_owned())
    };
    let first_index = destination.join("snapshot-first.index");
    let second_index = destination.join("snapshot-second.index");
    let tree = build_tree(&first_index, &paths)?;
    let final_paths = worktree_paths(repo)?;
    let final_head = git_output(repo, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let final_tree = build_tree(&second_index, &final_paths)?;
    if final_tree != tree
        || final_paths != paths
        || String::from_utf8_lossy(&final_head).trim() != source_commit
    {
        return Err(invalid(
            "working tree changed while preparing the home sync snapshot; retry",
        ));
    }
    let mut commit = private_git(&first_index);
    commit
        .args(["commit-tree", &tree])
        .env("GIT_AUTHOR_NAME", "kedra-lab")
        .env("GIT_AUTHOR_EMAIL", "kedra-lab@example.invalid")
        .env("GIT_AUTHOR_DATE", "1970-01-01T00:00:00Z")
        .env("GIT_COMMITTER_NAME", "kedra-lab")
        .env("GIT_COMMITTER_EMAIL", "kedra-lab@example.invalid")
        .env("GIT_COMMITTER_DATE", "1970-01-01T00:00:00Z")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = commit.spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| invalid("private snapshot commit has no stdin"))?
        .write_all(b"kedra-lab snapshot\n")?;
    let committed = child.wait_with_output()?;
    if !committed.status.success() {
        return Err(Error::Command {
            what: "private snapshot commit-tree".into(),
            exit: i64::from(committed.status.code().unwrap_or(-1)),
            stderr: String::from_utf8_lossy(&committed.stderr).into_owned(),
        });
    }
    let snapshot_commit = String::from_utf8_lossy(&committed.stdout).trim().to_owned();
    checked(
        private_git(&first_index).args(["update-ref", "HEAD", &snapshot_commit]),
        "private snapshot update-ref",
    )?;
    Ok((snapshot_commit, Some(source_commit)))
}

fn remove_private_snapshot(repo: &Path, path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("private home sync snapshot changed type"));
    }
    #[cfg(unix)]
    if metadata.uid() != fs::symlink_metadata(repo)?.uid() {
        return Err(invalid("private home sync snapshot changed owner"));
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

/// Archive the current tree with the already-prepared host CLI, without Docker/Cargo.
pub fn archive_worktree(target: &str, cache: &Path) -> Result<SourceArchive> {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(cache.join("home-source.lock"))?;
    lock.lock()?;
    let repo = crate::repository_root();
    let binary = prepared_archiver(&archiver_directory(cache)?)?;
    let scratch = cache.join(format!("home-source-{}", crate::execution_id()));
    let outcome = (|| {
        let (snapshot_commit, source_commit) = snapshot_worktree_bare(&repo, &scratch)?;
        let directory = cache.join("home-payloads");
        fs::create_dir_all(&directory)?;
        let payload = directory.join(format!("{target}-{snapshot_commit}.tar"));
        if !payload.is_file() {
            let pending = directory.join(format!("{target}-{}.pending", crate::execution_id()));
            let result = Command::new(&binary)
                .args(["source", "archive", "--repo"])
                .arg(&scratch)
                .args(["--host", target, "--output"])
                .arg(&pending)
                .stdin(Stdio::null())
                .output()?;
            if !result.status.success() {
                let _ = fs::remove_file(&pending);
                return Err(Error::Command {
                    what: "host sysroot source archive".into(),
                    exit: i64::from(result.status.code().unwrap_or(-1)),
                    stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
                });
            }
            fs::rename(pending, &payload)?;
        }
        Ok(SourceArchive {
            payload,
            snapshot_commit,
            source_commit,
        })
    })();
    if let Err(error) = remove_private_snapshot(&repo, &scratch) {
        return Err(invalid(format!(
            "could not safely remove source snapshot {}: {error}",
            scratch.display()
        )));
    }
    outcome
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
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_LITERAL_PATHSPECS", "1")
        .args([
            "-c",
            "core.hooksPath=",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.autocrlf=false",
        ])
        .arg("-C")
        .arg(repo)
        .stdin(Stdio::null());
    command
}

/// Read the owner's effective ignore rules without allowing inherited Git
/// environment state to redirect the checkout. `ls-files` does not run hooks
/// or content filters; private staging uses the fully sanitized `git` above.
fn git_listing(repo: &Path) -> Command {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-c", "core.hooksPath=", "-C"])
        .arg(repo)
        .stdin(Stdio::null());
    command
}

fn git_listing_output(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = git_listing(repo).args(args).output()?;
    if !output.status.success() {
        return Err(Error::Command {
            what: format!("git {}", args.join(" ")),
            exit: i64::from(output.status.code().unwrap_or(-1)),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(output.stdout)
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
            let listed = git_listing_output(
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
        let pending = bundles.join(format!(
            "{}-{}.pending",
            snapshot_commit,
            crate::execution_id()
        ));
        git_output(
            &snapshot_dir,
            &["bundle", "create", &pending.to_string_lossy(), "HEAD"],
        )?;
        fs::rename(pending, &bundle)?;
    }

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

    let sysroot;
    let helper;
    // Prebuilt binaries never enter the shared Cargo target volume.
    let archiver;
    match prebuilt {
        Some(directory) => {
            sysroot = cache_object(cache, "sysroot", &fs::read(directory.join("sysroot"))?)?;
            helper = cache_object(
                cache,
                "sysroot-helper",
                &fs::read(directory.join("sysroot-helper"))?,
            )?;
            archiver = "/usr/local/bin/kedra-lab-sysroot";
            docker.write_file(&id, archiver, &fs::read(&sysroot)?, "root", "0755")?;
        }
        None => {
            // The Cargo volume is shared across checkouts and engines' clients.
            // Hold its lock through copying into this container's private /tmp.
            let mut build = Exec::new([
                "flock", "--timeout", "1800", "/target/kedra-build.lock", "sh", "-ec",
                "cargo build --release --locked -p sysroot -p sysroot-helper; cp /target/release/sysroot /tmp/kedra-sysroot; cp /target/release/sysroot-helper /tmp/kedra-sysroot-helper",
            ])
            .timeout(Duration::from_secs(30 * 60));
            // Build the stage itself: the snapshot is the working tree or the commit.
            build.workdir = Some("/snapshot".into());
            let output = docker.run(&id, &build)?;
            let log = output.stderr_text();
            if let Some(last) = log.lines().rev().find(|line| !line.trim().is_empty()) {
                eprintln!("kedra-lab: {}", last.trim());
            }
            sysroot = cache_object(
                cache,
                "sysroot",
                &docker.read_file(&id, "/tmp/kedra-sysroot")?,
            )?;
            helper = cache_object(
                cache,
                "sysroot-helper",
                &docker.read_file(&id, "/tmp/kedra-sysroot-helper")?,
            )?;
            archiver = "/tmp/kedra-sysroot";
        }
    }

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
    let payload = cache_object(
        cache,
        "payload.tar",
        &docker.read_file(&id, "/tmp/payload.tar")?,
    )?;
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
