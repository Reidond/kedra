//! Docker Engine operations on containers the harness already owns.
//!
//! Testcontainers creates, starts and removes containers. Operations that must
//! also work on a retained lab container from a later process (exec, file
//! transfer, listing by ownership label) use the Engine API it re-exports.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use testcontainers::bollard::{self, container::LogOutput, exec::StartExecResults};
use tokio::io::AsyncWriteExt;

use crate::{Error, Result, invalid};

/// Label every harness-owned resource carries; cleanup only ever selects it.
pub const OWNER_LABEL: &str = "dev.kedra.lab.owner";
pub const OWNER: &str = "kedra-container-tests";
pub const EXECUTION_LABEL: &str = "dev.kedra.lab.execution";
pub const KIND_LABEL: &str = "dev.kedra.lab.kind";
pub const TARGET_LABEL: &str = "dev.kedra.lab.target";
pub const IMAGE_LABEL: &str = "dev.kedra.lab.image";

pub struct Docker {
    runtime: tokio::runtime::Runtime,
    api: bollard::Docker,
}

/// One command inside a container. `argv` is executed directly, never through a shell.
#[derive(Clone, Debug)]
pub struct Exec {
    pub argv: Vec<String>,
    pub user: Option<String>,
    pub env: Vec<(String, String)>,
    pub workdir: Option<String>,
    pub stdin: Option<Vec<u8>>,
    pub timeout: Duration,
}

impl Exec {
    pub fn new<S: Into<String>>(argv: impl IntoIterator<Item = S>) -> Self {
        Exec {
            argv: argv.into_iter().map(Into::into).collect(),
            user: None,
            env: Vec::new(),
            workdir: None,
            stdin: None,
            timeout: Duration::from_secs(60),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn stdin(mut self, input: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(input.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct Output {
    pub exit: i64,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration: Duration,
}

impl Output {
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// `timeout(1)` reports an expired deadline as 124 (137 after SIGKILL).
    pub fn timed_out(&self) -> bool {
        matches!(self.exit, 124 | 137)
    }
}

/// One image build: Containerfile plus context files named as the
/// Containerfile refers to them.
pub struct Build<'a> {
    pub tag: &'a str,
    pub what: &'a str,
    pub containerfile: &'a std::path::Path,
    pub files: &'a [(String, std::path::PathBuf)],
    pub args: &'a [(&'a str, String)],
    /// `linux/<arch>`; empty for the engine's native platform.
    pub platform: &'a str,
}

/// A harness-owned container as listed by the Engine.
#[derive(Clone, Debug)]
pub struct Owned {
    pub id: String,
    pub name: String,
    pub state: String,
    pub labels: HashMap<String, String>,
}

impl Docker {
    /// BuildKit requires a named FROM reference, not a bare local sha256 image ID.
    pub fn pin_local(&self, id: &str) -> Result<String> {
        let digest = id
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| invalid("container engine returned an invalid image ID"))?;
        let reference = format!("kedra-lab-base:{digest}");
        if let Some((found, _)) = self.image(&reference)? {
            if found != id {
                return Err(invalid("local base cache tag points at another image"));
            }
        } else {
            let options = bollard::query_parameters::TagImageOptions {
                repo: Some("kedra-lab-base".into()),
                tag: Some(digest.into()),
            };
            self.runtime
                .block_on(self.api.tag_image(id, Some(options)))?;
        }
        Ok(reference)
    }

    pub fn connect() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let api = bollard::Docker::connect_with_defaults()?;
        Ok(Docker { runtime, api })
    }

    /// Engine architecture as `uname -m` spells it (`x86_64`, `aarch64`).
    pub fn architecture(&self) -> Result<String> {
        let info = self.runtime.block_on(self.api.info())?;
        info.architecture
            .ok_or_else(|| Error::Docker("engine did not report its architecture".into()))
    }

    /// Bind native disk preparation to the same engine as Testcontainers.
    pub fn identity(&self) -> Result<String> {
        self.runtime
            .block_on(self.api.info())?
            .id
            .ok_or_else(|| Error::Docker("engine did not report its identity".into()))
    }

