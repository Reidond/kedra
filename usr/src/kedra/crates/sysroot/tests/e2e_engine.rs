//! Real public engine workflows. Docker qualification is explicitly opt-in; an
//! ignored listing is not evidence of building, transferring or running a package.
#![cfg(unix)]

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BUILDER: &str = "sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546";
const RUNTIME: &str = "sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3";
const CAPTURE_LIMIT: usize = 2 * 1024 * 1024;
const COMMAND_DEADLINE: Duration = Duration::from_secs(600);
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Capture {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Capture {
    fn success(self) -> Self {
        assert!(
            self.status.success(),
            "status {}\nstdout: {}\nstderr: {}",
            self.status,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        );
        self
    }

    fn json(self) -> Value {
        let output = self.success();
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid CLI JSON: {error}: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    fn refused(self, reason: &str) -> Self {
        assert!(!self.status.success(), "unexpected success for {reason}");
        assert!(self.stdout.is_empty(), "refusal wrote stdout");
        let diagnostic = String::from_utf8_lossy(&self.stderr);
        assert!(
            diagnostic
                .to_ascii_lowercase()
                .contains(&reason.to_ascii_lowercase()),
            "wanted {reason:?}, got {diagnostic}"
        );
        self
    }
}

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = pipe.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            let keep = read.min(CAPTURE_LIMIT.saturating_sub(retained.len()));
            retained.extend_from_slice(&buffer[..keep]);
        }
        retained
    })
}

struct Running {
    child: Child,
    stdout: Option<JoinHandle<Vec<u8>>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
    started: Instant,
}

impl Running {
    fn start(command: &mut Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = Some(drain(child.stdout.take().unwrap()));
        let stderr = Some(drain(child.stderr.take().unwrap()));
        Self {
            child,
            stdout,
            stderr,
            started: Instant::now(),
        }
    }

