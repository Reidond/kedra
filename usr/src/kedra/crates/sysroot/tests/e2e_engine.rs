//! Real public engine workflows. Sandbox qualification is explicitly opt-in: it
//! needs bubblewrap with user namespaces and the retained pinned images, which
//! the fixture imports through the local Docker daemon. An ignored listing is
//! not evidence of building, transferring or running a package.
#![cfg(unix)]

mod cache_recovery;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
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
/// Well-formed root identities for pure planning; no store holds them.
const PLANNED_ROOT: &str = "root-3333333333333333333333333333333333333333333333333333333333333333";
const PLANNED_RUNTIME: &str =
    "root-4444444444444444444444444444444444444444444444444444444444444444";
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
        if matches!(
            operation,
            "add-source" | "add-image" | "add-root" | "import"
        ) && !store.exists()
        {
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

    /// The root of a retained image, created or reused by the public CLI.
    fn root(&self, store: &Path, image: &str) -> String {
        string(
            &self.store("add-root", store, &["--image", image]).json(),
            "root",
        )
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

/// Point every derived receipt at another runtime root, keeping the closure's
/// single runtime root, every member and the outer digest self-consistent.
fn bundle_with_forged_runtime_root(original: &Path, output: &Path, runtime: &str) {
    let mut archive = tar::Archive::new(File::open(original).unwrap());
    let mut builder = tar::Builder::new(File::create(output).unwrap());
    let mut changed = false;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.path().unwrap().as_ref() == Path::new("manifest.json") {
            let mut manifest: Value = serde_json::from_reader(&mut entry).unwrap();
            for object in manifest["objects"].as_object_mut().unwrap().values_mut() {
                let receipt = &mut object["receipt"];
                if receipt["runtime_root"].is_string() {
                    assert_ne!(receipt["runtime_root"], runtime);
                    receipt["runtime_root"] = json!(runtime);
                    receipt["derivation"]["runtime_root"] = json!(runtime);
                }
            }
            append_tar_bytes(
                &mut builder,
                "manifest.json",
                &serde_json::to_vec(&manifest).unwrap(),
            );
            changed = true;
        } else {
            let header = entry.header().clone();
            builder.append(&header, &mut entry).unwrap();
        }
    }
    assert!(changed, "forgery did not alter the bundle manifest");
    builder.finish().unwrap();
}

fn bare_graph() -> Value {
    json!({"schema":1,"nodes":{"app":{
        "builder_root":PLANNED_ROOT,"runtime_root":PLANNED_RUNTIME,"inputs":{},
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
        .args(["--root", "app", "--dry-run"]);
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
        builder_root: builder.clone(), runtime_root: runtime.clone(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.clone()))]),
        argv: vec![Argument::literal("/bin/sh"), Argument::input("source", "library.sh"),
            Argument::input("source", "library.c"), Argument::output("")],
        env: BTreeMap::new(), runtime_inputs: vec![], timeout_seconds: 120,
    };
    let app = BuildNode {
        builder_root: builder.clone(), runtime_root: runtime.clone(),
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

fn authored_plan(
    f: &Fixture,
    author: &Path,
    roots: (&str, &str),
    source: &str,
    name: &str,
) -> PathBuf {
    let (builder, runtime) = roots;
    let value = run(Command::new(author).args([source, builder, runtime])).json();
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
#[ignore = "requires bubblewrap user namespaces and retained pinned images; run --ignored"]
fn package_build_transfer_profiles_and_collection() {
    let f = Fixture::new();
    let producer = f.path("producer");
    let receiver = f.path("receiver");
    f.images(&producer);
    let builder = f.root(&producer, BUILDER);
    assert_eq!(f.root(&producer, BUILDER), builder, "root not reused");
    let runtime = f.root(&producer, RUNTIME);
    let author = authoring_consumer(&f);
    let source_one = c_source(&f, "version-one");
    let source_one_id = f.source(&producer, &source_one);
    assert_eq!(f.source(&producer, &source_one), source_one_id);
    let plan_one = authored_plan(
        &f,
        &author,
        (&builder, &runtime),
        &source_one_id,
        "one.json",
    );
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
    let plan_two = authored_plan(
        &f,
        &author,
        (&builder, &runtime),
        &source_two_id,
        "two.json",
    );
    let second = f.build(&producer, &plan_two, &[]).json();
    let two = app_id(&second);
    assert_ne!(one, two);
    let closure = f.store("closure", &producer, &["--object", &one]).json();
    assert_eq!(
        closure["objects"].as_array().unwrap().len(),
        2,
        "compiler sources leaked into runtime closure: {closure}"
    );
    assert_eq!(closure["runtime_roots"], json!([runtime]));
    // Runtime roots are not transferred: a receiver without the same
    // input-addressed root refuses before admitting anything.
    let rootless = f.path("rootless-receiver");
    let rootless_bundle = f.path("rootless.tar");
    f.store(
        "export",
        &producer,
        &["--object", &one, "--output", text(&rootless_bundle)],
    )
    .json();
    f.store(
        "import",
        &rootless,
        &[
            "--bundle",
            text(&rootless_bundle),
            "--expected-sha256",
            &sha256(&rootless_bundle),
        ],
    )
    .refused("no root");
    assert!(!rootless.join("objects").join(&one).exists());
    f.images(&receiver);
    assert_eq!(f.root(&receiver, RUNTIME), runtime);
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
    f.store("unpin-root", &receiver, &["--root", &runtime])
        .json();
    f.store("gc", &receiver, &[]).json();
    f.store("verify", &receiver, &["--object", &orphan]).json();
    f.store("gc", &receiver, &["--delete"]).json();
    assert!(!receiver.join("objects").join(&orphan).exists());
    // The unpinned image archive goes; the runtime root the profiles' objects
    // run in stays, and running never reads the archive.
    assert!(
        !receiver
            .join("images")
            .join(RUNTIME.strip_prefix("sha256:").unwrap())
            .exists()
    );
    assert!(receiver.join("rootfs").join(&runtime).exists());
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
    let forged_bundle = f.path("forged-runtime-root.tar");
    bundle_with_forged_runtime_root(&bundles[0].0, &forged_bundle, &builder);
    let forged_receiver = f.path("forged-runtime-receiver");
    f.images(&forged_receiver);
    f.root(&forged_receiver, BUILDER);
    // The outer digest agrees with the forged bytes and the receiver holds the
    // forged root. Only the derivation no longer names this output.
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
    .refused("derivation identity");
    assert!(!forged_receiver.join("objects").join(&one).exists());
    assert!(!forged_receiver.join("profiles/fixture").exists());
    let empty = f.store("gc", &forged_receiver, &[]).json();
    assert_eq!(empty["objects"], json!([]));
    assert_version(f.program(&receiver, &one, &[]), "version-one");
    eprintln!(
        "engine lifecycle exercised: authoring, real compile, reuse/rebuild, closure transfer, profiles/develop, GC and corruption"
    );
}

fn script_plan(roots: (&str, &str), source: &str, script: &str, timeout: u64) -> Value {
    let (builder, runtime) = roots;
    json!({"schema":1,"nodes":{"app":{
        "builder_root":builder,"runtime_root":runtime,
        "inputs":{"source":{"kind":"object","value":source}},
        "argv":[[{"kind":"literal","value":"/bin/sh"}],
            [{"kind":"input","name":"source","path":script}],
            [{"kind":"output","path":""}]],
        "env":{},"runtime_inputs":[],"timeout_seconds":timeout
    }}})
}

fn proc_text(pid: u32, name: &str) -> String {
    fs::read_to_string(format!("/proc/{pid}/{name}")).unwrap_or_default()
}

/// Processes whose parent chain reaches `ancestor`, read from /proc.
fn descendants(ancestor: u32) -> Vec<u32> {
    let mut parents = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let stat = proc_text(pid, "stat");
        if let Some((_, tail)) = stat.rsplit_once(')')
            && let Some(parent) = tail.split_whitespace().nth(1)
        {
            parents.push((pid, parent.parse::<u32>().unwrap_or(0)));
        }
    }
    let mut found = vec![ancestor];
    let mut index = 0;
    while index < found.len() {
        let current = found[index];
        found.extend(
            parents
                .iter()
                .filter(|(_, p)| *p == current)
                .map(|(c, _)| *c),
        );
        index += 1;
    }
    found.remove(0);
    found
}

/// The journal of a running sandbox and its innermost non-bubblewrap process.
fn active_sandbox(store: &Path) -> (Value, u32) {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Ok(bytes) = fs::read(store.join("transactions/execution.json"))
            && let Ok(journal) = serde_json::from_slice::<Value>(&bytes)
            && let Some(pid) = journal["process"]["pid"].as_u64()
            && let Some(inner) = descendants(pid as u32).into_iter().find(|child| {
                // Settled: executed the sandboxed program (no longer bwrap) with
                // its environment in place.
                !proc_text(*child, "comm").starts_with("bwrap")
                    && fs::read(format!("/proc/{child}/environ"))
                        .is_ok_and(|environment| !environment.is_empty())
            })
        {
            return (journal, inner);
        }
        assert!(
            Instant::now() < deadline,
            "never observed an active owned sandbox in {}",
            store.display()
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Isolation of the observed sandboxed process: separate user, network and PID
/// namespaces, no capabilities, no new privileges, a non-root owner, a cleared
/// environment and a read-only root with only the output writable in the store.
fn assert_isolation(pid: u32, writable_output: bool) {
    for namespace in ["user", "net", "pid", "ipc", "uts"] {
        assert_ne!(
            fs::read_link(format!("/proc/{pid}/ns/{namespace}")).unwrap(),
            fs::read_link(format!("/proc/self/ns/{namespace}")).unwrap(),
            "sandbox shares the {namespace} namespace"
        );
    }
    let status = proc_text(pid, "status");
    let field = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned()
    };
    assert_ne!(field("Uid:"), "0");
    assert_eq!(field("CapEff:"), "0000000000000000");
    assert_eq!(field("NoNewPrivs:"), "1");
    let environment = fs::read(format!("/proc/{pid}/environ")).unwrap();
    let environment: Vec<_> = environment.split(|b| *b == 0).collect();
    assert!(environment.contains(&&b"HOME=/build"[..]));
    assert!(
        environment
            .iter()
            .all(|v| !v.starts_with(b"KEDRA_ENGINE_HOST_ONLY_SENTINEL="))
    );
    let mut writable = 0;
    let mut root_read_only = false;
    for line in proc_text(pid, "mountinfo").lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let (point, options) = (fields[4], fields[5]);
        let read_only = options.split(',').any(|o| o == "ro");
        if point == "/" {
            root_read_only = read_only;
        }
        if point.starts_with("/usr/lib/sysroot/store/") && !read_only {
            writable += 1;
        }
        assert!(!point.starts_with("/home") && !point.starts_with("/var/run/docker"));
    }
    assert!(root_read_only, "sandbox root is writable");
    assert_eq!(writable, usize::from(writable_output));
}

/// An unrelated process of the same user that recovery must leave alone.
struct Sentinel(Child);

impl Sentinel {
    fn new() -> Self {
        Self(Command::new("sleep").arg("600").spawn().unwrap())
    }

    fn assert_alive(&mut self) {
        assert!(self.0.try_wait().unwrap().is_none(), "sentinel was killed");
    }
}

impl Drop for Sentinel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Kernel start time of a process, from field 22 of /proc/<pid>/stat.
fn start_time(pid: u32) -> u64 {
    let stat = proc_text(pid, "stat");
    let (_, tail) = stat.rsplit_once(')').unwrap();
    tail.split_whitespace().nth(19).unwrap().parse().unwrap()
}

/// The same process still runs: present, not a zombie, same start time.
fn running(pid: u32, start: u64) -> bool {
    let stat = proc_text(pid, "stat");
    stat.rsplit_once(')').is_some_and(|(_, tail)| {
        tail.split_whitespace().next() != Some("Z") && start_time(pid) == start
    })
}

#[test]
#[ignore = "requires bubblewrap user namespaces and retained pinned images; run --ignored"]
fn executor_failures_rebuild_divergence_and_owned_recovery() {
    let f = Fixture::new();
    let store = f.path("store");
    f.images(&store);
    let builder = f.root(&store, BUILDER);
    let runtime = f.root(&store, RUNTIME);
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
    fs::write(
        source.join("offline.sh"),
        "set -eu\nmkdir -p \"$1\"\nif getent hosts example.org >/dev/null 2>&1; then exit 51; fi\n\
         if [ \"$(ls /proc | grep -c \'^[0-9]\')\" -gt 8 ]; then exit 52; fi\n\
         printf offline > \"$1/done\"\n",
    )
    .unwrap();
    let source_id = f.source(&store, &source);
    let plan = |name: &str, script: &str, timeout: u64| {
        f.write_json(
            name,
            &script_plan((&builder, &runtime), &source_id, script, timeout),
        )
    };
    let entropy = plan("entropy.json", "entropy.sh", 120);
    let first = f.build(&store, &entropy, &[]).json();
    let winner = app_id(&first);
    let before = f.store("verify", &store, &["--object", &winner]).json();
    f.build(&store, &entropy, &["--rebuild"]).refused("diverg");
    let after = f.store("verify", &store, &["--object", &winner]).json();
    assert_eq!(before, after, "divergent rebuild changed the valid winner");
    nonempty_result(&f.build(&store, &entropy, &[]).json(), "reused");
    f.build(&store, &plan("offline.json", "offline.sh", 120), &[])
        .json();

    let failure = plan("failure.json", "fail.sh", 120);
    let failure_id = app_id(
        &f.build(&f.path("unopened"), &failure, &["--dry-run"])
            .json(),
    );
    f.build(&store, &failure, &[])
        .refused("intentional-builder-failure");
    assert!(!store.join("objects").join(&failure_id).exists());
    let timeout = plan("timeout.json", "slow.sh", 1);
    let timeout_id = app_id(
        &f.build(&f.path("unopened"), &timeout, &["--dry-run"])
            .json(),
    );
    f.build(&store, &timeout, &[]).refused("deadline");
    assert!(!store.join("objects").join(&timeout_id).exists());
    assert!(!store.join("transactions/execution.json").exists());
    let noisy = plan("noisy.json", "noisy.sh", 120);
    let noisy_result = f.build(&store, &noisy, &[]).json();
    f.store("verify", &store, &["--object", &app_id(&noisy_result)])
        .json();

    // A real active build holds the management lock while a separate GC waits.
    let short = plan("short.json", "short.sh", 120);
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&short)
        .args(["--root", "app"]);
    let mut active = Running::start(&mut command);
    let (_, inner) = active_sandbox(&store);
    assert_isolation(inner, true);
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

    // Killing the controller ends its sandbox (--die-with-parent); the journal
    // stays until recovery verifies ownership and that the process is gone.
    let mut sentinel = Sentinel::new();
    let slow = plan("interrupted.json", "slow.sh", 120);
    let mut command = f.command(&["build", "--store"]);
    command
        .arg(&store)
        .arg("--plan")
        .arg(&slow)
        .args(["--root", "app"]);
    let mut active = Running::start(&mut command);
    let (journal, inner) = active_sandbox(&store);
    assert_isolation(inner, true);
    let sandbox = journal["process"]["pid"].as_u64().unwrap() as u32;
    let sandbox_start = start_time(sandbox);
    active.child.kill().unwrap();
    active.child.wait().unwrap();
    f.store("gc", &store, &[]).refused("recovery");
    let journal_path = store.join("transactions/execution.json");
    let original_journal = fs::read(&journal_path).unwrap();
    let mut wrong_owner = journal.clone();
    wrong_owner["store_token"] = json!("foreign-generated-token");
    fs::write(&journal_path, serde_json::to_vec(&wrong_owner).unwrap()).unwrap();
    f.store("recover", &store, &[]).refused("ownership");
    sentinel.assert_alive();
    assert!(journal_path.exists());
    fs::write(&journal_path, &original_journal).unwrap();
    let recovered = f.store("recover", &store, &[]).json();
    nonempty_result(&recovered, "recovered");
    let interrupted = active.finish();
    assert!(!interrupted.status.success());
    assert!(!journal_path.exists());
    assert!(
        !running(sandbox, sandbox_start),
        "killed controller left its sandbox running"
    );
    sentinel.assert_alive();

    // A sandbox that outlived its controller is killed by recovery, and only it.
    let mut orphan = Sentinel(
        Command::new("/usr/bin/bwrap")
            .args(["--ro-bind", "/", "/", "--unshare-all", "--", "sleep", "600"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut orphan_start = None;
    for _ in 0..200 {
        if proc_text(orphan.0.id(), "comm").starts_with("bwrap") {
            orphan_start = Some(start_time(orphan.0.id()));
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let mut surviving = journal;
    surviving["process"] = json!({"pid": orphan.0.id(), "start_time": orphan_start.unwrap()});
    surviving["staging"] = json!("");
    fs::write(&journal_path, serde_json::to_vec(&surviving).unwrap()).unwrap();
    fs::set_permissions(&journal_path, fs::Permissions::from_mode(0o600)).unwrap();
    f.store("gc", &store, &[]).refused("recovery");
    nonempty_result(&f.store("recover", &store, &[]).json(), "recovered");
    assert_eq!(orphan.0.wait().unwrap().signal(), Some(9));
    assert!(!journal_path.exists());
    sentinel.assert_alive();
    f.store("verify", &store, &["--object", &winner]).json();
    nonempty_result(&f.build(&store, &short, &[]).json(), "reused");

    // A root stays while pinned or while its image is retained; then GC retires it.
    f.store("unpin-root", &store, &["--root", &builder]).json();
    let kept = f.store("gc", &store, &[]).json();
    assert_eq!(
        kept["roots"],
        json!([]),
        "root of a retained image was collected"
    );
    f.store("unpin-image", &store, &["--image", BUILDER]).json();
    let collected = f.store("gc", &store, &["--delete"]).json();
    assert_eq!(collected["roots"], json!([builder]));
    assert!(!store.join("rootfs").join(&builder).exists());
    nonempty_result(&f.build(&store, &short, &[]).json(), "reused");
    f.build(&store, &short, &["--rebuild"]).refused("no root");
    eprintln!(
        "engine faults exercised: divergent winner, offline and failing/timed-out/noisy builders, live isolation, concurrent GC, killed-controller and surviving-sandbox recovery, foreign-owner refusal and root collection"
    );
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One uncompressed layer: (path, type, contents, link target).
fn layer(entries: &[(&str, tar::EntryType, &[u8], Option<&str>)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, kind, bytes, link) in entries {
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(*kind);
        header.set_path(path).unwrap();
        header.set_mode(if kind.is_dir() { 0o755 } else { 0o644 });
        header.set_size(bytes.len() as u64);
        if let Some(link) = link {
            header.set_link_name(link).unwrap();
        }
        header.set_cksum();
        builder.append(&header, *bytes).unwrap();
    }
    builder.into_inner().unwrap()
}

/// A Docker 29 OCI save archive of native layers (blob, media type, diff ID);
/// returns the image identity, the digest of its single manifest.
fn oci_archive(path: &Path, layers: &[(Vec<u8>, &str, String)]) -> String {
    let config = serde_json::to_vec(&json!({
        "architecture":"arm64","os":"linux","config":{},
        "rootfs":{"type":"layers","diff_ids":layers.iter().map(|l| l.2.clone()).collect::<Vec<_>>()},
    }))
    .unwrap();
    let manifest = serde_json::to_vec(&json!({
        "schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json",
        "config":{"mediaType":"application/vnd.oci.image.config.v1+json",
            "digest":format!("sha256:{}", hex(&config)),"size":config.len()},
        "layers":layers.iter().map(|(blob, media, _)| json!({"mediaType":media,
            "digest":format!("sha256:{}", hex(blob)),"size":blob.len()})).collect::<Vec<_>>(),
    }))
    .unwrap();
    let image = format!("sha256:{}", hex(&manifest));
    let index = serde_json::to_vec(&json!({
        "schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json",
        "manifests":[{"mediaType":"application/vnd.oci.image.manifest.v1+json",
            "digest":image,"size":manifest.len()}],
    }))
    .unwrap();
    let docker = serde_json::to_vec(&json!([{
        "Config":format!("blobs/sha256/{}", hex(&config)),
        "Layers":layers.iter().map(|(blob, _, _)| format!("blobs/sha256/{}", hex(blob))).collect::<Vec<_>>(),
    }]))
    .unwrap();
    let mut builder = tar::Builder::new(File::create(path).unwrap());
    for directory in ["blobs/", "blobs/sha256/"] {
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_path(directory).unwrap();
        header.set_mode(0o755);
        header.set_size(0);
        header.set_cksum();
        builder.append(&header, &[][..]).unwrap();
    }
    let mut blobs = vec![config, manifest];
    blobs.extend(layers.iter().map(|(blob, _, _)| blob.clone()));
    for blob in &blobs {
        append_tar_bytes(&mut builder, &format!("blobs/sha256/{}", hex(blob)), blob);
    }
    append_tar_bytes(&mut builder, "index.json", &index);
    append_tar_bytes(&mut builder, "manifest.json", &docker);
    append_tar_bytes(
        &mut builder,
        "oci-layout",
        br#"{"imageLayoutVersion":"1.0.0"}"#,
    );
    builder.finish().unwrap();
    image
}

fn plain(blob: Vec<u8>) -> (Vec<u8>, &'static str, String) {
    let diff = format!("sha256:{}", hex(&blob));
    (blob, "application/vnd.oci.image.layer.v1.tar", diff)
}

#[test]
fn image_roots_unpack_saved_archives_without_a_daemon() {
    use tar::EntryType::{Char, Directory, Link, Regular, Symlink};
    let f = Fixture::new();
    let store = f.path("store");
    let lower = layer(&[
        ("etc/", Directory, b"", None),
        ("etc/keep", Regular, b"lower", None),
        ("etc/gone", Regular, b"lower", None),
        ("bin/", Directory, b"", None),
        ("bin/tool", Regular, b"replaced by a device", None),
        ("opaque/", Directory, b"", None),
        ("opaque/old", Regular, b"hidden by opaque", None),
        ("data/", Directory, b"", None),
        ("data/x", Regular, b"linked", None),
        ("data/y", Link, b"", Some("data/x")),
        ("alias", Symlink, b"", Some("etc")),
    ]);
    // Whiteouts and the opaque marker follow same-layer entries they must keep.
    let upper = layer(&[
        ("etc/new", Regular, b"added", None),
        ("etc/.wh.new", Regular, b"", None),
        ("etc/.wh.gone", Regular, b"", None),
        ("bin/tool", Char, b"", None),
        ("opaque/fresh", Regular, b"same layer", None),
        ("opaque/.wh..wh..opq", Regular, b"", None),
    ]);
    let archive = f.path("image.tar");
    let image = oci_archive(&archive, &[plain(lower.clone()), plain(upper)]);
    let admitted = f
        .store(
            "add-image",
            &store,
            &["--image", &image, "--archive", text(&archive)],
        )
        .json();
    assert_eq!(admitted["image"], image);
    let root = f.root(&store, &image);
    assert_eq!(f.root(&store, &image), root, "image root was not reused");
    let tree = store.join("rootfs").join(&root).join("tree");
    let read = |path: &str| fs::read(tree.join(path)).ok();
    assert_eq!(read("etc/keep").as_deref(), Some(&b"lower"[..]));
    assert_eq!(read("etc/new").as_deref(), Some(&b"added"[..]));
    assert_eq!(read("opaque/fresh").as_deref(), Some(&b"same layer"[..]));
    for absent in ["etc/gone", "bin/tool", "opaque/old", "etc/.wh.gone"] {
        assert!(
            fs::symlink_metadata(tree.join(absent)).is_err(),
            "{absent} survived its layer"
        );
    }
    let x = fs::metadata(tree.join("data/x")).unwrap();
    let y = fs::metadata(tree.join("data/y")).unwrap();
    assert_eq!((x.dev(), x.ino()), (y.dev(), y.ino()));
    assert_eq!(fs::read_link(tree.join("alias")).unwrap(), Path::new("etc"));
    for point in ["dev", "proc", "tmp", "build", "usr/lib/sysroot/store"] {
        assert!(tree.join(point).is_dir(), "missing mount point {point}");
    }

    // A later layer may not write through a lower symlink.
    let escape = layer(&[("alias/escape", Regular, b"outside", None)]);
    let archive = f.path("escape.tar");
    let image = oci_archive(&archive, &[plain(lower.clone()), plain(escape)]);
    f.store(
        "add-image",
        &store,
        &["--image", &image, "--archive", text(&archive)],
    )
    .json();
    f.store("add-root", &store, &["--image", &image])
        .refused("traverses a non-directory");

    // A compressed layer must decompress to its configured DiffID.
    let compressed = f.path("lower.tar.gz");
    fs::write(f.path("lower.tar"), &lower).unwrap();
    let gzip = run(Command::new("gzip")
        .args(["-n", "-k", "-f"])
        .arg(f.path("lower.tar")))
    .success();
    assert!(gzip.stdout.is_empty());
    let blob = fs::read(&compressed).unwrap();
    let wrong = format!("sha256:{}", "0".repeat(64));
    let archive = f.path("wrong-diff.tar");
    let image = oci_archive(
        &archive,
        &[(blob, "application/vnd.oci.image.layer.v1.tar+gzip", wrong)],
    );
    f.store(
        "add-image",
        &store,
        &["--image", &image, "--archive", text(&archive)],
    )
    .json();
    f.store("add-root", &store, &["--image", &image])
        .refused("uncompressed layer differs");
    let collected = f.store("gc", &store, &[]).json();
    assert_eq!(
        collected["roots"],
        json!([]),
        "a refused root was published"
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
        "schema":2,"token":marker["token"],"stage":stage_name,
        "objects":[object],"roots":[object]
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
    assert_eq!(closure["runtime_roots"], json!([]));
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