    /// Run `argv` bounded by `timeout(1)` inside the container, so an expired
    /// deadline kills the process instead of leaving it running.
    pub fn exec(&self, container: &str, exec: &Exec) -> Result<Output> {
        if exec.argv.is_empty() {
            return Err(invalid("empty command"));
        }
        let timeout = format!("{:.3}s", exec.timeout.as_secs_f64().max(0.001));
        let mut cmd = vec!["timeout".to_owned(), "--kill-after=5s".to_owned(), timeout];
        cmd.extend(exec.argv.iter().cloned());
        let env: Vec<String> = exec
            .env
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        let config = bollard::models::ExecConfig {
            attach_stdin: Some(exec.stdin.is_some()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            env: Some(env),
            cmd: Some(cmd),
            user: exec.user.clone(),
            working_dir: exec.workdir.clone(),
            ..Default::default()
        };
        let started = Instant::now();
        // Backstop in case the Engine stream itself hangs past timeout(1)'s kill.
        let backstop = crate::cancel::budget(exec.timeout + Duration::from_secs(30));
        if backstop.is_zero() {
            return Err(Error::Timeout {
                what: "diagnostics collection".into(),
                after: Duration::from_secs(30),
            });
        }
        let result = self.runtime.block_on(crate::cancel::interrupt(async {
            tokio::time::timeout(backstop, async {
                let created = self.api.create_exec(container, config).await?;
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let StartExecResults::Attached {
                    mut output,
                    mut input,
                } = self.api.start_exec(&created.id, None).await?
                {
                    if let Some(data) = &exec.stdin {
                        input.write_all(data).await.map_err(Error::Io)?;
                        input.shutdown().await.map_err(Error::Io)?;
                    }
                    while let Some(item) = output.next().await {
                        match item? {
                            LogOutput::StdOut { message } | LogOutput::Console { message } => {
                                stdout.extend_from_slice(&message)
                            }
                            LogOutput::StdErr { message } => stderr.extend_from_slice(&message),
                            LogOutput::StdIn { .. } => {}
                        }
                    }
                }
                let inspected = self.api.inspect_exec(&created.id).await?;
                Ok::<_, Error>((inspected.exit_code, stdout, stderr))
            })
            .await
        }))?;
        let (exit, stdout, stderr) = match result {
            Ok(outcome) => outcome?,
            Err(_) => {
                return Err(Error::Timeout {
                    what: format!("command {:?}", exec.argv),
                    after: backstop,
                });
            }
        };
        Ok(Output {
            exit: exit.ok_or_else(|| Error::Docker("exec finished without an exit code".into()))?,
            stdout,
            stderr,
            duration: started.elapsed(),
        })
    }

    /// Run and require exit 0.
    pub fn run(&self, container: &str, exec: &Exec) -> Result<Output> {
        let output = self.exec(container, exec)?;
        if output.exit != 0 {
            return Err(Error::Command {
                what: format!("{:?}", exec.argv),
                exit: output.exit,
                stderr: output.stderr_text(),
            });
        }
        Ok(output)
    }

    /// Read a file through the container's own view, including tmpfs mounts.
    pub fn read_file(&self, container: &str, path: &str) -> Result<Vec<u8>> {
        let exec = Exec::new(["cat", "--", path]);
        Ok(self.run(container, &exec)?.stdout)
    }

    /// Write a file through the container's own view with an explicit owner and mode.
    pub fn write_file(
        &self,
        container: &str,
        path: &str,
        contents: &[u8],
        owner: &str,
        mode: &str,
    ) -> Result<()> {
        let exec = Exec::new([
            "install",
            "-D",
            "-o",
            owner,
            "-g",
            owner,
            "-m",
            mode,
            "/dev/stdin",
            path,
        ])
        .stdin(contents.to_vec());
        self.run(container, &exec).map(|_| ())
    }

    /// Harness-owned containers, optionally narrowed by extra label filters.
    pub fn owned(&self, filters: &[(&str, &str)]) -> Result<Vec<Owned>> {
        let mut labels = vec![format!("{OWNER_LABEL}={OWNER}")];
        labels.extend(filters.iter().map(|(key, value)| format!("{key}={value}")));
        let options = bollard::query_parameters::ListContainersOptions {
            all: true,
            filters: Some(HashMap::from([("label".to_owned(), labels)])),
            ..Default::default()
        };
        let listed = self
            .runtime
            .block_on(self.api.list_containers(Some(options)))?;
        Ok(listed
            .into_iter()
            .map(|summary| Owned {
                id: summary.id.unwrap_or_default(),
                name: summary
                    .names
                    .and_then(|names| names.into_iter().next())
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_owned(),
                state: summary
                    .state
                    .map(|state| state.to_string())
                    .unwrap_or_default(),
                labels: summary.labels.unwrap_or_default(),
            })
            .collect())
    }

    /// Force-remove one container; only ever called for harness-owned IDs.
    pub fn remove(&self, id: &str) -> Result<()> {
        let options = bollard::query_parameters::RemoveContainerOptions {
            force: true,
            v: true,
            ..Default::default()
        };
        match self.runtime.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(30),
                self.api.remove_container(id, Some(options)),
            )
            .await
        }) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            })) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err(Error::Timeout {
                what: "container removal".into(),
                after: Duration::from_secs(30),
            }),
        }
    }

    /// Remove harness-built images (lab tools, overlays, builders, local
    /// builds) that no container uses and that are not in `keep`. They are
    /// caches: the harness rebuilds them on demand. Returns removed tags.
    pub fn prune_images(&self, keep: &[String]) -> Result<Vec<String>> {
        let listed = self.runtime.block_on(self.api.list_images(Some(
            bollard::query_parameters::ListImagesOptions {
                all: false,
                ..Default::default()
            },
        )))?;
        let used: std::collections::HashSet<String> = self
            .runtime
            .block_on(self.api.list_containers(Some(
                bollard::query_parameters::ListContainersOptions {
                    all: true,
                    ..Default::default()
                },
            )))?
            .into_iter()
            .filter_map(|container| container.image_id)
            .collect();
        let mut removed = Vec::new();
        for image in listed {
            if used.contains(&image.id) {
                continue;
            }
            for tag in &image.repo_tags {
                let ours = [
                    "kedra-lab:",
                    "kedra-lab-tools:",
                    "kedra-lab-builder:",
                    "kedra-lab-inputs:",
                    "kedra-local-",
                ]
                .iter()
                .any(|prefix| tag.starts_with(prefix));
                if ours && !keep.contains(tag) {
                    self.runtime.block_on(self.api.remove_image(
                        tag,
                        None::<bollard::query_parameters::RemoveImageOptions>,
                        None,
                    ))?;
                    removed.push(tag.clone());
                }
            }
        }
        Ok(removed)
    }

    /// Local image ID and repository digests, or `None` when absent.
    pub fn image(&self, reference: &str) -> Result<Option<(String, Vec<String>)>> {
        match self.runtime.block_on(self.api.inspect_image(reference)) {
            Ok(image) => Ok(Some((
                image.id.unwrap_or_default(),
                image.repo_digests.unwrap_or_default(),
            ))),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Build `tag` with BuildKit from a Containerfile and named context files,
    /// unless it already exists. Step names and `RUN` output stream to stderr;
    /// a failure carries the tail of the step output.
    pub fn build(&self, build: &Build<'_>) -> Result<()> {
        if self.image(build.tag)?.is_some() {
            eprintln!("kedra-lab: {} is cached", build.tag);
            return Ok(());
        }
        eprintln!("kedra-lab: building {} ({})", build.tag, build.what);
        let mut archive = tar::Builder::new(Vec::new());
        let mut append = |name: &str, data: &[u8], mode: u32| -> Result<()> {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_mtime(0);
            header.set_cksum();
            archive
                .append_data(&mut header, name, data)
                .map_err(Error::Io)
        };
        append("Dockerfile", &std::fs::read(build.containerfile)?, 0o644)?;
        for (name, path) in build.files {
            let metadata = std::fs::metadata(path)?;
            #[cfg(unix)]
            let mode = std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o777;
            #[cfg(not(unix))]
            let mode = 0o644;
            append(name, &std::fs::read(path)?, mode)?;
        }
        let context = archive.into_inner().map_err(Error::Io)?;
        let session = format!("kedra-lab-{}", crate::execution_id());
        let options = bollard::query_parameters::BuildImageOptions {
            dockerfile: "Dockerfile".into(),
            t: Some(build.tag.to_owned()),
            rm: true,
            buildargs: Some(
                build
                    .args
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), value.clone()))
                    .collect(),
            ),
            platform: build.platform.to_owned(),
            version: bollard::query_parameters::BuilderVersion::BuilderBuildKit,
            session: Some(session),
            ..Default::default()
        };
        let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        let mut remember = |line: String| {
            eprintln!("  | {line}");
            if tail.len() == 60 {
                tail.pop_front();
            }
            tail.push_back(line);
        };
        let outcome = self.runtime.block_on(crate::cancel::interrupt(async {
            let mut stream = self.api.build_image(
                options,
                None,
                Some(bollard::body_full(bytes::Bytes::from(context))),
            );
            let mut named = std::collections::HashSet::new();
            while let Some(item) = stream.next().await {
                let info = item?;
                if let Some(text) = info.stream {
                    for line in text.lines().filter(|line| !line.trim().is_empty()) {
                        remember(line.to_owned());
                    }
                }
                if let Some(bollard::models::BuildInfoAux::BuildKit(status)) = info.aux {
                    for vertex in status.vertexes {
                        if vertex.started.is_some() && named.insert(vertex.digest.clone()) {
                            remember(format!("> {}", vertex.name));
                        }
                        if !vertex.error.is_empty() {
                            remember(format!("! {}", vertex.error));
                        }
                    }
                    for log in status.logs {
                        for line in String::from_utf8_lossy(&log.msg).lines() {
                            if !line.trim().is_empty() {
                                remember(line.to_owned());
                            }
                        }
                    }
                }
                if let Some(detail) = info.error_detail {
                    return Err(Error::Docker(detail.message.unwrap_or_default()));
                }
            }
            Ok(())
        }))?;
        outcome.map_err(|error| Error::Command {
            what: format!("image build {}", build.tag),
            exit: 1,
            stderr: format!(
                "{error}\n{}",
                tail.iter().cloned().collect::<Vec<_>>().join("\n")
            ),
        })
    }

    /// Pull an image reference, printing coarse progress to stderr.
    pub fn pull(&self, reference: &str, platform: &str) -> Result<()> {
        let options = bollard::query_parameters::CreateImageOptions {
            from_image: Some(reference.to_owned()),
            platform: platform.to_owned(),
            ..Default::default()
        };
        eprintln!("kedra-lab: pulling {reference} ({platform})");
        self.runtime.block_on(crate::cancel::interrupt(async {
            let mut stream = self.api.create_image(Some(options), None, None);
            while let Some(item) = stream.next().await {
                item?;
            }
            Ok::<_, Error>(())
        }))?
    }
}
