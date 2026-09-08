//! Ordinary-user transport to the fixed installed helper; no privilege decisions.
use clap::{Args, Subcommand};
use std::path::PathBuf;

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
    #[command(flatten)]
    release: SignedRelease,
    #[arg(long)]
    checkpoint: PathBuf,
    #[arg(long)]
    checkpoint_signature: PathBuf,
}
#[derive(Subcommand)]
enum Operation {
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
    Status,
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
        use std::io::Write;
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
        let request = match options.command {
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
                Request::Enroll {
                    release: document(value.release.manifest, value.release.signature)?,
                    checkpoint: document(value.checkpoint, value.checkpoint_signature)?,
                    installed_release,
                }
            }
            Operation::Status => Request::Status {},
            Operation::Stage {
                release: value,
                replace_staged,
                resume,
            } => Request::Stage {
                release: document(value.release.manifest, value.release.signature)?,
                checkpoint: document(value.checkpoint, value.checkpoint_signature)?,
                replace_staged,
                resume,
            },
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
            .spawn()?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            child
                .stdin
                .take()
                .ok_or("helper input is unavailable")?
                .write_all(&bytes)?;
            if !child.wait()?.success() {
                return Err(
                    "installed management helper refused or could not complete the request".into(),
                );
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
