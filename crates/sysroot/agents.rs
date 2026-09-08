//! Ordinary-user runtime selection. No helper, agent installation or model API.
use clap::{Args, ValueEnum};
use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy, Debug, ValueEnum, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Bundled,
    User,
}

#[derive(Clone, Copy, Debug, ValueEnum, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    Management,
    Personal,
}

#[derive(Args, Debug)]
pub struct Options {
    /// Kedra checkout; defaults to SYSROOT_REPO or ~/src/kedra.
    #[arg(long)]
    pub repo: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "bundled")]
    pub runtime: Runtime,
    /// Absolute personal executable; requires --runtime user. Otherwise use PATH.
    #[arg(long)]
    pub executable: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "management")]
    pub config_scope: ConfigScope,
    /// Target to edit; never selects a machine for execution or deployment.
    #[arg(long, default_value = "desktop")]
    pub host: String,
    /// Print selection JSON (runs --version); do not create profiles or launch.
    #[arg(long)]
    pub print_plan: bool,
    /// Arguments to the official CLI, after --.
    #[arg(last = true)]
    pub arguments: Vec<OsString>,
}

#[cfg(target_os = "linux")]
#[path = "agents/linux.rs"]
mod linux;

pub fn run(name: &str, options: Options) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    return linux::run(name, options);
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (name, options);
        Err("agent launchers require Linux; no profile was changed".into())
    }
}
