//! Public offline context replay after deleting the producing repository and store.
//! Docker cases are opt-in and use the separately built sanctioned lab executable.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sysroot_engine::{
    Argument, BuildGraph, BuildNode, Input, PLATFORM, Segment, SystemContent, SystemDefinition,
    SystemFile,
};

const FOUNDATION: &str = "sha256:9d6eb030a55f86232e7f6df46551d5394599b70b2ad7837a41a5cde300550e71";
const BUILDER: &str = "sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546";
const CONFIG: &str = "etc/sysroot-context-fixture.conf";
const CONFIG_BYTES: &[u8] = b"context-config-v1\n";

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
        let mut result = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = pipe.read(&mut buffer).unwrap();
            if count == 0 {
                return result;
            }
            let retained = count.min((8 * 1024 * 1024_usize).saturating_sub(result.len()));
            result.extend_from_slice(&buffer[..retained]);
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
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            start.elapsed() < Duration::from_secs(900),
            "context subprocess timed out: {command:?}"
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

fn refused(output: Output, case: &str) {
    assert!(!output.status.success(), "unexpected success: {case}");
    assert!(
        output.stdout.is_empty(),
        "refusal returned success data: {case}"
    );
    assert!(
        !output.stderr.is_empty(),
        "refusal lacks a diagnostic: {case}"
    );
    eprintln!(
        "context refusal {case}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
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

struct Fixture {
    root: PathBuf,
    retain: bool,
    foundation: String,
}
impl Fixture {
    fn new(retain: bool) -> Self {
        let retained = retain
            .then(|| std::env::var_os("KEDRA_CONTEXT_E2E_RETAIN_DIR"))
            .flatten();
        let root = retained.as_ref().map_or_else(
            || {
                std::env::temp_dir().join(format!(
                    "kedra-context-e2e-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
            },
            PathBuf::from,
        );
        assert!(root.is_absolute(), "retained directory must be absolute");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for name in ["producer", "snapshots"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
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

    fn store(&self, operation: &str, args: &[&str]) -> Value {
        let mut command = self.cli(&["store", operation, "--store"]);
        json_output(run(command.arg(self.producer("store")).args(args)))
    }

    fn verify(&self, identity: &str) -> Output {
        let mut command = self.cli(&["system", "verify", "--context"]);
        let output = run(command
            .arg(self.path("context"))
            .args(["--expected-identity", identity, "--workdir"])
            .arg(self.path("snapshots")));
        assert_eq!(
            fs::read_dir(self.path("snapshots")).unwrap().count(),
            0,
            "consumer leaked private snapshots"
        );
        output
    }

    fn replay(&self, identity: &str, execute: bool) -> Output {
        let binary = std::env::var_os("KEDRA_CONTEXT_LAB_BINARY")
            .expect("set KEDRA_CONTEXT_LAB_BINARY to the already built kedra-lab executable");
        let mut command = Command::new(binary);
        command
            .args(["replay", "--image"])
            .arg(format!("composition:{}", self.path("context").display()))
            .args(["--composition-identity", identity, "--target", "qemu-arm64"]);
        if execute {
            command.args([
                "--output",
                "fixture",
                "--program",
                "bin/context-fixture",
                "--",
                "replay",
            ]);
        }
        run(&mut command)
    }

    fn repository(&self) {
        let repo = self.producer("repo");
        fs::create_dir(&repo).unwrap();
        for (path, bytes) in [
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
        ] {
            let path = repo.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        for args in [
            vec!["init", "-q", "--template=", "-b", "fixture"],
            vec!["add", "."],
            vec!["commit", "-q", "-m", "generated replay fixture"],
        ] {
            let mut git = Command::new("git");
            for (name, _) in std::env::vars_os() {
                if name.to_string_lossy().starts_with("GIT_") {
                    git.env_remove(name);
                }
            }
            success(run(git
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .arg("-C")
                .arg(&repo)
                .args([
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)));
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.retain || thread::panicking() {
            eprintln!("context E2E fixture retained: {}", self.root.display());
        } else {
            let _ = remove_fixture(&self.root);
        }
    }
}

#[test]
fn context_cli_refuses_invalid_identity_and_missing_context_without_docker() {
    let f = Fixture::new(false);
    let result = f.verify("not-an-identity");
    assert!(String::from_utf8_lossy(&result.stderr).contains("identity"));
    refused(result, "malformed independent identity");
    refused(f.verify(&"0".repeat(64)), "absent context");
    assert!(!f.producer("store").exists());
}

fn graph(source: &str, foundation: &str) -> BuildGraph {
    let library = BuildNode {
        builder_image: BUILDER.into(),
        runtime_image: foundation.into(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.into()))]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::input("source", "library.sh"),
            Argument::input("source", "library.c"),
            Argument::output(""),
        ],
        env: BTreeMap::new(),
        runtime_inputs: vec![],
        timeout_seconds: 120,
    };
    let app = BuildNode {
        builder_image: BUILDER.into(),
        runtime_image: foundation.into(),
        inputs: BTreeMap::from([
            ("source".into(), Input::Object(source.into())),
            ("library".into(), Input::Node("library".into())),
        ]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::input("source", "app.sh"),
            Argument::input("source", "main.c"),
            Argument::input("library", "lib"),
            Argument::output(""),
            Argument(vec![
                Segment::Literal {
                    value: "-Wl,-rpath,".into(),
                },
                Segment::Input {
                    name: "library".into(),
                    path: "lib".into(),
                },
            ]),
        ],
        env: BTreeMap::new(),
        runtime_inputs: vec!["library".into()],
        timeout_seconds: 120,
    };
    BuildGraph {
        schema: 1,
        nodes: BTreeMap::from([("app".into(), app), ("library".into(), library)]),
    }
}

fn package_source(f: &Fixture) -> PathBuf {
    let source = f.producer("package-source");
    fs::create_dir(&source).unwrap();
    for (name, contents) in [
        (
            "library.c",
            "const char *context_version(void) { return \"runtime-v1\"; }\n",
        ),
        (
            "library.sh",
            "set -eu\nmkdir -p \"$2/lib\"\ncc -fPIC -shared -Wl,-soname,libcontext_fixture.so -o \"$2/lib/libcontext_fixture.so\" \"$1\"\n",
        ),
        (
            "app.sh",
            "set -eu\nmkdir -p \"$3/bin\"\ncc -o \"$3/bin/context-fixture\" \"$1\" -L\"$2\" -lcontext_fixture \"$4\"\n",
        ),
        (
            "main.c",
            r#"#include <stdio.h>
#include <string.h>
extern const char *context_version(void);
int main(int argc, char **argv) {
    char config[128];
    FILE *input = fopen("/etc/sysroot-context-fixture.conf", "r");
    if (!input || !fgets(config, sizeof(config), input)) return 40;
    if (fclose(input)) return 41;
    const char *mode = argc > 1 ? argv[1] : "replay";
    printf("context-fixture:%s:%s:%s", mode, context_version(), config);
    if (strcmp(mode, "unit") == 0) {
        FILE *marker = fopen("/run/sysroot-context-fixture.marker", "w");
        if (!marker) return 42;
        if (fprintf(marker, "context-fixture:%s:%s:%s", mode, context_version(), config) < 0) return 43;
        if (fclose(marker)) return 44;
    }
    return 0;
}
"#,
        ),
    ] {
        fs::write(source.join(name), contents).unwrap();
    }
    source
}

fn definition(object: &str, foundation: &str) -> SystemDefinition {
    SystemDefinition {
        schema: 1, platform: PLATFORM.into(), foundation: foundation.into(),
        provenance: BTreeMap::from([("fixture".into(), "context-replay-v1".into())]),
        outputs: BTreeMap::from([("fixture".into(), object.into())]),
        required_packages: vec!["bash".into()], removed_packages: vec![],
        files: vec![
            SystemFile { path: format!("/{CONFIG}"), mode: 0o644, provenance: "context.config".into(),
                priority: 0, replaces: None, content: SystemContent::Bytes(CONFIG_BYTES.to_vec()) },
            SystemFile { path: "/usr/lib/systemd/system/sysroot-context-fixture.service".into(),
                mode: 0o644, provenance: "context.unit".into(), priority: 0, replaces: None,
                content: SystemContent::Template(Argument(vec![
                    Segment::Literal { value: "[Unit]\nDescription=Context replay fixture\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=".into() },
                    Segment::Input { name: "fixture".into(), path: "bin/context-fixture".into() },
                    Segment::Literal { value: " unit\n".into() },
                ])) },
        ],
    }
}

fn compose(f: &Fixture) -> Value {
    f.repository();
    f.store("init", &[]);
    for image in [f.foundation.as_str(), BUILDER] {
        let inspected = json_output(run(Command::new("docker").args(["image", "inspect", image])));
        assert_eq!(inspected[0]["Id"], image);
        assert_eq!(inspected[0]["Architecture"], "arm64");
        assert_eq!(f.store("add-image", &["--image", image])["image"], image);
    }
    let source = package_source(f);
    let admitted = f.store("add-source", &["--source", text(&source)]);
    let graph_path = f.producer("build.json");
    write_json(
        &graph_path,
        &graph(admitted["object"].as_str().unwrap(), &f.foundation),
    );
    let built = json_output(run(f
        .cli(&["build", "--store"])
        .arg(f.producer("store"))
        .arg("--plan")
        .arg(graph_path)
        .args(["--root", "app"])));
    let object = built["outputs"]["app"].as_str().unwrap();
    let definition_path = f.producer("definition.json");
    write_json(&definition_path, &definition(object, &f.foundation));
    let composed = json_output(run(f
        .cli(&["system", "compose", "--repo"])
        .arg(f.producer("repo"))
        .args(["--target", "qemu-arm64", "--store"])
        .arg(f.producer("store"))
        .args(["--foundation", &f.foundation, "--definition"])
        .arg(definition_path)
        .arg("--output-dir")
        .arg(f.path("context"))));
    let objects = composed["plan"]["objects"].as_object().unwrap();
    assert_eq!(
        objects.len(),
        2,
        "context must retain the distinct application and runtime library"
    );
    assert_eq!(objects[object]["references"].as_array().unwrap().len(), 1);
    composed
}

fn large_configuration_roundtrip(f: &Fixture, object: &str) {
    let mut definition = definition(object, &f.foundation);
    let mut config = vec![b'A'; 1024 * 1024];
    config[..8].copy_from_slice(b"payload=");
    *config.last_mut().unwrap() = b'\n';
    definition.files.push(SystemFile {
        path: "/etc/context-large-fixture.conf".into(),
        mode: 0o644,
        provenance: "context.large-config".into(),
        priority: 0,
        replaces: None,
        content: SystemContent::Bytes(config.clone()),
    });
    let definition_path = f.producer("large-definition.json");
    write_json(&definition_path, &definition);
    assert!(fs::metadata(&definition_path).unwrap().len() < 8 * 1024 * 1024);
    let context = f.path("large-context");
    let output = success(run(f
        .cli(&["system", "compose", "--repo"])
        .arg(f.producer("repo"))
        .args(["--target", "qemu-arm64", "--store"])
        .arg(f.producer("store"))
        .args(["--foundation", &f.foundation, "--definition"])
        .arg(definition_path)
        .arg("--output-dir")
        .arg(&context)));
    #[derive(serde::Deserialize)]
    struct Identity {
        identity: String,
    }
    let expected: Identity = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        fs::metadata(context.join("composition.json"))
            .unwrap()
            .len()
            > 8 * 1024 * 1024,
        "fixture must exercise a valid export above the old consumer manifest limit"
    );
    let verified = json_output(run(f
        .cli(&["system", "verify", "--context"])
        .arg(&context)
        .args(["--expected-identity", &expected.identity, "--workdir"])
        .arg(f.path("snapshots"))));
    assert_eq!(verified["identity"], expected.identity);
    assert_eq!(fs::read_dir(f.path("snapshots")).unwrap().count(), 0);
    let mut count = 0;
    let mut payload = tar::Archive::new(File::open(context.join("payload.tar")).unwrap());
    for entry in payload.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.path().unwrap() == Path::new("etc/context-large-fixture.conf") {
            assert!(entry.header().entry_type().is_file());
            assert_eq!(entry.header().mode().unwrap(), 0o644);
            let mut actual = Vec::new();
            entry.read_to_end(&mut actual).unwrap();
            assert_eq!(actual, config);
            count += 1;
        }
    }
    assert_eq!(
        count, 1,
        "large configuration must survive the public export exactly once"
    );
    drop(payload);
    remove_fixture(&context).unwrap();
}

fn forged_artifact(composition: &Value, name: &str, bytes: &[u8]) -> Value {
    let mut forged = composition.clone();
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    forged["artifacts"][name] = json!({"sha256": hash, "bytes": bytes.len()});
    forged
}

enum Tamper {
    Config,
    MissingLibrary,
    Program,
    Link,
    Special,
}

fn tamper_payload(original: &[u8], tamper: &Tamper) -> Vec<u8> {
    let mut archive = tar::Archive::new(Cursor::new(original));
    let mut output = tar::Builder::new(Vec::new());
    let mut touched = false;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().into_owned();
        let mut header = entry.header().clone();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        let name = text(&path);
        match tamper {
            Tamper::MissingLibrary if name.ends_with("/lib/libcontext_fixture.so") => {
                touched = true;
                continue;
            }
            Tamper::Program if name.ends_with("/bin/context-fixture") => {
                assert!(
                    bytes.starts_with(b"\x7fELF"),
                    "fixture must execute a compiled ELF"
                );
                bytes[0] ^= 1;
                touched = true;
            }
            Tamper::Config if name == CONFIG => {
                assert_eq!(bytes, CONFIG_BYTES);
                bytes = b"context-config-forged\n".to_vec();
                touched = true;
            }
            Tamper::Link | Tamper::Special if name == CONFIG => {
                bytes.clear();
                if matches!(tamper, Tamper::Link) {
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_link_name("/etc/passwd").unwrap();
                } else {
                    header.set_entry_type(tar::EntryType::Fifo);
                }
                touched = true;
            }
            _ => {}
        }
        header.set_size(bytes.len() as u64);
        header.set_cksum();
        output.append(&header, Cursor::new(bytes)).unwrap();
    }
    assert!(touched, "tamper must change the relevant runtime artifact");
    output.into_inner().unwrap()
}

fn refusal_matrix(f: &Fixture, composition: &Value, identity: &str) {
    let context = f.path("context");
    let manifest_path = context.join("composition.json");
    let payload_path = context.join("payload.tar");
    let original = fs::read(&payload_path).unwrap();
    refused(f.verify(&"0".repeat(64)), "independent identity mismatch");
    for (case, tamper) in [
        ("config with recomputed outer hash", Tamper::Config),
        ("missing runtime library", Tamper::MissingLibrary),
        ("corrupt compiled output", Tamper::Program),
        ("payload symlink", Tamper::Link),
        ("payload FIFO", Tamper::Special),
    ] {
        let payload = tamper_payload(&original, &tamper);
        fs::write(&payload_path, &payload).unwrap();
        write_json(
            &manifest_path,
            &forged_artifact(composition, "payload.tar", &payload),
        );
        refused(f.verify(identity), case);
    }
    fs::write(&payload_path, &original).unwrap();
    write_json(&manifest_path, composition);

    let containerfile = context.join("Containerfile");
    let original_containerfile = fs::read(&containerfile).unwrap();
    let mut forged_containerfile = original_containerfile.clone();
    forged_containerfile.extend_from_slice(b"RUN touch /unapproved\n");
    fs::write(&containerfile, &forged_containerfile).unwrap();
    write_json(
        &manifest_path,
        &forged_artifact(composition, "Containerfile", &forged_containerfile),
    );
    refused(
        f.verify(identity),
        "additional Containerfile instruction with recomputed hash",
    );
    fs::write(&containerfile, &original_containerfile).unwrap();
    write_json(&manifest_path, composition);

    fs::remove_file(&containerfile).unwrap();
    refused(f.verify(identity), "missing context artifact");
    let linked = f.path("linked-containerfile");
    fs::write(&linked, &original_containerfile).unwrap();
    symlink(&linked, &containerfile).unwrap();
    refused(f.verify(identity), "context member symlink");
    fs::remove_file(&containerfile).unwrap();
    fs::hard_link(&linked, &containerfile).unwrap();
    refused(f.verify(identity), "context member hardlink");
    fs::remove_file(&containerfile).unwrap();
    success(run(Command::new("mkfifo").arg(&containerfile)));
    refused(f.verify(identity), "context member FIFO");
    fs::remove_file(&containerfile).unwrap();
    fs::remove_file(linked).unwrap();
    fs::write(&containerfile, &original_containerfile).unwrap();

    let mut forged_manifest = composition.clone();
    forged_manifest["plan"]["definition"]["provenance"]["fixture"] = json!("forged-plan");
    write_json(&manifest_path, &forged_manifest);
    refused(f.verify(identity), "plan changed under retained identity");
    write_json(&manifest_path, composition);
    let extra = context.join("unexpected");
    fs::write(&extra, b"unexpected context member").unwrap();
    refused(f.verify(identity), "extra context member");
    fs::remove_file(extra).unwrap();

    // Corrupt only one byte of the large retained archive, then restore it in place.
    let foundation_path = context.join("foundation.tar");
    let permissions = fs::metadata(&foundation_path).unwrap().permissions();
    fs::set_permissions(&foundation_path, fs::Permissions::from_mode(0o600)).unwrap();
    let mut foundation = File::options()
        .read(true)
        .write(true)
        .open(&foundation_path)
        .unwrap();
    let mut first = [0_u8; 1];
    foundation.read_exact(&mut first).unwrap();
    foundation.seek(SeekFrom::Start(0)).unwrap();
    foundation.write_all(&[first[0] ^ 1]).unwrap();
    foundation.sync_all().unwrap();
    refused(f.verify(identity), "corrupt retained foundation");
    foundation.seek(SeekFrom::Start(0)).unwrap();
    foundation.write_all(&first).unwrap();
    foundation.sync_all().unwrap();
    fs::set_permissions(foundation_path, permissions).unwrap();
}

struct RestoreImageTag {
    image: String,
    tag: String,
    active: bool,
}

impl Drop for RestoreImageTag {
    fn drop(&mut self) {
        if self.active {
            let restored = Command::new("docker")
                .args(["image", "tag", &self.image, &self.tag])
                .output();
            if !restored.is_ok_and(|output| output.status.success()) {
                eprintln!(
                    "failed restoring fixture tag {} to {}",
                    self.tag, self.image
                );
            }
        }
    }
}

fn refuse_foreign_cached_tag(f: &Fixture, identity: &str) {
    let tag = format!("kedra-composition:{identity}");
    let inspected = json_output(run(Command::new("docker").args(["image", "inspect", &tag])));
    let mut restore = RestoreImageTag {
        image: inspected[0]["Id"].as_str().unwrap().into(),
        tag,
        active: true,
    };
    success(run(Command::new("docker").args([
        "image",
        "tag",
        BUILDER,
        &restore.tag,
    ])));
    let refusal = f.replay(identity, false);
    success(run(Command::new("docker").args([
        "image",
        "tag",
        &restore.image,
        &restore.tag,
    ])));
    restore.active = false;
    assert!(
        String::from_utf8_lossy(&refusal.stderr).contains("binding mismatch"),
        "foreign cache tag failed for an unrelated reason: {}",
        String::from_utf8_lossy(&refusal.stderr)
    );
    refused(
        refusal,
        "cached composition tag replaced by a foreign image",
    );
}

#[test]
#[ignore = "requires native ARM Docker, restored exact signed foundation, and KEDRA_CONTEXT_LAB_BINARY"]
fn copied_context_replays_real_closure_after_producer_removal() {
    let f = Fixture::new(true);
    let composed = compose(&f);
    large_configuration_roundtrip(
        &f,
        composed["plan"]["definition"]["outputs"]["fixture"]
            .as_str()
            .unwrap(),
    );
    let identity = composed["identity"].as_str().unwrap().to_owned();
    write_json(
        &f.path("expected-identity.json"),
        &json!({"identity": identity, "foundation": f.foundation}),
    );
    remove_fixture(&f.path("producer")).unwrap();
    assert!(
        !f.path("producer").exists(),
        "producer repository, package source and store remain"
    );

    refusal_matrix(&f, &composed, &identity);
    let verified = json_output(f.verify(&identity));
    assert_eq!(verified["identity"], identity);
    let replay = success(f.replay(&identity, true));
    assert_eq!(
        replay.stdout,
        b"context-fixture:replay:runtime-v1:context-config-v1\n"
    );
    // Repeat through the public consumer after its image is cached: identity remains mandatory.
    refused(
        f.replay(&"0".repeat(64), true),
        "cached replay with a wrong identity",
    );
    refuse_foreign_cached_tag(&f, &identity);
    let repeated = success(f.replay(&identity, true));
    assert_eq!(repeated.stdout, replay.stdout);
}
