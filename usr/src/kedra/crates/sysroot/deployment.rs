//! Ordinary-user transport to the fixed installed helper; no privilege decisions.
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Operation,
}
#[derive(Subcommand)]
enum Operation {
    /// Report whether fixed GHCR stable is a newer image in scope; never stages.
    /// Its signature is verified by bootc when it is staged.
    Check {
        #[arg(long)]
        json: bool,
    },
    /// Observe native bootc, the booted image identity and the rollback hold.
    Status {
        /// Emit structured installed status (currently also the default).
        #[arg(long)]
        json: bool,
        /// Also assess the invoking user's accepted home baselines; never activate them.
        #[arg(long)]
        home: bool,
        /// Existing private home store; requires --home and never initializes state.
        #[arg(long, requires = "home")]
        home_state: Option<PathBuf>,
    },
    /// Stage fixed GHCR stable's exact digest without rebooting; bootc verifies its
    /// signature and the helper refuses an image older than the booted one.
    Stage {
        /// Exact currently staged digest to replace after reviewing that change.
        #[arg(long)]
        replace_staged: Option<String>,
        /// Explicitly clear the update hold established by a previous rollback.
        #[arg(long)]
        resume: bool,
        /// Require the verified stable channel to have this reviewed digest.
        #[arg(long)]
        expected_digest: Option<String>,
    },
    /// Queue the retained rollback deployment and hold forward updates.
    Rollback {
        #[arg(long)]
        replace_staged: Option<String>,
    },
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::io::{Read, Write};
        use std::process::{Command, Stdio};
        use sysroot_core::protocol::{Envelope, Request};
        let mut home_assessment = None;
        let request = match options.command {
            Operation::Check { json } => {
                let _ = json;
                Request::ChannelCheck {}
            }
            Operation::Status {
                home,
                home_state,
                json,
            } => {
                let _ = json;
                if home {
                    if rustix::process::getuid().as_raw() == 0
                        || rustix::process::geteuid().as_raw() == 0
                    {
                        return Err(
                            "caller-home assessment runs as the ordinary invoking user, never root"
                                .into(),
                        );
                    }
                    home_assessment = Some(home_state);
                }
                Request::ChannelStatus {}
            }
            Operation::Stage {
                replace_staged,
                resume,
                expected_digest,
            } => Request::ChannelStage {
                expected_digest,
                replace_staged,
                resume,
            },
            Operation::Rollback { replace_staged } => Request::ChannelRollback { replace_staged },
        };
        // Status only observes; every other operation runs as the ordinary owner.
        if !matches!(request, Request::ChannelStatus {})
            && (rustix::process::getuid().as_raw() == 0
                || rustix::process::geteuid().as_raw() != rustix::process::getuid().as_raw())
        {
            return Err("run channel management as the ordinary owner; the installed helper requests authorization".into());
        }
        let bytes = serde_json::to_vec(&Envelope {
            schema_version: sysroot_core::protocol::PROTOCOL,
            request,
        })?;
        if bytes.len() > sysroot_core::protocol::MAX_REQUEST {
            return Err("helper request too large".into());
        }
        eprintln!(
            "sysroot: the installed helper will independently verify this request; no reboot or home activation is requested"
        );
        let mut child = Command::new("/usr/bin/sudo")
            .env_remove("BW_SESSION")
            .args(["--", "/usr/libexec/sysroot/helper"])
            .stdin(Stdio::piped())
            .stdout(if home_assessment.is_some() {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .spawn()?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            child
                .stdin
                .take()
                .ok_or("helper input is unavailable")?
                .write_all(&bytes)?;
            let response = if home_assessment.is_some() {
                const MAX_STATUS: usize = 1_048_576;
                let mut response = Vec::new();
                child
                    .stdout
                    .take()
                    .ok_or("helper output is unavailable")?
                    .take(MAX_STATUS as u64 + 1)
                    .read_to_end(&mut response)?;
                if response.len() > MAX_STATUS {
                    return Err("installed helper status exceeded the output bound".into());
                }
                Some(response)
            } else {
                None
            };
            if !child.wait()?.success() {
                return Err(
                    "installed management helper refused or could not complete the request".into(),
                );
            }
            if let (Some(state), Some(response)) = (home_assessment, response) {
                let mut response: serde_json::Value = serde_json::from_slice(&response)
                    .map_err(|_| "installed helper status is malformed")?;
                let object = response
                    .as_object_mut()
                    .ok_or("installed helper status is not an object")?;
                if object
                    .get("schema_version")
                    .and_then(serde_json::Value::as_u64)
                    != Some(sysroot_core::protocol::PROTOCOL.into())
                    || object.contains_key("caller_home")
                {
                    return Err("installed helper status is incompatible".into());
                }
                object.insert(
                    "caller_home".to_owned(),
                    crate::home::assess(state.as_deref()),
                );
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = options;
        Err("deployment control requires the installed Linux helper and release trust".into())
    }
}
