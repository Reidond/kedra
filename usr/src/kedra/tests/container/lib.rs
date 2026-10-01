//! Container end-to-end harness and local lab for Kedra OS images.
//!
//! Development only: nothing here enters the OS image, installer or release.
//! Testcontainers provisions and removes containers; this crate prepares the
//! image under test, starts the real systemd and desktop session inside it and
//! runs declarative scenarios and native tests against it. Boot, firmware,
//! SELinux enforcement and bootc deployment changes stay in the VM workflows.

pub mod actions;
pub mod builder;
pub mod cancel;
pub mod capture;
pub mod composition;
pub mod docker;
pub mod environment;
pub mod image;
pub mod lab_sync;
pub mod localbuild;
pub mod report;
pub mod scenario;
pub mod session;
pub mod vm;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Harness failure with enough context to act on without a debugger.
#[derive(Debug)]
pub enum Error {
    Interrupted,
    /// Docker Engine API or Testcontainers failure.
    Docker(String),
    Io(std::io::Error),
    /// A command inside a container or on the host did not succeed.
    Command {
        what: String,
        exit: i64,
        stderr: String,
    },
    /// A bounded wait or operation ran out of time.
    Timeout {
        what: String,
        after: Duration,
    },
    /// Invalid scenario, option or state, detected before or during a run.
    Invalid(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Interrupted => f.write_str("execution interrupted"),
            Error::Docker(message) => write!(f, "container runtime: {message}"),
            Error::Io(error) => write!(f, "{error}"),
            Error::Command { what, exit, stderr } => {
                write!(f, "{what} exited with {exit}")?;
                let stderr = stderr.trim();
                if !stderr.is_empty() {
                    // Tracebacks and final errors are at the end.
                    write!(f, ": {}", clip_tail(stderr, 2000))?;
                }
                Ok(())
            }
            Error::Timeout { what, after } => write!(f, "{what} timed out after {after:?}"),
            Error::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<testcontainers::TestcontainersError> for Error {
    fn from(error: testcontainers::TestcontainersError) -> Self {
        Error::Docker(error.to_string())
    }
}

impl From<testcontainers::bollard::errors::Error> for Error {
    fn from(error: testcontainers::bollard::errors::Error) -> Self {
        Error::Docker(error.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

/// Keep the head of long diagnostics; the full text goes to artifacts.
pub fn clip(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… ({} bytes total)", &text[..end], text.len())
}

/// Keep the tail of long diagnostics, where errors usually are.
pub fn clip_tail(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("(…{} bytes) {}", text.len(), &text[start..])
}

/// Repository root of the checkout this harness was built from.
pub fn repository_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // usr/src/kedra/tests/container -> repository root.
    manifest
        .ancestors()
        .nth(5)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest.to_path_buf())
}

/// Directory of this crate's non-Rust inputs (lab layer, scenarios, probes).
pub fn harness_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Default artifact root; ignored by Git through `/target/`.
pub fn artifact_root() -> PathBuf {
    match std::env::var_os("KEDRA_LAB_ARTIFACTS") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => repository_root().join("target/kedra-lab"),
    }
}

/// Unique, sortable identifier for one execution: `<unix seconds>-<pid>`.
pub fn execution_id() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    format!("{seconds}-{}", std::process::id())
}

/// Parse `500ms`, `30s` or `2m`.
pub fn parse_duration(text: &str) -> Result<Duration> {
    let text = text.trim();
    let (number, unit) = text
        .find(|c: char| !c.is_ascii_digit())
        .map(|index| text.split_at(index))
        .ok_or_else(|| invalid(format!("duration {text:?} needs a unit (ms, s or m)")))?;
    let value: u64 = number
        .parse()
        .map_err(|_| invalid(format!("invalid duration {text:?}")))?;
    let duration = match unit {
        "ms" => Duration::from_millis(value),
        "s" => Duration::from_secs(value),
        "m" => Duration::from_secs(value * 60),
        _ => {
            return Err(invalid(format!(
                "duration {text:?} needs a unit (ms, s or m)"
            )));
        }
    };
    if duration.is_zero() || duration > Duration::from_secs(3600) {
        return Err(invalid(format!(
            "duration {text:?} must be between 1ms and 60m"
        )));
    }
    Ok(duration)
}
