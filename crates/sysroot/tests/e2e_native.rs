//! Native derivation through the public `sysroot derive` commands; installed
//! behavior uses the System harness. The fixture's committed `.kedra` target
//! block declares the steps, as the product's targets do.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};
use sysroot_engine::{PLATFORM, RPM_ARCHITECTURE, SystemContent, SystemDefinition, SystemFile};

const FOUNDATION: &str = "sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71";
/// Committed home baseline files (`system/home/`) and the check each declares.
const BASELINE: &[(&str, &str, &str)] = &[
    (
        ".config/niri/config.kdl",
        "niri/config.kdl",
        r#"["/usr/bin/niri", "validate", "--config", "{file}"]"#,
    ),
    (
        ".config/noctalia/config.toml",
        "noctalia/config.toml",
        r#"["/usr/bin/noctalia", "config", "validate", "{file}"]"#,
    ),
    (
        ".config/noctalia/palettes/Adwaita.json",
        "noctalia/palettes/Adwaita.json",
        "",
    ),
];
/// Image files and the home baseline the fixture target declares.
const FIXTURE: &str = r#"
set native {
    fedora = ["bash"];
    files = {
        "/usr/share/glib-2.0/schemas/org.kedra.NativeFixture.gschema.xml" = text "<schemalist><schema id='org.kedra.NativeFixture' path='/org/kedra/native-fixture/'><key name='greeting' type='s'><default>'baseline'</default></key></schema></schemalist>\n";
        "/usr/lib/systemd/system/sysroot-native-fixture.service" = text "[Unit]\nDescription=Native derivation fixture\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=/usr/bin/printf native-fixture-ready\\n\n[Install]\nWantedBy=multi-user.target\n";
        "/usr/lib/systemd/system/sysroot-native-masked.service" = text "[Unit]\nDescription=Native mask fixture\n[Service]\nType=oneshot\nExecStart=/usr/bin/true\n";
    };
    home = {
        ".native-fixture" = text "derived-home-v1\n";
{home}
    };
}
"#;
/// Declared image steps of the fixture target; `{extra}` adds invalid variants.
const STEPS: &str = r#"
    glib_schemas = true;
    systemd = systemd {
        enable = ["sysroot-native-fixture.service"{enable}];
        mask = ["sysroot-native-masked.service"{mask}];
        default_target = "multi-user.target";
    };
    skel = true;
    initramfs = initramfs {
        modules = ["bootc", "ostree", "crypt", "dm", "rootfs-block", "qemu"];
        drivers = ["virtio_dma_buf", "virtio_gpu", "virtio_input"];
    };{extra}
"#;

/// The Kedra target of this machine's native architecture.
fn native_target() -> &'static str {
    if RPM_ARCHITECTURE == "aarch64" {
        "qemu-arm64"
    } else {
        "desktop"
    }
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn drain(mut stream: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                return bytes;
            }
            let keep = count.min((8 * 1024 * 1024_usize).saturating_sub(bytes.len()));
            bytes.extend_from_slice(&buffer[..keep]);
        }
    })
}

fn run(command: &mut Command) -> Output {
    let mut process = Process(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = drain(process.0.stdout.take().unwrap());
    let stderr = drain(process.0.stderr.take().unwrap());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = process.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(1800),
            "native CLI deadline exceeded: {command:?}"
        );
        thread::sleep(Duration::from_millis(25));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "status {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json_output(output: Output) -> Value {
    serde_json::from_slice(&success(output).stdout).unwrap()
}

