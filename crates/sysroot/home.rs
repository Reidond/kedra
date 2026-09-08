//! Ordinary-user home review. Live activation and privileged deployment are separate.
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Args)]
pub struct Options {
    /// Existing private review store, or a new directory for init.
    #[arg(long)]
    state: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Adopt only the three supported Noctalia fields from the installed baseline.
    Init,
    /// Capture effective settings and show each field's independent dispositions.
    Status,
    /// Pin the current value for later source export; later app writes stay unselected.
    Stage { key: Key },
    /// Remove a selected value without changing the live setting.
    Unstage { key: Key },
    /// Hide exactly the current local value; a different later value returns to review.
    KeepLocal { key: Key },
    /// Keep a field application-owned across future values; exclude it from export.
    AppOwn { key: Key },
    /// Clear exact-local or application-owned policy for a field.
    ClearLocal { key: Key },
    /// Show only pinned, publishable field changes as JSON.
    Selection,
}

#[derive(Clone, Copy, ValueEnum)]
enum Key {
    #[value(name = "theme.mode")]
    Theme,
    #[value(name = "shell.button_borders")]
    ButtonBorders,
    #[value(name = "shell.input_borders")]
    InputBorders,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        linux::run(options)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = options;
        Err("home review requires an installed Linux Kedra desktop".into())
    }
}
