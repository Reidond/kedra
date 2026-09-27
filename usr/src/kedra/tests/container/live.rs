//! Live view on the developer's Mac: cocoa-way (a macOS Wayland compositor)
//! shows the nested niri window through waypipe.
//!
//! OrbStack refuses container connections to macOS-created Unix sockets, so
//! the waypipe client's socket is bridged over loopback TCP: this process
//! forwards `127.0.0.1:<port>` to the client socket, and the lab container
//! reaches it as `host.docker.internal` (see lab/kedra-lab-host).
//!
//! `live-tools` builds both tools from pinned, checksummed source tarballs
//! (the ones the J-x-Z Homebrew tap uses) into target/kedra-lab/tools, without
//! Homebrew taps. Native build dependencies must already be installed.

use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::{Error, Result, artifact_root, invalid};

struct Source {
    name: &'static str,
    url: &'static str,
    sha256: &'static str,
    directory: &'static str,
    cargo_args: &'static [&'static str],
    binaries: &'static [&'static str],
}

/// Pinned exactly as J-x-Z/homebrew-tap's cocoa-way.rb and waypipe-darwin.rb.
const SOURCES: &[Source] = &[
    Source {
        name: "cocoa-way 2.0.3",
        url: "https://github.com/J-x-Z/cocoa-way/archive/refs/tags/v2.0.3.tar.gz",
        sha256: "5251280acbf81f7e9b8e42476b0d6f78d1f008aee9d83e7c39a32f90fdaa2054",
        directory: "cocoa-way-2.0.3",
        cargo_args: &[],
        binaries: &["cocoa-way", "cocoa-wayctl"],
    },
    Source {
        name: "waypipe-darwin 0.11.0-darwin.1",
        url: "https://github.com/J-x-Z/waypipe-darwin/archive/refs/tags/v0.11.0-darwin.1.tar.gz",
        sha256: "5d1d0f1c384a1ca0fc7ef492ca73055343842fde4856cdbb77a7c2a2836cd6fa",
        directory: "waypipe-darwin-0.11.0-darwin.1",
        cargo_args: &["--no-default-features", "--features", "lz4,zstd"],
        binaries: &["waypipe"],
    },
];

const BINDGEN_VERSION: &str = "0.72.1";
const XCODE_LIBCLANG: &str =
    "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib";

pub fn tools_dir() -> PathBuf {
    artifact_root().join("tools")
}

pub fn tool(name: &str) -> PathBuf {
    tools_dir().join("bin").join(name)
}

pub fn directory(name: &str) -> PathBuf {
    artifact_root().join("live").join(name)
}

fn run(what: &str, command: &mut Command) -> Result<()> {
    let status = command.stdin(Stdio::null()).status()?;
    if !status.success() {
        return Err(Error::Command {
            what: what.into(),
            exit: i64::from(status.code().unwrap_or(-1)),
            stderr: String::new(),
        });
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    crate::builder::file_sha256(path)
}

/// Download, verify and build cocoa-way and waypipe into target/kedra-lab/tools/bin.
pub fn build_tools() -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(invalid(
            "cocoa-way is a macOS compositor; live tools build on macOS only",
        ));
    }
    let missing: Vec<&str> = ["xkbcommon", "liblz4", "libzstd", "pixman-1"]
        .into_iter()
        .filter(|module| {
            !Command::new("pkg-config")
                .args(["--exists", module])
                .status()
                .is_ok_and(|status| status.success())
        })
        .collect();
    if !missing.is_empty() {
        return Err(invalid(format!(
            "missing native libraries for pkg-config: {}; install them (Homebrew core: libxkbcommon, lz4, zstd, pixman)",
            missing.join(", ")
        )));
    }
    if !Path::new(XCODE_LIBCLANG).join("libclang.dylib").is_file() {
        return Err(invalid("bindgen needs Xcode's libclang; install Xcode"));
    }
    let root = tools_dir();
    let sources = root.join("src");
    let target = root.join("build");
    fs::create_dir_all(&sources)?;
    fs::create_dir_all(root.join("bin"))?;
    if !tool("bindgen").is_file() {
        eprintln!(
            "kedra-lab: building bindgen-cli {BINDGEN_VERSION} (build-time tool for waypipe)"
        );
        run(
            "cargo install bindgen-cli",
            Command::new("cargo")
                .args([
                    "install",
                    "--locked",
                    "--quiet",
                    "bindgen-cli",
                    "--version",
                    BINDGEN_VERSION,
                    "--root",
                ])
                .arg(&root)
                .env("CARGO_TARGET_DIR", &target)
                .env("LIBCLANG_PATH", XCODE_LIBCLANG),
        )?;
    }
    for source in SOURCES {
        let archive = sources.join(format!("{}.tar.gz", source.directory));
        if !archive.is_file() || sha256_file(&archive)? != source.sha256 {
            eprintln!("kedra-lab: downloading {}", source.name);
            run(
                "curl",
                Command::new("curl")
                    .args(["-sSLf", "--proto", "=https", "-o"])
                    .arg(&archive)
                    .arg(source.url),
            )?;
        }
        let digest = sha256_file(&archive)?;
        if digest != source.sha256 {
            fs::remove_file(&archive)?;
            return Err(invalid(format!(
                "{}: sha256 {digest} does not match the pinned {}",
                source.name, source.sha256
            )));
        }
        run(
            "tar",
            Command::new("tar")
                .arg("-xzf")
                .arg(&archive)
                .arg("-C")
                .arg(&sources),
        )?;
        eprintln!("kedra-lab: building {}", source.name);
        let path = format!(
            "{}:{}",
            root.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        run(
            &format!("cargo build {}", source.name),
            Command::new("cargo")
                .args(["build", "--release", "--locked", "--quiet"])
                .args(source.cargo_args)
                .current_dir(sources.join(source.directory))
                .env("CARGO_TARGET_DIR", &target)
                .env("LIBCLANG_PATH", XCODE_LIBCLANG)
                .env("PATH", path),
        )?;
        for binary in source.binaries {
            fs::copy(target.join("release").join(binary), tool(binary))?;
        }
    }
    println!(
        "cocoa-way and waypipe are in {}",
        root.join("bin").display()
    );
    Ok(())
}

