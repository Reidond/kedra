//! Test account and the real Kedra graphical session inside a container.
//!
//! The session runs the image's own `/usr/libexec/kedra-session` through the
//! Fedora `greetd` PAM service as a systemd service with `PAMName=greetd`, so
//! pam_systemd registers a real logind session and the packaged niri.service,
//! kedra-noctalia.service, portals and keyring start as they do after login.
//! Only greetd/tuigreet on a VT and PAM password authentication are skipped;
//! the login keyring is unlocked with the account password the way
//! pam_gnome_keyring hands it over. Real VT login stays a VM qualification.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::docker::{Docker, Exec};
use crate::environment::Environment;
use crate::{Error, Result, invalid};

pub const USER: &str = "kedra-test";

/// Generated disposable account; the password never leaves the harness.
#[derive(Clone)]
pub struct TestUser {
    pub name: String,
    pub uid: u32,
    pub home: String,
    password: String,
}

impl std::fmt::Debug for TestUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestUser")
            .field("name", &self.name)
            .field("uid", &self.uid)
            .field("home", &self.home)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Parent-compositor output and niri scale for the session.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Display {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

impl Default for Display {
    /// A 16:10 HiDPI laptop panel: 2560×1600 pixels, 1280×800 logical.
    fn default() -> Self {
        Display {
            width: 2560,
            height: 1600,
            scale: 2.0,
        }
    }
}

impl std::str::FromStr for Display {
    type Err = Error;

    /// `2560x1600` or `2560x1600@2`.
    fn from_str(text: &str) -> Result<Self> {
        let (size, scale) = text.split_once('@').unwrap_or((text, "1"));
        let (width, height) = size
            .split_once('x')
            .ok_or_else(|| invalid(format!("display {text:?} must look like 2560x1600@2")))?;
        let parse = |value: &str| {
            value
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|v| (320..=7680).contains(v))
        };
        let scale: f64 = scale
            .trim()
            .parse()
            .map_err(|_| invalid(format!("bad scale in {text:?}")))?;
        match (parse(width), parse(height)) {
            (Some(width), Some(height)) if (0.5..=4.0).contains(&scale) => Ok(Display {
                width,
                height,
                scale,
            }),
            _ => Err(invalid(format!("display {text:?} is out of range"))),
        }
    }
}

/// How the nested niri window is presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// In-memory sway output: screenshots and CI.
    Headless,
    /// waypipe to a compositor outside the container (cocoa-way on macOS),
    /// through kedra-lab's loopback TCP bridge on this port.
    Waypipe { bridge_port: u16 },
}

#[derive(Clone, Debug)]
pub struct Session {
    pub user: TestUser,
    pub wayland_display: String,
    pub niri_socket: String,
    pub display: Display,
}

fn random_hex(docker: &Docker, environment: &Environment) -> Result<String> {
    let output = environment.run(
        docker,
        &Exec::new(["od", "-An", "-tx1", "-N16", "/dev/urandom"]),
    )?;
    let hex: String = output.stdout_text().split_whitespace().collect();
    if hex.len() != 32 {
        return Err(invalid("could not generate a disposable password"));
    }
    Ok(hex)
}

/// Create the disposable wheel account from /etc/skel, as a new account would be.
pub fn create_user(docker: &Docker, environment: &Environment) -> Result<TestUser> {
    environment.run(docker, &Exec::new(["useradd", "-m", "-G", "wheel", USER]))?;
    let password = random_hex(docker, environment)?;
    environment.run(
        docker,
        &Exec::new(["chpasswd"]).stdin(format!("{USER}:{password}\n")),
    )?;
    let entry = environment.run(docker, &Exec::new(["getent", "passwd", USER]))?;
    let fields: Vec<String> = entry
        .stdout_text()
        .trim()
        .split(':')
        .map(str::to_owned)
        .collect();
    if fields.len() < 7 {
        return Err(invalid("unexpected passwd entry for the test account"));
    }
    Ok(TestUser {
        name: USER.into(),
        uid: fields[2]
            .parse()
            .map_err(|_| invalid("non-numeric test uid"))?,
        home: fields[5].clone(),
        password,
    })
}

