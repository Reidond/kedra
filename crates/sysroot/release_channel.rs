//! Verify a downloaded discovery bundle before creating public update inputs.
use clap::Args;
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use sysroot_core::release::{MAX_DOCUMENT, Scope, TrustState};

#[derive(Args)]
pub struct Options {
    #[arg(long)]
    bundle: PathBuf,
    #[arg(long)]
    public_key: PathBuf,
    /// Independently confirmed SHA-256 of the P-256 public key's SPKI DER.
    #[arg(long)]
    expected_fingerprint: String,
    #[arg(long)]
    target: String,
    /// Independently expected image repository, without a tag or digest.
    #[arg(long)]
    repository: String,
    /// New directory; existing files/directories are never replaced.
    #[arg(long)]
    output_dir: PathBuf,
    /// Independently retained public state; never installed machine authority.
    #[arg(long)]
    previous_state: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    payload: String,
    signature: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema_version: u32,
    release: Document,
    checkpoint: Document,
}

fn write_new(directory: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(directory.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub fn unpack(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let bundle: Bundle = serde_json::from_slice(&super::limited_file(&options.bundle, 300_000)?)?;
    if bundle.schema_version != 1 {
        return Err("unsupported channel bundle version".into());
    }
    for document in [&bundle.release, &bundle.checkpoint] {
        if document.payload.len() > MAX_DOCUMENT || document.signature.len() > 1024 {
            return Err(sysroot_core::release::Error::SizeLimit.into());
        }
    }
    let key = super::limited_file(&options.public_key, 4096)?;
    let key = std::str::from_utf8(&key).map_err(|_| sysroot_core::release::Error::InvalidKey)?;
    let fingerprint = sysroot_core::release::public_key_fingerprint(key)?;
    if fingerprint != options.expected_fingerprint {
        return Err("public key differs from the independently expected fingerprint".into());
    }
    let scope = Scope {
        target: options.target,
        architecture: "x86_64".into(),
        fedora_release: 44,
        repository: options.repository,
    };
    let release = sysroot_core::release::verify_release(
        bundle.release.payload.as_bytes(),
        bundle.release.signature.as_bytes(),
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
    let update = sysroot_core::release::verify_update(
        release,
        bundle.checkpoint.payload.as_bytes(),
        bundle.checkpoint.signature.as_bytes(),
        key,
        &scope,
        previous.as_ref(),
        now,
    )?;
    let state = serde_json::to_vec_pretty(&update.next_trust_state)?;
    let mut directory = std::fs::DirBuilder::new();
    directory.recursive(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    // Exclusive creation occurs only after all authority/freshness checks pass.
    // A failed write may leave a partial verified directory; never overwrite it.
    directory.create(&options.output_dir)?;
    for (name, bytes) in [
        ("release.json", bundle.release.payload.as_bytes()),
        ("release.sig", bundle.release.signature.as_bytes()),
        ("checkpoint.json", bundle.checkpoint.payload.as_bytes()),
        ("checkpoint.sig", bundle.checkpoint.signature.as_bytes()),
        ("next-trust-state.json", state.as_slice()),
    ] {
        write_new(&options.output_dir, name, bytes)?;
    }
    if options.json {
        println!(
            "{}",
            serde_json::json!({"signature_valid":true,"channel_freshness_verified":true,
                "replay_checked":previous.is_some(),"verified_at":now,
                "key_fingerprint_sha256":fingerprint,"release_sha256":update.release.sha256(),
                "image_reference":update.release.release().image_reference(),
                "output":options.output_dir,"next_trust_state":update.next_trust_state,
                "deployment_authorized":false})
        );
    } else {
        println!("Verified channel files: {}", options.output_dir.display());
        println!("Image: {}", update.release.release().image_reference());
        if previous.is_none() {
            println!("No previous state supplied; earlier accepted history was not checked.");
        }
        println!("The installed helper independently verifies enrollment and staging.");
    }
    Ok(())
}