fn cocoa_socket() -> Option<(PathBuf, String)> {
    let runtime = std::env::temp_dir().join("cocoa-way");
    let mut sockets: Vec<String> = fs::read_dir(&runtime)
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| {
            name.starts_with("wayland-") && name[8..].bytes().all(|b| b.is_ascii_digit())
        })
        .collect();
    sockets.sort();
    let name = sockets.into_iter().next()?;
    UnixStream::connect(runtime.join(&name)).ok()?;
    Some((runtime, name))
}

fn wait_for(what: &str, limit: Duration, mut ready: impl FnMut() -> bool) -> Result<()> {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if ready() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(Error::Timeout {
        what: what.into(),
        after: limit,
    })
}

fn spawn_detached(command: &mut Command, log: &Path, pid_file: &Path) -> Result<()> {
    let output = fs::File::create(log)?;
    let child = command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .spawn()?;
    fs::write(pid_file, child.id().to_string())?;
    Ok(())
}

/// Start (or reuse) cocoa-way, a waypipe client for `name`, and the TCP
/// bridge; return the bridge port for the container.
pub fn prepare(name: &str) -> Result<u16> {
    for binary in ["cocoa-way", "waypipe"] {
        if !tool(binary).is_file() {
            return Err(invalid(format!(
                "{} is missing; build it once with `kedra-lab live-tools`",
                tool(binary).display()
            )));
        }
    }
    let live = directory(name);
    fs::create_dir_all(&live)?;
    stop(name);
    if cocoa_socket().is_none() {
        eprintln!("kedra-lab: starting cocoa-way");
        spawn_detached(
            &mut Command::new(tool("cocoa-way")),
            &live.join("cocoa-way.log"),
            &live.join("cocoa-way.pid"),
        )?;
        wait_for(
            "cocoa-way's Wayland socket",
            Duration::from_secs(30),
            || cocoa_socket().is_some(),
        )?;
    }
    let (runtime, display) =
        cocoa_socket().ok_or_else(|| invalid("cocoa-way has no Wayland socket"))?;
    let socket = live.join("waypipe.sock");
    let _ = fs::remove_file(&socket);
    spawn_detached(
        Command::new(tool("waypipe"))
            .arg("--socket")
            .arg(&socket)
            .arg("client")
            .env("XDG_RUNTIME_DIR", &runtime)
            .env("WAYLAND_DISPLAY", &display),
        &live.join("waypipe-client.log"),
        &live.join("waypipe.pid"),
    )?;
    wait_for("the waypipe client socket", Duration::from_secs(10), || {
        socket.exists()
    })?;
    let port_file = live.join("bridge.port");
    let _ = fs::remove_file(&port_file);
    spawn_detached(
        Command::new(std::env::current_exe()?)
            .args(["bridge", "--socket"])
            .arg(&socket)
            .arg("--port-file")
            .arg(&port_file),
        &live.join("bridge.log"),
        &live.join("bridge.pid"),
    )?;
    wait_for("the TCP bridge", Duration::from_secs(10), || {
        port_file.exists()
    })?;
    fs::read_to_string(&port_file)?
        .trim()
        .parse()
        .map_err(|_| invalid("the bridge wrote no port"))
}

/// Stop the waypipe client and bridge of `name`; cocoa-way keeps running.
pub fn stop(name: &str) {
    let live = directory(name);
    for pid_file in ["bridge.pid", "waypipe.pid"] {
        if let Ok(pid) = fs::read_to_string(live.join(pid_file)) {
            let pid = pid.trim();
            if !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()) {
                let _ = Command::new("kill").arg(pid).stderr(Stdio::null()).status();
            }
            let _ = fs::remove_file(live.join(pid_file));
        }
    }
}

/// Forward every loopback TCP connection to the waypipe client socket.
pub fn bridge(socket: &Path, port_file: &Path) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let mut file = fs::File::create(port_file)?;
    writeln!(file, "{port}")?;
    for stream in listener.incoming() {
        let tcp = stream?;
        let unix = UnixStream::connect(socket)?;
        let (mut tcp_read, mut unix_write) = (tcp.try_clone()?, unix.try_clone()?);
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut tcp_read, &mut unix_write);
            let _ = unix_write.shutdown(std::net::Shutdown::Write);
        });
        let (mut unix_read, mut tcp_write): (UnixStream, TcpStream) = (unix, tcp);
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut unix_read, &mut tcp_write);
            let _ = tcp_write.shutdown(std::net::Shutdown::Write);
        });
    }
    Ok(())
}
