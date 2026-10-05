//! Public catalog workflows. Real upstream builds are explicitly opt-in and use
//! already acquired source trees and images; this target never downloads inputs.
#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use sysroot_catalog::{JQ_SOURCE, SQLITE_SOURCE};

static NEXT: AtomicU64 = AtomicU64::new(0);
const BUILDER: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RUNTIME: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    root: PathBuf,
    binary: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kedra-catalog-e2e-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let binary = std::env::var_os("KEDRA_ENGINE_E2E_BINARY").map_or_else(
            || PathBuf::from(env!("CARGO_BIN_EXE_sysroot")),
            PathBuf::from,
        );
        Self { root, binary }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(&self.binary).args(arguments).output().unwrap()
    }

    fn json(&self, arguments: &[&str]) -> Value {
        let output = success(self.run(arguments));
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn write(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }

    fn policy(&self, builder: &str, runtime: &str) -> Value {
        json!({"namespace":"kedra", "packages":["jq", "sqlite"],
            "builder_images":[builder], "runtime_images":[runtime],
            "source_objects":[JQ_SOURCE, SQLITE_SOURCE]})
    }

    fn selection(
        &self,
        command: &str,
        package: &str,
        policy: &Path,
        builder: &str,
        runtime: &str,
    ) -> Value {
        self.json(&[
            "catalog",
            command,
            "--package",
            package,
            "--policy",
            text(policy),
            "--builder",
            builder,
            "--runtime",
            runtime,
        ])
    }

    fn build(
        &self,
        store: &Path,
        package: &str,
        policy: &Path,
        builder: &str,
        runtime: &str,
        rebuild: bool,
    ) -> Value {
        let mut arguments = vec![
            "catalog",
            "build",
            "--package",
            package,
            "--policy",
            text(policy),
            "--builder",
            builder,
            "--runtime",
            runtime,
            "--store",
            text(store),
        ];
        if rebuild {
            arguments.push("--rebuild");
        }
        self.json(&arguments)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("catalog evidence retained at {}", self.root.display());
        } else {
            remove_owned(&self.root);
        }
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
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

fn denied(output: Output, diagnostic: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(diagnostic),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn remove_owned(path: &Path) {
    let metadata = fs::symlink_metadata(path).unwrap();
    if metadata.is_dir() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        for entry in fs::read_dir(path).unwrap() {
            remove_owned(&entry.unwrap().path());
        }
        fs::remove_dir(path).unwrap();
    } else {
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn catalog_planning_and_policy_refusal_precede_store_access() {
    let fixture = Fixture::new();
    let listing = fixture.json(&["catalog", "list"]);
    assert_eq!(listing["namespace"], "kedra");
    assert!(
        listing["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "sqlite")
    );
    let pins = success(fixture.run(&["catalog", "pins"]));
    assert_eq!(
        pins.stdout,
        success(fixture.run(&["catalog", "pins"])).stdout
    );
    let pins: Value = serde_json::from_slice(&pins.stdout).unwrap();
    assert_eq!(pins["packages"]["jq"]["sources"][0]["object"], JQ_SOURCE);
    assert_eq!(
        pins["packages"]["sqlite"]["sources"][0]["object"],
        SQLITE_SOURCE
    );
    let policy = fixture.write("policy.json", &fixture.policy(BUILDER, RUNTIME));
    let first = fixture.selection("resolve", "sqlite", &policy, BUILDER, RUNTIME);
    let second = fixture.selection("resolve", "sqlite", &policy, BUILDER, RUNTIME);
    assert_eq!(first, second);
    assert_eq!(first["recipe"]["program"], "bin/sqlite3");
    assert_eq!(first["plan"]["order"], json!(["sqlite-library", "sqlite"]));
    let graph = fixture.selection("plan", "sqlite", &policy, BUILDER, RUNTIME);
    assert_eq!(graph, first["recipe"]["graph"]);
    let absent_store = fixture.path("must-not-exist");
    for (field, replacement, expected) in [
        (
            "namespace",
            json!("fieldkit"),
            "catalog policy denied: package",
        ),
        ("packages", json!(["jq"]), "catalog policy denied: package"),
        (
            "builder_images",
            json!([]),
            "catalog policy denied: builder",
        ),
        (
            "runtime_images",
            json!([]),
            "catalog policy denied: runtime",
        ),
        ("source_objects", json!([]), "catalog policy denied: source"),
    ] {
        let mut value = fixture.policy(BUILDER, RUNTIME);
        value[field] = replacement;
        let policy = fixture.write("denied.json", &value);
        denied(
            fixture.run(&[
                "catalog",
                "build",
                "--package",
                "sqlite",
                "--policy",
                text(&policy),
                "--builder",
                BUILDER,
                "--runtime",
                RUNTIME,
                "--store",
                text(&absent_store),
            ]),
            expected,
        );
        assert!(!absent_store.exists());
    }
}

#[test]
fn independently_authored_catalog_cannot_hide_an_unapproved_source() {
    let fixture = Fixture::new();
    let catalog = sysroot_catalog::builtin(BUILDER, RUNTIME);
    let mut catalog = serde_json::to_value(catalog).unwrap();
    let foreign_source = "src-3333333333333333333333333333333333333333333333333333333333333333";
    catalog["packages"]["jq"]["recipe"]["graph"]["nodes"]["jq"]["inputs"]["source"]["value"] =
        json!(foreign_source);
    let catalog_path = fixture.write("catalog.json", &catalog);
    let policy = fixture.write("policy.json", &fixture.policy(BUILDER, RUNTIME));
    let store = fixture.path("must-not-exist");
    denied(
        fixture.run(&[
            "catalog",
            "build",
            "--catalog",
            text(&catalog_path),
            "--package",
            "jq",
            "--policy",
            text(&policy),
            "--store",
            text(&store),
        ]),
        "undeclared source",
    );
    assert!(!store.exists());
}

#[test]
#[ignore = "requires retained native ARM builder/Fedora images and verified upstream sources"]
fn real_catalog_build_reproduce_transfer_and_query() {
    let fixture = Fixture::new();
    let builder =
        std::env::var("KEDRA_CATALOG_BUILDER").expect("set exact retained KEDRA_CATALOG_BUILDER");
    let runtime =
        std::env::var("KEDRA_CATALOG_RUNTIME").expect("set exact retained KEDRA_CATALOG_RUNTIME");
    let jq_source = std::env::var("KEDRA_CATALOG_JQ_SOURCE")
        .expect("set normalized upstream jq source directory");
    let sqlite_source = std::env::var("KEDRA_CATALOG_SQLITE_SOURCE")
        .expect("set normalized upstream SQLite source directory");
    let producer = fixture.path("producer");
    let receiver = fixture.path("receiver");
    let policy = fixture.write("policy.json", &fixture.policy(&builder, &runtime));
    fixture.json(&["store", "init", "--store", text(&producer)]);
    for (source, expected) in [(&jq_source, JQ_SOURCE), (&sqlite_source, SQLITE_SOURCE)] {
        let admitted = fixture.json(&[
            "store",
            "add-source",
            "--store",
            text(&producer),
            "--source",
            source,
        ]);
        assert_eq!(admitted["object"], expected);
    }
    for image in [&builder, &runtime] {
        fixture.json(&[
            "store",
            "add-image",
            "--store",
            text(&producer),
            "--image",
            image,
        ]);
    }
    let sqlite = fixture.build(&producer, "sqlite", &policy, &builder, &runtime, false);
    let jq = fixture.build(&producer, "jq", &policy, &builder, &runtime, false);
    let sqlite_object = sqlite["result"]["outputs"]["sqlite"].as_str().unwrap();
    let library_object = sqlite["result"]["outputs"]["sqlite-library"]
        .as_str()
        .unwrap();
    let jq_object = jq["result"]["outputs"]["jq"].as_str().unwrap();
    let version = success(fixture.run(&[
        "run",
        "--store",
        text(&producer),
        "--object",
        jq_object,
        "--program",
        "bin/jq",
        "--",
        "--version",
    ]));
    assert_eq!(version.stdout, b"jq-1.8.2\n");
    let closure = fixture.json(&[
        "store",
        "closure",
        "--store",
        text(&producer),
        "--object",
        sqlite_object,
    ]);
    assert!(
        closure["objects"]
            .as_array()
            .unwrap()
            .contains(&json!(library_object))
    );
    for (package, count) in [("jq", 1), ("sqlite", 2)] {
        let reused = fixture.build(&producer, package, &policy, &builder, &runtime, false);
        assert_eq!(reused["result"]["reused"].as_array().unwrap().len(), count);
        let rebuilt = fixture.build(&producer, package, &policy, &builder, &runtime, true);
        assert_eq!(
            rebuilt["result"]["reproduced"].as_array().unwrap().len(),
            count
        );
    }
    fixture.json(&[
        "profile",
        "switch",
        "--store",
        text(&producer),
        "--name",
        "database",
        "--object",
        sqlite_object,
        "--program",
        "bin/sqlite3",
    ]);
    let script = format!(
        "LD_DEBUG=libs '{}/{sqlite_object}/bin/sqlite3' :memory: 'SELECT 41+1;'",
        sysroot_engine::LOGICAL_PREFIX
    );
    let loader = success(fixture.run(&[
        "develop",
        "--store",
        text(&producer),
        "--name",
        "database",
        "--program",
        "/bin/sh",
        "--",
        "-c",
        &script,
    ]));
    assert_eq!(loader.stdout, b"42\n");
    assert!(String::from_utf8_lossy(&loader.stderr).contains(&format!(
        "calling init: {}/{library_object}/lib/libsqlite3.so",
        sysroot_engine::LOGICAL_PREFIX
    )));

    // The author consumes generated public material with actual selected input
    // bytes. Complete D2 release-material/installed delivery is qualified separately.
    let pins = success(fixture.run(&["catalog", "pins"]));
    let pins_hash = hash(&pins.stdout);
    let author_hash = hash(&fs::read(&fixture.binary).unwrap());
    let material = fixture.write("material.json", &json!({
        "schema_version":1, "source":{"target":{"id":"qemu-arm64","architecture":"aarch64"}},
        "artifacts":{"sysroot":author_hash,"catalog-author":author_hash,"catalog-pins":pins_hash}
    }));
    let material_hash = hash(&fs::read(&material).unwrap());
    let contribution = fixture.path("contribution");
    let result = fixture.json(&[
        "catalog",
        "contribute",
        "--store",
        text(&producer),
        "--builder",
        &builder,
        "--foundation",
        &runtime,
        "--policy",
        text(&policy),
        "--source-revision",
        "1111111111111111111111111111111111111111",
        "--input-material",
        text(&material),
        "--expected-input-material-sha256",
        &material_hash,
        "--output-dir",
        text(&contribution),
    ]);
    assert_eq!(result["outputs"]["jq"], jq_object);
    assert_eq!(result["outputs"]["sqlite"], sqlite_object);
    let definition_bytes = fs::read(contribution.join("system.json")).unwrap();
    assert_eq!(result["definition_sha256"], hash(&definition_bytes));
    let definition: Value = serde_json::from_slice(&definition_bytes).unwrap();
    assert_eq!(definition["foundation"], runtime);
    assert_eq!(definition["platform"], "aarch64-linux");
    let receipt_bytes = fs::read(contribution.join("contribution.json")).unwrap();
    assert_eq!(result["contribution_receipt_sha256"], hash(&receipt_bytes));
    let receipt: Value = serde_json::from_slice(&receipt_bytes).unwrap();
    assert_eq!(receipt["outputs"], definition["outputs"]);
    assert_eq!(receipt["catalog_pins_sha256"], pins_hash);
    let mut rejected_material: Value =
        serde_json::from_slice(&fs::read(&material).unwrap()).unwrap();
    rejected_material["artifacts"]["catalog-pins"] = json!("0".repeat(64));
    let rejected_material = fixture.write("wrong-material.json", &rejected_material);
    let rejected_hash = hash(&fs::read(&rejected_material).unwrap());
    let rejected_directory = fixture.path("rejected-contribution");
    denied(
        fixture.run(&[
            "catalog",
            "contribute",
            "--store",
            text(&producer),
            "--builder",
            &builder,
            "--foundation",
            &runtime,
            "--policy",
            text(&policy),
            "--source-revision",
            "1111111111111111111111111111111111111111",
            "--input-material",
            text(&rejected_material),
            "--expected-input-material-sha256",
            &rejected_hash,
            "--output-dir",
            text(&rejected_directory),
        ]),
        "differ from preflight",
    );
    assert!(!rejected_directory.exists());

    fixture.json(&["store", "init", "--store", text(&receiver)]);
    for object in [sqlite_object, jq_object] {
        let bundle = fixture.path("closure.tar");
        let exported = fixture.json(&[
            "store",
            "export",
            "--store",
            text(&producer),
            "--object",
            object,
            "--output",
            text(&bundle),
        ]);
        let hash = exported["sha256"].as_str().unwrap();
        fixture.json(&[
            "store",
            "import",
            "--store",
            text(&receiver),
            "--bundle",
            text(&bundle),
            "--expected-sha256",
            hash,
        ]);
        fs::remove_file(bundle).unwrap();
    }
    remove_owned(&producer);
    let sql = "CREATE TABLE work(name TEXT, count INT); INSERT INTO work VALUES('alpha',2),('beta',1),('gamma',3); CREATE INDEX by_count ON work(count); SELECT json_group_array(json_object('name',name,'count',count)) FROM (SELECT * FROM work ORDER BY name);";
    let rows = success(fixture.run(&[
        "run",
        "--store",
        text(&receiver),
        "--object",
        sqlite_object,
        "--program",
        "bin/sqlite3",
        "--",
        ":memory:",
        sql,
    ]));
    let rows = String::from_utf8(rows.stdout).unwrap();
    let transformed = success(fixture.run(&[
        "run",
        "--store",
        text(&receiver),
        "--object",
        jq_object,
        "--program",
        "bin/jq",
        "--",
        "-cn",
        "--argjson",
        "rows",
        &rows,
        "$rows | map(select(.count >= 2 and (.name | test(\"^a|^g\")))) | map(.name)",
    ]));
    assert_eq!(transformed.stdout, b"[\"alpha\",\"gamma\"]\n");
    fixture.json(&[
        "profile",
        "switch",
        "--store",
        text(&receiver),
        "--name",
        "database",
        "--object",
        sqlite_object,
        "--program",
        "bin/sqlite3",
        "--",
        ":memory:",
    ]);
    let selected = success(fixture.run(&[
        "profile",
        "run",
        "--store",
        text(&receiver),
        "--name",
        "database",
        "--",
        "SELECT sqlite_version();",
    ]));
    assert_eq!(selected.stdout, b"3.53.4\n");
}
