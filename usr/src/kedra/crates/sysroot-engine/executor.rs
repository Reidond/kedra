//! Process execution. Package builds, roots and `run`/`develop` run in an
//! unprivileged bubblewrap sandbox; the Docker helpers below serve only image
//! tooling (RPM resolution, image import and native image composition).
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
/// A started process whose stdout and stderr are drained into bounded buffers.
pub(crate) struct Captured {
    child: std::process::Child,
    stdout: mpsc::Receiver<(Vec<u8>, bool)>,
    stderr: mpsc::Receiver<(Vec<u8>, bool)>,
}
pub(crate) fn spawn(mut cmd: Command) -> Result<Captured> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        Error::Invalid(format!(
            "cannot start required local program {}: {e}",
            cmd.get_program().to_string_lossy()
        ))
    })?;
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
    Ok(Captured {
        child,
        stdout,
        stderr,
    })
}
impl Captured {
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }
    /// Wait until the process exits or the deadline kills it, then collect output.
    pub(crate) fn wait(mut self, seconds: u64) -> Result<RunResult> {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = self.child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                timed_out = true;
                self.child.kill()?;
                break self.child.wait()?;
            }
            thread::sleep(Duration::from_millis(20));
        };
        let (out, ot) = self
            .stdout
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| Error::RecoveryRequired("stdout drain did not terminate".into()))?;
        let (err, et) = self
            .stderr
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
}
pub(crate) fn command(cmd: Command, seconds: u64) -> Result<RunResult> {
    spawn(cmd)?.wait(seconds)
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
        || !v["Architecture"]
            .as_str()
            .is_some_and(|a| a == crate::RPM_ARCHITECTURE || a == crate::OCI_ARCHITECTURE)
    {
        return Err(Error::Invalid(format!(
            "executor requires a native {PLATFORM} Docker daemon"
        )));
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
        || info.architecture != crate::OCI_ARCHITECTURE
        || info.config.volumes.as_ref().is_some_and(|v| !v.is_empty())
    {
        return Err(Error::Invalid(format!(
            "image {id} is not exact native {PLATFORM} without volumes"
        )));
    }
    Ok(info)
}

/// Bubblewrap program; an absolute path, so no inherited PATH chooses it.
const BWRAP: &str = "/usr/bin/bwrap";
/// Where a root build sees its offline RPM repository.
pub(crate) const REPOSITORY: &str = "/run/kedra-rpms";
/// Directories every root provides for the sandbox's own mounts.
pub(crate) const MOUNT_POINTS: [&str; 6] = [
    "dev",
    "proc",
    "tmp",
    "build",
    "run",
    "usr/lib/sysroot/store",
];

/// The running sandbox a journal names. Recovery identifies it by PID and
/// kernel start time within the same boot and PID namespace.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Process {
    pub pid: u32,
    pub start_time: u64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    pub schema: u32,
    pub store_token: String,
    pub nonce: String,
    pub boot_id: String,
    pub pid_namespace: String,
    pub process: Option<Process>,
    pub staging: String,
}

/// Run in the unpacked root of an exact store image (created on first use).
pub(crate) struct Execution<'a> {
    pub image: &'a str,
    pub mounts: &'a [String],
    pub output: Option<(&'a str, &'a Path)>,
    pub argv: &'a [String],
    pub env: &'a BTreeMap<String, String>,
    pub seconds: u64,
}
pub(crate) fn execute(store: &Store, exec: Execution<'_>) -> Result<RunResult> {
    let root = store.image_root(exec.image)?;
    sandbox(
        store,
        Sandbox {
            root: &store.root_tree(&root),
            writable_root: false,
            stage: exec.output.and_then(|(_, path)| path.parent()),
            mounts: exec.mounts,
            output: exec.output,
            repository: None,
            argv: exec.argv,
            env: exec.env,
            seconds: exec.seconds,
        },
    )
}

/// One unprivileged bubblewrap process: no network, no host files except the
/// root, read-only store inputs and at most one writable output directory.
pub(crate) struct Sandbox<'a> {
    pub root: &'a Path,
    /// Only a root build writes its own staging copy of a root.
    pub writable_root: bool,
    /// The transaction staging directory recovery removes after a crash.
    pub stage: Option<&'a Path>,
    pub mounts: &'a [String],
    pub output: Option<(&'a str, &'a Path)>,
    pub repository: Option<&'a Path>,
    pub argv: &'a [String],
    pub env: &'a BTreeMap<String, String>,
    pub seconds: u64,
}

