//! Owned data-only frontend process. Never runs an author executable.
use std::io::{Read, Write};
use std::process::{Command, ExitCode, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use sysroot_engine::{Error, Result};

const OUTPUT_LIMIT: usize = 56 * 1024 * 1024;
const DIAGNOSTIC_LIMIT: usize = 1024 * 1024;
const MEMORY_LIMIT: u64 = 512 * 1024 * 1024;
static WORKER: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_worker() -> bool {
    WORKER.load(Ordering::Relaxed)
}

pub(crate) fn worker_limits() -> Result<()> {
    WORKER.store(true, Ordering::Relaxed);
    // Linux enforces address-space allocation. macOS has no effective equivalent;
    // its owner observes resident memory every 100ms and kills/reaps on excess.
    #[cfg(target_os = "linux")]
    rustix::process::setrlimit(
        rustix::process::Resource::As,
        rustix::process::Rlimit {
            current: Some(MEMORY_LIMIT),
            maximum: Some(MEMORY_LIMIT),
        },
    )?;
    rustix::process::setrlimit(
        rustix::process::Resource::Core,
        rustix::process::Rlimit {
            current: Some(0),
            maximum: Some(0),
        },
    )?;
    let owner = rustix::process::getppid();
    std::thread::spawn(move || {
        let started = Instant::now();
        loop {
            if rustix::process::getppid() != owner || started.elapsed() >= Duration::from_secs(60) {
                std::process::exit(78);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    Ok(())
}

fn capture(
    mut stream: impl Read + Send + 'static,
    limit: usize,
) -> mpsc::Receiver<Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut bytes = Vec::new();
            stream
                .by_ref()
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                return Err(Error::Invalid(
                    "frontend output/diagnostic limit exceeded".into(),
                ));
            }
            Ok(bytes)
        })();
        let _ = sender.send(result);
    });
    receiver
}

#[cfg(target_os = "macos")]
fn memory_exceeded(pid: u32) -> Result<bool> {
    let output = Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Ok(false); // Child may have just exited; try_wait settles it.
    }
    let rss = std::str::from_utf8(&output.stdout)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .ok_or_else(|| Error::Invalid("cannot observe frontend memory".into()))?;
    Ok(rss > MEMORY_LIMIT / 1024)
}

#[cfg(not(target_os = "macos"))]
fn memory_exceeded(_pid: u32) -> Result<bool> {
    Ok(false)
}

fn supervised(mut command: Command, request: Option<&[u8]>) -> Result<Output> {
    command
        .stdin(if request.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = capture(
        child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid("missing frontend stdout".into()))?,
        OUTPUT_LIMIT,
    );
    let stderr = capture(
        child
            .stderr
            .take()
            .ok_or_else(|| Error::Invalid("missing frontend stderr".into()))?,
        DIAGNOSTIC_LIMIT,
    );
    let result = (|| {
        if let Some(request) = request {
            child
                .stdin
                .take()
                .ok_or_else(|| Error::Invalid("missing frontend stdin".into()))?
                .write_all(request)?;
        }
        let started = Instant::now();
        let mut bytes = None;
        let mut diagnostics = None;
        loop {
            if bytes.is_none()
                && let Ok(result) = stdout.try_recv()
            {
                bytes = Some(result?);
            }
            if diagnostics.is_none()
                && let Ok(result) = stderr.try_recv()
            {
                diagnostics = Some(result?);
            }
            if let Some(status) = child.try_wait()? {
                return Ok(Output {
                    status,
                    stdout: match bytes {
                        Some(bytes) => bytes,
                        None => stdout
                            .recv()
                            .map_err(|_| Error::Invalid("frontend reader failed".into()))??,
                    },
                    stderr: match diagnostics {
                        Some(bytes) => bytes,
                        None => stderr.recv().map_err(|_| {
                            Error::Invalid("frontend diagnostic reader failed".into())
                        })??,
                    },
                });
            }
            if started.elapsed() >= Duration::from_secs(60) || memory_exceeded(child.id())? {
                return Err(Error::Invalid(
                    "frontend deadline/memory limit exceeded".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

pub(crate) fn planning() -> Result<ExitCode> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--frontend-process")
        .args(std::env::args_os().skip(1));
    let output = supervised(command, None)?;
    std::io::stderr().lock().write_all(&output.stderr)?;
    if !output.status.success() {
        return Err(Error::Invalid(
            "frontend refused input or exceeded its process limit".into(),
        ));
    }
    std::io::stdout().lock().write_all(&output.stdout)?;
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn input(request: &[u8]) -> Result<Vec<u8>> {
    if request.len() > 65536 {
        return Err(Error::Invalid("frontend request limit exceeded".into()));
    }
    let mut command = Command::new(std::env::current_exe()?);
    command.args(["--frontend-process", "frontend-input"]);
    let output = supervised(command, Some(request))?;
    if !output.status.success() {
        std::io::stderr().lock().write_all(&output.stderr)?;
        return Err(Error::Invalid(
            "frontend refused input or exceeded its process limit".into(),
        ));
    }
    Ok(output.stdout)
}