    fn finish(mut self) -> Capture {
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if self.started.elapsed() > COMMAND_DEADLINE {
                self.child.kill().unwrap();
                let _ = self.child.wait();
                panic!("E2E subprocess exceeded its observation deadline");
            }
            thread::sleep(Duration::from_millis(25));
        };
        Capture {
            status,
            stdout: self.stdout.take().unwrap().join().unwrap(),
            stderr: self.stderr.take().unwrap().join().unwrap(),
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn run(command: &mut Command) -> Capture {
    Running::start(command).finish()
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let name = format!(
            "kedra-engine-e2e-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn command(&self, args: &[&str]) -> Command {
        let binary = std::env::var_os("KEDRA_ENGINE_E2E_BINARY").map_or_else(
            || PathBuf::from(env!("CARGO_BIN_EXE_sysroot")),
            PathBuf::from,
        );
        let mut command = Command::new(binary);
        command
            .args(args)
            .env("KEDRA_ENGINE_HOST_ONLY_SENTINEL", "generated-host-only");
        command
    }

    fn store(&self, operation: &str, store: &Path, extra: &[&str]) -> Capture {
        if matches!(operation, "add-source" | "add-image" | "import") && !store.exists() {
            self.store("init", store, &[]).json();
        }
        let mut command = self.command(&["store", operation, "--store"]);
        command.arg(store).args(extra);
        run(&mut command)
    }

    fn build(&self, store: &Path, plan: &Path, extra: &[&str]) -> Capture {
        let mut command = self.command(&["build", "--store"]);
        command
            .arg(store)
            .arg("--plan")
            .arg(plan)
            .args(["--root", "app"])
            .args(extra);
        run(&mut command)
    }

    fn program(&self, store: &Path, object: &str, args: &[&str]) -> Capture {
        let mut command = self.command(&["run", "--store"]);
        command
            .arg(store)
            .args(["--object", object, "--program", "bin/hello", "--"])
            .args(args);
        run(&mut command)
    }

    fn source(&self, store: &Path, directory: &Path) -> String {
        let result = self
            .store("add-source", store, &["--source", text(directory)])
            .json();
        string(&result, "object")
    }

    fn images(&self, store: &Path) {
        for image in [BUILDER, RUNTIME] {
            let observed = run(Command::new("docker").args(["image", "inspect", image])).json();
            assert_eq!(observed[0]["Id"], image);
            assert_eq!(observed[0]["Architecture"], "arm64");
            assert_eq!(observed[0]["Os"], "linux");
            let admitted = self.store("add-image", store, &["--image", image]).json();
            assert_eq!(admitted["image"], image);
        }
    }

    fn write_json(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if thread::panicking() {
            eprintln!(
                "engine E2E fixture retained after failure: {}",
                self.0.display()
            );
        } else {
            let _ = remove_fixture(&self.0);
        }
    }
}

fn remove_fixture(directory: &Path) -> std::io::Result<()> {
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            remove_fixture(&entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    fs::remove_dir(directory)
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn string(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("missing string {key}: {value}"))
        .to_owned()
}

fn app_id(value: &Value) -> String {
    string(&value["outputs"], "app")
}

fn sha256(path: &Path) -> String {
    let mut input = File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = input.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn bundle_with_extra_member(original: &Path, output: &Path) {
    let mut archive = tar::Archive::new(File::open(original).unwrap());
    let mut builder = tar::Builder::new(File::create(output).unwrap());
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let header = entry.header().clone();
        builder.append(&header, &mut entry).unwrap();
    }
    let bytes = b"not admitted by the manifest";
    let mut header = tar::Header::new_gnu();
    header.set_path("unexpected-member").unwrap();
    header.set_mode(0o600);
    header.set_size(bytes.len() as u64);
    header.set_cksum();
    builder.append(&header, bytes.as_slice()).unwrap();
    builder.finish().unwrap();
}

fn append_tar_bytes<W: Write>(builder: &mut tar::Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_path(path).unwrap();
    header.set_mode(0o600);
    header.set_size(bytes.len() as u64);
    header.set_cksum();
    builder.append(&header, bytes).unwrap();
}

fn bundle_with_self_consistent_wrong_image(original: &Path, output: &Path) {
    let mut fake = tar::Builder::new(Vec::new());
    append_tar_bytes(&mut fake, "payload", b"ordinary tar; not the claimed image");
    fake.finish().unwrap();
    let fake = fake.into_inner().unwrap();
    let fake_sha256: String = Sha256::digest(&fake)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let image_member = format!("images/{}.tar", RUNTIME.strip_prefix("sha256:").unwrap());
    let mut archive = tar::Archive::new(File::open(original).unwrap());
    let mut builder = tar::Builder::new(File::create(output).unwrap());
    let mut changed_manifest = false;
    let mut changed_image = false;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().into_owned();
        if path == Path::new("manifest.json") {
            let mut manifest: Value = serde_json::from_reader(&mut entry).unwrap();
            assert_eq!(manifest["images"][RUNTIME]["image"], RUNTIME);
            manifest["images"][RUNTIME]["sha256"] = json!(fake_sha256);
            manifest["images"][RUNTIME]["bytes"] = json!(fake.len());
            append_tar_bytes(
                &mut builder,
                "manifest.json",
                &serde_json::to_vec(&manifest).unwrap(),
            );
            changed_manifest = true;
        } else if path == Path::new(&image_member) {
            append_tar_bytes(&mut builder, &image_member, &fake);
            changed_image = true;
        } else {
            let header = entry.header().clone();
            builder.append(&header, &mut entry).unwrap();
        }
    }
    assert!(
        changed_manifest && changed_image,
        "forgery did not alter the admitted image and its matching receipt"
    );
    builder.finish().unwrap();
}

fn bare_graph() -> Value {
    json!({"schema":1,"nodes":{"app":{
        "builder_image":BUILDER,"runtime_image":RUNTIME,"inputs":{},
        "argv":[[{"kind":"literal","value":"/bin/true"}]],
        "env":{},"runtime_inputs":[],"timeout_seconds":120
    }}})
}

#[test]
fn planning_is_public_pure_and_rejects_invalid_graphs() {
    let f = Fixture::new();
    let absent_store = f.path("must-not-be-created");
    let graph = bare_graph();
    let plan = f.write_json("valid.json", &graph);
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&absent_store)
        .arg("--plan")
        .arg(&plan)
        .args(["--root", "app", "--dry-run"])
        .env("DOCKER_HOST", "unix:///nonexistent/kedra-engine-e2e.sock");
    let first = run(&mut command).json();
    let second = run(&mut command).json();
    assert_eq!(first["outputs"], second["outputs"]);
    assert!(app_id(&first).starts_with("out-"));
    assert!(
        !absent_store.exists(),
        "pure planning initialized the store"
    );

    let mut changed = graph.clone();
    changed["nodes"]["app"]["env"] = json!({"FIXTURE_VALUE":[{"kind":"literal","value":"two"}]});
    let changed_path = f.write_json("changed.json", &changed);
    assert_ne!(
        app_id(&first),
        app_id(&f.build(&absent_store, &changed_path, &["--dry-run"]).json())
    );
    assert!(!absent_store.exists());

    let mut unknown = graph.clone();
    unknown["unexpected"] = json!(true);
    let mut schema = graph.clone();
    schema["schema"] = json!(2);
    let mut missing = graph.clone();
    missing["nodes"]["app"]["inputs"] = json!({"dependency":{"kind":"node","value":"absent"}});
    let mut cycle = graph.clone();
    cycle["nodes"]["app"]["inputs"] = json!({"dependency":{"kind":"node","value":"app"}});
    for (name, invalid, reason) in [
        ("unknown", unknown, "unknown"),
        ("schema", schema, "schema"),
        ("missing", missing, "absent"),
        ("cycle", cycle, "cycle"),
    ] {
        let path = f.write_json(&format!("{name}.json"), &invalid);
        f.build(&absent_store, &path, &["--dry-run"])
            .refused(reason);
        assert!(!absent_store.exists());
    }
    let oversized = f.path("oversized.json");
    let mut file = File::create(&oversized).unwrap();
    file.write_all(&serde_json::to_vec(&graph).unwrap())
        .unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    f.build(&absent_store, &oversized, &["--dry-run"])
        .refused("8 MiB");

    for field in ["argv", "env", "nodes"] {
        let mut boundary = graph.clone();
        for index in 0..256 {
            match field {
                "argv" => {
                    boundary["nodes"]["app"]["argv"] =
                        json!(vec![json!([{"kind":"literal","value":"x"}]); index + 1])
                }
                "env" => {
                    boundary["nodes"]["app"]["env"][format!("VALUE_{index}")] =
                        json!([{"kind":"literal","value":"x"}])
                }
                "nodes" if index > 0 => {
                    boundary["nodes"][format!("node_{index}")] = graph["nodes"]["app"].clone()
                }
                "nodes" => {}
                _ => unreachable!(),
            }
        }
        let accepted = f.write_json(&format!("{field}-boundary.json"), &boundary);
        f.build(&absent_store, &accepted, &["--dry-run"]).json();
        match field {
            "argv" => boundary["nodes"]["app"]["argv"]
                .as_array_mut()
                .unwrap()
                .push(json!([{"kind":"literal","value":"x"}])),
            "env" => {
                boundary["nodes"]["app"]["env"]["ONE_TOO_MANY"] =
                    json!([{"kind":"literal","value":"x"}])
            }
            "nodes" => boundary["nodes"]["one_too_many"] = graph["nodes"]["app"].clone(),
            _ => unreachable!(),
        }
        let refused = f.write_json(&format!("{field}-over.json"), &boundary);
        f.build(&absent_store, &refused, &["--dry-run"])
            .refused(if field == "nodes" { "count" } else { "limits" });
    }
    assert!(!absent_store.exists());
}

fn c_source(f: &Fixture, version: &str) -> PathBuf {
    let path = f.path(&format!("source-{version}"));
    fs::create_dir(&path).unwrap();
    fs::write(
        path.join("library.c"),
        format!("const char *fixture_version(void) {{ return \"{version}\"; }}\n"),
    )
    .unwrap();
    fs::write(
        path.join("main.c"),
        r#"#include <stdio.h>
#include <string.h>
#include <unistd.h>
extern const char *fixture_version(void);
int main(int argc, char **argv) {
    puts(fixture_version());
    fflush(stdout);
    if (argc > 1 && strcmp(argv[1], "hold") == 0) sleep(8);
    if (argc > 1 && strcmp(argv[1], "exit-seven") == 0) return 7;
    return 0;
}
"#,
    )
    .unwrap();
    fs::write(
        path.join("library.sh"),
        r#"set -eu
[ "$(id -u)" -ne 0 ]
[ -z "${KEDRA_ENGINE_HOST_ONLY_SENTINEL:-}" ]
if touch "$(dirname "$1")/forbidden-write" 2>/dev/null; then exit 41; fi
if touch /forbidden-root-write 2>/dev/null; then exit 42; fi
[ ! -S /var/run/docker.sock ]
mkdir -p "$2/lib"
cc -fPIC -shared -Wl,-soname,libfixture.so -o "$2/lib/libfixture.so" "$1"
"#,
    )
    .unwrap();
    fs::write(
        path.join("app.sh"),
        r#"set -eu
mkdir -p "$3/bin"
cc -o "$3/bin/hello" "$1" -L"$2/lib" -lfixture -Wl,-rpath,"$2/lib"
"#,
    )
    .unwrap();
    fs::write(
        path.join("entropy.sh"),
        r#"set -eu
mkdir -p "$1"
head -c 32 /dev/urandom > "$1/random"
"#,
    )
    .unwrap();
    fs::write(
        path.join("slow.sh"),
        "set -eu\nsleep 30\nmkdir -p \"$1\"\nprintf finished > \"$1/done\"\n",
    )
    .unwrap();
    path
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
name = "engine-authoring-fixture"
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
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = &args[1];
    let builder = &args[2];
    let runtime = &args[3];
    let library = BuildNode {
        builder_image: builder.clone(), runtime_image: runtime.clone(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.clone()))]),
        argv: vec![Argument::literal("/bin/sh"), Argument::input("source", "library.sh"),
            Argument::input("source", "library.c"), Argument::output("")],
        env: BTreeMap::new(), runtime_inputs: vec![], timeout_seconds: 120,
    };
    let app = BuildNode {
        builder_image: builder.clone(), runtime_image: runtime.clone(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.clone())),
            ("library".into(), Input::Node("library".into()))]),
        argv: vec![Argument::literal("/bin/sh"), Argument::input("source", "app.sh"),
            Argument::input("source", "main.c"), Argument::input("library", ""), Argument::output("")],
        env: BTreeMap::new(), runtime_inputs: vec!["library".into()], timeout_seconds: 120,
    };
    let graph = BuildGraph { schema: 1, nodes: BTreeMap::from([
        ("app".into(), app), ("library".into(), library)]) };
    println!("{}", serde_json::to_string(&graph).unwrap());
}
"#).unwrap();
    let target = directory.join("target");
    run(Command::new("cargo")
        .args(["build", "--offline", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target))
    .success();
    target.join("debug/author")
}

