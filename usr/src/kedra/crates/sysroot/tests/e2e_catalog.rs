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

static NEXT: AtomicU64 = AtomicU64::new(0);
const LANGUAGE_SAMPLE: &str = r##"language 1;
namespace "example";
foundation system { fedora = 44; packages = ["coreutils"]; }
builder compiler { base = system; packages = ["gcc"]; }
package hello(builder: Builder, runtime: Foundation) {
    version = "1"; summary = "Inline program"; license = "MIT";
    source = files {
        "hello.c" = text "#include <stdio.h>\nint main(void) { puts(\"original\"); return 0; }\n";
    };
    build = shell """
        mkdir -p "$out/bin"
        /usr/bin/gcc -O2 -ffile-prefix-map="$src"=. "$src/hello.c" -o "$out/bin/hello"
        """;
    export command "hello" = "bin/hello";
}
target "demo" { foundation = system; packages = [hello(builder: compiler, runtime: system)]; }
"##;
const BUILDER: &str = "root-1111111111111111111111111111111111111111111111111111111111111111";
const RUNTIME: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    root: PathBuf,
    binary: PathBuf,
}

impl Fixture {
    fn language(&self, source: &str) -> Vec<String> {
        self.language_for(source, "example", "demo")
    }

    fn language_for(&self, source: &str, namespace: &str, target: &str) -> Vec<String> {
        fs::write(self.path("catalog.kedra"), source).unwrap();
        self.write("pins.json", &json!({"schema_version":1,"sources":{}}));
        self.write("target-policy.json", &json!({"schema_version":1,"namespace":namespace,"targets":[target],"repositories":["fedora","updates"],"required_packages":["coreutils"]}));
        vec![
            "--entry".into(),
            "catalog.kedra".into(),
            "--input-root".into(),
            text(&self.root).into(),
            "--target".into(),
            target.into(),
            "--lock".into(),
            "pins.json".into(),
            "--target-policy".into(),
            text(&self.path("target-policy.json")).into(),
        ]
    }

    fn language_run(&self, operation: &str, language: &[String], extra: &[&str]) -> Output {
        Command::new(&self.binary)
            .args(["catalog", operation])
            .args(language)
            .args(extra)
            .output()
            .unwrap()
    }

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

