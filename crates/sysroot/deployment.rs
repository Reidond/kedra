//! Ordinary-user transport to the fixed installed helper; no privilege decisions.
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod check;

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Operation,
}
#[derive(Args)]
struct SignedRelease {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long)]
    signature: PathBuf,
}
#[derive(Args)]
struct FreshRelease {
    /// Download and verify the installed target's fixed public channel.
    #[arg(long, conflicts_with_all = ["manifest", "signature", "checkpoint", "checkpoint_signature"])]
    channel: bool,
    /// Signed release JSON; supply all four files instead of --channel.
    #[arg(long, required_unless_present = "channel")]
    manifest: Option<PathBuf>,
    /// Detached signature for the release JSON.
    #[arg(long, required_unless_present = "channel")]
    signature: Option<PathBuf>,
    /// Fresh signed channel checkpoint JSON.
    #[arg(long, required_unless_present = "channel")]
    checkpoint: Option<PathBuf>,
    /// Detached signature for the checkpoint JSON.
    #[arg(long, required_unless_present = "channel")]
    checkpoint_signature: Option<PathBuf>,
}
#[derive(Subcommand)]
enum Operation {
    /// Check the installed target's published channel without staging an update.
    Check {
        #[arg(long)]
        json: bool,
    },
    /// Enroll only the exact running signed release; never overwrite enrollment.
    Enroll {
        #[command(flatten)]
        latest: FreshRelease,
        /// Signed record for the running ISO when the current channel is newer.
        #[arg(long, requires = "installed_signature")]
        installed_manifest: Option<PathBuf>,
        #[arg(long, requires = "installed_manifest")]
        installed_signature: Option<PathBuf>,
    },
    /// Observe native bootc and reconcile an existing deployment journal.
    Status {
        /// Also assess the invoking user's accepted home baselines; never activate them.
        #[arg(long)]
        home: bool,
        /// Existing private home store; requires --home and never initializes state.
        #[arg(long, requires = "home")]
        home_state: Option<PathBuf>,
    },
    /// Verify a fresh promoted release and stage its exact image, without rebooting.
    Stage {
        #[command(flatten)]
        release: FreshRelease,
        /// Exact currently staged digest to replace after reviewing that change.
        #[arg(long)]
        replace_staged: Option<String>,
        /// Explicitly clear the update hold established by a previous rollback.
        #[arg(long)]
        resume: bool,
    },
    /// Queue the native retained signed rollback image; preserve trust high-water state.
    Rollback {
        #[command(flatten)]
        release: SignedRelease,
        #[arg(long)]
        replace_staged: Option<String>,
    },
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::io::{Read, Write};
        use std::process::{Command, Stdio};
        use sysroot_helper::protocol::{Envelope, Request, SignedDocument};
        fn document(
            payload: PathBuf,
            signature: PathBuf,
        ) -> Result<SignedDocument, Box<dyn std::error::Error>> {
            // These caller-selected files are unprivileged input, not helper paths.
            let payload = super::limited_file(&payload, sysroot_core::release::MAX_DOCUMENT)?;
            let signature = super::limited_file(&signature, 1024)?;
            Ok(SignedDocument {
                payload: String::from_utf8(payload)?,
                signature: String::from_utf8(signature)?,
            })
        }
        fn fresh(
            value: FreshRelease,
        ) -> Result<(SignedDocument, SignedDocument), Box<dyn std::error::Error>> {
            if value.channel {
                return check::documents();
            }
            Ok((
                document(
                    value.manifest.ok_or("release manifest is required")?,
                    value.signature.ok_or("release signature is required")?,
                )?,
                document(
                    value.checkpoint.ok_or("checkpoint is required")?,
                    value
                        .checkpoint_signature
                        .ok_or("checkpoint signature is required")?,
                )?,
            ))
        }
        let mut home_assessment = None;
        let request = match options.command {
            Operation::Check { json } => return check::run(json),
            Operation::Enroll {
                latest: value,
                installed_manifest,
                installed_signature,
            } => {
                let installed_release = match (installed_manifest, installed_signature) {
                    (None, None) => None,
                    (Some(manifest), Some(signature)) => Some(document(manifest, signature)?),
                    _ => return Err("both installed release files are required".into()),
                };
                let (release, checkpoint) = fresh(value)?;
                Request::Enroll {
                    release,
                    checkpoint,
                    installed_release,
                }
            }
            Operation::Status { home, home_state } => {
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
                Request::Status {}
            }
            Operation::Stage {
                release: value,
                replace_staged,
                resume,
            } => {
                let (release, checkpoint) = fresh(value)?;
                Request::Stage {
                    release,
                    checkpoint,
                    replace_staged,
                    resume,
                }
            }
            Operation::Rollback {
                release: value,
                replace_staged,
            } => Request::Rollback {
                release: document(value.manifest, value.signature)?,
                replace_staged,
            },
        };
        let bytes = serde_json::to_vec(&Envelope {
            schema_version: 1,
            request,
        })?;
        if bytes.len() > sysroot_helper::protocol::MAX_REQUEST {
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
                if object.get("schema_version") != Some(&serde_json::json!(1))
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