fn authored_plan(f: &Fixture, author: &Path, source: &str, name: &str) -> PathBuf {
    let value = run(Command::new(author)
        .args([source, BUILDER, RUNTIME])
        .env("DOCKER_HOST", "unix:///nonexistent/kedra-engine-e2e.sock"))
    .json();
    f.write_json(name, &value)
}

fn assert_version(output: Capture, version: &str) {
    let output = output.success();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{version}\n")
    );
}

fn nonempty_result(value: &Value, field: &str) {
    assert!(
        !value[field]
            .as_array()
            .unwrap_or_else(|| panic!("missing {field}: {value}"))
            .is_empty()
    );
}

fn profile(f: &Fixture, operation: &str, store: &Path, extra: &[&str]) -> Capture {
    let mut command = f.command(&["profile", operation, "--store"]);
    command.arg(store).args(["--name", "fixture"]).args(extra);
    run(&mut command)
}

#[test]
#[ignore = "requires native aarch64 Docker and retained pinned images; run --ignored"]
fn package_build_transfer_profiles_and_collection() {
    let f = Fixture::new();
    let producer = f.path("producer");
    let receiver = f.path("receiver");
    f.images(&producer);
    let author = authoring_consumer(&f);
    let source_one = c_source(&f, "version-one");
    let source_one_id = f.source(&producer, &source_one);
    assert_eq!(f.source(&producer, &source_one), source_one_id);
    let plan_one = authored_plan(&f, &author, &source_one_id, "one.json");
    let planned = f.build(&f.path("unused"), &plan_one, &["--dry-run"]).json();
    assert!(!f.path("unused").exists());
    let first = f.build(&producer, &plan_one, &[]).json();
    let one = app_id(&first);
    assert_eq!(one, app_id(&planned));
    assert_eq!(first["built"].as_array().unwrap().len(), 2);
    assert_version(f.program(&producer, &one, &[]), "version-one");
    let reused = f.build(&producer, &plan_one, &[]).json();
    nonempty_result(&reused, "reused");
    let reproduced = f.build(&producer, &plan_one, &["--rebuild"]).json();
    assert_eq!(reproduced["reproduced"].as_array().unwrap().len(), 2);
    assert_version(f.program(&producer, &one, &[]), "version-one");

    let source_two = c_source(&f, "version-two");
    let source_two_id = f.source(&producer, &source_two);
    assert_ne!(source_one_id, source_two_id);
    let plan_two = authored_plan(&f, &author, &source_two_id, "two.json");
    let second = f.build(&producer, &plan_two, &[]).json();
    let two = app_id(&second);
    assert_ne!(one, two);
    let closure = f.store("closure", &producer, &["--object", &one]).json();
    assert_eq!(
        closure["objects"].as_array().unwrap().len(),
        2,
        "compiler sources leaked into runtime closure: {closure}"
    );
    assert_eq!(closure["images"], json!([RUNTIME]));
    let mut bundles = Vec::new();
    for (label, object) in [("one", &one), ("two", &two)] {
        let bundle = f.path(&format!("{label}.tar"));
        let exported = f
            .store(
                "export",
                &producer,
                &["--object", object, "--output", text(&bundle)],
            )
            .json();
        let digest = sha256(&bundle);
        assert_eq!(exported["sha256"], digest);
        let imported = f
            .store(
                "import",
                &receiver,
                &["--bundle", text(&bundle), "--expected-sha256", &digest],
            )
            .json();
        assert!(
            imported["roots"]
                .as_array()
                .unwrap()
                .contains(&json!(object))
        );
        bundles.push((bundle, digest));
    }
    // Removing producer paths ensures the receiver cannot resolve host data there.
    let hidden_producer = f.path("producer-unavailable");
    fs::rename(&producer, &hidden_producer).unwrap();
    fs::remove_dir_all(&source_one).unwrap();
    fs::remove_dir_all(&source_two).unwrap();
    assert_version(f.program(&receiver, &one, &[]), "version-one");
    assert_version(f.program(&receiver, &two, &[]), "version-two");
    let child_failure = f.program(&receiver, &one, &["exit-seven"]);
    assert_eq!(child_failure.status.code(), Some(7));
    assert_eq!(child_failure.stdout, b"version-one\n");

    profile(
        &f,
        "switch",
        &receiver,
        &["--object", &one, "--program", "bin/hello"],
    )
    .json();
    assert_version(profile(&f, "run", &receiver, &[]), "version-one");
    profile(
        &f,
        "switch",
        &receiver,
        &["--object", &two, "--program", "bin/hello"],
    )
    .json();
    assert_version(profile(&f, "run", &receiver, &[]), "version-two");
    profile(&f, "list", &receiver, &[]).json();
    profile(&f, "rollback", &receiver, &[]).json();
    assert_version(profile(&f, "run", &receiver, &[]), "version-one");
    let develop = run(f.command(&["develop", "--store"]).arg(&receiver).args([
        "--name",
        "fixture",
        "--program",
        "/bin/sh",
        "--",
        "-c",
        "printf development; exit 7",
    ]));
    assert_eq!(develop.status.code(), Some(7));
    assert_eq!(develop.stdout, b"development");
    let noisy_runtime = run(f.command(&["develop", "--store"]).arg(&receiver).args([
        "--name",
        "fixture",
        "--program",
        "/bin/sh",
        "--",
        "-c",
        "head -c 2097152 /dev/zero",
    ]));
    assert!(!noisy_runtime.status.success());
    assert_eq!(noisy_runtime.stdout.len(), 1024 * 1024);
    assert!(String::from_utf8_lossy(&noisy_runtime.stderr).contains("capture limit"));
    run(f.command(&["run", "--store"]).arg(&receiver).args([
        "--object",
        &one,
        "--program",
        "bin/hello",
        "--timeout",
        "1",
        "--",
        "hold",
    ]))
    .refused("deadline");
    assert_version(f.program(&receiver, &one, &[]), "version-one");

    let orphan_source = f.path("orphan-source");
    fs::create_dir(&orphan_source).unwrap();
    fs::write(orphan_source.join("value"), "unreferenced").unwrap();
    let orphan = f.source(&receiver, &orphan_source);
    f.store("unpin", &receiver, &["--object", &orphan]).json();
    f.store("pin", &receiver, &["--object", &orphan]).json();
    f.store("gc", &receiver, &["--delete"]).json();
    f.store("verify", &receiver, &["--object", &orphan]).json();
    f.store("unpin", &receiver, &["--object", &orphan]).json();
    for object in [&one, &two] {
        f.store("unpin", &receiver, &["--object", object]).json();
    }
    f.store("unpin-image", &receiver, &["--image", RUNTIME])
        .json();
    f.store("gc", &receiver, &[]).json();
    f.store("verify", &receiver, &["--object", &orphan]).json();
    f.store("gc", &receiver, &["--delete"]).json();
    assert!(!receiver.join("objects").join(&orphan).exists());
    assert_version(f.program(&receiver, &one, &[]), "version-one");
    assert_version(f.program(&receiver, &two, &[]), "version-two");

    f.store(
        "import",
        &f.path("wrong-hash"),
        &[
            "--bundle",
            text(&bundles[0].0),
            "--expected-sha256",
            &"0".repeat(64),
        ],
    )
    .refused("SHA256");
    f.store(
        "import",
        &receiver,
        &[
            "--bundle",
            text(&bundles[0].0),
            "--expected-sha256",
            &bundles[0].1,
        ],
    )
    .json();
    let binary = receiver.join("objects").join(&one).join("data/bin/hello");
    let original = fs::read(&binary).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&binary, b"corrupt executable").unwrap();
    f.program(&receiver, &one, &[]).refused("digest");
    fs::write(&binary, original).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o555)).unwrap();
    assert_version(f.program(&receiver, &one, &[]), "version-one");

    let image_archive = receiver
        .join("images")
        .join(RUNTIME.strip_prefix("sha256:").unwrap())
        .join("image.tar");
    fs::set_permissions(&image_archive, fs::Permissions::from_mode(0o600)).unwrap();
    let mut image = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&image_archive)
        .unwrap();
    let mut original_byte = [0_u8; 1];
    image.read_exact(&mut original_byte).unwrap();
    image.seek(SeekFrom::Start(0)).unwrap();
    image.write_all(&[original_byte[0] ^ 1]).unwrap();
    image.sync_all().unwrap();
    run(Command::new("docker").args(["image", "inspect", RUNTIME])).success();
    f.program(&receiver, &one, &[]).refused("image evidence");
    image.seek(SeekFrom::Start(0)).unwrap();
    image.write_all(&original_byte).unwrap();
    image.sync_all().unwrap();
    drop(image);
    fs::set_permissions(&image_archive, fs::Permissions::from_mode(0o444)).unwrap();
    assert_version(f.program(&receiver, &one, &[]), "version-one");
    let tampered_bundle = f.path("unknown-member.tar");
    bundle_with_extra_member(&bundles[0].0, &tampered_bundle);
    f.store(
        "import",
        &f.path("unknown-member"),
        &[
            "--bundle",
            text(&tampered_bundle),
            "--expected-sha256",
            &sha256(&tampered_bundle),
        ],
    )
    .refused("member");
    let forged_bundle = f.path("self-consistent-wrong-image.tar");
    bundle_with_self_consistent_wrong_image(&bundles[0].0, &forged_bundle);
    let forged_receiver = f.path("forged-image-receiver");
    // The outer digest and image receipt agree with the forged bytes. Only the
    // archive's claimed image identity is wrong, despite the real daemon cache.
    run(Command::new("docker").args(["image", "inspect", RUNTIME])).success();
    f.store(
        "import",
        &forged_receiver,
        &[
            "--bundle",
            text(&forged_bundle),
            "--expected-sha256",
            &sha256(&forged_bundle),
        ],
    )
    .refused("image archive");
    assert!(!forged_receiver.join("objects").join(&one).exists());
    assert!(!forged_receiver.join("profiles/fixture").exists());
    let empty = f.store("gc", &forged_receiver, &[]).json();
    assert_eq!(empty["objects"], json!([]));
    assert_eq!(empty["images"], json!([]));
    assert_version(f.program(&receiver, &one, &[]), "version-one");
    eprintln!(
        "engine lifecycle exercised: authoring, real compile, reuse/rebuild, closure transfer, profiles/develop, GC and corruption"
    );
}

