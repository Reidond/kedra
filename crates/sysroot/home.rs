//! Ordinary-user home review. Live activation and privileged deployment are separate.
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod activation;
#[cfg(target_os = "linux")]
mod export;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod text;

#[derive(Args)]
pub struct Options {
    /// Private review store; defaults to $XDG_STATE_HOME/sysroot/home or ~/.local/state/sysroot/home.
    #[arg(long)]
    state: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Review selected lines in an explicitly adopted ordinary configuration file.
    File {
        /// Installed baseline/live-home relative path; currently niri is supported.
        #[arg(long, default_value = ".config/niri/config.kdl")]
        path: String,
        #[command(subcommand)]
        command: TextCommand,
    },
    /// Adopt only the three supported Noctalia fields from the installed baseline.
    Init,
    /// Capture effective settings and show each field's independent dispositions.
    Status {
        /// Show the last captured values without invoking Noctalia.
        #[arg(long)]
        last_capture: bool,
    },
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
    /// Review reconciliation with the installed image, or discard one live field.
    Plan {
        #[arg(long)]
        discard: Option<Key>,
    },
    /// Apply the exact previously reviewed plan to supported native settings.
    Apply {
        #[arg(long)]
        plan: String,
    },
    /// Restore one field to its selected/published/baseline value using a plan.
    Discard {
        key: Key,
        #[arg(long)]
        plan: String,
    },
    /// Inspect or explicitly recover an interrupted home activation.
    Recover {
        #[arg(value_enum)]
        action: Option<Recovery>,
    },
    /// Write a new patch containing only selected settings; preserve the checkout.
    Export {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Verify and remember the exact source commit containing the selected values.
    RecordSource {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        commit: String,
    },
}

#[derive(Subcommand)]
enum TextCommand {
    /// Preview restoring one live change to its pinned selection or public reference.
    DiscardPlan { change: String },
    /// Apply an exact discard plan and load the managed file into the running niri session.
    Discard {
        change: String,
        #[arg(long)]
        plan: String,
        /// Explicitly select .config/niri/config.kdl as the session's active configuration.
        #[arg(long, required = true)]
        activate_managed_file: bool,
    },
    /// Inspect or recover an interrupted niri file discard.
    Recover {
        #[arg(value_enum)]
        action: Option<Recovery>,
        /// Required for recovery actions that reload the managed configuration.
        #[arg(long)]
        activate_managed_file: bool,
    },
    /// Preview a committed source baseline without changing live files or accepted state.
    Plan {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Full source commit; defaults to the checkout's committed HEAD.
        #[arg(long)]
        commit: Option<String>,
    },
    /// Adopt the installed niri baseline after checking live custom commands for secrets.
    Init {
        /// Acknowledge reviewing this file and finding it safe to display/store selected changes.
        #[arg(long, required = true)]
        reviewed_safe: bool,
    },
    /// Show changes with stable content-bound IDs, plus selected/local dispositions.
    Status,
    /// Pin one displayed change; equal-size replacements are selectable one line at a time.
    Stage { change: String },
    /// Remove one pinned change without modifying the live file.
    Unstage { change: String },
    /// Keep exactly one displayed change local; later changed content returns to review.
    KeepLocal { change: String },
    /// Clear one local-only decision without modifying the live file.
    ClearLocal { change: String },
    /// Show only pinned changes that may cross into source.
    Selection,
    /// Export a new patch through Git; never replace the checkout/index or live file.
    Export {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Record the exact source commit containing the selection, separate from deployment.
    RecordSource {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        commit: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Recovery {
    Resume,
    Abort,
    KeepCurrent,
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