    /// The root of a retained image, created by the public CLI.
    fn root(&self, store: &Path, image: &str) -> String {
        self.json(&[
            "store",
            "add-root",
            "--store",
            text(store),
            "--image",
            image,
        ])["root"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn write(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }
}

#[test]
fn language_edit_format_plan_and_refusal() {
    let fixture = Fixture::new();
    let args = fixture.language(LANGUAGE_SAMPLE);
    let before: Value =
        serde_json::from_slice(&success(fixture.language_run("check", &args, &[])).stdout).unwrap();
    success(fixture.run(&["catalog", "fmt", text(&fixture.path("catalog.kedra"))]));
    success(fixture.run(&[
        "catalog",
        "fmt",
        "--check",
        text(&fixture.path("catalog.kedra")),
    ]));
    let after: Value =
        serde_json::from_slice(&success(fixture.language_run("check", &args, &[])).stdout).unwrap();
    assert_eq!(before["intent"], after["intent"]);
    assert_ne!(before["inputs"], after["inputs"]);
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_roots":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
    let selection = [
        "--package",
        "hello",
        "--builder",
        BUILDER,
        "--runtime",
        RUNTIME,
        "--policy",
        text(&policy),
    ];
    let first: Value =
        serde_json::from_slice(&success(fixture.language_run("plan", &args, &selection)).stdout)
            .unwrap();
    let changed = LANGUAGE_SAMPLE.replace(
        "mkdir -p",
        &format!("{}mkdir -p", "# long script data\n        ".repeat(2500)),
    );
    fixture.language(&changed);
    let second: Value =
        serde_json::from_slice(&success(fixture.language_run("plan", &args, &selection)).stdout)
            .unwrap();
    assert_ne!(first, second);
    assert_eq!(second["nodes"]["package_0"]["argv"][1][0]["value"], "-eu");
    assert_eq!(
        second["nodes"]["package_0"]["argv"][2][0]["path"],
        "build.sh"
    );
    assert!(!fixture.path("sentinel").exists());
    for (source, reason) in [
        (
            LANGUAGE_SAMPLE.replace("language 1", "language 2"),
            "version",
        ),
        (
            LANGUAGE_SAMPLE.replace("fedora = 44", "fedora = \"44\""),
            "type",
        ),
        (
            LANGUAGE_SAMPLE.replace("namespace \"example\"", "namespace \"denied\""),
            "namespace",
        ),
        (
            LANGUAGE_SAMPLE.replace(
                "version = \"1\";",
                "version = \"1\"; unknown = \"secret not echoed\";",
            ),
            "field",
        ),
        (LANGUAGE_SAMPLE.replace("hello.c", "../outside"), "path"),
        (LANGUAGE_SAMPLE.replace("\n", "\r\n"), "input"),
    ] {
        fixture.language(&source);
        let refused = fixture.language_run("check", &args, &[]);
        assert!(!refused.status.success());
        assert!(refused.stdout.is_empty());
        let message = String::from_utf8_lossy(&refused.stderr);
        assert!(message.contains(reason), "{message}");
        assert!(!message.contains("secret not echoed"));
    }
}

#[test]
fn language_import_admission_and_no_effects() {
    let fixture = Fixture::new();
    let source = LANGUAGE_SAMPLE.replace("mkdir -p", "touch sentinel\n        mkdir -p");
    let args = fixture.language(&source);
    success(fixture.language_run("check", &args, &[]));
    assert!(!fixture.path("sentinel").exists());
    fixture.language(&LANGUAGE_SAMPLE.replace(
        "namespace \"example\";",
        "namespace \"example\"; import { other } from \"./other.kedra\";",
    ));
    fs::write(
        fixture.path("other.kedra"),
        "language 1; import { hello } from \"./catalog.kedra\"; set other {}",
    )
    .unwrap();
    let cycle = fixture.language_run("check", &args, &[]);
    assert!(!cycle.status.success());
    assert!(String::from_utf8_lossy(&cycle.stderr).contains("cycle"));
    fs::remove_file(fixture.path("other.kedra")).unwrap();
    std::os::unix::fs::symlink(fixture.path("catalog.kedra"), fixture.path("other.kedra")).unwrap();
    assert!(!fixture.language_run("check", &args, &[]).status.success());
}

#[test]
fn language_packet_publication_is_complete_and_preserves_existing_winner() {
    let fixture = Fixture::new();
    let args = fixture.language(LANGUAGE_SAMPLE);
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_roots":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
    let packet = fixture.path("plan.tar");
    let selection = [
        "--package",
        "hello",
        "--builder",
        BUILDER,
        "--runtime",
        RUNTIME,
        "--policy",
        text(&policy),
        "--output",
        text(&packet),
    ];
    let result: Value =
        serde_json::from_slice(&success(fixture.language_run("plan", &args, &selection)).stdout)
            .unwrap();
    let bytes = fs::read(&packet).unwrap();
    assert_eq!(result["sha256"], hash(&bytes));
    let mut archive = tar::Archive::new(std::io::Cursor::new(&bytes));
    let mut graph = None;
    let mut frontend = None;
    let mut script = false;
    for member in archive.entries().unwrap() {
        let mut member = member.unwrap();
        let name = member.path().unwrap().to_string_lossy().into_owned();
        if name == "graph.json" {
            graph = Some(serde_json::from_reader::<_, Value>(&mut member).unwrap());
        } else if name == "frontend.json" {
            frontend = Some(serde_json::from_reader::<_, Value>(&mut member).unwrap());
        } else if name.ends_with("/build.sh") {
            script = true;
        }
    }
    assert!(!graph.unwrap()["nodes"].as_object().unwrap().is_empty());
    assert_eq!(frontend.unwrap()["frontend"]["frontend_version"], 1);
    assert!(script);
    let refused = fixture.language_run("plan", &args, &selection);
    assert!(!refused.status.success());
    assert_eq!(bytes, fs::read(&packet).unwrap());
    let recovered = fixture.json(&["system", "recover", "--workdir", text(&fixture.root)]);
    assert!(recovered["refused"].as_array().unwrap().is_empty());
}

#[test]
fn language_selection_replacement_and_bounds() {
    let fixture = Fixture::new();
    let base = LANGUAGE_SAMPLE.replace(
        "packages = [\"coreutils\"]",
        "packages = [\"coreutils\",\"unzip\"]",
    );
    let replaced = base.replace("foundation = system; packages = [hello", "foundation = system; replace = [replacement { origin = \"catalog.kedra#system\"; name = \"unzip\"; action = \"remove\"; }]; packages = [hello");
    let args = fixture.language(&replaced);
    let result: Value =
        serde_json::from_slice(&success(fixture.language_run("check", &args, &[])).stdout).unwrap();
    assert_eq!(result["intent"]["remove"], json!(["unzip"]));
    for (source, reason) in [
        (
            replaced.replace("catalog.kedra#system", "absent.kedra#system"),
            "replacement",
        ),
        (
            replaced.replace("name = \"unzip\"", "name = \"coreutils\""),
            "selection",
        ),
        (
            base.replace(
                "foundation = system; packages = [hello",
                "foundation = system; remove = [\"unzip\"]; packages = [hello",
            ),
            "selection",
        ),
        (
            format!("{}\nset cycle {{ use = [cycle]; }}", LANGUAGE_SAMPLE),
            "cycle",
        ),
        (
            LANGUAGE_SAMPLE.replace("build = shell", "build = eval"),
            "syntax",
        ),
        (
            LANGUAGE_SAMPLE.replace(
                "source = files",
                &format!("source = {}files", "executable(".repeat(70)),
            ),
            "limit",
        ),
    ] {
        fixture.language(&source);
        let refused = fixture.language_run("check", &args, &[]);
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains(reason),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
    }
    let body = "x".repeat(8 * 1024 * 1024);
    fixture.language(&LANGUAGE_SAMPLE.replace(
        "summary = \"Inline program\"",
        &format!("summary = \"{body}\""),
    ));
    let refused = fixture.language_run("check", &args, &[]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("ordinary file"));
}

#[test]
fn language_admission_boundaries_preserve_accepted_input() {
    let fixture = Fixture::new();
    let args = fixture.language(LANGUAGE_SAMPLE);
    let accepted = fs::read(fixture.path("catalog.kedra")).unwrap();
    let imports = (0..127)
        .map(|i| format!("import {{ item{i} }} from \"./module{i}.kedra\";\n"))
        .collect::<String>();
    for i in 0..128 {
        fs::write(
            fixture.path(&format!("module{i}.kedra")),
            format!("language 1; set item{i} {{}}\n"),
        )
        .unwrap();
    }
    fixture.language(&LANGUAGE_SAMPLE.replace(
        "namespace \"example\";",
        &format!("namespace \"example\";\n{imports}"),
    ));
    success(fixture.language_run("check", &args, &[]));
    let source = fs::read_to_string(fixture.path("catalog.kedra")).unwrap();
    fixture.language(&format!(
        "{source}\nimport {{ item127 }} from \"./module127.kedra\";"
    ));
    denied(
        fixture.language_run("check", &args, &[]),
        "module/deadline limit",
    );
    let files = (0..4095)
        .map(|i| format!("\"file{i}\" = text \"\";\n"))
        .collect::<String>();
    let source =
        LANGUAGE_SAMPLE.replace("source = files {", &format!("source = files {{\n{files}"));
    // hello.c plus the build script are resources too: 4094 extra files is the
    // exact declared-resource boundary, independently of the per-map bound.
    let boundary = source.replace("\"file4094\" = text \"\";\n", "");
    fixture.language(&boundary);
    success(fixture.language_run("check", &args, &[]));
    fixture.language(&source);
    denied(fixture.language_run("check", &args, &[]), "resource count");
    let mut boundary = LANGUAGE_SAMPLE.to_owned();
    boundary.push_str("\n//");
    boundary.push_str(&"x".repeat(8 * 1024 * 1024 - boundary.len()));
    fixture.language(&boundary);
    success(fixture.language_run("check", &args, &[]));
    boundary.push('x');
    fixture.language(&boundary);
    denied(fixture.language_run("check", &args, &[]), "ordinary file");
    fs::write(fixture.path("catalog.kedra"), &accepted).unwrap();
    success(fixture.language_run("check", &args, &[]));
    assert_eq!(fs::read(fixture.path("catalog.kedra")).unwrap(), accepted);
}

#[test]
fn language_expansion_counts_preparation_nodes_in_the_graph_bound() {
    let fixture = Fixture::new();
    let prefix = LANGUAGE_SAMPLE.split_once("package hello").unwrap().0;
    let body = LANGUAGE_SAMPLE
        .split_once("package hello")
        .unwrap()
        .1
        .split_once("target \"demo\"")
        .unwrap()
        .0;
    for (count, prepare, accepted, reason) in [
        (256, false, true, ""),
        (257, false, false, "package expansion"),
        (128, true, true, ""),
        (129, true, false, "lowered graph"),
    ] {
        let mut source = prefix.to_owned();
        let mut selected = Vec::new();
        let mut names = Vec::new();
        for i in 0..count {
            let name = format!("program{i}");
            let body = body.replace(
                "export command \"hello\"",
                &format!("export command \"command{i}\""),
            );
            let body = if prepare {
                body.replace(
                    "build = shell",
                    "files = files { \"header.h\" = text \"/* generated */\\n\"; }; build = shell",
                )
            } else {
                body
            };
            source.push_str(&format!("package {name}{body}"));
            selected.push(format!("{name}(builder: compiler, runtime: system)"));
            names.push(name);
        }
        source.push_str(&format!(
            "target \"demo\" {{ foundation = system; packages = [{}]; }}",
            selected.join(",")
        ));
        let args = fixture.language(&source);
        let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":names.into_iter().take(256).collect::<Vec<_>>(),"builder_roots":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
        let output = fixture.language_run(
            "plan",
            &args,
            &[
                "--package",
                "program0",
                "--builder",
                BUILDER,
                "--runtime",
                RUNTIME,
                "--policy",
                text(&policy),
            ],
        );
        if accepted {
            let graph: Value = serde_json::from_slice(&success(output).stdout).unwrap();
            assert_eq!(
                graph["nodes"].as_object().unwrap().len(),
                if prepare { 2 } else { 1 }
            );
        } else {
            denied(output, reason);
        }
    }
}

#[test]
fn formatter_admits_the_complete_bounded_selection_before_writing() {
    let fixture = Fixture::new();
    fixture.language(LANGUAGE_SAMPLE);
    let original = fs::read(fixture.path("catalog.kedra")).unwrap();
    fs::write(fixture.path("invalid.kedra"), "language 2;").unwrap();
    denied(
        fixture.run(&[
            "catalog",
            "fmt",
            text(&fixture.path("catalog.kedra")),
            text(&fixture.path("invalid.kedra")),
        ]),
        "version",
    );
    assert_eq!(fs::read(fixture.path("catalog.kedra")).unwrap(), original);
    let large = format!("{LANGUAGE_SAMPLE}\n//{}", "x".repeat(3 * 1024 * 1024));
    let mut paths = Vec::new();
    for i in 0..11 {
        let path = fixture.path(&format!("large{i}.kedra"));
        fs::write(&path, &large).unwrap();
        paths.push(path);
    }
    let output = Command::new(&fixture.binary)
        .args(["catalog", "fmt"])
        .args(&paths)
        .output()
        .unwrap();
    denied(output, "aggregate input");
    assert!(
        paths
            .iter()
            .all(|path| fs::read(path).unwrap() == large.as_bytes())
    );
}

#[test]
#[ignore = "requires explicitly retained native ARM compiler and runtime images and bubblewrap"]
fn language_real_inline_build_transfer_and_rebuild() {
    let fixture = Fixture::new();
    let builder_image = std::env::var("KEDRA_CATALOG_BUILDER").expect("set exact retained builder");
    let runtime = std::env::var("KEDRA_CATALOG_RUNTIME").expect("set exact retained runtime");
    let args = fixture.language(LANGUAGE_SAMPLE);
    let store = fixture.path("language-store");
    success(fixture.run(&["store", "init", "--store", text(&store)]));
    for image in [&builder_image, &runtime] {
        success(fixture.run(&[
            "store",
            "add-image",
            "--store",
            text(&store),
            "--image",
            image,
        ]));
    }
    let builder = fixture.root(&store, &builder_image);
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_roots":[builder],"runtime_images":[runtime],"source_objects":[]}));
    let build = [
        "--package",
        "hello",
        "--builder",
        &builder,
        "--runtime",
        &runtime,
        "--policy",
        text(&policy),
        "--store",
        text(&store),
    ];
    let first: Value =
        serde_json::from_slice(&success(fixture.language_run("build", &args, &build)).stdout)
            .unwrap();
    let root = first["result"]["outputs"]["package_0"].as_str().unwrap();
    let output = success(fixture.run(&[
        "run",
        "--store",
        text(&store),
        "--object",
        root,
        "--program",
        "bin/hello",
    ]));
    assert_eq!(output.stdout, b"original\n");
    let mut rebuild = build.to_vec();
    rebuild.push("--rebuild");
    let second: Value =
        serde_json::from_slice(&success(fixture.language_run("build", &args, &rebuild)).stdout)
            .unwrap();
    assert_eq!(first["result"]["outputs"], second["result"]["outputs"]);
    assert!(
        !second["result"]["reproduced"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let bundle = fixture.path("language-bundle.tar");
    success(fixture.run(&[
        "store",
        "export",
        "--store",
        text(&store),
        "--object",
        root,
        "--output",
        text(&bundle),
    ]));
    let receiver = fixture.path("language-receiver");
    success(fixture.run(&["store", "init", "--store", text(&receiver)]));
    success(fixture.run(&[
        "store",
        "import",
        "--store",
        text(&receiver),
        "--bundle",
        text(&bundle),
        "--expected-sha256",
        &hash(&fs::read(&bundle).unwrap()),
    ]));
    let transferred = success(fixture.run(&[
        "run",
        "--store",
        text(&receiver),
        "--object",
        root,
        "--program",
        "bin/hello",
    ]));
    assert_eq!(output.stdout, transferred.stdout);
    let overlay = LANGUAGE_SAMPLE.replace("    export command", r##"    files = files { "config.h" = text "#define MESSAGE \"generated\"\n"; };
    replace_files = files { "hello.c" = text "#include <stdio.h>\n#include \"config.h\"\nint main(void) { puts(MESSAGE); return 0; }\n"; };
    export command"##);
    fixture.language(&overlay);
    let prepared: Value =
        serde_json::from_slice(&success(fixture.language_run("build", &args, &build)).stdout)
            .unwrap();
    let prepared_root = prepared["result"]["outputs"]["package_0"].as_str().unwrap();
    assert_eq!(
        success(fixture.run(&[
            "run",
            "--store",
            text(&store),
            "--object",
            prepared_root,
            "--program",
            "bin/hello"
        ]))
        .stdout,
        b"generated\n"
    );
    let implicit = overlay.replace("replace_files = files", "files = files").replace("    files = files { \"config.h\" = text \"#define MESSAGE \\\"generated\\\"\\n\"; };\n", "");
    fixture.language(&implicit);
    let refused = fixture.language_run("build", &args, &build);
    assert!(!refused.status.success());
    assert_eq!(
        success(fixture.run(&[
            "run",
            "--store",
            text(&store),
            "--object",
            prepared_root,
            "--program",
            "bin/hello"
        ]))
        .stdout,
        b"generated\n"
    );
    let patched = LANGUAGE_SAMPLE.replace("    export command", r##"    patches = [patch { strip = 1; contents = text "--- a/hello.c\n+++ b/hello.c\n@@ -1,2 +1,2 @@\n #include <stdio.h>\n-int main(void) { puts(\"original\"); return 0; }\n+int main(void) { puts(\"patched\"); return 0; }\n"; }];
    export command"##).replace("hello.c", "fuzz-offset.c");
    fixture.language(&patched);
    if std::env::var_os("KEDRA_CATALOG_PATCH_BUILDER").is_some() {
        let patch_image = std::env::var("KEDRA_CATALOG_PATCH_BUILDER").unwrap();
        success(fixture.run(&[
            "store",
            "add-image",
            "--store",
            text(&store),
            "--image",
            &patch_image,
        ]));
        let patch_builder = fixture.root(&store, &patch_image);
        let patch_policy = fixture.write("patch-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_roots":[patch_builder],"runtime_images":[runtime],"source_objects":[]}));
        let patch_build: Value = serde_json::from_slice(
            &success(fixture.language_run(
                "build",
                &args,
                &[
                    "--package",
                    "hello",
                    "--builder",
                    &patch_builder,
                    "--runtime",
                    &runtime,
                    "--policy",
                    text(&patch_policy),
                    "--store",
                    text(&store),
                ],
            ))
            .stdout,
        )
        .unwrap();
        let root = patch_build["result"]["outputs"]["package_0"]
            .as_str()
            .unwrap();
        assert_eq!(
            success(fixture.run(&[
                "run",
                "--store",
                text(&store),
                "--object",
                root,
                "--program",
                "bin/hello"
            ]))
            .stdout,
            b"patched\n"
        );
        fixture.language(&patched.replace("@@ -1,2 +1,2 @@", "@@ -2,2 +2,2 @@"));
        let refused = fixture.language_run(
            "build",
            &args,
            &[
                "--package",
                "hello",
                "--builder",
                &patch_builder,
                "--runtime",
                &runtime,
                "--policy",
                text(&patch_policy),
                "--store",
                text(&store),
            ],
        );
        assert!(!refused.status.success());
        assert_eq!(
            success(fixture.run(&[
                "run",
                "--store",
                text(&store),
                "--object",
                root,
                "--program",
                "bin/hello"
            ]))
            .stdout,
            b"patched\n"
        );
    } else {
        let refused = fixture.language_run("build", &args, &build);
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("compiler Fedora requests"));
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
    let args = fixture.language(LANGUAGE_SAMPLE);
    let policy = json!({"namespace":"example","packages":["hello"],"builder_roots":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]});
    let path = fixture.write("engine-policy.json", &policy);
    let selection = |policy: &Path| {
        vec![
            "--package".to_owned(),
            "hello".into(),
            "--builder".into(),
            BUILDER.into(),
            "--runtime".into(),
            RUNTIME.into(),
            "--policy".into(),
            text(policy).into(),
        ]
    };
    let resolve = |policy: &Path| -> Value {
        let extra = selection(policy);
        let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
        serde_json::from_slice(&success(fixture.language_run("resolve", &args, &extra)).stdout)
            .unwrap()
    };
    let first = resolve(&path);
    assert_eq!(first, resolve(&path));
    assert_eq!(first["recipe"]["program"], "bin/hello");
    let extra = selection(&path);
    let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
    let graph: Value =
        serde_json::from_slice(&success(fixture.language_run("plan", &args, &extra)).stdout)
            .unwrap();
    assert_eq!(graph, first["recipe"]["graph"]);
    let absent_store = fixture.path("must-not-exist");
    for (field, replacement, expected) in [
        ("namespace", json!("fieldkit"), "denied frontend namespace"),
        (
            "packages",
            json!(["other"]),
            "catalog policy denied: package",
        ),
        ("builder_roots", json!([]), "denied/incompatible image role"),
        ("runtime_images", json!([]), "denied frontend namespace"),
    ] {
        let mut value = policy.clone();
        value[field] = replacement;
        let denied_policy = fixture.write("denied.json", &value);
        let mut extra = selection(&denied_policy);
        extra.extend(["--store".to_owned(), text(&absent_store).into()]);
        let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
        denied(fixture.language_run("build", &args, &extra), expected);
        assert!(!absent_store.exists());
    }
}

/// A selected `kedra` package for the x86_64 `desktop` target resolves, and its
/// build takes the same path as any other target's: into the store, where this
/// empty one lacks the foundation image. Realizing it needs a native x86_64 run.
#[test]
fn kedra_desktop_selection_resolves_and_builds_like_any_target() {
    let fixture = Fixture::new();
    let store = fixture.path("store");
    success(fixture.run(&["store", "init", "--store", text(&store)]));
    let run = |operation: &str, namespace: &str, target: &str| {
        let source = LANGUAGE_SAMPLE
            .replace(
                "namespace \"example\"",
                &format!("namespace \"{namespace}\""),
            )
            .replace("target \"demo\"", &format!("target \"{target}\""));
        let args = fixture.language_for(&source, namespace, target);
        let policy = fixture.write(
            "engine-policy.json",
            &json!({"namespace":namespace,"packages":["hello"],"builder_roots":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}),
        );
        let mut extra = vec![
            "--package",
            "hello",
            "--builder",
            BUILDER,
            "--runtime",
            RUNTIME,
            "--policy",
            text(&policy),
        ];
        if operation == "build" {
            extra.extend(["--store", text(&store)]);
        }
        fixture.language_run(operation, &args, &extra)
    };
    let resolved: Value =
        serde_json::from_slice(&success(run("resolve", "kedra", "desktop")).stdout).unwrap();
    assert_eq!(resolved["namespace"], "kedra");
    assert_eq!(resolved["recipe"]["program"], "bin/hello");
    let desktop = run("build", "kedra", "desktop");
    denied(desktop.clone(), "No such file or directory");
    assert_eq!(desktop.stderr, run("build", "example", "demo").stderr);
}
