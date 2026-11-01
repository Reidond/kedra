//! Public `sysroot source` CLI over generated repositories. The `.kedra`
//! modules under `system/` declare each target: enabled target and
//! architecture pairs plan and archive, their files, home baseline and managed
//! home list enter the payload, and invalid declarations, unknown targets and
//! files committed outside `system/` and the development tree are refused.
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const IMAGE: &str = "usr/src/kedra/image";
const DESCRIPTOR: &str = r#"{"schema_version":1,"format":"kedra","entry":"system/catalog.kedra","lock":"system/packages.lock.json"}"#;
const REQUIRED: [&str; 10] = [
    "niri",
    "noctalia",
    "greetd",
    "tuigreet",
    "NetworkManager",
    "pipewire",
    "wireplumber",
    "polkit",
    "gnome-keyring",
    "gnome-keyring-pam",
];
const STEPS: &str = "glib_schemas = true; systemd = systemd { default_target = \"graphical.target\"; }; skel = true; initramfs = initramfs { modules = [\"bootc\", \"ostree\", \"crypt\", \"dm\", \"rootfs-block\", \"qemu\"]; drivers = [\"virtio_dma_buf\", \"virtio_gpu\", \"virtio_input\"]; };";
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
                    let fixture = Self(path);
                    fixture.git(&["init", "-q", "--template=", "-b", "fixture"]);
                    return fixture;
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
    /// The descriptor, policy and lock, and an entry module with `body`.
    fn system(&self, body: &str) {
        self.write(&format!("{IMAGE}/package-inputs.json"), DESCRIPTOR);
        let policy = serde_json::json!({"schema_version":1,"namespace":"kedra","targets":["desktop","qemu-arm64"],"repositories":["fedora","updates"],"required_packages":REQUIRED});
        self.write(&format!("{IMAGE}/package-policy.json"), &policy.to_string());
        self.write(
            "system/packages.lock.json",
            r#"{"schema_version":1,"sources":{}}"#,
        );
        self.catalog(body);
    }
    fn catalog(&self, body: &str) {
        self.write(
            "system/catalog.kedra",
            &format!(
                "language 1;\nnamespace \"kedra\";\nfoundation system {{ fedora = 44; packages = {}; }}\n{body}\n",
                serde_json::to_string(&REQUIRED).unwrap()
            ),
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
    fn plan(&self, host: &str) -> serde_json::Value {
        let output = self.source("plan", host, &["--json"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn refused(&self, host: &str, reason: &str) {
        let output = self.source("plan", host, &["--json"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && output.stdout.is_empty());
        assert!(stderr.contains(reason), "{host}: {stderr}");
    }
    /// Commit one entry module, require refusal, then restore `restore`.
    fn refused_catalog(&self, body: &str, reason: &str, restore: &str) {
        self.catalog(body);
        self.commit("refused declaration");
        self.refused("desktop", reason);
        self.catalog(restore);
        self.commit("restore declaration");
    }
    /// Commit one path, require refusal, then remove it again.
    fn refused_path(&self, path: &str, reason: &str) {
        self.write(path, "generated\n");
        self.commit("unrecognized path");
        self.refused("desktop", reason);
        self.git(&["rm", "-q", "--", path]);
        self.commit("remove unrecognized path");
        self.plan("desktop");
    }
    /// Destination to (mode, bytes) of one target's source archive, and its manifest.
    fn archive(&self, host: &str) -> (BTreeMap<String, (u32, Vec<u8>)>, serde_json::Value) {
        let archive = self.0.join(format!("{host}-{}.tar", rand_suffix()));
        let output = self.source("archive", host, &["--output", archive.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut files = BTreeMap::new();
        for entry in tar::Archive::new(fs::File::open(&archive).unwrap())
            .entries()
            .unwrap()
        {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().into_owned();
            let mode = entry.header().mode().unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            files.insert(path, (mode, bytes));
        }
        let manifest = serde_json::from_slice(&files["usr/share/sysroot/source.json"].1).unwrap();
        (files, manifest)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn rand_suffix() -> u64 {
    NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
}
fn target(id: &str, architecture: &str, extra: &str) -> String {
    format!(
        "target \"{id}\" {{ architecture = \"{architecture}\"; image = \"ghcr.io/reidond/kedra-{id}\"; hardware = \"synthetic\"; foundation = system; {extra} {STEPS} }}"
    )
}

#[test]
fn source_cli_plans_declared_packages_and_steps_and_refuses_invalid_inputs() {
    let f = Fixture::new();
    let desktop = target("desktop", "x86_64", "");
    f.system(&desktop);
    f.commit("language package requests");
    let plan = f.plan("desktop");
    let mut required: Vec<_> = REQUIRED.to_vec();
    required.sort();
    assert_eq!(plan["packages"], serde_json::json!(required));
    assert_eq!(plan["package_frontend"]["format"], "kedra");
    assert_eq!(plan["native_steps"].as_array().map(Vec::len), Some(4));
    assert_eq!(plan["target"]["hardware_status"], "synthetic");
    let revision = plan["source_revision"].as_str().unwrap().to_owned();

    f.refused_catalog(
        &desktop.replace(
            "foundation = system;",
            "foundation = system; remove = [\"kernel-core\"];",
        ),
        "bootc foundation contract",
        &desktop,
    );
    f.refused_catalog(
        &desktop.replace(STEPS, ""),
        "must declare glib_schemas, systemd, skel and initramfs",
        &desktop,
    );
    f.refused_catalog(
        &desktop.replace("\"ostree\", ", ""),
        "omits required boot, storage or QEMU display modules",
        &desktop,
    );
    f.refused_catalog(
        &desktop.replace("hardware = \"synthetic\"; ", ""),
        "declares no hardware",
        &desktop,
    );

    // Uncommitted edits never change a plan.
    let committed = f.plan("desktop");
    f.write("system/catalog.kedra", "uncommitted invalid data");
    assert_eq!(f.plan("desktop"), committed);
    f.git(&["checkout", "--", "system/catalog.kedra"]);

    f.write(
        &format!("{IMAGE}/package-inputs.json"),
        &DESCRIPTOR.replace("\"schema_version\":1", "\"schema_version\":2"),
    );
    f.commit("unknown format version");
    f.refused("desktop", "unsupported package format");
    f.git(&["rm", "-q", &format!("{IMAGE}/package-inputs.json")]);
    f.commit("no descriptor");
    f.refused("desktop", "lacks the system/ declaration descriptor");
    f.git(&["checkout", "-q", "--detach", &revision]);
    let lock_path = "system/packages.lock.json";
    fs::remove_file(f.repo().join(lock_path)).unwrap();
    std::os::unix::fs::symlink(
        r#"{"schema_version":1,"sources":{}}"#,
        f.repo().join(lock_path),
    )
    .unwrap();
    f.commit("symlink-shaped package lock");
    f.refused("desktop", "ordinary file");
}

#[test]
fn source_cli_accepts_only_enabled_target_architectures() {
    let f = Fixture::new();
    let qemu = target(
        "qemu-arm64",
        "aarch64",
        "fedora = [\"qemu-guest-agent\"]; files = { \"/usr/lib/environment.d/70-fixture.conf\" = text \"GSK_RENDERER=gl\\n\"; };",
    );
    let both = format!("{}\n{qemu}", target("desktop", "x86_64", ""));
    f.system(&both);
    f.commit("generated targets");
    for (host, architecture) in [("desktop", "x86_64"), ("qemu-arm64", "aarch64")] {
        let plan = f.plan(host);
        assert_eq!(plan["target"]["id"], host);
        assert_eq!(plan["target"]["architecture"], architecture);
        assert_eq!(plan["target"]["candidate_target"], true);
    }
    let (files, manifest) = f.archive("qemu-arm64");
    assert_eq!(
        files["usr/lib/environment.d/70-fixture.conf"],
        (0o644, b"GSK_RENDERER=gl\n".to_vec())
    );
    assert_eq!(manifest["target"]["id"], "qemu-arm64");
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

    // Targets outside the policy, and pairs outside the release table, are refused.
    f.refused("xps", "denied target");
    f.refused_catalog(
        &both.replace("\"x86_64\"", "\"aarch64\""),
        "target desktop with architecture \"aarch64\"",
        &both,
    );
    f.refused_catalog(
        &both.replace("kedra-desktop", "kedra-other"),
        "is not enabled",
        &both,
    );
}

#[test]
fn source_cli_builds_the_payload_from_system_declarations() {
    let f = Fixture::new();
    let sets = r##"
set session {
    files = {
        "/etc/greetd/config.toml" = text """
            [terminal]
            vt = 1
            """;
        "/usr/libexec/fixture-session" = executable(text "#!/bin/sh\n");
        "/usr/share/doc/fixture/README.md" = file("./doc/README.md");
    };
    home = {
        ".config/plain.conf" = text "plain\n";
        ".config/app/main.conf" = managed {
            content = file("./home/main.conf");
            check = ["/usr/bin/test", "-s", "{file}"];
            reload = ["/usr/bin/true"];
        };
    };
    systemd = systemd { enable = ["greetd.service"]; };
}"##;
    let desktop = format!(
        "{sets}\n{}",
        target("desktop", "x86_64", "use = [session];")
    );
    f.write("system/doc/README.md", "readme\n");
    f.write("system/home/main.conf", "main\n");
    f.system(&desktop);
    // Development files beside system/ never become payload.
    f.write("README.md", "generated\n");
    f.write(".github/workflows/check.yml", "on: push\n");
    f.write("usr/src/kedra/docs/fixture.md", "generated\n");
    f.write("crates/fixture/main.rs", "fn main() {}\n");
    f.commit("system declarations");

    let (files, manifest) = f.archive("desktop");
    assert_eq!(
        files["etc/greetd/config.toml"],
        (0o644, b"[terminal]\nvt = 1\n".to_vec())
    );
    assert_eq!(
        files["usr/libexec/fixture-session"],
        (0o755, b"#!/bin/sh\n".to_vec())
    );
    assert_eq!(files["usr/share/doc/fixture/README.md"].1, b"readme\n");
    assert_eq!(
        files["usr/share/sysroot/home/default/.config/plain.conf"].1,
        b"plain\n"
    );
    assert_eq!(
        files["usr/share/sysroot/home/default/.config/app/main.conf"].1,
        b"main\n"
    );
    // Only managed home files are listed for `sysroot home`.
    let list = String::from_utf8(files["usr/share/kedra/home-files.toml"].1.clone()).unwrap();
    assert!(list.contains("schema_version = 1\n"));
    assert!(list.contains(
        "[[file]]\npath = \".config/app/main.conf\"\ncheck = [\"/usr/bin/test\",\"-s\",\"{file}\"]\nreload = [\"/usr/bin/true\"]\n"
    ));
    assert!(!list.contains("plain.conf"));
    assert_eq!(files.len(), 7, "{:?}", files.keys());

    let file = |destination: &str| {
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["destination"] == destination)
            .unwrap()
            .clone()
    };
    let main = file("usr/share/sysroot/home/default/.config/app/main.conf");
    assert_eq!(main["source_path"], "system/home/main.conf");
    assert_eq!(main["origin"], "system/catalog.kedra#session");
    assert_eq!(main["home_baseline"], true);
    assert!(main["git_blob"].is_string());
    let inline = file("etc/greetd/config.toml");
    assert_eq!(inline["source_path"], "system/catalog.kedra");
    assert!(inline.get("git_blob").is_none());
    assert_eq!(inline["home_baseline"], false);
    let steps = manifest["native_steps"].as_array().unwrap();
    assert!(steps.iter().any(|step| step["kind"] == "home_checks"
        && step["checks"][".config/app/main.conf"][0] == "/usr/bin/test"));
    assert!(
        steps.iter().any(|step| step["kind"] == "systemd"
            && step["enable"] == serde_json::json!(["greetd.service"]))
    );

    // Modules compose; they never override each other.
    let twice = desktop.replace(
        "use = [session];",
        "use = [session]; files = { \"/etc/greetd/config.toml\" = text \"x\\n\"; };",
    );
    f.refused_catalog(&twice, "file declared twice", &desktop);
    for (path, reason) in [
        ("/usr/share/kedra/home-files.toml", "generated path"),
        ("/etc/shadow", "credential, runtime or generated path"),
        ("/etc/skel/.bashrc", "allowed absolute /etc or /usr path"),
        ("/var/lib/fixture", "allowed absolute /etc or /usr path"),
    ] {
        let declared = desktop.replace(
            "use = [session];",
            &format!("use = [session]; files = {{ \"{path}\" = text \"x\\n\"; }};"),
        );
        f.refused_catalog(&declared, reason, &desktop);
    }
    f.refused_catalog(
        &desktop.replace(
            "content = file(\"./home/main.conf\");",
            "content = text \"main\\n\";",
        ),
        "managed home content must be an admitted file",
        &desktop,
    );
    f.refused_catalog(
        &desktop.replace("\".config/plain.conf\"", "\".ssh/config\""),
        "unsafe, private or case-colliding home path",
        &desktop,
    );

    // Only system/ declares what enters an image.
    for (path, reason) in [
        (
            "etc/greetd/config.toml",
            "unexpected committed path etc/greetd/config.toml",
        ),
        (
            "usr/lib/environment.d/60-fixture.conf",
            "declared in system/*.kedra",
        ),
        ("home/.config/niri/config.kdl", "unexpected committed path"),
        ("opt/fixture/tool", "unexpected committed path"),
        ("usr/src/other/main.rs", "unexpected committed path"),
    ] {
        f.refused_path(path, reason);
    }
}