pub(crate) fn sandbox(store: &Store, exec: Sandbox<'_>) -> Result<RunResult> {
    plan::timeout(exec.seconds)?;
    if exec.argv.is_empty() {
        return Err(Error::Invalid("empty command".into()));
    }
    if rustix::process::geteuid().is_root() {
        return Err(Error::Invalid("engine refuses root builders".into()));
    }
    let command = bwrap(store, &exec)?;
    let mut journal = Journal {
        schema: 3,
        store_token: store.token.clone(),
        nonce: crate::store::nonce()?,
        boot_id: boot_id()?,
        pid_namespace: pid_namespace()?,
        process: None,
        staging: exec
            .stage
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned(),
    };
    let journal_path = store.path().join("transactions/execution.json");
    crate::store::write_json_new(&journal_path, &journal)?;
    let result = (|| {
        let running = spawn(command)?;
        let pid = running.id();
        // The PID is recorded only while the process exists; --die-with-parent
        // ends the sandbox if this process dies before or after the record.
        journal.process = start_time(pid)?.map(|start_time| Process { pid, start_time });
        crate::store::atomic_json(&journal_path, &journal)?;
        running.wait(exec.seconds)
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

fn bwrap(store: &Store, exec: &Sandbox<'_>) -> Result<Command> {
    let mut cmd = Command::new(BWRAP);
    cmd.env_clear();
    cmd.args([
        "--unshare-all",
        "--unshare-user",
        "--disable-userns",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--hostname",
        "sysroot",
    ]);
    cmd.arg(if exec.writable_root {
        "--bind"
    } else {
        "--ro-bind"
    })
    .arg(exec.root)
    .arg("/");
    cmd.args(["--dev", "/dev", "--proc", "/proc"]);
    cmd.args([
        "--tmpfs",
        "/tmp",
        "--tmpfs",
        "/build",
        "--tmpfs",
        LOGICAL_PREFIX,
    ]);
    for id in exec.mounts {
        plan::object_id(id)?;
        cmd.arg("--ro-bind")
            .arg(store.object_path(id).join("data"))
            .arg(format!("{LOGICAL_PREFIX}/{id}"));
    }
    if let Some((id, path)) = exec.output {
        plan::object_id(id)?;
        cmd.arg("--bind")
            .arg(path)
            .arg(format!("{LOGICAL_PREFIX}/{id}"));
    }
    cmd.args(["--remount-ro", LOGICAL_PREFIX]);
    if let Some(repository) = exec.repository {
        cmd.arg("--ro-bind").arg(repository).arg(REPOSITORY);
    }
    cmd.args(["--chdir", "/build"]);
    let path = if exec.writable_root {
        "/usr/bin:/usr/sbin"
    } else {
        "/usr/bin:/bin"
    };
    for (key, value) in [
        ("HOME", "/build"),
        ("TMPDIR", "/tmp"),
        ("PATH", path),
        ("LANG", "C"),
        ("LC_ALL", "C"),
        ("TZ", "UTC"),
    ]
    .into_iter()
    .chain(exec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    {
        cmd.args(["--setenv", key, value]);
    }
    cmd.arg("--").args(exec.argv);
    Ok(cmd)
}

fn boot_id() -> Result<String> {
    let id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    let id = id.trim();
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err(Error::Invalid("unexpected kernel boot identity".into()));
    }
    Ok(id.into())
}

fn pid_namespace() -> Result<String> {
    std::fs::read_link("/proc/self/ns/pid")?
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid("unexpected PID namespace identity".into()))
}

/// Kernel start time of a live bubblewrap process, or None once it is gone.
fn start_time(pid: u32) -> Result<Option<u64>> {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let (head, tail) = stat
        .rsplit_once(')')
        .ok_or_else(|| Error::Invalid("unexpected process status".into()))?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    // Field 3 (state) is the first after the command name; start time is field 22.
    if fields.first() == Some(&"Z") || !head.ends_with("(bwrap") {
        return Ok(None);
    }
    fields
        .get(19)
        .and_then(|s| s.parse().ok())
        .map(Some)
        .ok_or_else(|| Error::Invalid("unexpected process status".into()))
}

/// Make sure the journaled sandbox no longer runs: it is gone, belongs to an
/// earlier boot, or is killed and observed gone within ten seconds.
pub(crate) fn cleanup(store: &Store, journal: &Journal) -> Result<()> {
    if journal.schema != 3 || journal.store_token != store.token || !plan::hex(&journal.nonce) {
        return Err(Error::RecoveryRequired(
            "execution journal ownership or schema differs".into(),
        ));
    }
    let Some(process) = &journal.process else {
        return Ok(());
    };
    if journal.boot_id != boot_id()? {
        return Ok(());
    }
    if journal.pid_namespace != pid_namespace()? {
        return Err(Error::RecoveryRequired(
            "sandbox ran in another PID namespace of this boot; recover there".into(),
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while start_time(process.pid)? == Some(process.start_time) {
        if Instant::now() >= deadline {
            return Err(Error::RecoveryRequired(format!(
                "sandbox process {} survived termination",
                process.pid
            )));
        }
        let pid = i32::try_from(process.pid)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
            .ok_or_else(|| Error::RecoveryRequired("invalid sandbox process".into()))?;
        match rustix::process::kill_process(pid, rustix::process::Signal::KILL) {
            Ok(()) | Err(rustix::io::Errno::SRCH) => {}
            Err(error) => return Err(error.into()),
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}
