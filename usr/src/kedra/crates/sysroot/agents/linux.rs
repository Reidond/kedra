//! Run a selected official CLI as the ordinary user in a Kedra checkout. Checkout, target and
//! profile selection catch mistakes and keep state apart; they are not a sandbox.
use super::{ConfigScope, Options, Runtime};
use crate::source;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Bundled runtimes live at BUNDLED/NAME/bin/NAME, outside ordinary PATH.
const BUNDLED: &str = "/usr/libexec/sysroot/agents";

#[derive(Serialize)]
struct Selection {
    agent: String,
    executable: PathBuf,
    version: String,
    runtime: Runtime,
    config_scope: ConfigScope,
    profile: Option<PathBuf>,
    checkout: PathBuf,
    editing_target: String,
    deployment_authorized: bool,
}

fn absolute_env(name: &str) -> Result<Option<PathBuf>> {
    match std::env::var_os(name).filter(|value| !value.is_empty()) {
        Some(value) if !Path::new(&value).is_absolute() => {
            Err(format!("{name} must be an absolute path").into())
        }
        value => Ok(value.map(PathBuf::from)),
    }
}

fn executable(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err(format!("{} is not an executable file", path.display()).into());
    }
    Ok(path)
}

/// The bundled runtime, or a personal one; a missing personal runtime never falls back.
fn runtime(name: &str, options: &Options) -> Result<PathBuf> {
    let missing = match options.runtime {
        Runtime::Bundled => format!("bundled {name} is unavailable; use --runtime user"),
        Runtime::User => {
            format!("no personal {name} on absolute PATH entries; no bundled fallback")
        }
    };
    match (options.runtime, &options.executable) {
        (Runtime::Bundled, Some(_)) => Err("--executable requires --runtime user".into()),
        (Runtime::Bundled, None) => {
            executable(&Path::new(BUNDLED).join(name).join("bin").join(name))
                .map_err(|_| missing.into())
        }
        (Runtime::User, Some(path)) if !path.is_absolute() => {
            Err("--executable must be an absolute path".into())
        }
        (Runtime::User, Some(path)) => executable(path),
        (Runtime::User, None) => {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .filter(|directory| directory.is_absolute())
                .find_map(|directory| executable(&directory.join(name)).ok())
                .ok_or_else(|| missing.into())
        }
    }
}

/// Native Codex sets up helper files in its configuration directory before it parses
/// --version, so the probe gets a disposable one instead of the caller's profile.
fn version(binary: &Path) -> Result<String> {
    let probe = std::env::temp_dir().join(format!("kedra-agent-version-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&probe)?;
    let output = Command::new(binary)
        .arg("--version")
        .env("CODEX_HOME", &probe)
        .env("CLAUDE_CONFIG_DIR", &probe)
        .env_remove("BW_SESSION")
        .stdin(Stdio::null())
        .output();
    let _ = fs::remove_dir_all(&probe);
    let output = output?;
    if !output.status.success() {
        return Err("the selected runtime's --version failed".into());
    }
    // The version names the profile directory, so it may hold no path separator.
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|word| {
            word.starts_with(|c: char| c.is_ascii_digit())
                && word
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
        })
        .map(str::to_owned)
        .ok_or_else(|| "the runtime version was not recognized".into())
}

fn select(name: &str, options: &Options, home: &Path) -> Result<Selection> {
    let repo = match &options.repo {
        Some(repo) => repo.clone(),
        None => absolute_env("SYSROOT_REPO")?.unwrap_or_else(|| home.join("src/kedra")),
    };
    let top = String::from_utf8(source::git(&repo, &["rev-parse", "--show-toplevel"])?)?;
    let checkout = PathBuf::from(top.trim()).canonicalize()?;
    // --host only tells the agent which target to edit; it never selects a machine.
    let host = &options.host;
    let declaration = checkout
        .join(sysroot_core::layout::IMAGE)
        .join("targets")
        .join(host)
        .join("target.toml");
    if !host
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || !declaration.is_file()
    {
        return Err(format!(
            "{host:?} is not a target declared in {}",
            checkout.display()
        )
        .into());
    }
    let executable = runtime(name, options)?;
    let version = version(&executable)?;
    let state = absolute_env("XDG_STATE_HOME")?.unwrap_or_else(|| home.join(".local/state"));
    // Version-specific, so a newer runtime never migrates state an older bundled one reads.
    let profile = (options.config_scope == ConfigScope::Management).then(|| {
        let runtime = match options.runtime {
            Runtime::Bundled => "bundled",
            Runtime::User => "user",
        };
        state
            .join("sysroot/agents")
            .join(name)
            .join(runtime)
            .join(&version)
    });
    Ok(Selection {
        agent: name.into(),
        executable,
        version,
        runtime: options.runtime,
        config_scope: options.config_scope,
        profile,
        checkout,
        editing_target: host.clone(),
        deployment_authorized: false,
    })
}

fn prepare_profile(agent: &str, profile: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(profile)?;
    if agent != "codex" {
        return Ok(());
    }
    // New Codex profiles keep the official login in the Secret Service, never in a file.
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(profile.join("config.toml"))
    {
        Ok(mut file) => Ok(file.write_all(
            b"# Private sysroot profile. Official login; no plaintext fallback.\ncli_auth_credentials_store = \"keyring\"\n",
        )?),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn run(name: &str, options: Options) -> Result<()> {
    use rustix::process::{geteuid, getuid};
    if geteuid().is_root() || getuid() != geteuid() {
        return Err(
            "launch agents as the ordinary user, never through sudo or a privileged helper".into(),
        );
    }
    let home = absolute_env("HOME")?.ok_or("HOME is required")?;
    let selected = select(name, &options, &home)?;
    if options.print_plan {
        println!("{}", serde_json::to_string_pretty(&selected)?);
        return Ok(());
    }
    let mut command = Command::new(&selected.executable);
    command
        .current_dir(&selected.checkout)
        .args(&options.arguments)
        .env_remove("BW_SESSION")
        .env("SYSROOT_EDITING_TARGET", &selected.editing_target);
    if let Some(profile) = &selected.profile {
        prepare_profile(name, profile)?;
        let key = if name == "codex" {
            "CODEX_HOME"
        } else {
            "CLAUDE_CONFIG_DIR"
        };
        command.env(key, profile);
    }
    // The image updates bundled Claude; personal runtimes keep their own update settings.
    if name == "claude" && options.runtime == Runtime::Bundled {
        command.env("DISABLE_UPDATES", "1");
    }
    eprintln!(
        "sysroot: {name} {} ({:?}); editing {} in {}; {}",
        selected.version,
        selected.runtime,
        selected.editing_target,
        selected.checkout.display(),
        match &selected.profile {
            Some(profile) => format!("private profile {}", profile.display()),
            None => "personal configuration".into(),
        }
    );
    Err(command.exec().into())
}
