//! Public `sysroot source` CLI over generated repositories: enabled target and
//! architecture pairs plan and archive; mismatched, unknown and disabled targets fail.
//! A retained legacy-layout commit and its moved root-filesystem form resolve to
//! the same payload; mixed or unrecognized layouts are refused.
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const IMAGE: &str = "usr/src/kedra/image";
const TARGET: &str = "id='{id}'\narchitecture='{architecture}'\nimage='ghcr.io/reidond/kedra-{id}'\nfedora_release=44\ncandidate_target={enabled}\nhardware_status='synthetic'\n";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "kedra-source-targets-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::create_dir(path.join("checkout")).unwrap();
                    return Self(path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!(
                    "cannot create fixture directory {}: {error}",
                    path.display()
                ),
            }
        }
    }
    fn repo(&self) -> PathBuf {
        self.0.join("checkout")
    }
    fn git(&self, args: &[&str]) {
        let mut command = Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let output = command
            .env("HOME", &self.0)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .arg("-C")
            .arg(self.repo())
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn host(&self, id: &str, architecture: &str, enabled: bool) {
        self.declare(
            &format!("{IMAGE}/targets/{id}/target.toml"),
            id,
            architecture,
            enabled,
        );
    }
    fn declare(&self, path: &str, id: &str, architecture: &str, enabled: bool) {
        let text = TARGET
            .replace("{id}", id)
            .replace("{architecture}", architecture)
            .replace("{enabled}", &enabled.to_string());
        self.write(path, &text);
    }
    fn commit(&self, message: &str) {
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", message]);
    }
    fn source(&self, operation: &str, host: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sysroot"))
            .env("HOME", &self.0)
            .args(["source", operation, "--repo"])
            .arg(self.repo())
            .args(["--host", host])
            .args(extra)
            .output()
            .unwrap()
    }
    fn plan(&self, host: &str) -> serde_json::Value {
        let output = self.source("plan", host, &["--json"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    /// Commit one unrecognized path, require refusal, then remove it again.
    fn refused_path(&self, path: &str, reason: &str) {
        self.write(path, "generated\n");
        self.commit("unrecognized path");
        self.refused("desktop", reason);
        self.git(&["rm", "-q", "--", path]);
        self.commit("remove unrecognized path");
        self.plan("desktop");
    }
    fn refused(&self, host: &str, reason: &str) {
        let output = self.source("plan", host, &["--json"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && output.stdout.is_empty());
        assert!(stderr.contains(reason), "{host}: {stderr}");
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn source_cli_accepts_only_enabled_target_architectures() {
    let f = Fixture::new();
    f.git(&["init", "-q", "--template=", "-b", "fixture"]);
    f.write(&format!("{IMAGE}/packages.list"), "niri\n");
    f.write(&format!("{IMAGE}/remove.list"), "# none\n");
    f.host("desktop", "x86_64", true);
    f.write(
        &format!("{IMAGE}/targets/desktop/packages.list"),
        "# none\n",
    );
    f.host("qemu-arm64", "aarch64", true);
    f.write(
        &format!("{IMAGE}/targets/qemu-arm64/packages.list"),
        "qemu-guest-agent\n",
    );
    f.write(
        &format!("{IMAGE}/targets/qemu-arm64/usr/lib/environment.d/70-fixture.conf"),
        "GSK_RENDERER=gl\n",
    );
    f.host("xps", "x86_64", false);
    f.write(&format!("{IMAGE}/targets/xps/packages.list"), "# none\n");
    f.host("arm", "aarch64", true);
    f.write(&format!("{IMAGE}/targets/arm/packages.list"), "# none\n");
    f.commit("generated targets");

    for (host, architecture) in [("desktop", "x86_64"), ("qemu-arm64", "aarch64")] {
        let plan = f.plan(host);
        assert_eq!(plan["target"]["id"], host);
        assert_eq!(plan["target"]["architecture"], architecture);
    }

    let archive = f.0.join("qemu-arm64.tar");
    let output = f.source(
        "archive",
        "qemu-arm64",
        &["--output", archive.to_str().unwrap()],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut paths = Vec::new();
    let mut manifest = None;
    for entry in tar::Archive::new(fs::File::open(&archive).unwrap())
        .entries()
        .unwrap()
    {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        if path == "usr/share/sysroot/source.json" {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            manifest = Some(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap());
        }
        paths.push(path);
    }
    assert!(paths.contains(&"usr/lib/environment.d/70-fixture.conf".to_owned()));
    let manifest = manifest.expect("archive embeds its source manifest");
    assert_eq!(manifest["target"]["id"], "qemu-arm64");
    assert_eq!(manifest["target"]["architecture"], "aarch64");
    assert_eq!(
        manifest["target"]["image"],
        "ghcr.io/reidond/kedra-qemu-arm64"
    );
    assert!(
        manifest["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|name| name == "qemu-guest-agent")
    );

    f.refused("xps", "target xps is disabled");
    f.refused(
        "arm",
        "target arm with architecture \"aarch64\" is not enabled",
    );
    f.host("qemu-arm64", "x86_64", true);
    f.host("desktop", "aarch64", true);
    f.host("utm", "aarch64", true);
    f.commit("mismatched architectures");
    f.refused(
        "qemu-arm64",
        "target qemu-arm64 with architecture \"x86_64\" is not enabled",
    );
    f.refused(
        "desktop",
        "target desktop with architecture \"aarch64\" is not enabled",
    );
    f.refused(
        "utm",
        "target utm with architecture \"aarch64\" is not enabled",
    );
}

/// Payload identity of a plan: everything except where the file was committed.
fn payload(plan: &serde_json::Value) -> Vec<(String, String, String, bool)> {
    plan["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| {
            (
                file["destination"].as_str().unwrap().to_owned(),
                file["sha256"].as_str().unwrap().to_owned(),
                file["mode"].as_str().unwrap().to_owned(),
                file["home_baseline"].as_bool().unwrap(),
            )
        })
        .collect()
}

#[test]
fn source_cli_resolves_a_legacy_commit_and_its_moved_root_filesystem_form_alike() {
    let f = Fixture::new();
    f.git(&["init", "-q", "--template=", "-b", "fixture"]);
    f.write("packages/common.list", "niri\n");
    f.write("packages/remove.list", "# none\n");
    f.declare("hosts/desktop/host.toml", "desktop", "x86_64", true);
    f.write("hosts/desktop/packages.list", "greetd\n");
    f.write("home/.config/niri/config.kdl", "layout {}\n");
    f.write("etc/greetd/config.toml", "[terminal]\nvt = 1\n");
    f.write("usr/lib/environment.d/60-fixture.conf", "SHARED=1\n");
    f.write(
        "hosts/desktop/usr/lib/environment.d/60-fixture.conf",
        "SHARED=desktop\n",
    );
    f.write(
        "hosts/desktop/home/.config/noctalia/config.toml",
        "[theme]\nmode='dark'\n",
    );
    f.commit("legacy layout");
    let legacy = f.plan("desktop");

    let overlay = format!("{IMAGE}/targets/desktop");
    for (from, to) in [
        ("packages/common.list", format!("{IMAGE}/packages.list")),
        ("packages/remove.list", format!("{IMAGE}/remove.list")),
        ("hosts/desktop/host.toml", format!("{overlay}/target.toml")),
        (
            "hosts/desktop/packages.list",
            format!("{overlay}/packages.list"),
        ),
        ("home/.config", "etc/skel/.config".to_owned()),
        ("hosts/desktop/usr", format!("{overlay}/usr")),
        (
            "hosts/desktop/home/.config",
            format!("{overlay}/etc/skel/.config"),
        ),
    ] {
        fs::create_dir_all(f.repo().join(&to).parent().unwrap()).unwrap();
        f.git(&["mv", from, &to]);
    }
    // Development files beside the image filesystem never become payload.
    f.write("README.md", "generated\n");
    f.write(".github/workflows/check.yml", "on: push\n");
    f.write("usr/src/kedra/crates/fixture/main.rs", "fn main() {}\n");
    f.write(&format!("{overlay}/README.md"), "generated\n");
    f.commit("root-filesystem layout");
    let moved = f.plan("desktop");

    assert_eq!(payload(&moved), payload(&legacy));
    for key in ["target", "packages", "remove_packages"] {
        assert_eq!(moved[key], legacy[key], "{key}");
    }
    let source = |plan: &serde_json::Value, destination: &str| {
        plan["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["destination"] == destination)
            .unwrap()["source_path"]
            .clone()
    };
    let niri = "usr/share/sysroot/home/default/.config/niri/config.kdl";
    let noctalia = "usr/share/sysroot/home/default/.config/noctalia/config.toml";
    let environment = "usr/lib/environment.d/60-fixture.conf";
    assert_eq!(source(&legacy, niri), "home/.config/niri/config.kdl");
    assert_eq!(source(&moved, niri), "etc/skel/.config/niri/config.kdl");
    assert_eq!(
        source(&moved, noctalia),
        format!("{overlay}/etc/skel/.config/noctalia/config.toml")
    );
    assert_eq!(
        source(&moved, environment),
        format!("{overlay}/usr/lib/environment.d/60-fixture.conf")
    );

    f.refused_path(
        "hosts/desktop/host.toml",
        "mixes the root-filesystem layout with legacy",
    );
    f.refused_path("opt/fixture/tool", "unexpected directory opt/");
    f.refused_path("usr/src/other/main.rs", "usr/src/ holds only");
    f.refused_path(
        &format!("{overlay}/notes.txt"),
        "unexpected file in target overlay",
    );
    f.refused_path(
        &format!("{overlay}/var/lib/fixture"),
        "unexpected directory var/",
    );
}
