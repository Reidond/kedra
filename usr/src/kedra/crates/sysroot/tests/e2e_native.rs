//! Native derivation through public CLI commands; installed behavior uses the System harness.
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
use sysroot_engine::{
    NativeDefinition, NativeStep, PLATFORM, SystemContent, SystemDefinition, SystemFile,
};

const FOUNDATION: &str = "sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71";
const BASELINE: &[&str] = &[
    ".config/niri/config.kdl",
    ".config/noctalia/config.toml",
    ".config/noctalia/palettes/Adwaita.json",
];

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

    fn repository(&self) {
        fs::create_dir(self.producer("repo")).unwrap();
        git(
            &self.producer("repo"),
            &["init", "-q", "--template=", "-b", "fixture"],
        );
        for (name, contents) in [
            ("usr/src/kedra/image/packages.list", "bash\n"),
            ("usr/src/kedra/image/remove.list", "# none\n"),
            (
                "usr/src/kedra/image/targets/qemu-arm64/packages.list",
                "# none\n",
            ),
            (
                "usr/src/kedra/image/targets/qemu-arm64/target.toml",
                "id='qemu-arm64'\narchitecture='aarch64'\nimage='ghcr.io/reidond/kedra-qemu-arm64'\nfedora_release=44\ncandidate_target=true\nhardware_status='synthetic'\n",
            ),
            (
                "usr/share/glib-2.0/schemas/org.kedra.NativeFixture.gschema.xml",
                "<schemalist><schema id='org.kedra.NativeFixture' path='/org/kedra/native-fixture/'><key name='greeting' type='s'><default>'baseline'</default></key></schema></schemalist>\n",
            ),
            (
                "usr/lib/systemd/system/sysroot-native-fixture.service",
                "[Unit]\nDescription=Native derivation fixture\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=/usr/bin/printf native-fixture-ready\\n\n[Install]\nWantedBy=multi-user.target\n",
            ),
            (
                "usr/lib/systemd/system/sysroot-native-masked.service",
                "[Unit]\nDescription=Native mask fixture\n[Service]\nType=oneshot\nExecStart=/usr/bin/true\n",
            ),
            ("etc/skel/.native-fixture", "derived-home-v1\n"),
        ] {
            self.source(name, contents.as_bytes());
        }
        // These are reviewed product inputs from committed source, never the controller's home.
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(5)
            .unwrap();
        for relative in BASELINE {
            let path = format!("etc/skel/{relative}");
            self.source(
                &path,
                &git(workspace, &["show", &format!("HEAD:{path}")]).stdout,
            );
        }
        self.commit();
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
            .args(["--target", "qemu-arm64", "--store"])
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
        spec: &Path,
        expected: Option<&str>,
    ) -> Output {
        let binary = std::env::var_os("KEDRA_NATIVE_LAB_BINARY")
            .or_else(|| std::env::var_os("KEDRA_CONTEXT_LAB_BINARY"))
            .expect("set KEDRA_NATIVE_LAB_BINARY to the already built kedra-lab executable");
        let mut process = Command::new(binary);
        process
            .args([command, "--image"])
            .arg(format!("composition:{}", context.display()))
            .args([
                "--composition-identity",
                parent,
                "--target",
                "qemu-arm64",
                "--native-plan",
            ])
            .arg(spec);
        if let Some(identity) = expected {
            process.args(["--derivation-identity", identity]);
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

fn declaration(parent: &str) -> NativeDefinition {
    NativeDefinition {
        schema: 1,
        parent_identity: parent.into(),
        steps: vec![
            NativeStep::GlibSchemas,
            NativeStep::Systemd {
                enable: vec!["sysroot-native-fixture.service".into()],
                disable: vec![],
                mask: vec!["sysroot-native-masked.service".into()],
                default_target: Some("multi-user.target".into()),
            },
            NativeStep::InitialSkel,
            NativeStep::QemuInitramfs {
                required_modules: sysroot_engine::default_native_modules(),
            },
        ],
    }
}

fn reject_baseline_deletion(f: &Fixture) {
    let missing = f.producer("repo/etc/skel").join(BASELINE[0]);
    let original = fs::read(&missing).unwrap();
    fs::remove_file(&missing).unwrap();
    f.commit();
    let context = f.path("deletion-context");
    let composition = f.compose(&context);
    let parent = composition["identity"].as_str().unwrap();
    let definition = NativeDefinition {
        schema: 1,
        parent_identity: parent.into(),
        steps: vec![NativeStep::InitialSkel],
    };
    let spec = f.path("deletion-plan.json");
    write_json(&spec, &definition);
    let planned = json_output(f.native("derive-plan", &context, parent, &spec, None));
    refused(
        f.native(
            "derive",
            &context,
            parent,
            &spec,
            planned["identity"].as_str(),
        ),
        "refuses deletion of an inherited managed baseline file",
    );
    remove_fixture(&context).unwrap();
    fs::remove_file(spec).unwrap();
    fs::write(missing, original).unwrap();
    f.commit();
}

fn reject_invalid_plans(f: &Fixture, parent: &str, expected: &str) {
    let context = f.path("context");
    let spec = f.path("invalid-plan.json");
    write_json(
        &spec,
        &json!({"schema":1,"parent_identity":parent,"steps":[{"kind":"run","argv":["touch","/unapproved"]}]}),
    );
    refused(
        f.native("derive-plan", &context, parent, &spec, None),
        "unknown variant",
    );
    let mut invalid = declaration(parent);
    if let NativeStep::Systemd { mask, .. } = &mut invalid.steps[1] {
        mask.push("sysroot-native-fixture.service".into());
    }
    write_json(&spec, &invalid);
    refused(
        f.native("derive-plan", &context, parent, &spec, None),
        "conflicting unit",
    );
    let mut invalid = declaration(parent);
    if let NativeStep::Systemd { enable, .. } = &mut invalid.steps[1] {
        enable[0] = "../outside.service".into();
    }
    write_json(&spec, &invalid);
    refused(
        f.native("derive-plan", &context, parent, &spec, None),
        "concrete unit",
    );
    fs::remove_file(spec).unwrap();
    let wrong = if expected.starts_with('0') {
        "1".repeat(64)
    } else {
        "0".repeat(64)
    };
    refused(
        f.native(
            "derive",
            &context,
            parent,
            &f.path("native-plan.json"),
            Some(&wrong),
        ),
        "native derivation identity differs",
    );
}

#[test]
#[ignore = "requires native ARM Docker, exact restored foundation, and KEDRA_NATIVE_LAB_BINARY"]
fn native_artifacts_derive_from_committed_configuration() {
    let f = Fixture::new();
    f.repository();
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
    let spec = f.path("native-plan.json");
    write_json(&spec, &declaration(parent));
    remove_fixture(&f.path("producer")).unwrap();
    assert!(!f.path("producer").exists());
    let planned = json_output(f.native("derive-plan", &context, parent, &spec, None));
    let identity = planned["identity"].as_str().unwrap();
    assert_ne!(identity, parent);
    write_json(
        &f.path("expected-identity.json"),
        &json!({"composition_identity":parent,"derivation_identity":identity,"foundation":f.foundation}),
    );
    reject_invalid_plans(&f, parent, identity);
    let derived = json_output(f.native("derive", &context, parent, &spec, Some(identity)));
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
    write_json(&f.path("derived-receipt.json"), &derived);
}
