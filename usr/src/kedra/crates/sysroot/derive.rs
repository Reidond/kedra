//! Offline native image derivation over an independently selected composition.
use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sysroot_engine::Result;

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
    /// Committed native step declaration (schema_version 1).
    #[arg(long)]
    steps: PathBuf,
    /// Owner-private directory for locks, journals, bindings and snapshots.
    #[arg(long)]
    state: PathBuf,
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
            &inputs.steps,
            &inputs.state,
        )?),
        Command::Build {
            inputs,
            expected_derivation,
        } => json(&sysroot_engine::derive_native(
            &inputs.context,
            &inputs.expected_identity,
            &inputs.steps,
            &expected_derivation,
            &inputs.state,
        )?),
        Command::Apply { plan } => sysroot_engine::apply_native(&plan),
        Command::Observe { plan } => sysroot_engine::observe_native(&plan),
    }
}
