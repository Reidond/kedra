use super::{Application, Result, Settings, capture};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
const UNIT: &str = "kedra-noctalia.service";
const FRAGMENT: &str = "/usr/lib/systemd/user/kedra-noctalia.service";
const FEDORA_TIMEOUT: &str = "/usr/lib/systemd/user/service.d/10-timeout-abort.conf";

fn vendor_dropin(path: &str, bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|_| "systemd drop-in is not UTF-8")?;
    let directives: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if path != FEDORA_TIMEOUT || directives != ["[Service]", "TimeoutStopFailureMode=abort"] {
        return Err("Noctalia has an unqualified service override".into());
    }
    Ok(())
}

pub(super) struct Native {
    pub directory: PathBuf,
    wayland: String,
}
fn systemctl(arguments: &[&str]) -> Result<String> {
    let mut process = Command::new("/usr/bin/timeout")
        .args(["--kill-after=2s", "25s", "/usr/bin/systemctl", "--user"])
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let result = (|| -> Result<String> {
        let mut bytes = Vec::new();
        process
            .stdout
            .take()
            .ok_or("service output unavailable")?
            .take(65537)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 65536 || !process.wait()?.success() {
            return Err("Noctalia service operation failed; inspect systemctl --user status kedra-noctalia.service".into());
        }
        String::from_utf8(bytes).map_err(|_| "service output is malformed".into())
    })();
    if result.is_err() {
        let _ = process.kill();
        let _ = process.wait();
    }
    result
}
fn profile(home: &Path, values: impl IntoIterator<Item = (String, String)>) -> Result<()> {
    for (name, value) in values {
        let expected = match name.as_str() {
            "HOME" => home.to_path_buf(),
            "XDG_CONFIG_HOME" | "NOCTALIA_CONFIG_HOME" => home.join(".config"),
            "XDG_STATE_HOME" | "NOCTALIA_STATE_HOME" => home.join(".local/state"),
            _ => continue,
        };
        if !value.is_empty() && Path::new(&value) != expected {
            return Err("home activation supports the default Noctalia profile only".into());
        }
    }
    Ok(())
}
fn writers() -> Result<Vec<u32>> {
    use std::os::unix::fs::MetadataExt;
    let owner = rustix::process::geteuid().as_raw();
    let mut found = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|v| v.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.uid() != owner {
            continue;
        }
        let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) else {
            continue;
        };
        if comm.trim() != "noctalia" {
            continue;
        }
        if std::fs::read_link(entry.path().join("exe"))? != Path::new("/usr/bin/noctalia") {
            return Err("an unmanaged Noctalia executable is running".into());
        }
        found.push(pid);
    }
    Ok(found)
}
impl Native {
    pub fn open() -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable")?);
        if !home.is_absolute() {
            return Err("HOME must be absolute".into());
        }
        profile(&home, std::env::vars())?;
        let manager = systemctl(&["show-environment"])?;
        let wayland = manager
            .lines()
            .find_map(|line| line.strip_prefix("WAYLAND_DISPLAY="))
            .ok_or("Noctalia activation requires an active graphical user session")?
            .to_owned();
        profile(
            &home,
            manager
                .lines()
                .filter_map(|line| line.split_once('='))
                .map(|(k, v)| (k.to_owned(), v.to_owned())),
        )?;
        let fragment = systemctl(&["show", UNIT, "--property=FragmentPath", "--value"])?;
        let dropins = systemctl(&["show", UNIT, "--property=DropInPaths", "--value"])?;
        if fragment.trim() != FRAGMENT {
            return Err(
                "activation requires the installed Noctalia service without user overrides".into(),
            );
        }
        for path in dropins.split_whitespace() {
            if path != FEDORA_TIMEOUT {
                return Err("Noctalia has an unqualified service override".into());
            }
            vendor_dropin(
                path,
                &sysroot_helper::trusted_file::read(Path::new(path), 0, 8192)?,
            )?;
        }
        sysroot_helper::trusted_file::read(Path::new(FRAGMENT), 0, 8192)?;
        // Resolve only the standard bootc /home alias. Nested profile symlinks
        // are rejected by Directory::open rather than canonicalized away.
        let directory = home.canonicalize()?.join(".local/state/noctalia");
        Ok(Self { directory, wayland })
    }
    fn managed(&self) -> Result<()> {
        let pid: u32 = systemctl(&["show", UNIT, "--property=MainPID", "--value"])?
            .trim()
            .parse()
            .map_err(|_| "Noctalia service PID is malformed")?;
        let active = writers()?;
        if (pid == 0 && !active.is_empty()) || (pid != 0 && active != [pid]) {
            return Err("another Noctalia writer is active; close it before activation".into());
        }
        Ok(())
    }
}

impl Application for Native {
    fn observe(&self) -> Result<Settings> {
        capture::output(&["config", "validate"], 65536)?;
        capture::live()
    }
    fn stop(&self) -> Result<()> {
        self.managed()?;
        systemctl(&["stop", UNIT])?;
        if !writers()?.is_empty() {
            return Err("Noctalia still has an active writer; no files were changed".into());
        }
        Ok(())
    }
    fn start(&self) -> Result<()> {
        systemctl(&["start", UNIT])?;
        for _ in 0..30 {
            if capture::output_in(
                &["msg", "log-level-status"],
                65536,
                &[("WAYLAND_DISPLAY", &self.wayland)],
            )
            .is_ok()
            {
                self.managed()?;
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Err("Noctalia did not become ready; inspect home recover".into())
    }
}
