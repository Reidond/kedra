//! Offline native image derivation over an independently selected composition.
//! The steps are the ones the composition's committed source manifest declares
//! (its target's `.kedra` block), so a composition carries its own derivation.
use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde::Deserialize;
use sysroot_engine::{Error, NativeStep, Result, SystemComposition, SystemFileDisposition};

const SOURCE_MANIFEST: &str = "/usr/share/sysroot/source.json";

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolve the closed native plan; builds nothing and runs no image tool.
    Plan(Inputs),
    /// Build the native image offline, re-read its material and bind the result.
    Build {
        #[command(flatten)]
        inputs: Inputs,
        /// Independently selected identity printed by `sysroot derive plan`.
        #[arg(long)]
        expected_derivation: String,
    },
    /// Executor inside the offline derive build only; never run it on a machine.
    #[command(hide = true)]
    Apply { plan: PathBuf },
    /// Readback inside the throwaway observation stage only.
    #[command(hide = true)]
    Observe { plan: PathBuf },
}

#[derive(Args)]
struct Inputs {
    /// Exported composition context directory.
    #[arg(long)]
    context: PathBuf,
    /// Independently retained composition identity.
    #[arg(long)]
    expected_identity: String,
    /// Owner-private directory for locks, journals, bindings and snapshots.
    #[arg(long)]
    state: PathBuf,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    native_steps: Vec<NativeStep>,
}

/// The steps of the verified composition's committed source manifest. Authored
/// definitions cannot place files under /usr/share/sysroot (`sysroot system`).
fn declared_steps(composition: &SystemComposition) -> Result<Vec<NativeStep>> {
    let file = composition
        .plan
        .files
        .iter()
        .find(|file| file.path == SOURCE_MANIFEST)
        .filter(|file| {
            file.disposition == SystemFileDisposition::Payload
                && file.provenance == format!("{}generated-manifest", crate::system::SOURCE_OWNER)
        })
        .ok_or_else(|| Error::Invalid("composition has no committed source manifest".into()))?;
    let manifest: Manifest = serde_json::from_slice(&file.bytes)?;
    if manifest.native_steps.is_empty() {
        return Err(Error::Invalid(
            "the composition's target declares no native steps".into(),
        ));
    }
    Ok(manifest.native_steps)
}

fn json(value: &impl serde::Serialize) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

pub fn run(options: Options) -> Result<()> {
    match options.command {
        Command::Plan(inputs) => json(&sysroot_engine::plan_derivation(
            &inputs.context,
            &inputs.expected_identity,
            &declared_steps,
            &inputs.state,
        )?),
        Command::Build {
            inputs,
            expected_derivation,
        } => json(&sysroot_engine::derive_native(
            &inputs.context,
            &inputs.expected_identity,
            &declared_steps,
            &expected_derivation,
            &inputs.state,
        )?),
        Command::Apply { plan } => sysroot_engine::apply_native(&plan),
        Command::Observe { plan } => sysroot_engine::observe_native(&plan),
    }
}