fn script_plan(source: &str, script: &str, timeout: u64) -> Value {
    json!({"schema":1,"nodes":{"app":{
        "builder_image":BUILDER,"runtime_image":RUNTIME,
        "inputs":{"source":{"kind":"object","value":source}},
        "argv":[[{"kind":"literal","value":"/bin/sh"}],
            [{"kind":"input","name":"source","path":script}],
            [{"kind":"output","path":""}]],
        "env":{},"runtime_inputs":[],"timeout_seconds":timeout
    }}})
}

fn active_container(store: &Path) -> (Value, Value) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Ok(bytes) = fs::read(store.join("transactions/execution.json"))
            && let Ok(journal) = serde_json::from_slice::<Value>(&bytes)
            && let Some(name) = journal["container"].as_str()
        {
            let inspected = run(Command::new("docker").args(["container", "inspect", name]));
            if inspected.status.success() {
                let info: Value = serde_json::from_slice(&inspected.stdout).unwrap();
                if info[0]["State"]["Running"] == true {
                    return (journal, info[0].clone());
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "never observed an active owned executor in {}",
            store.display()
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn assert_isolation(info: &Value, writable_output: bool) {
    assert_eq!(info["HostConfig"]["NetworkMode"], "none");
    assert_eq!(info["HostConfig"]["ReadonlyRootfs"], true);
    assert_eq!(info["HostConfig"]["Privileged"], false);
    assert_eq!(info["HostConfig"]["PidsLimit"], 256);
    assert_eq!(info["HostConfig"]["Memory"], 1024 * 1024 * 1024_u64);
    assert!(
        info["HostConfig"]["CapDrop"]
            .as_array()
            .unwrap()
            .contains(&json!("ALL"))
    );
    assert!(
        info["Config"]["User"]
            .as_str()
            .unwrap()
            .split(':')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
            > 0
    );
    let mounts = info["Mounts"].as_array().unwrap();
    let bindings: Vec<_> = mounts.iter().filter(|m| m["Type"] == "bind").collect();
    assert!(!bindings.is_empty());
    assert_eq!(
        bindings.iter().filter(|m| m["RW"] == true).count(),
        usize::from(writable_output)
    );
    for mount in bindings {
        assert!(
            mount["Destination"]
                .as_str()
                .unwrap()
                .starts_with("/usr/lib/sysroot/store/")
        );
    }
    assert!(info["Config"]["Env"].as_array().unwrap().iter().all(|v| {
        !v.as_str()
            .unwrap()
            .starts_with("KEDRA_ENGINE_HOST_ONLY_SENTINEL=")
    }));
}

struct SentinelContainer(String);

impl SentinelContainer {
    fn new() -> Self {
        let result = run(Command::new("docker").args([
            "run",
            "--detach",
            "--pull",
            "never",
            "--network",
            "none",
            "--read-only",
            "--user",
            "65534:65534",
            "--cap-drop",
            "ALL",
            "--entrypoint",
            "/bin/sleep",
            RUNTIME,
            "600",
        ]))
        .success();
        let id = String::from_utf8(result.stdout).unwrap().trim().to_owned();
        assert_eq!(id.len(), 64);
        Self(id)
    }

    fn assert_alive(&self) {
        let result = run(Command::new("docker").args(["container", "inspect", &self.0])).json();
        assert_eq!(result[0]["State"]["Running"], true);
    }
}

impl Drop for SentinelContainer {
    fn drop(&mut self) {
        let _ = run(Command::new("docker").args(["container", "rm", "--force", &self.0]));
    }
}

#[test]
#[ignore = "requires native aarch64 Docker and retained pinned images; run --ignored"]
fn executor_failures_rebuild_divergence_and_owned_recovery() {
    let f = Fixture::new();
    let store = f.path("store");
    f.images(&store);
    let source = c_source(&f, "faults");
    fs::write(
        source.join("fail.sh"),
        "printf intentional-builder-failure >&2\nexit 23\n",
    )
    .unwrap();
    fs::write(
        source.join("short.sh"),
        "set -eu\nsleep 4\nprintf collected > \"$1/done\"\n",
    )
    .unwrap();
    fs::write(
        source.join("noisy.sh"),
        "set -eu\nhead -c 2097152 /dev/zero\nprintf complete > \"$1/done\"\n",
    )
    .unwrap();
    let source_id = f.source(&store, &source);
    let entropy = f.write_json("entropy.json", &script_plan(&source_id, "entropy.sh", 120));
    let first = f.build(&store, &entropy, &[]).json();
    let winner = app_id(&first);
    let before = f.store("verify", &store, &["--object", &winner]).json();
    f.build(&store, &entropy, &["--rebuild"]).refused("diverg");
    let after = f.store("verify", &store, &["--object", &winner]).json();
    assert_eq!(before, after, "divergent rebuild changed the valid winner");
    nonempty_result(&f.build(&store, &entropy, &[]).json(), "reused");

    let failure = f.write_json("failure.json", &script_plan(&source_id, "fail.sh", 120));
    let failure_id = app_id(
        &f.build(&f.path("unopened"), &failure, &["--dry-run"])
            .json(),
    );
    f.build(&store, &failure, &[])
        .refused("intentional-builder-failure");
    assert!(!store.join("objects").join(&failure_id).exists());
    let timeout = f.write_json("timeout.json", &script_plan(&source_id, "slow.sh", 1));
    let timeout_id = app_id(
        &f.build(&f.path("unopened"), &timeout, &["--dry-run"])
            .json(),
    );
    f.build(&store, &timeout, &[]).refused("deadline");
    assert!(!store.join("objects").join(&timeout_id).exists());
    assert!(!store.join("transactions/execution.json").exists());
    let noisy = f.write_json("noisy.json", &script_plan(&source_id, "noisy.sh", 120));
    let noisy_result = f.build(&store, &noisy, &[]).json();
    f.store("verify", &store, &["--object", &app_id(&noisy_result)])
        .json();

    // A real active build holds the management lock while a separate GC waits.
    let short = f.write_json("short.json", &script_plan(&source_id, "short.sh", 120));
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&short)
        .args(["--root", "app"]);
    let mut active = Running::start(&mut command);
    let (_, info) = active_container(&store);
    assert_isolation(&info, true);
    let mut gc = Running::start(
        f.command(&["store", "gc", "--store"])
            .arg(&store)
            .arg("--delete"),
    );
    thread::sleep(Duration::from_millis(200));
    assert!(
        active.child.try_wait().unwrap().is_none(),
        "active-use precondition was lost"
    );
    assert!(
        gc.child.try_wait().unwrap().is_none(),
        "GC completed during a live build"
    );
    active.finish().json();
    gc.finish().json();

    let sentinel = SentinelContainer::new();
    let slow = f.write_json("interrupted.json", &script_plan(&source_id, "slow.sh", 120));
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&slow)
        .args(["--root", "app"]);
    let mut active = Running::start(&mut command);
    let (journal, info) = active_container(&store);
    assert_isolation(&info, true);
    let owned_id = string(&info, "Id");
    active.child.kill().unwrap();
    active.child.wait().unwrap();
    f.store("gc", &store, &[]).refused("recovery");
    let journal_path = store.join("transactions/execution.json");
    let original_journal = fs::read(&journal_path).unwrap();
    let mut wrong_owner = journal;
    wrong_owner["store_token"] = json!("foreign-generated-token");
    fs::write(&journal_path, serde_json::to_vec(&wrong_owner).unwrap()).unwrap();
    f.store("recover", &store, &[]).refused("ownership");
    sentinel.assert_alive();
    assert!(journal_path.exists());
    fs::write(&journal_path, original_journal).unwrap();
    let recovered = f.store("recover", &store, &[]).json();
    nonempty_result(&recovered, "recovered");
    let interrupted = active.finish();
    assert!(!interrupted.status.success());
    assert!(!journal_path.exists());
    let removed = run(Command::new("docker").args(["container", "inspect", &owned_id]));
    assert!(!removed.status.success());
    sentinel.assert_alive();
    f.store("verify", &store, &["--object", &winner]).json();
    nonempty_result(&f.build(&store, &short, &[]).json(), "reused");
    eprintln!(
        "engine faults exercised: divergent winner, failing/timed-out/noisy builders, live isolation, concurrent GC, killed-controller recovery and foreign-owner refusal"
    );
}

#[test]
fn source_admission_uses_canonical_content_and_rejects_unsafe_nodes() {
    let f = Fixture::new();
    let store = f.path("store");
    let source = f.path("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("data"), "generated bytes\n").unwrap();
    fs::create_dir(source.join("directory")).unwrap();
    std::os::unix::fs::symlink("../data", source.join("directory/link")).unwrap();
    let first = f.source(&store, &source);
    File::open(source.join("data"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(10)))
        .unwrap();
    assert_eq!(
        f.source(&store, &source),
        first,
        "mtime changed canonical identity"
    );
    f.store("verify", &store, &["--object", &first]).json();
    fs::set_permissions(source.join("data"), fs::Permissions::from_mode(0o755)).unwrap();
    let executable = f.source(&store, &source);
    assert_ne!(
        first, executable,
        "executable flag omitted from canonical identity"
    );
    fs::hard_link(source.join("data"), source.join("hardlink")).unwrap();
    f.store("add-source", &store, &["--source", text(&source)])
        .refused("hardlink");
    fs::remove_file(source.join("hardlink")).unwrap();
    std::os::unix::fs::symlink("../outside", source.join("escape")).unwrap();
    f.store("add-source", &store, &["--source", text(&source)])
        .refused("symlink");
    fs::remove_file(source.join("escape")).unwrap();
    assert_eq!(f.source(&store, &source), executable);
    f.store("unpin", &store, &["--object", &first]).json();
    let preview = f.store("gc", &store, &[]).json();
    assert!(
        preview["objects"]
            .as_array()
            .unwrap()
            .contains(&json!(&first))
    );
    f.store("verify", &store, &["--object", &first]).json();
    f.store("gc", &store, &["--delete"]).json();
    assert!(!store.join("objects").join(&first).exists());
    f.store("verify", &store, &["--object", &executable]).json();
}

#[test]
fn planning_rejects_a_cycle_disconnected_from_the_selected_root() {
    let f = Fixture::new();
    let absent_store = f.path("must-remain-absent");
    let mut graph = bare_graph();
    graph["nodes"]["detached_a"] = graph["nodes"]["app"].clone();
    graph["nodes"]["detached_b"] = graph["nodes"]["app"].clone();
    graph["nodes"]["detached_b"]["inputs"] = json!({"other":{"kind":"node","value":"detached_a"}});
    let valid = f.write_json("disconnected-valid.json", &graph);
    let resolved = f.build(&absent_store, &valid, &["--dry-run"]).json();
    assert!(app_id(&resolved).starts_with("out-"));
    assert!(!absent_store.exists());

    graph["nodes"]["detached_a"]["inputs"] = json!({"other":{"kind":"node","value":"detached_b"}});
    let invalid = f.write_json("disconnected-cycle.json", &graph);
    f.build(&absent_store, &invalid, &["--dry-run"])
        .refused("cycle");
    assert!(!absent_store.exists());
}

#[test]
fn source_admission_refuses_targets_traversing_an_intermediate_symlink() {
    let f = Fixture::new();
    let store = f.path("store");
    let source = f.path("source");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("deep")).unwrap();
    fs::write(source.join("data"), "ordinary source\n").unwrap();
    let sentinel = f.path("outside");
    fs::write(&sentinel, "outside-source-sentinel\n").unwrap();
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o440)).unwrap();
    let sentinel_mode = fs::metadata(&sentinel).unwrap().permissions().mode();
    std::os::unix::fs::symlink("..", source.join("deep/pivot")).unwrap();
    let control = f.source(&store, &source);
    f.store("verify", &store, &["--object", &control]).json();

    let escape = source.join("deep/escape");
    std::os::unix::fs::symlink("pivot/../outside", &escape).unwrap();
    // Both links look lexically internal; following the intermediate pivot exits.
    assert_eq!(fs::read(&escape).unwrap(), b"outside-source-sentinel\n");
    f.store("add-source", &store, &["--source", text(&source)])
        .refused("symlink");
    assert_eq!(fs::read(&sentinel).unwrap(), b"outside-source-sentinel\n");
    assert_eq!(
        fs::metadata(&sentinel).unwrap().permissions().mode(),
        sentinel_mode
    );
    f.store("verify", &store, &["--object", &control]).json();
    fs::remove_file(escape).unwrap();
    assert_eq!(f.source(&store, &source), control);
}