fn refused(output: Output, message: &str) {
    assert!(
        !output.status.success(),
        "unexpected native success: {message}"
    );
    assert!(
        output.stdout.is_empty(),
        "native refusal wrote success data"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "expected {message:?}, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "native refusal: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

fn write_json(path: &Path, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn remove_fixture(path: &Path) -> std::io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            remove_fixture(&entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::remove_dir(path)
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    success(run(command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)))
}

struct Fixture {
    root: PathBuf,
    retain: bool,
    foundation: String,
}
impl Fixture {
    fn new() -> Self {
        let retained = std::env::var_os("KEDRA_NATIVE_E2E_RETAIN_DIR");
        let root = retained.as_ref().map_or_else(
            || {
                std::env::temp_dir().join(format!(
                    "kedra-native-e2e-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
            },
            PathBuf::from,
        );
        assert!(root.is_absolute(), "retained fixture path must be absolute");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.join("producer")).unwrap();
        Self {
            root,
            retain: retained.is_some(),
            foundation: std::env::var("KEDRA_CONTEXT_FOUNDATION")
                .unwrap_or_else(|_| FOUNDATION.into()),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    fn producer(&self, name: &str) -> PathBuf {
        self.path("producer").join(name)
    }
    fn cli(&self, args: &[&str]) -> Command {
        let binary = std::env::var_os("KEDRA_ENGINE_E2E_BINARY").map_or_else(
            || PathBuf::from(env!("CARGO_BIN_EXE_sysroot")),
            PathBuf::from,
        );
        let mut command = Command::new(binary);
        command.args(args);
        command
    }

    fn source(&self, relative: &str, bytes: &[u8]) {
        let path = self.producer("repo").join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn commit(&self) {
        git(&self.producer("repo"), &["add", "."]);
        git(
            &self.producer("repo"),
            &["commit", "-q", "-m", "generated native fixture"],
        );
    }

    /// Commit the catalog with the declared steps and optional invalid variants.
    fn catalog(&self, enable: &str, mask: &str, extra: &str) {
        self.catalog_with(enable, mask, extra, BASELINE);
    }

    /// The catalog declaring only `baseline` of the committed home files.
    fn catalog_with(&self, enable: &str, mask: &str, extra: &str, baseline: &[(&str, &str, &str)]) {
        let policy: Value =
            serde_json::from_slice(&workspace_file("usr/src/kedra/image/package-policy.json"))
                .unwrap();
        let mut packages: Vec<String> = policy["required_packages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| format!("{:?}", name.as_str().unwrap()))
            .collect();
        packages.push("\"bash\"".into());
        let steps = STEPS
            .replace("{enable}", enable)
            .replace("{mask}", mask)
            .replace("{extra}", extra);
        let home: Vec<String> = baseline
            .iter()
            .map(|(path, source, check)| {
                let check = if check.is_empty() {
                    String::new()
                } else {
                    format!(" check = {check};")
                };
                format!(
                    "        \"{path}\" = managed {{ content = file(\"./home/{source}\");{check} }};"
                )
            })
            .collect();
        let target = native_target();
        let catalog = format!(
            "language 1;\nnamespace \"kedra\";\nfoundation system {{\n    fedora = 44;\n    packages = [{}];\n}}\n{}\ntarget \"{target}\" {{\n    architecture = \"{}\";\n    image = \"ghcr.io/reidond/kedra-{target}\";\n    hardware = \"synthetic\";\n    foundation = system;\n    use = [native];{steps}}}\n",
            packages.join(", "),
            FIXTURE.replace("{home}", &home.join("\n")),
            sysroot_engine::RPM_ARCHITECTURE
        );
        self.source("system/catalog.kedra", catalog.as_bytes());
        self.commit();
    }

    fn repository(&self) {
        fs::create_dir(self.producer("repo")).unwrap();
        git(
            &self.producer("repo"),
            &["init", "-q", "--template=", "-b", "fixture"],
        );
        self.source(
            "usr/src/kedra/image/package-policy.json",
            &workspace_file("usr/src/kedra/image/package-policy.json"),
        );
        for (name, contents) in [
            (
                "usr/src/kedra/image/package-inputs.json",
                "{\"schema_version\":1,\"format\":\"kedra\",\"entry\":\"system/catalog.kedra\",\"lock\":\"system/packages.lock.json\"}\n",
            ),
            (
                "system/packages.lock.json",
                "{\"schema_version\":1,\"sources\":{}}\n",
            ),
        ] {
            self.source(name, contents.as_bytes());
        }
        // These are reviewed product inputs from committed source, never the controller's home.
        for (_, source, _) in BASELINE {
            let path = format!("system/home/{source}");
            self.source(&path, &workspace_file(&path));
        }
        self.catalog("", "", "");
    }

    /// `sysroot source plan` of the fixture's current commit.
    fn plan(&self) -> Output {
        run(self
            .cli(&["source", "plan", "--repo"])
            .arg(self.producer("repo"))
            .args(["--host", native_target(), "--json"]))
    }

    fn compose(&self, destination: &Path) -> Value {
        let definition = SystemDefinition {
            schema: 1,
            platform: PLATFORM.into(),
            foundation: self.foundation.clone(),
            provenance: BTreeMap::from([("fixture".into(), "native-artifacts-v1".into())]),
            outputs: BTreeMap::new(),
            required_packages: vec!["bash".into()],
            removed_packages: vec![],
            files: vec![SystemFile {
                path: "/usr/share/glib-2.0/schemas/99_native_fixture.gschema.override".into(),
                mode: 0o644,
                provenance: "native-fixture.glib-default".into(),
                priority: 0,
                replaces: None,
                content: SystemContent::Bytes(
                    b"[org.kedra.NativeFixture]\ngreeting='derived-default'\n".to_vec(),
                ),
            }],
        };
        let definition_path = self.producer("definition.json");
        write_json(&definition_path, &definition);
        json_output(run(self
            .cli(&["system", "compose", "--repo"])
            .arg(self.producer("repo"))
            .args(["--target", native_target(), "--store"])
            .arg(self.producer("store"))
            .args(["--foundation", &self.foundation, "--definition"])
            .arg(definition_path)
            .arg("--output-dir")
            .arg(destination)))
    }

    fn native(
        &self,
        command: &str,
        context: &Path,
        parent: &str,
        expected: Option<&str>,
    ) -> Output {
        let mut process = self.cli(&["derive", command, "--context"]);
        process
            .arg(context)
            .args(["--expected-identity", parent])
            .arg("--state")
            // KEDRA_LAB_ARTIFACTS=<retained root> lets the harness reuse these bindings.
            .arg(self.path("derive"));
        if let Some(identity) = expected {
            process.args(["--expected-derivation", identity]);
        }
        run(&mut process)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.retain || thread::panicking() {
            eprintln!("native E2E retained: {}", self.root.display());
        } else {
            let _ = remove_fixture(&self.root);
        }
    }
}

/// The repository's own committed file, never the controller's home.
fn workspace_file(path: &str) -> Vec<u8> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    git(workspace, &["show", &format!("HEAD:{path}")]).stdout
}

fn reject_baseline_deletion(f: &Fixture) {
    // The parent still has the first file; the new declaration drops it.
    f.catalog_with("", "", "", &BASELINE[1..]);
    let context = f.path("deletion-context");
    let composition = f.compose(&context);
    let parent = composition["identity"].as_str().unwrap();
    let planned = json_output(f.native("plan", &context, parent, None));
    refused(
        f.native("build", &context, parent, planned["identity"].as_str()),
        "refuses deletion of an inherited managed baseline file",
    );
    remove_fixture(&context).unwrap();
    f.catalog("", "", "");
}

/// Invalid step declarations are refused when the source is planned, before
/// anything is composed or derived.
fn reject_invalid_declarations(f: &Fixture) {
    for (enable, mask, extra, message) in [
        (
            "",
            "",
            "\n    run = [\"touch\"];",
            "unknown declaration field",
        ),
        (
            "",
            ", \"sysroot-native-fixture.service\"",
            "",
            "conflicting unit",
        ),
        (", \"../outside.service\"", "", "", "concrete unit"),
    ] {
        f.catalog(enable, mask, extra);
        refused(f.plan(), message);
    }
    f.catalog("", "", "");
    json_output(f.plan());
}

fn reject_invalid_derivations(f: &Fixture, parent: &str, expected: &str) {
    let context = f.path("context");
    let wrong = if expected.starts_with('0') {
        "1".repeat(64)
    } else {
        "0".repeat(64)
    };
    refused(
        f.native("build", &context, parent, Some(&wrong)),
        "native derivation identity differs",
    );
    // The in-image executor refuses to run on an ordinary machine.
    refused(
        run(f.cli(&["derive", "apply"]).arg(f.path("native-plan.json"))),
        "runs only as root inside the offline sysroot derive build",
    );
}

#[test]
#[ignore = "requires native Docker with BuildKit and a retained Kedra foundation (KEDRA_CONTEXT_FOUNDATION)"]
fn native_artifacts_derive_from_committed_configuration() {
    let f = Fixture::new();
    f.repository();
    reject_invalid_declarations(&f);
    json_output(run(f
        .cli(&["store", "init", "--store"])
        .arg(f.producer("store"))));
    json_output(run(f
        .cli(&["store", "add-image", "--store"])
        .arg(f.producer("store"))
        .args(["--image", &f.foundation])));
    reject_baseline_deletion(&f);
    let context = f.path("context");
    let composition = f.compose(&context);
    let parent = composition["identity"].as_str().unwrap();
    remove_fixture(&f.path("producer")).unwrap();
    assert!(!f.path("producer").exists());
    let planned = json_output(f.native("plan", &context, parent, None));
    write_json(&f.path("native-plan.json"), &planned);
    let identity = planned["identity"].as_str().unwrap();
    assert_ne!(identity, parent);
    write_json(
        &f.path("expected-identity.json"),
        &json!({"composition_identity":parent,"derivation_identity":identity,"foundation":f.foundation}),
    );
    reject_invalid_derivations(&f, parent, identity);
    let derived = json_output(f.native("build", &context, parent, Some(identity)));
    assert_eq!(derived["material"]["identity"], identity);
    assert_eq!(derived["material"]["parent_identity"], parent);
    assert_eq!(derived["material"]["rpm_sha256"], planned["rpm_sha256"]);
    assert!(
        !derived["material"]["kernels"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_ne!(derived["image"], derived["parent_image"]);
    // A retained binding is re-read from the image, not trusted, and reused.
    let repeated = json_output(f.native("build", &context, parent, Some(identity)));
    assert_eq!(repeated, derived);
    write_json(&f.path("derived-receipt.json"), &derived);
}
