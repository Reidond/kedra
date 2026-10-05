use crate::{Error, LOGICAL_PREFIX, PLATFORM, Result, RunResult, Store, plan};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
const CAP: usize = 1024 * 1024;
#[derive(Clone)]
pub(crate) struct Connection {
    endpoint: String,
}
impl Connection {
    pub(crate) fn resolve() -> Result<Self> {
        if ["DOCKER_TLS", "DOCKER_TLS_VERIFY"]
            .iter()
            .any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()))
        {
            return Err(Error::Invalid(
                "the engine supports a local Unix Docker socket only, without TLS".into(),
            ));
        }
        let context = std::env::var("DOCKER_CONTEXT")
            .ok()
            .filter(|s| !s.is_empty());
        let endpoint = if context.is_none() {
            std::env::var("DOCKER_HOST").ok().filter(|s| !s.is_empty())
        } else {
            None
        };
        let endpoint = match endpoint {
            Some(endpoint) => endpoint,
            None => {
                let context = match context {
                    Some(context) => context,
                    None => {
                        let mut cmd = Command::new("docker");
                        cmd.args(["context", "show"]);
                        let result = checked(command(cmd, 300)?)?;
                        String::from_utf8(result.stdout)
                            .map_err(|_| Error::Invalid("Docker context is not UTF-8".into()))?
                            .trim()
                            .to_owned()
                    }
                };
                if context.is_empty()
                    || context.starts_with('-')
                    || context.bytes().any(|b| b < 32 || b == 127)
                {
                    return Err(Error::Invalid("unsafe Docker context name".into()));
                }
                let mut cmd = Command::new("docker");
                cmd.env_remove("DOCKER_HOST")
                    .args(["context", "inspect", &context]);
                let result = checked(command(cmd, 300)?)?;
                let values: Vec<serde_json::Value> = serde_json::from_slice(&result.stdout)?;
                values
                    .first()
                    .and_then(|v| v["Endpoints"]["docker"]["Host"].as_str())
                    .ok_or_else(|| Error::Invalid("Docker context has no endpoint".into()))?
                    .to_owned()
            }
        };
        let connection = Self::recorded(&endpoint)?;
        let path = std::fs::canonicalize(connection.endpoint.trim_start_matches("unix://"))?;
        let path = path
            .to_str()
            .ok_or_else(|| Error::Invalid("Docker socket path is not UTF-8".into()))?;
        Self::recorded(&format!("unix://{path}"))
    }
    fn recorded(endpoint: &str) -> Result<Self> {
        let path = endpoint.strip_prefix("unix://").ok_or_else(|| {
            Error::Invalid("only a local Unix Docker endpoint is supported".into())
        })?;
        if !Path::new(path).is_absolute()
            || path.bytes().any(|b| b < 32 || b == 127)
            || Path::new(path)
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(Error::Invalid("invalid local Docker endpoint".into()));
        }
        Ok(Self {
            endpoint: endpoint.into(),
        })
    }
    pub(crate) fn command(&self) -> Command {
        let mut command = Command::new("docker");
        for key in [
            "DOCKER_CONTEXT",
            "DOCKER_HOST",
            "DOCKER_TLS",
            "DOCKER_TLS_VERIFY",
            "DOCKER_CERT_PATH",
        ] {
            command.env_remove(key);
        }
        command.args(["--host", &self.endpoint]);
        command
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    pub schema: u32,
    pub store_token: String,
    pub daemon: String,
    pub endpoint: String,
    pub nonce: String,
    pub container: String,
    pub staging: String,
}
#[derive(Deserialize)]
pub(crate) struct ImageInfo {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Architecture")]
    pub architecture: String,
    #[serde(rename = "Os")]
    pub os: String,
    #[serde(rename = "Config")]
    pub config: ImageConfig,
}
#[derive(Deserialize)]
pub(crate) struct ImageConfig {
    #[serde(rename = "Volumes")]
    pub volumes: Option<BTreeMap<String, serde_json::Value>>,
}
fn reader(mut stream: impl Read + Send + 'static) -> mpsc::Receiver<(Vec<u8>, bool)> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut data = Vec::new();
        let mut truncated = false;
        let mut buf = [0u8; 16384];
        loop {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let keep = n.min(CAP - data.len());
                    data.extend_from_slice(&buf[..keep]);
                    truncated |= keep < n;
                }
            }
        }
        let _ = tx.send((data, truncated));
    });
    rx
}
pub(crate) fn command(mut cmd: Command, seconds: u64) -> Result<RunResult> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Invalid(format!("cannot start required local Docker/process: {e}")))?;
    let stdout = reader(
        child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid("missing stdout pipe".into()))?,
    );
    let stderr = reader(
        child
            .stderr
            .take()
            .ok_or_else(|| Error::Invalid("missing stderr pipe".into()))?,
    );
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            child.kill()?;
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let (out, ot) = stdout
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| Error::RecoveryRequired("stdout drain did not terminate".into()))?;
    let (err, et) = stderr
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| Error::RecoveryRequired("stderr drain did not terminate".into()))?;
    if timed_out {
        return Err(Error::Process {
            code: 124,
            message: format!("deadline exceeded after {seconds} seconds"),
        });
    }
    Ok(RunResult {
        code: status.code().unwrap_or(128),
        stdout: out,
        stderr: err,
        stdout_truncated: ot,
        stderr_truncated: et,
    })
}
fn docker(connection: &Connection, args: &[&str]) -> Result<RunResult> {
    let mut cmd = connection.command();
    cmd.args(args);
    command(cmd, 300)
}
pub(crate) fn checked(result: RunResult) -> Result<RunResult> {
    if result.code != 0 {
        return Err(Error::Process {
            code: result.code,
            message: String::from_utf8_lossy(&result.stderr).into_owned(),
        });
    }
    if result.stdout_truncated {
        return Err(Error::Invalid(
            "Docker control response exceeds 1 MiB".into(),
        ));
    }
    Ok(result)
}
pub(crate) fn daemon(connection: &Connection) -> Result<String> {
    let r = checked(docker(connection, &["info", "--format", "{{json .}}"])?)?;
    let v: serde_json::Value = serde_json::from_slice(&r.stdout)?;
    if v["OSType"].as_str() != Some("linux")
        || !matches!(v["Architecture"].as_str(), Some("aarch64" | "arm64"))
    {
        return Err(Error::Invalid(
            "executor requires native aarch64 Linux Docker daemon".into(),
        ));
    }
    let id = v["ID"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Invalid("Docker daemon has no identity".into()))?;
    Ok(id.into())
}
pub(crate) fn inspect_image(connection: &Connection, id: &str) -> Result<ImageInfo> {
    plan::image_id(id)?;
    let result = checked(docker(connection, &["image", "inspect", id])?)?;
    let infos: Vec<ImageInfo> = serde_json::from_slice(&result.stdout)?;
    let info = infos
        .into_iter()
        .next()
        .ok_or_else(|| Error::Invalid("image inspection empty".into()))?;
    if info.id != id
        || info.os != "linux"
        || info.architecture != "arm64"
        || info.config.volumes.as_ref().is_some_and(|v| !v.is_empty())
    {
        return Err(Error::Invalid(format!(
            "image {id} is not exact native {PLATFORM} without volumes"
        )));
    }
    Ok(info)
}
pub(crate) fn ensure_image(store: &Store, connection: &Connection, id: &str) -> Result<()> {
    store.verify_image(id)?;
    daemon(connection)?;
    if inspect_image(connection, id).is_err() {
        let path = store.image_path(id).join("image.tar");
        let mut cmd = connection.command();
        cmd.args(["image", "load", "--input"]).arg(path);
        checked(command(cmd, 300)?)?;
        inspect_image(connection, id)?;
    }
    Ok(())
}
pub(crate) struct Execution<'a> {
    pub image: &'a str,
    pub mounts: &'a [String],
    pub output: Option<(&'a str, &'a Path)>,
    pub argv: &'a [String],
    pub env: &'a BTreeMap<String, String>,
    pub seconds: u64,
}
pub(crate) fn execute(store: &Store, exec: Execution<'_>) -> Result<RunResult> {
    plan::timeout(exec.seconds)?;
    let connection = store.connection()?;
    ensure_image(store, &connection, exec.image)?;
    if exec.argv.is_empty() {
        return Err(Error::Invalid("empty command".into()));
    }
    let nonce = crate::store::nonce()?;
    let container = format!("sysroot-engine-{}-{nonce}", &store.token[..12]);
    let staging = exec
        .output
        .map(|(_, p)| {
            p.parent()
                .and_then(Path::file_name)
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned()
        })
        .unwrap_or_default();
    let journal = Journal {
        schema: 2,
        store_token: store.token.clone(),
        daemon: daemon(&connection)?,
        endpoint: connection.endpoint.clone(),
        nonce,
        container,
        staging,
    };
    let journal_path = store.path().join("transactions/execution.json");
    crate::store::write_json_new(&journal_path, &journal)?;
    let result = (|| {
        let uid = rustix::process::geteuid().as_raw();
        if uid == 0 {
            return Err(Error::Invalid("engine refuses root builders".into()));
        }
        let mut cmd = connection.command();
        cmd.args([
            "create",
            "--pull",
            "never",
            "--name",
            &journal.container,
            "--label",
            &format!("sysroot.engine.store={}", store.token),
            "--label",
            &format!("sysroot.engine.operation={}", journal.nonce),
            "--platform",
            "linux/arm64",
            "--read-only",
            "--no-healthcheck",
            "--network",
            "none",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--pids-limit",
            "256",
            "--memory",
            "1g",
            "--memory-swap",
            "1g",
            "--ipc",
            "private",
            "--user",
            &format!("{uid}:{uid}"),
            "--tmpfs",
            "/tmp:rw,nosuid,nodev,size=268435456,mode=1777",
            "--tmpfs",
            "/build:rw,nosuid,nodev,size=268435456,mode=1777",
            "--workdir",
            "/build",
            "--env",
            "HOME=/build",
            "--env",
            "TMPDIR=/tmp",
            "--env",
            "PATH=/usr/bin:/bin",
            "--env",
            "LANG=C",
            "--env",
            "LC_ALL=C",
            "--env",
            "TZ=UTC",
        ]);
        for id in exec.mounts {
            plan::object_id(id)?;
            cmd.args([
                "--mount",
                &format!(
                    "type=bind,src={},dst={LOGICAL_PREFIX}/{id},readonly",
                    store.object_path(id).join("data").display()
                ),
            ]);
        }
        if let Some((id, path)) = exec.output {
            cmd.args([
                "--mount",
                &format!("type=bind,src={},dst={LOGICAL_PREFIX}/{id}", path.display()),
            ]);
        }
        for (key, value) in exec.env {
            cmd.args(["--env", &format!("{key}={value}")]);
        }
        cmd.args(["--entrypoint", &exec.argv[0], exec.image])
            .args(&exec.argv[1..]);
        checked(command(cmd, 300)?)?;
        let mut cmd = connection.command();
        cmd.args(["start", "--attach", &journal.container]);
        command(cmd, exec.seconds)
    })();
    if let Err(error) = cleanup(store, &journal) {
        return Err(Error::RecoveryRequired(format!(
            "{error}; journal retained at {}",
            journal_path.display()
        )));
    }
    std::fs::remove_file(&journal_path)?;
    File::open(
        journal_path
            .parent()
            .ok_or_else(|| Error::Invalid("journal parent missing".into()))?,
    )?
    .sync_all()?;
    result
}
pub(crate) fn cleanup(store: &Store, journal: &Journal) -> Result<()> {
    let connection = &Connection::recorded(&journal.endpoint)?;
    if journal.schema != 2
        || journal.store_token != store.token
        || journal.daemon != daemon(connection)?
        || journal.container != format!("sysroot-engine-{}-{}", &store.token[..12], journal.nonce)
    {
        return Err(Error::RecoveryRequired(
            "journal ownership or Docker daemon differs".into(),
        ));
    }
    if !plan::hex(&journal.nonce) {
        return Err(Error::RecoveryRequired("invalid execution nonce".into()));
    }
    let result = docker(connection, &["container", "inspect", &journal.container])?;
    if result.code != 0 {
        let listing = checked(docker(
            connection,
            &["container", "ls", "--all", "--format", "{{.Names}}"],
        )?)?;
        if String::from_utf8_lossy(&listing.stdout)
            .lines()
            .any(|n| n == journal.container)
        {
            return Err(Error::RecoveryRequired(
                "cannot inspect possibly existing container".into(),
            ));
        }
        return Ok(());
    }
    if result.stdout_truncated {
        return Err(Error::RecoveryRequired(
            "container inspection truncated".into(),
        ));
    }
    let values: Vec<serde_json::Value> = serde_json::from_slice(&result.stdout)?;
    let info = values
        .first()
        .ok_or_else(|| Error::RecoveryRequired("container inspection empty".into()))?;
    if info["Name"].as_str() != Some(format!("/{}", journal.container).as_str())
        || info["Config"]["Labels"]["sysroot.engine.store"].as_str() != Some(store.token.as_str())
        || info["Config"]["Labels"]["sysroot.engine.operation"].as_str()
            != Some(journal.nonce.as_str())
    {
        return Err(Error::RecoveryRequired(
            "container ownership labels differ".into(),
        ));
    }
    let id = info["Id"]
        .as_str()
        .ok_or_else(|| Error::RecoveryRequired("container missing identity".into()))?;
    checked(docker(connection, &["container", "rm", "--force", id])?)?;
    let listing = checked(docker(
        connection,
        &[
            "container",
            "ls",
            "--all",
            "--no-trunc",
            "--format",
            "{{.ID}}",
        ],
    )?)?;
    if String::from_utf8_lossy(&listing.stdout)
        .lines()
        .any(|s| s == id)
    {
        return Err(Error::RecoveryRequired(
            "container still exists after removal".into(),
        ));
    }
    Ok(())
}