#[test]
fn import_recovery_refuses_a_symlinked_stage_without_touching_its_target() {
    let f = Fixture::new();
    let foreign = Fixture::new();
    let store = f.path("store");
    let source = f.path("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("data"), "retained source\n").unwrap();
    let object = f.source(&store, &source);
    let marker: Value =
        serde_json::from_slice(&fs::read(store.join("marker.json")).unwrap()).unwrap();
    let stage_name = format!("stage-{}", "a".repeat(64));
    let stage = store.join("transactions").join(&stage_name);
    let journal_path = store.join("transactions/import.json");
    let journal = serde_json::to_vec(&json!({
        "schema":1,"token":marker["token"],"stage":stage_name,
        "objects":[object],"images":[],"roots":[object]
    }))
    .unwrap();

    // Establish that this otherwise valid journal reaches the import cleanup path.
    fs::create_dir(&stage).unwrap();
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(stage.join("incomplete"), "owned staging\n").unwrap();
    fs::write(&journal_path, &journal).unwrap();
    fs::set_permissions(&journal_path, fs::Permissions::from_mode(0o600)).unwrap();
    f.store("recover", &store, &[]).json();
    assert!(!stage.exists());
    assert!(!journal_path.exists());
    f.store("verify", &store, &["--object", &object]).json();

    let sentinel = foreign.path("sentinel");
    fs::write(&sentinel, "foreign-recovery-sentinel\n").unwrap();
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o440)).unwrap();
    fs::set_permissions(&foreign.0, fs::Permissions::from_mode(0o555)).unwrap();
    let foreign_mode = fs::metadata(&foreign.0).unwrap().permissions().mode();
    let sentinel_mode = fs::metadata(&sentinel).unwrap().permissions().mode();
    std::os::unix::fs::symlink(&foreign.0, &stage).unwrap();
    fs::write(&journal_path, &journal).unwrap();
    fs::set_permissions(&journal_path, fs::Permissions::from_mode(0o600)).unwrap();
    f.store("recover", &store, &[]).refused("symlink");
    assert_eq!(fs::read(&sentinel).unwrap(), b"foreign-recovery-sentinel\n");
    assert_eq!(
        fs::metadata(&foreign.0).unwrap().permissions().mode(),
        foreign_mode
    );
    assert_eq!(
        fs::metadata(&sentinel).unwrap().permissions().mode(),
        sentinel_mode
    );
    assert!(
        fs::symlink_metadata(&stage)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&journal_path).unwrap(), journal);

    // Removing only the generated damaged link allows explicit recovery to finish.
    fs::remove_file(&stage).unwrap();
    f.store("recover", &store, &[]).json();
    f.store("verify", &store, &["--object", &object]).json();
    assert_eq!(fs::read(&sentinel).unwrap(), b"foreign-recovery-sentinel\n");
}