/// Existing account in a retained lab container (password not needed later).
pub fn existing_user(docker: &Docker, environment: &Environment) -> Result<TestUser> {
    let entry = environment.run(docker, &Exec::new(["getent", "passwd", USER]))?;
    let fields: Vec<String> = entry
        .stdout_text()
        .trim()
        .split(':')
        .map(str::to_owned)
        .collect();
    if fields.len() < 7 {
        return Err(invalid("no test account in this lab environment"));
    }
    Ok(TestUser {
        name: USER.into(),
        uid: fields[2]
            .parse()
            .map_err(|_| invalid("non-numeric test uid"))?,
        home: fields[5].clone(),
        password: String::new(),
    })
}

impl TestUser {
    fn runtime_dir(&self) -> String {
        format!("/run/user/{}", self.uid)
    }

    /// Environment of a process in this account's user manager, without a display.
    pub fn base_env(&self) -> Vec<(String, String)> {
        vec![
            ("HOME".into(), self.home.clone()),
            ("USER".into(), self.name.clone()),
            ("LOGNAME".into(), self.name.clone()),
            (
                "PATH".into(),
                "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin".into(),
            ),
            ("LANG".into(), "C.UTF-8".into()),
            ("XDG_RUNTIME_DIR".into(), self.runtime_dir()),
            (
                "DBUS_SESSION_BUS_ADDRESS".into(),
                format!("unix:path={}/bus", self.runtime_dir()),
            ),
        ]
    }

    /// `argv` as this account, in its home directory.
    pub fn exec<S: Into<String>>(&self, argv: impl IntoIterator<Item = S>) -> Exec {
        let mut exec = Exec::new(argv);
        exec.user = Some(self.name.clone());
        exec.env = self.base_env();
        exec.workdir = Some(self.home.clone());
        exec
    }
}

impl Session {
    /// Environment of a client in the running niri session.
    pub fn env(&self) -> Vec<(String, String)> {
        let mut env = self.user.base_env();
        env.extend([
            ("WAYLAND_DISPLAY".into(), self.wayland_display.clone()),
            ("NIRI_SOCKET".into(), self.niri_socket.clone()),
            ("XDG_SESSION_TYPE".into(), "wayland".into()),
            ("XDG_CURRENT_DESKTOP".into(), "niri".into()),
        ]);
        env
    }

    /// `argv` as the session user with the session's display environment.
    pub fn exec<S: Into<String>>(&self, argv: impl IntoIterator<Item = S>) -> Exec {
        let mut exec = self.user.exec(argv);
        exec.env = self.env();
        exec
    }
}

