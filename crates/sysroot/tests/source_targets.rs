//! Public `sysroot source` CLI over generated repositories: enabled target and
//! architecture pairs plan and archive; mismatched, unknown and disabled targets fail.
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-source-targets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.join("checkout")).unwrap();
        Self(path)
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
        self.write(
            &format!("hosts/{id}/host.toml"),
            &format!("id='{id}'\narchitecture='{architecture}'\nimage='ghcr.io/reidond/kedra-{id}'\nfedora_release=44\ncandidate_target={enabled}\nhardware_status='synthetic'\n"),
        );
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
    f.write("packages/common.list", "niri\n");
    f.write("packages/remove.list", "# none\n");
    f.host("desktop", "x86_64", true);
    f.write("hosts/desktop/packages.list", "# none\n");
    f.host("utm", "aarch64", true);
    f.write("hosts/utm/packages.list", "qemu-guest-agent\n");
    f.write(
        "hosts/utm/usr/lib/environment.d/70-fixture.conf",
        "GSK_RENDERER=gl\n",
    );
    f.host("xps", "x86_64", false);
    f.write("hosts/xps/packages.list", "# none\n");
    f.host("arm", "aarch64", true);
    f.write("hosts/arm/packages.list", "# none\n");
    f.commit("generated targets");

    for (host, architecture) in [("desktop", "x86_64"), ("utm", "aarch64")] {
        let output = f.source("plan", host, &["--json"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(plan["target"]["id"], host);
        assert_eq!(plan["target"]["architecture"], architecture);
    }

    let archive = f.0.join("utm.tar");
    let output = f.source("archive", "utm", &["--output", archive.to_str().unwrap()]);
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
    assert_eq!(manifest["target"]["id"], "utm");
    assert_eq!(manifest["target"]["architecture"], "aarch64");
    assert_eq!(manifest["target"]["image"], "ghcr.io/reidond/kedra-utm");
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
    f.host("utm", "x86_64", true);
    f.host("desktop", "aarch64", true);
    f.commit("mismatched architectures");
    f.refused(
        "utm",
        "target utm with architecture \"x86_64\" is not enabled",
    );
    f.refused(
        "desktop",
        "target desktop with architecture \"aarch64\" is not enabled",
    );
}
