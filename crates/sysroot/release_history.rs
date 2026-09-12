//! Authenticate predecessor ordering without granting channel freshness.
use clap::Args;
use std::path::PathBuf;
use sysroot_core::release::{MAX_DOCUMENT, Scope, TrustState};

#[derive(Args)]
pub struct Options {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long)]
    signature: PathBuf,
    #[arg(long)]
    checkpoint: PathBuf,
    #[arg(long)]
    checkpoint_signature: PathBuf,
    #[arg(long)]
    public_key: PathBuf,
    #[arg(long)]
    target: String,
    #[arg(long)]
    repository: String,
    /// Independently retained ordering floor; never installed machine authority.
    #[arg(long)]
    previous_state: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let scope = Scope {
        target: options.target,
        architecture: "x86_64".into(),
        fedora_release: 44,
        repository: options.repository,
    };
    let key = super::limited_file(&options.public_key, 4096)?;
    let key = std::str::from_utf8(&key).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
    let release = sysroot_core::release::verify_release(
        &super::limited_file(&options.manifest, MAX_DOCUMENT)?,
        &super::limited_file(&options.signature, 1024)?,
        key,
        Some(&scope),
    )?;
    let previous: Option<TrustState> = options
        .previous_state
        .map(|path| -> Result<TrustState, Box<dyn std::error::Error>> {
            Ok(serde_json::from_slice(&super::limited_file(
                &path,
                MAX_DOCUMENT,
            )?)?)
        })
        .transpose()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let history = sysroot_core::release::verify_history(
        release,
        &super::limited_file(&options.checkpoint, MAX_DOCUMENT)?,
        &super::limited_file(&options.checkpoint_signature, 1024)?,
        key,
        &scope,
        previous.as_ref(),
        now,
    )?;
    if options.json {
        println!(
            "{}",
            serde_json::json!({"schema_version":1,"signature_valid":true,
                "historical_only":true,"expired":history.expired,"verified_at":now,
                "channel_freshness_verified":false,"deployment_authorized":false,
                "replay_checked":previous.is_some(),"ordering_state":history.ordering_state,
                "key_fingerprint_sha256":history.release.key_fingerprint(),
                "release_sha256":history.release.sha256(),
                "image_reference":history.release.release().image_reference()})
        );
    } else {
        println!(
            "Authenticated predecessor: {}",
            history.release.release().image_reference()
        );
        println!(
            "Checkpoint generation: {}",
            history.ordering_state.generation
        );
        println!("Checkpoint expired: {}", history.expired);
        println!(
            "Historical ordering only; no channel freshness or deployment eligibility granted."
        );
        if previous.is_none() {
            println!("No previous state supplied; earlier accepted history was not checked.");
        }
    }
    Ok(())
}
