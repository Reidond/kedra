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
const BUILDER: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const RUNTIME: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

struct Fixture {
    root: PathBuf,
    binary: PathBuf,
}

impl Fixture {
    fn language(&self, source: &str) -> Vec<String> {
        fs::write(self.path("catalog.kedra"), source).unwrap();
        self.write("pins.json", &json!({"schema_version":1,"sources":{}}));
        self.write("target-policy.json", &json!({"schema_version":1,"namespace":"example","targets":["demo"],"repositories":["fedora","updates"],"required_packages":["coreutils"]}));
        vec![
            "--entry".into(),
            "catalog.kedra".into(),
            "--input-root".into(),
            text(&self.root).into(),
            "--target".into(),
            "demo".into(),
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

    fn language_consumer(&self, builder: &str, runtime: &str) -> Value {
        let directory = self.path("independent-language-consumer");
        fs::create_dir(&directory).unwrap();
        let catalog = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("sysroot-catalog");
        fs::write(directory.join("Cargo.toml"),format!("[package]\nname=\"language-consumer\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\n[[bin]]\nname=\"language-consumer\"\npath=\"main.rs\"\n[dependencies]\nsysroot-catalog={{path={}}}\nserde_json=\"1.0\"\n[profile.dev]\ndebug=false\n",serde_json::to_string(text(&catalog)).unwrap())).unwrap();
        fs::write(
            directory.join("main.rs"),
            include_str!("fixtures/reuse_language.rs"),
        )
        .unwrap();
        let target = self.path("independent-consumer-target");
        success(
            Command::new("cargo")
                .args(["build", "--offline", "--manifest-path"])
                .arg(directory.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&target)
                .output()
                .unwrap(),
        );
        serde_json::from_slice(
            &success(
                Command::new(target.join("debug/language-consumer"))
                    .args([builder, runtime])
                    .output()
                    .unwrap(),
            )
            .stdout,
        )
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
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_images":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
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
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_images":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
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
        let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":names.into_iter().take(256).collect::<Vec<_>>(),"builder_images":[BUILDER],"runtime_images":[RUNTIME],"source_objects":[]}));
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
#[ignore = "requires explicitly retained native ARM compiler and runtime images"]
fn language_real_inline_build_transfer_and_rebuild() {
    let fixture = Fixture::new();
    let builder = std::env::var("KEDRA_CATALOG_BUILDER").expect("set exact retained builder");
    let runtime = std::env::var("KEDRA_CATALOG_RUNTIME").expect("set exact retained runtime");
    let args = fixture.language(LANGUAGE_SAMPLE);
    let consumer = fixture.language_consumer(&builder, &runtime);
    let parsed: Value =
        serde_json::from_slice(&success(fixture.language_run("check", &args, &[])).stdout).unwrap();
    assert_eq!(consumer["intent"], parsed["intent"]);
    let policy = fixture.write("engine-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_images":[builder],"runtime_images":[runtime],"source_objects":[]}));
    let store = fixture.path("language-store");
    success(fixture.run(&["store", "init", "--store", text(&store)]));
    for image in [&builder, &runtime] {
        success(fixture.run(&[
            "store",
            "add-image",
            "--store",
            text(&store),
            "--image",
            image,
        ]));
    }
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
    let api_catalog = fixture.write("api-catalog.json", &consumer["catalog"]);
    let mut api_policy: Value = serde_json::from_slice(&fs::read(&policy).unwrap()).unwrap();
    api_policy["source_objects"] = json!(
        consumer["catalog"]["packages"]["hello"]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source["object"].clone())
            .collect::<Vec<_>>()
    );
    let api_policy = fixture.write("api-policy.json", &api_policy);
    let api_build = fixture.json(&[
        "catalog",
        "build",
        "--catalog",
        text(&api_catalog),
        "--package",
        "hello",
        "--policy",
        text(&api_policy),
        "--store",
        text(&store),
    ]);
    assert_eq!(first["result"]["outputs"], api_build["result"]["outputs"]);
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
        let patch_builder = std::env::var("KEDRA_CATALOG_PATCH_BUILDER").unwrap();
        success(fixture.run(&[
            "store",
            "add-image",
            "--store",
            text(&store),
            "--image",
            &patch_builder,
        ]));
        let patch_policy = fixture.write("patch-policy.json", &json!({"namespace":"example","packages":["hello"],"builder_images":[patch_builder],"runtime_images":[runtime],"source_objects":[]}));
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