fn wait_until(what: &str, limit: Duration, mut probe: impl FnMut() -> Result<bool>) -> Result<()> {
    let deadline = Instant::now() + limit;
    loop {
        if probe()? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout {
                what: what.into(),
                after: limit,
            });
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn manager_environment(
    docker: &Docker,
    environment: &Environment,
    user: &TestUser,
) -> Result<Vec<(String, String)>> {
    let output = environment.run(
        docker,
        &user.exec(["systemctl", "--user", "show-environment"]),
    )?;
    Ok(output
        .stdout_text()
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect())
}

/// Start the real session for `user` and wait until niri and Noctalia answer IPC.
pub fn start(
    docker: &Docker,
    environment: &Environment,
    user: &TestUser,
    display: Display,
    host: Host,
) -> Result<Session> {
    let (mode, port) = match host {
        Host::Headless => ("headless", 0),
        Host::Waypipe { bridge_port } => ("waypipe", bridge_port),
    };
    docker.write_file(
        &environment.id,
        "/etc/kedra-lab/host.env",
        format!(
            "KEDRA_LAB_HOST={mode}\nKEDRA_LAB_WIDTH={}\nKEDRA_LAB_HEIGHT={}\nKEDRA_LAB_BRIDGE_PORT={port}\n",
            display.width, display.height
        )
        .as_bytes(),
        "root",
        "0644",
    )?;
    // Start the user manager first and create the unlocked login keyring with
    // the account password, as pam_gnome_keyring would during a password login.
    environment.run(
        docker,
        &Exec::new(["systemctl", "start", &format!("user@{}.service", user.uid)]),
    )?;
    let mut unlock = user.exec(["gnome-keyring-daemon", "--unlock", "--components=secrets"]);
    unlock.stdin = Some(user.password.clone().into_bytes());
    environment.run(docker, &unlock)?;
    environment.run(
        docker,
        &Exec::new([
            "systemd-run",
            "--quiet",
            "--unit=kedra-lab-session",
            "--property=PAMName=greetd",
            &format!("--property=User={}", user.name),
            "--property=WorkingDirectory=~",
            "--setenv=XDG_SESSION_TYPE=wayland",
            "--setenv=XDG_SESSION_CLASS=user",
            "/usr/libexec/kedra-session",
        ]),
    )?;
    let mut failures = 0;
    wait_until(
        "niri and kedra-noctalia user services",
        Duration::from_secs(90),
        || {
            let output = environment.exec(
                docker,
                &user.exec([
                    "systemctl",
                    "--user",
                    "is-active",
                    "niri.service",
                    "kedra-noctalia.service",
                ]),
            )?;
            let session = environment.exec(
                docker,
                &Exec::new(["systemctl", "is-active", "kedra-lab-session.service"]),
            )?;
            if session.stdout_text().trim() == "failed" {
                failures += 1;
            }
            if failures > 3 {
                return Err(invalid("kedra-lab-session.service failed; see journal.log"));
            }
            Ok(output.exit == 0)
        },
    )?;
    let manager = manager_environment(docker, environment, user)?;
    let lookup = |name: &str| {
        manager
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| invalid(format!("the session did not export {name}")))
    };
    let session = Session {
        user: user.clone(),
        wayland_display: lookup("WAYLAND_DISPLAY")?,
        niri_socket: lookup("NIRI_SOCKET")?,
        display,
    };
    wait_until("Noctalia IPC", Duration::from_secs(60), || {
        Ok(environment
            .exec(
                docker,
                &session
                    .exec(["noctalia", "msg", "log-level-status"])
                    .timeout(Duration::from_secs(10)),
            )?
            .exit
            == 0)
    })?;
    if host == Host::Headless {
        environment.run(
            docker,
            &session.exec([
                "niri",
                "msg",
                "output",
                "winit",
                "scale",
                &display.scale.to_string(),
            ]),
        )?;
    }
    Ok(session)
}

/// Rebuild session details for a retained lab container.
pub fn existing(docker: &Docker, environment: &Environment) -> Result<Session> {
    let user = existing_user(docker, environment)?;
    let manager = manager_environment(docker, environment, &user)?;
    let lookup = |name: &str| {
        manager
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| invalid(format!("the lab session has no {name}; is it running?")))
    };
    let host = docker.read_file(&environment.id, "/etc/kedra-lab/host.env")?;
    let host = String::from_utf8_lossy(&host);
    let field = |name: &str| {
        host.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default()
    };
    Ok(Session {
        wayland_display: lookup("WAYLAND_DISPLAY")?,
        niri_socket: lookup("NIRI_SOCKET")?,
        display: Display {
            width: field("KEDRA_LAB_WIDTH"),
            height: field("KEDRA_LAB_HEIGHT"),
            scale: 0.0,
        },
        user,
    })
}

/// Capture the whole niri output as PNG into `destination`.
pub fn screenshot(
    docker: &Docker,
    environment: &Environment,
    session: &Session,
    destination: &Path,
) -> Result<PathBuf> {
    let temporary = format!("/tmp/kedra-lab-shot-{}.png", std::process::id());
    environment.run(docker, &session.exec(["grim", "-t", "png", &temporary]))?;
    let data = docker.read_file(&environment.id, &temporary)?;
    let _ = environment.exec(docker, &Exec::new(["rm", "-f", &temporary]));
    if !data.starts_with(b"\x89PNG") {
        return Err(invalid("grim did not produce a PNG"));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(destination, data)?;
    Ok(destination.to_path_buf())
}
