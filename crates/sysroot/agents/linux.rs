use super::{ConfigScope, Options, Runtime};
use crate::source;
use serde::Serialize;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static PROBE_NUMBER: AtomicU64 = AtomicU64::new(0);
struct VersionProbe(PathBuf);
impl VersionProbe {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "kedra-agent-version-{}-{}",
            std::process::id(),
            PROBE_NUMBER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path)?;
        Ok(Self(path))
    }
}
impl Drop for VersionProbe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize)]
struct Selection {
    agent: String,
    executable: PathBuf,
    version: String,
    runtime: Runtime,
    config_scope: ConfigScope,
    profile: Option<PathBuf>,
    checkout: PathBuf,
    source_revision: String,
    editing_target: String,
    execution: &'static str,
    deployment_authorized: bool,
    #[serde(skip)]
    lock_path: PathBuf,
}

fn git_text(repo: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(source::git(repo, args)?)?.trim().into())
}

fn absolute_env(name: &str) -> Result<Option<PathBuf>> {
    let path = std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    if path.as_ref().is_some_and(|p| !p.is_absolute()) {
        return Err(format!("{name} must be an absolute path").into());
    }
    Ok(path)
}

fn executable(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err("selected runtime must be an executable regular file".into());
    }
    Ok(path)
}

fn select(
    name: &str,
    options: &Options,
    home: &Path,
    state: &Path,
    bundle: &Path,
    search_path: &std::ffi::OsStr,
) -> Result<Selection> {
    let repo = options
        .repo
        .clone()
        .unwrap_or_else(|| home.join("src/kedra"));
    let checkout =
        PathBuf::from(git_text(&repo, &["rev-parse", "--show-toplevel"])?).canonicalize()?;
    let origin = git_text(
        &checkout,
        &["config", "--local", "--get", "remote.origin.url"],
    )?;
    if !matches!(
        origin.as_str(),
        "https://github.com/Reidond/kedra.git"
            | "https://github.com/Reidond/kedra"
            | "git@github.com:Reidond/kedra.git"
            | "ssh://git@github.com/Reidond/kedra.git"
    ) {
        return Err(
            "checkout origin must identify Reidond/kedra; no fetch or clone was attempted".into(),
        );
    }
    if !options
        .host
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_lowercase())
        || options.host.len() > 63
        || !options
            .host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("invalid editing target".into());
    }
    let revision = git_text(&checkout, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    // Editing a disabled hardware target is allowed; this is not a source build.
    let target_path = format!("{revision}:hosts/{}/host.toml", options.host);
    let target: source::Target =
        toml::from_str(&git_text(&checkout, &["cat-file", "blob", &target_path])?)?;
    if target.id != options.host {
        return Err("editing target identity mismatch".into());
    }
    let binary = match options.runtime {
        Runtime::Bundled => {
            if options.executable.is_some() {
                return Err("--executable requires --runtime user".into());
            }
            executable(&bundle.join(name).join("bin").join(name)).map_err(|_| format!("bundled {name} is unavailable; use --runtime user with an installed personal executable"))?
        }
        Runtime::User => {
            let requested = if let Some(path) = &options.executable {
                if !path.is_absolute() {
                    return Err("--executable must be an absolute path".into());
                }
                path.clone()
            } else {
                std::env::split_paths(search_path)
                    .filter(|p| p.is_absolute())
                    .map(|p| p.join(name))
                    .find(|p| executable(p).is_ok())
                    .ok_or(
                        "personal runtime missing from absolute PATH entries; no bundled fallback",
                    )?
            };
            let path = executable(&requested)?;
            if path.starts_with(bundle) || path == std::env::current_exe()?.canonicalize()? {
                return Err(
                    "personal runtime resolves to sysroot or a private bundled executable".into(),
                );
            }
            path
        }
    };
    // Native Codex performs argument-zero setup before parsing --version. Do
    // not let this discovery step write into the caller's personal profile.
    let probe = VersionProbe::new()?;
    let output = Command::new(&binary)
        .arg("--version")
        .env("CODEX_HOME", &probe.0)
        .env("CLAUDE_CONFIG_DIR", &probe.0)
        .env_remove("BW_SESSION")
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() || output.stdout.len() > 1024 {
        return Err("selected runtime --version failed".into());
    }
    let version_output = std::str::from_utf8(&output.stdout)?;
    let version = version_output
        .split_whitespace()
        .find(|part| {
            part.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && part.len() <= 80
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
        })
        .ok_or("runtime version was not recognized")?
        .to_owned();
    let profile = (options.config_scope == ConfigScope::Management).then(|| {
        state
            .join("sysroot/agents")
            .join(name)
            .join(match options.runtime {
                Runtime::Bundled => "bundled",
                Runtime::User => "user",
            })
            .join(&version)
    });
    let lock_path = PathBuf::from(git_text(
        &checkout,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "sysroot-agent.lock",
        ],
    )?);
    Ok(Selection {
        agent: name.into(),
        executable: binary,
        version,
        runtime: options.runtime,
        config_scope: options.config_scope,
        profile,
        checkout,
        source_revision: revision,
        editing_target: options.host.clone(),
        execution: "local ordinary user",
        deployment_authorized: false,
        lock_path,
    })
}