#[test]
fn interrupted_collection_recovers_a_retired_object_and_preserves_live_roots() {
    let f = Fixture::new();
    let store = f.path("store");
    let keep_source = f.path("keep-source");
    fs::create_dir(&keep_source).unwrap();
    fs::write(keep_source.join("data"), "live root\n").unwrap();
    let keep = f.source(&store, &keep_source);
    let source = f.path("many-files");
    fs::create_dir(&source).unwrap();
    // Enough real directory entries to observe retirement before deletion ends.
    // This is an interruption fixture, not a throughput target.
    for index in 0..12_000 {
        fs::write(
            source.join(format!("entry-{index:05}")),
            b"generated GC payload\n",
        )
        .unwrap();
    }
    let doomed = f.source(&store, &source);
    f.store("unpin", &store, &["--object", &doomed]).json();
    let preview = f.store("gc", &store, &[]).json();
    assert_eq!(preview["objects"], json!([&doomed]));
    let sentinel = f.path("outside-store-sentinel");
    fs::write(&sentinel, "unrelated data\n").unwrap();
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o440)).unwrap();
    let sentinel_mode = fs::metadata(&sentinel).unwrap().permissions().mode();
    let mut collecting = Running::start(
        f.command(&["store", "gc", "--store"])
            .arg(&store)
            .arg("--delete"),
    );
    let deadline = Instant::now() + COMMAND_DEADLINE;
    let journal_path = store.join("transactions/gc.json");
    let tombstone = loop {
        if let Ok(bytes) = fs::read(&journal_path)
            && let Ok(journal) = serde_json::from_slice::<Value>(&bytes)
            && let Some(stage) = journal["stage"].as_str()
        {
            let retired = store
                .join("transactions")
                .join(stage)
                .join("objects")
                .join(&doomed);
            if retired.is_dir() && !store.join("objects").join(&doomed).exists() {
                assert!(
                    journal["objects"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(&doomed))
                );
                break retired;
            }
        }
        assert!(
            collecting.child.try_wait().unwrap().is_none(),
            "GC ended before a retirement could be interrupted; crash recovery was not exercised"
        );
        assert!(
            Instant::now() < deadline,
            "GC never reached an observable retirement"
        );
        thread::sleep(Duration::from_millis(1));
    };
    collecting.child.kill().unwrap();
    let killed = collecting.finish();
    assert_eq!(
        killed.status.signal(),
        Some(9),
        "GC did not stop from the injected SIGKILL"
    );
    assert!(
        journal_path.exists(),
        "GC journal disappeared before interruption"
    );
    f.store("gc", &store, &[]).refused("recovery");
    f.store("recover", &store, &[]).json();
    assert!(!journal_path.exists());
    assert!(!tombstone.exists());
    assert!(!store.join("objects").join(&doomed).exists());
    f.store("verify", &store, &["--object", &keep]).json();
    assert_eq!(fs::read(&sentinel).unwrap(), b"unrelated data\n");
    assert_eq!(
        fs::metadata(&sentinel).unwrap().permissions().mode(),
        sentinel_mode
    );
    fs::write(keep_source.join("after-recovery"), "new admission\n").unwrap();
    let new = f.source(&store, &keep_source);
    assert_ne!(new, keep);
    f.store("verify", &store, &["--object", &new]).json();
    assert_eq!(f.store("gc", &store, &[]).json()["objects"], json!([]));
}

