//! Public composition workflows over committed fixtures and retained native images.
//! The ignored case executes real packages; context export does not qualify OS boot.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FOUNDATION: &str = "sha256:e402cca673711fee025f9ce21c6c08b1bd25ee26b3c482119c8675fbaaedbc85";
const BUILDER: &str = "sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546";
const OTHER_RUNTIME: &str =
    "sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3";
const IMAGE: &str = "usr/src/kedra/image";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = pipe.read(&mut buffer).unwrap();
            if count == 0 {
                return bytes;
            }
            let keep = count.min((8 * 1024 * 1024_usize).saturating_sub(bytes.len()));
            bytes.extend_from_slice(&buffer[..keep]);
        }
    })
}

fn run(command: &mut Command) -> Output {
    let mut child = Process(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = drain(child.0.stdout.take().unwrap());
    let stderr = drain(child.0.stderr.take().unwrap());
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(600),
            "composition subprocess deadline exceeded"
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

fn refused(output: Output, reason: &str) {
    assert!(!output.status.success(), "unexpected success for {reason}");
    assert!(output.stdout.is_empty(), "refusal wrote stdout");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostic.to_lowercase().contains(&reason.to_lowercase()),
        "wanted {reason:?}, got {diagnostic}"
    );
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-system-e2e-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(path.join("repo")).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
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

    fn git(&self, args: &[&str]) -> Output {
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
            .arg(self.path("repo"))
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

    fn write_source(&self, path: &str, contents: &str) {
        let path = self.path("repo").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn commit(&self) -> String {
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", "generated composition inputs"]);
        String::from_utf8(self.git(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_owned()
    }

    fn repository(&self) -> String {
        self.git(&["init", "-q", "--template=", "-b", "fixture"]);
        self.write_source(&format!("{IMAGE}/packages.list"), "bash\n");
        self.write_source(&format!("{IMAGE}/remove.list"), "# none\n");
        self.write_source(
            &format!("{IMAGE}/targets/qemu-arm64/packages.list"),
            "# none\n",
        );
        self.write_source(&format!("{IMAGE}/targets/qemu-arm64/target.toml"),
            "id='qemu-arm64'\narchitecture='aarch64'\nimage='ghcr.io/reidond/kedra-qemu-arm64'\nfedora_release=44\ncandidate_target=true\nhardware_status='synthetic'\n");
        self.write_source("etc/kedra-fixture.conf", "baseline\n");
        self.write_source("etc/committed-fixture.conf", "committed\n");
        self.write_source(
            "etc/skel/.config/fixture/settings",
            "writable-home-baseline\n",
        );
        self.commit()
    }

    fn write_json(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }

    fn system(
        &self,
        operation: &str,
        foundation: &str,
        definition: Option<&Path>,
        destination: Option<&Path>,
    ) -> Output {
        let mut command = self.cli(&["system", operation, "--repo"]);
        command
            .arg(self.path("repo"))
            .args(["--target", "qemu-arm64", "--store"])
            .arg(self.path("store"))
            .args(["--foundation", foundation])
            .env("GIT_DIR", self.path("ambient-missing-git"))
            .env("GIT_WORK_TREE", self.path("ambient-missing-worktree"));
        if let Some(path) = definition {
            command.arg("--definition").arg(path);
        }
        if let Some(path) = destination {
            command.arg("--output-dir").arg(path);
        }
        run(&mut command)
    }

    fn store(&self, operation: &str, extra: &[&str]) -> Value {
        let mut command = self.cli(&["store", operation, "--store"]);
        json_output(run(command.arg(self.path("store")).args(extra)))
    }

    fn reject_definition(&self, value: &Value, name: &str, reason: &str) {
        let path = self.write_json(&format!("refuse-{name}.json"), value);
        let destination = self.path(&format!("refuse-{name}"));
        refused(
            self.system("compose", FOUNDATION, Some(&path), Some(&destination)),
            reason,
        );
        assert!(!destination.exists(), "refusal published a context");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if thread::panicking() {
            eprintln!("system E2E fixture retained: {}", self.0.display());
        } else {
            let _ = remove_fixture(&self.0);
        }
    }
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

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn digest(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut bytes = [0_u8; 65536];
    loop {
        let count = file.read(&mut bytes).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn empty_definition() -> Value {
    json!({"schema":1,"platform":"aarch64-linux","foundation":FOUNDATION,"provenance":{},
        "outputs":{},"required_packages":[],"removed_packages":[],"files":[]})
}

#[test]
fn system_cli_refuses_unsupported_target_and_mismatched_authoring() {
    let f = Fixture::new();
    f.repository();
    let mut command = f.cli(&["system", "plan", "--repo"]);
    refused(
        run(command
            .arg(f.path("repo"))
            .args(["--target", "desktop", "--store"])
            .arg(f.path("store"))
            .args(["--foundation", FOUNDATION])
            .env("DOCKER_HOST", "unix:///nonexistent/kedra-system-e2e.sock")),
        "only qemu-arm64",
    );
    let mut definition = empty_definition();
    definition["foundation"] = json!(OTHER_RUNTIME);
    f.reject_definition(&definition, "foundation-argument", "match the explicit");
    definition = empty_definition();
    definition["provenance"] = json!({"kedra.source_revision":"invented"});
    f.reject_definition(&definition, "source-provenance", "reserved");
    assert!(
        !f.path("store").exists(),
        "preflight refusal initialized a store"
    );
}

fn authoring_consumer(f: &Fixture) -> PathBuf {
    let directory = f.path("author");
    fs::create_dir(&directory).unwrap();
    let engine = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("sysroot-engine");
    fs::write(
        directory.join("Cargo.toml"),
        format!(
            r#"[package]
name = "system-authoring-fixture"
version = "0.0.0"
edition = "2024"
[workspace]
[[bin]]
name = "author"
path = "main.rs"
[dependencies]
sysroot-engine = {{ path = {} }}
serde_json = "1.0"
[profile.dev]
debug = false
"#,
            serde_json::to_string(text(&engine)).unwrap()
        ),
    )
    .unwrap();
    fs::write(directory.join("main.rs"), r#"use std::collections::BTreeMap;
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input, Segment, SystemContent, SystemDefinition, SystemFile, PLATFORM};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args[1] == "build" {
        let graph = BuildGraph { schema: 1, nodes: BTreeMap::from([("app".into(), BuildNode {
            builder_image: args[3].clone(), runtime_image: args[4].clone(),
            inputs: BTreeMap::from([("source".into(), Input::Object(args[2].clone()))]),
            argv: vec![Argument::literal("/bin/sh"), Argument::input("source", "build.sh"), Argument::output("")],
            env: BTreeMap::new(), runtime_inputs: vec![], timeout_seconds: 120,
        })]) };
        println!("{}", serde_json::to_string(&graph).unwrap());
    } else {
        let definition = SystemDefinition {
            schema: 1, platform: PLATFORM.into(), foundation: args[3].clone(),
            provenance: BTreeMap::from([("fixture".into(), "typed-rust-consumer".into())]),
            outputs: BTreeMap::from([("fixture".into(), args[2].clone())]),
            required_packages: vec!["bash".into()], removed_packages: vec![],
            files: vec![
                SystemFile { path: "/etc/kedra-fixture.conf".into(), mode: 0o644,
                    provenance: "fixture.config".into(), priority: 10,
                    replaces: Some("kedra-source:etc/kedra-fixture.conf".into()),
                    content: SystemContent::Bytes(format!("version={}\n", args[4]).into_bytes()) },
                SystemFile { path: "/usr/lib/systemd/system/kedra-fixture.service".into(), mode: 0o644,
                    provenance: "fixture.unit".into(), priority: 0, replaces: None,
                    content: SystemContent::Template(Argument(vec![
                        Segment::Literal { value: "[Service]\nExecStart=".into() },
                        Segment::Input { name: "fixture".into(), path: "bin/hello".into() },
                        Segment::Literal { value: " ready\n".into() },
                    ])) },
            ],
        };
        println!("{}", serde_json::to_string(&definition).unwrap());
    }
}
"#).unwrap();
    let target = directory.join("target");
    success(run(Command::new("cargo")
        .args(["build", "--offline", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)));
    target.join("debug/author")
}

fn build_output(f: &Fixture, author: &Path, source: &str, runtime: &str, name: &str) -> String {
    let graph = json_output(run(
        Command::new(author).args(["build", source, BUILDER, runtime])
    ));
    let path = f.write_json(name, &graph);
    let mut command = f.cli(&["build", "--store"]);
    let result = json_output(run(command
        .arg(f.path("store"))
        .arg("--plan")
        .arg(path)
        .args(["--root", "app"])));
    result["outputs"]["app"].as_str().unwrap().into()
}

fn run_program(f: &Fixture, object: &str) {
    let mut command = f.cli(&["run", "--store"]);
    let output = success(run(command.arg(f.path("store")).args([
        "--object",
        object,
        "--program",
        "bin/hello",
        "--",
        "ready",
    ])));
    assert_eq!(output.stdout, b"system-fixture:ready\n");
}

fn payload(context: &Path) -> BTreeMap<String, (u32, Vec<u8>)> {
    let mut files = BTreeMap::new();
    for entry in tar::Archive::new(File::open(context.join("payload.tar")).unwrap())
        .entries()
        .unwrap()
    {
        let mut entry = entry.unwrap();
        assert_eq!(entry.header().uid().unwrap(), 0);
        assert_eq!(entry.header().gid().unwrap(), 0);
        assert_eq!(entry.header().mtime().unwrap(), 0);
        if entry.header().entry_type().is_file() {
            let path = entry.path().unwrap().to_str().unwrap().to_owned();
            let mode = entry.header().mode().unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            assert!(files.insert(path, (mode, bytes)).is_none());
        }
    }
    files
}

fn artifact_hashes(context: &Path) -> BTreeMap<String, String> {
    [
        "payload.tar",
        "foundation.tar",
        "Containerfile",
        "composition.json",
    ]
    .into_iter()
    .map(|name| (name.into(), digest(&context.join(name))))
    .collect()
}

#[test]
#[ignore = "requires native aarch64 Docker and the retained Fedora foundation"]
fn niri_artifacts_follow_public_composition_and_survive_store_gc() {
    let f = Fixture::new();
    f.repository();
    const SOURCE: &str = "etc/skel/.config/niri/config.kdl";
    const BASELINE: &str = "usr/share/sysroot/home/default/.config/niri/config.kdl";
    const RECORD: &str = "usr/share/sysroot/home-artifacts.json";
    let public = "layout {\n    gaps 12\n}\n";
    f.write_source(SOURCE, public);
    let first_revision = f.commit();
    f.store("init", &[]);
    f.store("add-image", &["--image", FOUNDATION]);
    let roots = fs::read(f.path("store/roots/state.json")).unwrap();
    let plan = json_output(f.system("plan", FOUNDATION, None, None));
    assert_eq!(fs::read(f.path("store/roots/state.json")).unwrap(), roots);
    assert_eq!(fs::read_dir(f.path("store/objects")).unwrap().count(), 0);
    f.write_source(SOURCE, "PRIVATE_DIRTY_CANARY\n");
    let first = f.path("niri-first");
    let composed = json_output(f.system("compose", FOUNDATION, None, Some(&first)));
    assert_eq!(composed["identity"], plan["identity"]);
    let files = payload(&first);
    assert_eq!(files[BASELINE], (0o644, public.as_bytes().to_vec()));
    assert_eq!(files[RECORD].0, 0o644);
    let record: Value = serde_json::from_slice(&files[RECORD].1).unwrap();
    let receipt = &record["niri"];
    let object = receipt["object"].as_str().unwrap();
    assert!(object.starts_with("src-"));
    assert_eq!(record["schema"], 1);
    assert_eq!(receipt["references"], json!([]));
    assert!(receipt["runtime_image"].is_null());
    assert!(receipt["derivation"].is_null());
    assert_eq!(f.store("verify", &["--object", object]), *receipt);
    let data = f.path("store/objects").join(object).join("data");
    assert_eq!(fs::read_dir(&data).unwrap().count(), 1);
    assert_eq!(
        fs::read(data.join("config.kdl")).unwrap(),
        public.as_bytes()
    );
    assert_eq!(
        fs::metadata(data.join("config.kdl"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o444
    );
    assert!(
        !files
            .keys()
            .any(|path| path.starts_with("usr/lib/sysroot/store/"))
    );
    assert!(composed["plan"]["objects"].as_object().unwrap().is_empty());
    let source: Value = serde_json::from_slice(&files["usr/share/sysroot/source.json"].1).unwrap();
    assert_eq!(source["source_revision"], first_revision);

    f.write_source(SOURCE, public);
    f.write_source("etc/new-public-revision.conf", "second revision\n");
    let second_revision = f.commit();
    assert_ne!(second_revision, first_revision);
    let second = f.path("niri-second");
    let repeated = json_output(f.system("compose", FOUNDATION, None, Some(&second)));
    assert_ne!(repeated["identity"], composed["identity"]);
    assert_eq!(payload(&second)[RECORD].1, files[RECORD].1);
    assert_eq!(fs::read_dir(f.path("store/objects")).unwrap().count(), 1);

    let overlay = format!("{IMAGE}/targets/qemu-arm64/etc/skel/.config/niri/config.kdl");
    f.write_source(&overlay, public);
    f.commit();
    let target_context = f.path("niri-target");
    json_output(f.system("compose", FOUNDATION, None, Some(&target_context)));
    let target_files = payload(&target_context);
    assert_eq!(target_files[RECORD].1, files[RECORD].1);
    let target_manifest: Value =
        serde_json::from_slice(&target_files["usr/share/sysroot/source.json"].1).unwrap();
    assert!(
        target_manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["destination"] == BASELINE && file["source_path"] == overlay)
    );
    fs::remove_file(f.path("repo").join(&overlay)).unwrap();

    f.write_source(SOURCE, "layout {\n    gaps 18\n}\n");
    f.commit();
    let third = f.path("niri-third");
    json_output(f.system("compose", FOUNDATION, None, Some(&third)));
    let changed: Value = serde_json::from_slice(&payload(&third)[RECORD].1).unwrap();
    assert_ne!(changed["niri"]["object"], receipt["object"]);

    f.store("gc", &["--delete"]);
    assert_eq!(f.store("verify", &["--object", object]), *receipt);
    f.store("unpin", &["--object", object]);
    f.store("gc", &["--delete"]);
    assert!(!f.path("store/objects").join(object).exists());
    // The immutable context must remain self-contained after genuine collection.
    let identity = composed["identity"].as_str().unwrap();
    let work = f.path("verify-work");
    fs::create_dir(&work).unwrap();
    let verified = json_output(run(f
        .cli(&[
            "system",
            "verify",
            "--context",
            text(&first),
            "--expected-identity",
            identity,
            "--workdir",
            text(&work),
        ])
        .env("DOCKER_HOST", "unix:///unavailable.sock")));
    assert_eq!(verified["identity"], composed["identity"]);
    let original = fs::read(first.join("payload.tar")).unwrap();
    for member in [RECORD, BASELINE] {
        let mut tampered = original.clone();
        let offset = tampered
            .windows(files[member].1.len())
            .position(|bytes| bytes == files[member].1)
            .unwrap();
        tampered[offset] ^= 1;
        fs::write(first.join("payload.tar"), tampered).unwrap();
        refused(
            run(&mut f.cli(&[
                "system",
                "verify",
                "--context",
                text(&first),
                "--expected-identity",
                identity,
                "--workdir",
                text(&work),
            ])),
            "hash",
        );
    }
    fs::write(first.join("payload.tar"), original).unwrap();

    fs::remove_file(f.path("repo").join(SOURCE)).unwrap();
    f.commit();
    let absent = f.path("niri-absent");
    json_output(f.system("compose", FOUNDATION, None, Some(&absent)));
    assert!(!payload(&absent).contains_key(RECORD));
    assert!(!payload(&absent).contains_key(BASELINE));
    for (name, bytes, reason) in [
        ("no-newline", "layout {}", "ending with a newline"),
        (
            "reference",
            "/usr/lib/sysroot/store/src-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "cannot reference",
        ),
    ] {
        f.write_source(SOURCE, bytes);
        f.commit();
        let destination = f.path(name);
        refused(
            f.system("compose", FOUNDATION, None, Some(&destination)),
            reason,
        );
        assert!(!destination.exists());
    }
    f.write_source(SOURCE, public);
    for (name, contents) in [
        ("niri-size", format!("{}\n", "x".repeat(131_072))),
        ("niri-lines", "// public\n".repeat(8193)),
        ("niri-crlf", "layout {}\r\n".into()),
    ] {
        f.write_source(SOURCE, &contents);
        f.commit();
        let destination = f.path(name);
        refused(
            f.system("compose", FOUNDATION, None, Some(&destination)),
            "bounded LF text",
        );
        assert!(!destination.exists());
    }
    f.write_source(SOURCE, public);
    fs::set_permissions(
        f.path("repo").join(SOURCE),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    f.commit();
    let executable = f.path("niri-executable");
    refused(
        f.system("compose", FOUNDATION, None, Some(&executable)),
        "non-executable baseline",
    );
    assert!(!executable.exists());
}

#[test]
#[ignore = "requires native aarch64 Docker and retained exact images; run --ignored"]
fn native_system_composition_exports_verified_repeatable_context() {
    let f = Fixture::new();
    let revision = f.repository();
    f.store("init", &[]);
    for image in [FOUNDATION, BUILDER, OTHER_RUNTIME] {
        let observed = json_output(run(Command::new("docker").args(["image", "inspect", image])));
        assert_eq!(observed[0]["Id"], image);
        assert_eq!(observed[0]["Architecture"], "arm64");
        assert_eq!(f.store("add-image", &["--image", image])["image"], image);
    }
    let author = authoring_consumer(&f);
    let source = f.path("package-source");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("build.sh"),
        r#"set -eu
mkdir -p "$1/bin"
cat > "$1/bin/hello" <<'PROGRAM'
#!/bin/sh
set -eu
printf 'system-fixture:%s\n' "${1:-ready}"
PROGRAM
chmod 755 "$1/bin/hello"
"#,
    )
    .unwrap();
    let admitted = f.store("add-source", &["--source", text(&source)]);
    let object = build_output(
        &f,
        &author,
        admitted["object"].as_str().unwrap(),
        FOUNDATION,
        "build.json",
    );
    run_program(&f, &object);
    let definition = json_output(run(
        Command::new(&author).args(["system", &object, FOUNDATION, "one"])
    ));
    let definition_path = f.write_json("definition.json", &definition);
    let planned = json_output(f.system("plan", FOUNDATION, Some(&definition_path), None));
    assert_eq!(
        planned["definition"]["provenance"]["kedra.source_revision"],
        revision
    );
    assert_eq!(planned["foundation"]["receipt"]["image"], FOUNDATION);
    assert!(
        planned["foundation"]["rpm_inventory"]
            .as_str()
            .unwrap()
            .lines()
            .any(|line| line.starts_with("bash\t"))
    );
    let first = f.path("context-one");
    let composed =
        json_output(f.system("compose", FOUNDATION, Some(&definition_path), Some(&first)));
    assert_eq!(composed["identity"], planned["identity"]);
    let original_hashes = artifact_hashes(&first);
    for (name, receipt) in composed["artifacts"].as_object().unwrap() {
        assert_eq!(receipt["sha256"], original_hashes[name]);
        assert_eq!(
            receipt["bytes"],
            fs::metadata(first.join(name)).unwrap().len()
        );
    }
    let files = payload(&first);
    assert_eq!(
        files["etc/kedra-fixture.conf"],
        (0o644, b"version=one\n".to_vec())
    );
    assert_eq!(files["etc/committed-fixture.conf"].1, b"committed\n");
    assert_eq!(
        files["usr/share/sysroot/home/default/.config/fixture/settings"].1,
        b"writable-home-baseline\n"
    );
    assert!(!files.contains_key("etc/skel/.config/fixture/settings"));
    let program = format!("usr/lib/sysroot/store/{object}/bin/hello");
    assert_eq!(files[&program].0, 0o555);
    assert_eq!(
        files[&program].1,
        fs::read(f.path("store/objects").join(&object).join("data/bin/hello")).unwrap()
    );
    assert_eq!(
        composed["artifacts"]["foundation.tar"]["sha256"],
        planned["foundation"]["receipt"]["sha256"]
    );
    assert_eq!(
        files["usr/lib/systemd/system/kedra-fixture.service"].1,
        format!("[Service]\nExecStart=/{program} ready\n").as_bytes()
    );
    let source_manifest: Value =
        serde_json::from_slice(&files["usr/share/sysroot/source.json"].1).unwrap();
    assert_eq!(source_manifest["source_revision"], revision);
    assert_eq!(source_manifest["target"]["id"], "qemu-arm64");
    assert_eq!(
        fs::read_to_string(first.join("Containerfile")).unwrap(),
        format!(
            "FROM {}\nADD payload.tar /\n",
            composed["foundation_tag"].as_str().unwrap()
        )
    );

    f.write_source("etc/committed-fixture.conf", "uncommitted-must-not-enter\n");
    f.write_source("etc/untracked-fixture.conf", "untracked-must-not-enter\n");
    let repeated = f.path("context-repeat");
    let repeat = json_output(f.system(
        "compose",
        FOUNDATION,
        Some(&definition_path),
        Some(&repeated),
    ));
    assert_eq!(repeat["identity"], composed["identity"]);
    assert_eq!(artifact_hashes(&repeated), original_hashes);
    refused(
        f.system("compose", FOUNDATION, Some(&definition_path), Some(&first)),
        "exist",
    );
    assert_eq!(artifact_hashes(&first), original_hashes);

    let changed = json_output(run(
        Command::new(&author).args(["system", &object, FOUNDATION, "two"])
    ));
    let changed_path = f.write_json("changed.json", &changed);
    let second = f.path("context-two");
    let next = json_output(f.system("compose", FOUNDATION, Some(&changed_path), Some(&second)));
    assert_ne!(next["identity"], composed["identity"]);
    assert_eq!(
        payload(&second)["etc/kedra-fixture.conf"].1,
        b"version=two\n"
    );
    assert_eq!(artifact_hashes(&first), original_hashes);
    run_program(&f, &object);

    // A foundation executable is asserted byte-for-byte, never rewritten.
    let executable = success(run(Command::new("docker").args([
        "run",
        "--rm",
        "--pull",
        "never",
        "--platform",
        "linux/arm64",
        "--read-only",
        "--network",
        "none",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--user",
        "65534:65534",
        "--entrypoint",
        "/usr/bin/cat",
        FOUNDATION,
        "/usr/bin/true",
    ])))
    .stdout;
    let mut passthrough = changed.clone();
    passthrough["files"].as_array_mut().unwrap().push(json!({
        "path":"/usr/bin/true", "mode":493, "provenance":"fixture.foundation",
        "priority":0, "replaces":null,
        "content":{"kind":"bytes", "value":executable}
    }));
    let passthrough_path = f.write_json("passthrough.json", &passthrough);
    let passthrough_context = f.path("context-passthrough");
    let verified = json_output(f.system(
        "compose",
        FOUNDATION,
        Some(&passthrough_path),
        Some(&passthrough_context),
    ));
    assert!(
        verified["plan"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "/usr/bin/true" && file["disposition"] == "foundation")
    );
    assert!(!payload(&passthrough_context).contains_key("usr/bin/true"));

    refusal_matrix(
        &f,
        &definition,
        &author,
        admitted["object"].as_str().unwrap(),
        &object,
    );
    assert_eq!(artifact_hashes(&first), original_hashes);
}

fn refusal_matrix(f: &Fixture, definition: &Value, author: &Path, source: &str, object: &str) {
    let mut conflict = definition.clone();
    conflict["files"][0]["replaces"] = Value::Null;
    f.reject_definition(&conflict, "conflict", "replacement");
    let mut prefix = definition.clone();
    let mut child = prefix["files"][0].clone();
    child["path"] = json!("/etc/kedra-fixture.conf/child");
    child["replaces"] = Value::Null;
    prefix["files"].as_array_mut().unwrap().push(child);
    f.reject_definition(&prefix, "prefix", "prefix collision");
    for (name, path) in [
        ("reserved", "/usr/lib/sysroot/store/forged"),
        ("source-manifest", "/usr/share/sysroot/source.json"),
        ("unsafe", "/etc/../outside"),
        ("foundation-executable", "/usr/bin/bash"),
    ] {
        let mut invalid = definition.clone();
        invalid["files"][0]["path"] = json!(path);
        invalid["files"][0]["replaces"] = Value::Null;
        f.reject_definition(
            &invalid,
            name,
            if name == "foundation-executable" {
                "foundation"
            } else if name == "unsafe" {
                "path"
            } else {
                "reserved"
            },
        );
    }
    let mut missing_alias = definition.clone();
    missing_alias["files"][1]["content"]["value"][1]["name"] = json!("undeclared");
    f.reject_definition(&missing_alias, "reference", "undeclared");
    let mut missing_program = definition.clone();
    missing_program["files"][1]["content"]["value"][1]["path"] = json!("bin/missing");
    f.reject_definition(&missing_program, "missing-program", "missing");
    let mut required = definition.clone();
    required["required_packages"] = json!(["kedra-e2e-absent-package"]);
    f.reject_definition(&required, "missing-rpm", "lacks required RPM");
    let mut removed = definition.clone();
    removed["removed_packages"] = json!(["rpm"]);
    f.reject_definition(&removed, "removed-rpm", "still contains removed RPM");

    let mut linked = definition.clone();
    linked["files"][0]["path"] = json!("/etc/os-release");
    linked["files"][0]["replaces"] = Value::Null;
    f.reject_definition(&linked, "foundation-symlink", "symlink");

    let other = build_output(f, author, source, OTHER_RUNTIME, "other-runtime.json");
    let mut mixed = definition.clone();
    mixed["outputs"]["foreign"] = json!(other);
    f.reject_definition(&mixed, "mixed-runtime", "runtime foundation");
    let mut wrong = empty_definition();
    wrong["foundation"] = json!(OTHER_RUNTIME);
    let wrong_path = f.write_json("wrong-foundation.json", &wrong);
    let destination = f.path("wrong-foundation");
    refused(
        f.system(
            "compose",
            OTHER_RUNTIME,
            Some(&wrong_path),
            Some(&destination),
        ),
        "Fedora 44",
    );
    assert!(!destination.exists());

    let binary = f.path("store/objects").join(object).join("data/bin/hello");
    let original = fs::read(&binary).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&binary, b"corrupt executable").unwrap();
    f.reject_definition(definition, "corrupt-object", "digest");
    fs::write(&binary, original).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o555)).unwrap();
    run_program(f, object);
}
