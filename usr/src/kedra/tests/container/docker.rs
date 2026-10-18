//! Docker Engine operations on containers the harness already owns.
//!
//! Testcontainers creates, starts and removes containers. Operations that must
//! also work on a retained lab container from a later process (exec, file
//! transfer, listing by ownership label) use the Engine API it re-exports.
//!
//! Ownership is per checkout (AGENTS.md "Artifacts stay in the checkout"):
//! every container, volume and image the harness builds carries
//! `kedra.checkout=<id>`, and images it names get a tag scoped to that id, so
//! one checkout never reuses or removes another checkout's objects.

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
/// The checkout that created a container, volume or image ([`crate::checkout_id`]).
pub const CHECKOUT_LABEL: &str = "kedra.checkout";

/// An image tag scoped to this checkout: `<checkout id>-<key>`. Images the
/// engine derives cannot carry a label without changing the image under test,
/// so `kedra-lab clean` finds them by this tag.
pub fn scoped_tag(key: &str) -> String {
    format!("{}-{key}", crate::checkout_id())
}

/// What [`Docker::clean_checkout`] removed.
#[derive(Debug, Default)]
pub struct Cleaned {
    pub containers: Vec<String>,
    pub images: Vec<String>,
    pub volumes: Vec<String>,
}

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
        let tag = scoped_tag(digest);
        let reference = format!("kedra-lab-base:{tag}");
        if let Some((found, _)) = self.image(&reference)? {
            if found != id {
                return Err(invalid("local base cache tag points at another image"));
            }
        } else {
            let options = bollard::query_parameters::TagImageOptions {
                repo: Some("kedra-lab-base".into()),
                tag: Some(tag),
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

    /// This checkout's harness containers, optionally narrowed by extra label filters.
    pub fn owned(&self, filters: &[(&str, &str)]) -> Result<Vec<Owned>> {
        let mut labels = vec![
            format!("{OWNER_LABEL}={OWNER}"),
            format!("{CHECKOUT_LABEL}={}", crate::checkout_id()),
        ];
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
            tokio::time::timeout(Duration::from_secs(30), async {
                loop {
                    match self.api.remove_container(id, Some(options.clone())).await {
                        Err(bollard::errors::Error::DockerResponseServerError {
                            status_code: 409,
                            message,
                        }) if message.contains("removal")
                            && message.contains("already in progress") =>
                        {
                            // Testcontainers' signal watchdog may be removing the same ID.
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        outcome => break outcome,
                    }
                }
            })
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

    /// A named volume owned by this checkout, created with its labels when
    /// absent. A same-named volume without this checkout's label is refused.
    pub fn volume(&self, name: &str) -> Result<String> {
        let checkout = crate::checkout_id();
        let request = bollard::models::VolumeCreateRequest {
            name: Some(name.to_owned()),
            labels: Some(HashMap::from([
                (OWNER_LABEL.to_owned(), OWNER.to_owned()),
                (CHECKOUT_LABEL.to_owned(), checkout.to_owned()),
            ])),
            ..Default::default()
        };
        // The Engine returns an existing volume unchanged, labels included.
        let volume = self.runtime.block_on(self.api.create_volume(request))?;
        if volume.labels.get(CHECKOUT_LABEL).map(String::as_str) != Some(checkout) {
            return Err(invalid(format!(
                "volume {name} exists without {CHECKOUT_LABEL}={checkout}; remove it with `docker volume rm {name}`"
            )));
        }
        Ok(volume.name)
    }

    /// Remove every container, image and volume this checkout's label selects,
    /// and this checkout's scoped tags of unlabeled images. Other checkouts'
    /// objects and pulled images are never selected.
    pub fn clean_checkout(&self) -> Result<Cleaned> {
        let checkout = crate::checkout_id();
        let label = HashMap::from([(
            "label".to_owned(),
            vec![format!("{CHECKOUT_LABEL}={checkout}")],
        )]);
        let mut cleaned = Cleaned::default();
        let containers = self.runtime.block_on(self.api.list_containers(Some(
            bollard::query_parameters::ListContainersOptions {
                all: true,
                filters: Some(label.clone()),
                ..Default::default()
            },
        )))?;
        for container in containers {
            let id = container.id.unwrap_or_default();
            self.remove(&id)?;
            cleaned.containers.push(
                container
                    .names
                    .and_then(|names| names.into_iter().next())
                    .map(|name| name.trim_start_matches('/').to_owned())
                    .unwrap_or(id),
            );
        }
        cleaned.images = self.remove_checkout_images(&label, checkout)?;
        let volumes = self.runtime.block_on(self.api.list_volumes(Some(
            bollard::query_parameters::ListVolumesOptions {
                filters: Some(label),
            },
        )))?;
        for volume in volumes.volumes.unwrap_or_default() {
            self.runtime.block_on(self.api.remove_volume(
                &volume.name,
                None::<bollard::query_parameters::RemoveVolumeOptions>,
            ))?;
            cleaned.volumes.push(volume.name);
        }
        Ok(cleaned)
    }

    /// Labeled images by ID, then the scoped tags of unlabeled ones. A child
    /// image can hold its parent, so retry until a pass removes nothing.
    fn remove_checkout_images(
        &self,
        label: &HashMap<String, Vec<String>>,
        checkout: &str,
    ) -> Result<Vec<String>> {
        let scope = format!(":{checkout}-");
        let mut removed = Vec::new();
        loop {
            let mut selected = Vec::new();
            for image in self.runtime.block_on(self.api.list_images(Some(
                bollard::query_parameters::ListImagesOptions {
                    all: true,
                    filters: Some(label.clone()),
                    ..Default::default()
                },
            )))? {
                selected.push((image.id, true));
            }
            for image in self.runtime.block_on(
                self.api
                    .list_images(None::<bollard::query_parameters::ListImagesOptions>),
            )? {
                selected.extend(
                    image
                        .repo_tags
                        .into_iter()
                        .filter(|tag| tag.starts_with("kedra-") && tag.contains(&scope))
                        .map(|tag| (tag, false)),
                );
            }
            let mut last_error = None;
            let before = removed.len();
            for (reference, labeled) in selected {
                // A labeled image may carry several of this checkout's tags.
                let options = bollard::query_parameters::RemoveImageOptions {
                    force: labeled,
                    ..Default::default()
                };
                match self
                    .runtime
                    .block_on(self.api.remove_image(&reference, Some(options), None))
                {
                    Ok(_) => removed.push(reference),
                    Err(bollard::errors::Error::DockerResponseServerError {
                        status_code: 404,
                        ..
                    }) => {}
                    Err(error) => last_error = Some(error),
                }
            }
            match last_error {
                None => return Ok(removed),
                Some(error) if removed.len() == before => return Err(error.into()),
                Some(_) => {}
            }
        }
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
        self.build_internal(build, true)
    }

    /// Re-run the controller-owned networked fixture recipe instead of adopting a tag.
    pub(crate) fn build_fixture(&self, build: &Build<'_>) -> Result<()> {
        self.build_internal(build, false)
    }

    fn build_internal(&self, build: &Build<'_>, reuse_tag: bool) -> Result<()> {
        if reuse_tag && self.image(build.tag)?.is_some() {
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
            labels: Some(HashMap::from([
                (OWNER_LABEL.to_owned(), OWNER.to_owned()),
                (CHECKOUT_LABEL.to_owned(), crate::checkout_id().to_owned()),
            ])),
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

    pub fn inspect_image(&self, reference: &str) -> Result<bollard::models::ImageInspect> {
        Ok(self.runtime.block_on(self.api.inspect_image(reference))?)
    }

    /// Direct isolated process on this exact API endpoint; no pull or entrypoint inheritance.
    pub fn isolated(&self, image: &str, argv: Vec<String>) -> Result<Output> {
        self.isolated_process(image, argv, false)
    }

    /// Native inspectors get bounded guest scratch, never a host path or retained volume.
    pub(crate) fn isolated_native(&self, image: &str, argv: Vec<String>) -> Result<Output> {
        self.isolated_process(image, argv, true)
    }

    fn isolated_process(&self, image: &str, argv: Vec<String>, scratch: bool) -> Result<Output> {
        if argv.is_empty() {
            return Err(invalid("empty replay command"));
        }
        let config = bollard::models::ContainerCreateBody {
            image: Some(image.into()),
            entrypoint: Some(argv),
            cmd: Some(Vec::new()),
            user: Some("0:0".into()),
            working_dir: Some("/".into()),
            labels: Some(HashMap::from([
                (OWNER_LABEL.into(), OWNER.into()),
                (CHECKOUT_LABEL.into(), crate::checkout_id().into()),
                (KIND_LABEL.into(), "composition-probe".into()),
            ])),
            host_config: Some(bollard::models::HostConfig {
                network_mode: Some("none".into()),
                readonly_rootfs: Some(true),
                tmpfs: scratch.then(|| {
                    HashMap::from([(
                        "/tmp".into(),
                        "rw,nosuid,nodev,noexec,size=64m,mode=1777".into(),
                    )])
                }),
                cap_drop: Some(vec!["ALL".into()]),
                security_opt: Some(vec!["no-new-privileges".into()]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let created = self.runtime.block_on(self.api.create_container(
            None::<bollard::query_parameters::CreateContainerOptions>,
            config,
        ))?;
        let started = Instant::now();
        let outcome = self.runtime.block_on(crate::cancel::interrupt(async {
            tokio::time::timeout(Duration::from_secs(120), async {
                self.api
                    .start_container(
                        &created.id,
                        None::<bollard::query_parameters::StartContainerOptions>,
                    )
                    .await?;
                let mut wait = self.api.wait_container(
                    &created.id,
                    None::<bollard::query_parameters::WaitContainerOptions>,
                );
                let status = wait
                    .next()
                    .await
                    .ok_or_else(|| invalid("container wait returned no status"))??;
                let mut logs = self.api.logs(
                    &created.id,
                    Some(bollard::query_parameters::LogsOptions {
                        stdout: true,
                        stderr: true,
                        ..Default::default()
                    }),
                );
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                while let Some(item) = logs.next().await {
                    match item? {
                        LogOutput::StdOut { message } | LogOutput::Console { message } => {
                            stdout.extend_from_slice(&message)
                        }
                        LogOutput::StdErr { message } => stderr.extend_from_slice(&message),
                        LogOutput::StdIn { .. } => {}
                    }
                    if stdout.len() + stderr.len() > 32 * 1024 * 1024 {
                        return Err(invalid("replay output exceeds 32 MiB"));
                    }
                }
                Ok::<_, Error>(Output {
                    exit: status.status_code,
                    stdout,
                    stderr,
                    duration: started.elapsed(),
                })
            })
            .await
            .map_err(|_| Error::Timeout {
                what: "composition process".into(),
                after: Duration::from_secs(120),
            })?
        }));
        let cleanup = self.remove(&created.id);
        let result = outcome??;
        cleanup?;
        Ok(result)
    }

    /// `docker image save` of one local image, for importing it without a daemon.
    pub fn save(&self, image: &str) -> Result<Vec<u8>> {
        const LIMIT: usize = 256 * 1024 * 1024;
        self.runtime.block_on(crate::cancel::interrupt(async {
            let mut stream = self.api.export_image(image);
            let mut archive = Vec::new();
            while let Some(chunk) = stream.next().await {
                archive.extend_from_slice(&chunk?);
                if archive.len() > LIMIT {
                    return Err(invalid("saved image exceeds 256 MiB"));
                }
            }
            Ok::<_, Error>(archive)
        }))?
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
        }))??;
        // Pulled images stay unlabeled (a label would change the image under
        // test); record them so they can be removed once no checkout uses them.
        let root = crate::artifact_root();
        std::fs::create_dir_all(&root)?;
        use std::io::Write;
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join("pulled-images.txt"))?,
            "{reference} {platform}"
        )?;
        Ok(())
    }
}