#[test]
fn shared_source_dag_retains_each_dependency_once_and_collects_after_unpinning() {
    let f = Fixture::new();
    let store = f.path("store");
    let mut objects: Vec<String> = Vec::new();
    for index in 0..18 {
        let source = f.path(&format!("dag-source-{index}"));
        fs::create_dir(&source).unwrap();
        let mut contents = format!("source node {index}\n");
        for object in objects.iter().rev().take(2) {
            contents.push_str(&format!("/usr/lib/sysroot/store/{object}/data\n"));
        }
        fs::write(source.join("data"), contents).unwrap();
        objects.push(f.source(&store, &source));
    }
    let root = objects.last().unwrap();
    let closure = f.store("closure", &store, &["--object", root]).json();
    let mut expected = objects.clone();
    expected.sort();
    assert_eq!(closure["objects"], json!(expected));
    assert_eq!(closure["images"], json!([]));
    for object in &objects[..objects.len() - 1] {
        f.store("unpin", &store, &["--object", object]).json();
    }
    assert_eq!(
        f.store("gc", &store, &["--delete"]).json()["objects"],
        json!([])
    );
    f.store("verify", &store, &["--object", root]).json();
    f.store("unpin", &store, &["--object", root]).json();
    let collected = f.store("gc", &store, &["--delete"]).json();
    assert_eq!(
        collected["objects"].as_array().unwrap().len(),
        objects.len()
    );
    assert_eq!(f.store("gc", &store, &[]).json()["objects"], json!([]));
}