fn private_directory(path: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err("management profile must be an owned private directory, not a symlink".into());
    }
    Ok(())
}

fn prepare_profile(selection: &Selection) -> Result<()> {
    let Some(profile) = &selection.profile else {
        return Ok(());
    };
    private_directory(profile)?;
    if selection.agent == "codex" {
        let config = profile.join("config.toml");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&config)
        {
            Ok(mut file) => {
                file.write_all(b"# Private sysroot profile. Official login; no plaintext fallback.\ncli_auth_credentials_store = \"keyring\"\n")?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&config)?;
                if !metadata.is_file()
                    || metadata.uid() != rustix::process::geteuid().as_raw()
                    || metadata.mode() & 0o777 != 0o600
                    || metadata.nlink() != 1
                {
                    return Err("management config must be an owned private regular file".into());
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn checkout_lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.nlink() != 1
        || metadata.mode() & 0o777 != 0o600
    {
        return Err("unsafe checkout coordination file".into());
    }
    file.try_lock()
        .map_err(|_| "another sysroot agent holds this checkout; use a separate worktree")?;
    Ok(file)
}

fn command(selection: &Selection, arguments: &[OsString]) -> Command {
    command_with_environment(selection, arguments, |key| std::env::var_os(key))
}

fn command_with_environment(
    selection: &Selection,
    arguments: &[OsString],
    inherited: impl Fn(&str) -> Option<OsString>,
) -> Command {
    let mut command = Command::new(&selection.executable);
    command
        .current_dir(&selection.checkout)
        .args(arguments)
        .env_remove("BW_SESSION")
        .env("SYSROOT_EDITING_TARGET", &selection.editing_target);
    let config_key = if selection.agent == "codex" {
        "CODEX_HOME"
    } else {
        "CLAUDE_CONFIG_DIR"
    };
    let original_key = format!("SYSROOT_ORIGINAL_{config_key}");
    if let Some(profile) = &selection.profile {
        command.env(
            &original_key,
            inherited(&original_key)
                .or_else(|| inherited(config_key))
                .unwrap_or_default(),
        );
        command.env(config_key, profile);
    } else if let Some(original) = inherited(&original_key) {
        if original.is_empty() {
            command.env_remove(config_key);
        } else {
            command.env(config_key, original);
        }
        command.env_remove(&original_key);
    }
    if selection.agent == "claude" && selection.runtime == Runtime::Bundled {
        command.env("DISABLE_UPDATES", "1").env(
            "SYSROOT_ORIGINAL_DISABLE_UPDATES",
            inherited("SYSROOT_ORIGINAL_DISABLE_UPDATES")
                .or_else(|| inherited("DISABLE_UPDATES"))
                .unwrap_or_default(),
        );
    } else if let Some(original) = inherited("SYSROOT_ORIGINAL_DISABLE_UPDATES") {
        if original.is_empty() {
            command.env_remove("DISABLE_UPDATES");
        } else {
            command.env("DISABLE_UPDATES", original);
        }
        command.env_remove("SYSROOT_ORIGINAL_DISABLE_UPDATES");
    }
    command
}

pub(super) fn run(name: &str, mut options: Options) -> Result<()> {
    if rustix::process::geteuid().is_root()
        || rustix::process::getuid() != rustix::process::geteuid()
    {
        return Err(
            "launch agents as the ordinary user, never through sudo or a privileged helper".into(),
        );
    }
    let home = absolute_env("HOME")?.ok_or("HOME is required")?;
    let state = absolute_env("XDG_STATE_HOME")?.unwrap_or_else(|| home.join(".local/state"));
    if options.repo.is_none() {
        options.repo = absolute_env("SYSROOT_REPO")?;
    }
    let selected = select(
        name,
        &options,
        &home,
        &state,
        Path::new("/usr/libexec/sysroot/agents"),
        &std::env::var_os("PATH").unwrap_or_default(),
    )?;
    if options.print_plan {
        println!("{}", serde_json::to_string_pretty(&selected)?);
        return Ok(());
    }
    let lock = checkout_lock(&selected.lock_path)?;
    prepare_profile(&selected)?;
    eprintln!(
        "sysroot: {} {} ({:?}); editing {} at {} as the local user",
        name,
        selected.version,
        selected.runtime,
        selected.editing_target,
        selected.checkout.display()
    );
    if let Some(profile) = &selected.profile {
        eprintln!(
            "sysroot: private version-specific profile: {}",
            profile.display()
        );
    } else {
        eprintln!("sysroot: explicitly using personal configuration");
    }
    // Like flock's no-fork mode: retain the advisory lock through exec. Children
    // can retain it; this coordinates cooperative wrappers, not arbitrary writers.
    rustix::io::fcntl_setfd(&lock, rustix::io::FdFlags::empty())?;
    Err(command(&selected, &options.arguments).exec().into())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
