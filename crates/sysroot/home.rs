//! Managed home files: the image's data file names them, the installed baseline
//! supplies their next version and a three-way merge keeps local edits. This
//! module knows no application; checks and reloads come from the data file.
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub(crate) mod files;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod update;

#[cfg(target_os = "linux")]
pub(crate) fn assess(state: Option<&std::path::Path>) -> serde_json::Value {
    linux::assess(state)
}

#[derive(Args)]
pub struct Options {
    /// State directory; defaults to $XDG_STATE_HOME/kedra/home or ~/.local/state/kedra/home.
    #[arg(long)]
    state: Option<PathBuf>,
    /// Image root holding the data file and baselines, for an unpacked test image.
    #[arg(long, default_value = "/")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare each managed live file with the installed baseline and show the diff.
    Status {
        #[arg(long)]
        json: bool,
        /// Managed paths relative to the home directory; all when omitted.
        paths: Vec<String>,
    },
    /// Copy one live file into its source path in a Kedra checkout; never stages or commits.
    Capture {
        /// Managed path relative to the home directory.
        path: String,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Merge installed baseline changes into the live files, then check and reload them.
    Update {
        #[arg(long)]
        json: bool,
        /// Managed paths relative to the home directory; all when omitted.
        paths: Vec<String>,
    },
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        linux::run(options)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = options;
        Err("home file management requires an installed Linux Kedra system".into())
    }
}