struct PrivateDockerContexts {
    directory: PathBuf,
    endpoint: String,
}

impl PrivateDockerContexts {
    fn new(f: &Fixture) -> Self {
        let context_override = std::env::var("DOCKER_CONTEXT")
            .ok()
            .filter(|s| !s.is_empty());
        let host_override = std::env::var("DOCKER_HOST").ok().filter(|s| !s.is_empty());
        let endpoint = match (context_override, host_override) {
            (None, Some(endpoint)) => endpoint,
            (context_override, _) => {
                let mut command = Command::new("docker");
                command.args(["context", "inspect"]);
                if let Some(context) = context_override {
                    command.arg(context);
                }
                let context = run(&mut command).json();
                context[0]["Endpoints"]["docker"]["Host"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            }
        };
        assert!(
            endpoint.starts_with("unix:///"),
            "context-switch fixture requires the actual native Unix Docker socket: {endpoint}"
        );
        let directory = f.path("private-docker-config");
        fs::create_dir(&directory).unwrap();
        let contexts = Self {
            directory,
            endpoint,
        };
        run(contexts.docker().args([
            "context",
            "create",
            "fixture-a",
            "--docker",
            &format!("host={}", contexts.endpoint),
        ]))
        .success();
        run(contexts.docker().args([
            "context",
            "create",
            "fixture-b",
            "--docker",
            &format!(
                "host=unix:///tmp/kedra-engine-missing-{}-{}.sock",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
        ]))
        .success();
        contexts.select("fixture-a");
        let observed = run(contexts.docker().args(["info", "--format", "{{json .}}"])).json();
        assert_eq!(observed["OSType"], "linux");
        assert!(matches!(
            observed["Architecture"].as_str(),
            Some("aarch64" | "arm64")
        ));
        contexts
    }

    fn configure(&self, command: &mut Command) {
        command.env("DOCKER_CONFIG", &self.directory);
        for name in [
            "DOCKER_CONTEXT",
            "DOCKER_HOST",
            "DOCKER_TLS",
            "DOCKER_TLS_VERIFY",
            "DOCKER_CERT_PATH",
        ] {
            command.env_remove(name);
        }
    }

    fn docker(&self) -> Command {
        let mut command = Command::new("docker");
        self.configure(&mut command);
        command.arg("--config").arg(&self.directory);
        command
    }

    fn select(&self, context: &str) {
        run(self.docker().args(["context", "use", context])).success();
    }

    fn assert_broken_selection(&self) {
        let context = run(self.docker().args(["context", "show"])).success();
        assert_eq!(
            String::from_utf8(context.stdout).unwrap().trim(),
            "fixture-b"
        );
        assert!(
            !run(self.docker().args(["info", "--format", "{{.ID}}"]))
                .status
                .success(),
            "fixture B unexpectedly reached a Docker daemon"
        );
    }
}

#[test]
#[ignore = "requires native aarch64 Docker and retained pinned images; run --ignored"]
fn executor_and_recovery_pin_the_connection_across_private_context_changes() {
    let f = Fixture::new();
    let store = f.path("store");
    f.images(&store);
    let source = c_source(&f, "context-fixture");
    fs::write(
        source.join("context.sh"),
        "set -eu\nsleep 5\nprintf context-a > \"$1/data\"\n",
    )
    .unwrap();
    let source_id = f.source(&store, &source);
    let contexts = PrivateDockerContexts::new(&f);
    let sentinel = SentinelContainer::new();
    let plan = f.write_json(
        "context-operation.json",
        &script_plan(&source_id, "context.sh", 120),
    );
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&plan)
        .args(["--root", "app"]);
    contexts.configure(&mut command);
    let active = Running::start(&mut command);
    let (journal, observed) = active_container(&store);
    assert_eq!(journal["endpoint"], contexts.endpoint);
    let owned_id = string(&observed, "Id");
    contexts.select("fixture-b");
    contexts.assert_broken_selection();
    let result = active.finish().json();
    let built = app_id(&result);
    f.store("verify", &store, &["--object", &built]).json();
    assert!(!store.join("transactions/execution.json").exists());
    assert!(
        !run(Command::new("docker").args(["container", "inspect", &owned_id]))
            .status
            .success()
    );
    contexts.assert_broken_selection();
    sentinel.assert_alive();

    contexts.select("fixture-a");
    let interrupted_plan = f.write_json(
        "context-interruption.json",
        &script_plan(&source_id, "slow.sh", 120),
    );
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&interrupted_plan)
        .args(["--root", "app"]);
    contexts.configure(&mut command);
    let mut active = Running::start(&mut command);
    let (journal, observed) = active_container(&store);
    assert_eq!(journal["endpoint"], contexts.endpoint);
    let owned_id = string(&observed, "Id");
    contexts.select("fixture-b");
    contexts.assert_broken_selection();
    active.child.kill().unwrap();
    active.child.wait().unwrap();
    let mut recover = f.command(&["store", "recover", "--store"]);
    recover.arg(&store);
    contexts.configure(&mut recover);
    let recovered = run(&mut recover).json();
    nonempty_result(&recovered, "recovered");
    assert!(!active.finish().status.success());
    assert!(!store.join("transactions/execution.json").exists());
    assert!(
        !run(Command::new("docker").args(["container", "inspect", &owned_id]))
            .status
            .success()
    );
    contexts.assert_broken_selection();
    sentinel.assert_alive();
    f.store("verify", &store, &["--object", &built]).json();
}

#[path = "reuse.rs"]
mod reuse;
