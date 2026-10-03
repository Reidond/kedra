//! External projects exercise the public catalog/store lifecycle through real processes.
//! Registered as a child of e2e_engine so process bounds and cleanup stay shared.
use super::{BUILDER, Capture, Fixture, RUNTIME, run, sha256, string, text};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn consumer(fixture: &Fixture, project: &str, program: &str) -> PathBuf {
    let directory = fixture.path(&format!("{project}-consumer"));
    fs::create_dir(&directory).unwrap();
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::write(
        directory.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{project}-consumer"
version = "0.0.0"
edition = "2024"
[workspace]
[[bin]]
name = "{project}"
path = "main.rs"
[dependencies]
sysroot-engine = {{ path = {} }}
sysroot-catalog = {{ path = {} }}
serde = "1.0"
serde_json = "1.0"
rustix = {{ version = "1.1.4", features = ["fs"] }}
[profile.dev]
debug = false
"#,
            serde_json::to_string(text(&crates.join("sysroot-engine"))).unwrap(),
            serde_json::to_string(text(&crates.join("sysroot-catalog"))).unwrap(),
        ),
    )
    .unwrap();
    fs::write(
        directory.join("main.rs"),
        format!(
            "{}\nconst PROJECT: &str = {project:?};\nconst PROGRAM: &str = {program:?};\n",
            include_str!("fixtures/reuse_consumer.rs"),
        ),
    )
    .unwrap();
    fs::write(
        directory.join("definitions.rs"),
        include_str!("fixtures/reuse_definitions.rs"),
    )
    .unwrap();
    let target = fixture.path("consumer-target");
    run(Command::new("cargo")
        .args(["build", "--offline", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target))
    .success();
    target.join("debug").join(project)
}

fn invoke(binary: &Path, args: &[&str]) -> Capture {
    run(Command::new(binary).args(args))
}

fn source(
    fixture: &Fixture,
    project: &str,
    version: &str,
    multiplier: i32,
    offset: i32,
    readings: &str,
) -> PathBuf {
    let directory = fixture.path(&format!("{project}-{version}-source"));
    fs::create_dir(&directory).unwrap();
    fs::write(
        directory.join("support.c"),
        format!("long score(long reading) {{ return reading * {multiplier} + ({offset}); }}\n"),
    )
    .unwrap();
    fs::write(directory.join("readings"), readings).unwrap();
    fs::write(
        directory.join("report.c"),
        format!(
            r#"#include <stdio.h>
extern long score(long reading);
int main(void) {{
    FILE *input = fopen(DATA_FILE, "r");
    if (!input) return 3;
    long reading = 0, total = 0;
    int scanned = 0;
    while ((scanned = fscanf(input, "%ld", &reading)) == 1) total += score(reading);
    if (scanned != EOF || ferror(input)) {{ fclose(input); return 4; }}
    if (fclose(input)) return 5;
    printf("{project}-{version}: total=%ld\n", total);
    return 0;
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        directory.join("support.sh"),
        r#"set -eu
mkdir -p "$2/lib" "$2/share"
cc -fPIC -shared -Wl,-soname,libreadings.so -o "$2/lib/libreadings.so" "$1/support.c"
cp "$1/readings" "$2/share/readings"
"#,
    )
    .unwrap();
    fs::write(
        directory.join("report.sh"),
        r#"set -eu
mkdir -p "$3/bin"
cc -DDATA_FILE="\"$2/share/readings\"" -o "$3/$4" "$1" -L"$2/lib" -lreadings -Wl,-rpath,"$2/lib"
"#,
    )
    .unwrap();
    directory
}

struct Declaration {
    catalog: PathBuf,
    policy: PathBuf,
    document: Value,
}

fn declare(
    fixture: &Fixture,
    binary: &Path,
    source: &str,
    version: &str,
    label: &str,
) -> Declaration {
    let document = invoke(binary, &["declare", source, BUILDER, RUNTIME, version]).json();
    Declaration {
        catalog: fixture.write_json(&format!("{label}-catalog.json"), &document["catalog"]),
        policy: fixture.write_json(&format!("{label}-policy.json"), &document["policy"]),
        document,
    }
}

fn build(binary: &Path, store: &Path, declaration: &Declaration) -> Value {
    invoke(
        binary,
        &[
            "build",
            text(store),
            text(&declaration.catalog),
            text(&declaration.policy),
            "report",
        ],
    )
    .json()
}

fn report_id(result: &Value) -> String {
    result["outputs"]["report"].as_str().unwrap().to_owned()
}

fn expect_output(binary: &Path, args: &[&str], expected: &str) {
    let result = invoke(binary, args).success();
    assert_eq!(result.stdout, expected.as_bytes());
    assert!(result.stderr.is_empty());
}

fn policy_refusals(
    fixture: &Fixture,
    binary: &Path,
    declaration: &Declaration,
    other: &Declaration,
) {
    let mut variants = vec![("namespace", other.document["policy"].clone())];
    for (field, replacement) in [
        ("packages", json!(["other-report"])),
        ("builder_images", json!([RUNTIME])),
        ("runtime_images", json!([BUILDER])),
        (
            "source_objects",
            other.document["policy"]["source_objects"].clone(),
        ),
    ] {
        let mut policy = declaration.document["policy"].clone();
        policy[field] = replacement;
        variants.push((field, policy));
    }
    for (label, policy) in variants {
        let destination = fixture.path(&format!("refused-{label}"));
        let policy_file = fixture.write_json(&format!("refused-{label}.json"), &policy);
        run(Command::new(binary)
            .args([
                "build",
                text(&destination),
                text(&declaration.catalog),
                text(&policy_file),
                "report",
            ])
            .env("DOCKER_HOST", "unix:///nonexistent/reuse-policy.sock"))
        .refused("catalog policy denied:");
        assert!(
            !destination.exists(),
            "policy {label} mutated the destination"
        );
    }
    let unknown_destination = fixture.path("refused-unknown-package");
    let mut unknown_policy = declaration.document["policy"].clone();
    unknown_policy["packages"] = json!(["missing"]);
    let unknown_policy_file = fixture.write_json("unknown-package-policy.json", &unknown_policy);
    invoke(
        binary,
        &[
            "build",
            text(&unknown_destination),
            text(&declaration.catalog),
            text(&unknown_policy_file),
            "missing",
        ],
    )
    .refused("unknown package missing");
    assert!(!unknown_destination.exists());
}

#[test]
#[ignore = "requires native aarch64 Docker and retained pinned images; run --ignored"]
fn independent_projects_reuse_catalog_and_store_without_kedra_authority() {
    let fixture = Fixture::new();
    let fieldkit = consumer(&fixture, "fieldkit", "bin/field-report");
    let observatory = consumer(&fixture, "observatory", "bin/station-report");
    let field_store = fixture.path("fieldkit-store");
    let station_store = fixture.path("observatory-store");
    let field_one = source(&fixture, "fieldkit", "v1", 2, 1, "5\n7\n8\n");
    let field_two = source(&fixture, "fieldkit", "v2", 3, -1, "5\n7\n8\n");
    let station_source = source(&fixture, "observatory", "v1", 4, 2, "1\n3\n");
    let field_source_id = string(
        &invoke(
            &fieldkit,
            &[
                "admit",
                text(&field_store),
                text(&field_one),
                BUILDER,
                RUNTIME,
            ],
        )
        .json(),
        "object",
    );
    let station_source_id = string(
        &invoke(
            &observatory,
            &[
                "admit",
                text(&station_store),
                text(&station_source),
                BUILDER,
                RUNTIME,
            ],
        )
        .json(),
        "object",
    );
    let one = declare(&fixture, &fieldkit, &field_source_id, "1.0.0", "field-one");
    let station = declare(
        &fixture,
        &observatory,
        &station_source_id,
        "1.0.0",
        "station",
    );
    assert_eq!(one.document["catalog"]["namespace"], "fieldkit");
    assert_eq!(station.document["catalog"]["namespace"], "observatory");
    policy_refusals(&fixture, &fieldkit, &one, &station);
    policy_refusals(&fixture, &observatory, &station, &one);

    let resolved = run(Command::new(&fieldkit)
        .args(["resolve", text(&one.catalog), text(&one.policy), "report"])
        .env("DOCKER_HOST", "unix:///nonexistent/reuse-resolve.sock"))
    .json();
    assert_eq!(resolved["namespace"], "fieldkit");
    assert_eq!(resolved["recipe"]["root"], "report");
    assert_eq!(resolved["recipe"]["program"], "bin/field-report");

    let first = build(&fieldkit, &field_store, &one);
    let first_id = report_id(&first);
    assert_eq!(resolved["plan"]["outputs"]["report"], first_id);
    assert_eq!(first["built"].as_array().unwrap().len(), 2);
    let reused = build(&fieldkit, &field_store, &one);
    assert_eq!(report_id(&reused), first_id);
    assert_eq!(reused["reused"].as_array().unwrap().len(), 2);
    let station_id = report_id(&build(&observatory, &station_store, &station));
    assert_ne!(first_id, station_id);
    expect_output(
        &fieldkit,
        &["run", text(&field_store), &first_id],
        "fieldkit-v1: total=43\n",
    );
    expect_output(
        &observatory,
        &["run", text(&station_store), &station_id],
        "observatory-v1: total=20\n",
    );
    invoke(&observatory, &["switch", text(&station_store), &station_id]).json();
    let station_profile = invoke(&observatory, &["profile", text(&station_store)]).json();
    let profile_index = station_store.join("profiles/development/index.json");
    let station_index = fs::read(&profile_index).unwrap();
    let station_receipt =
        invoke(&observatory, &["verify", text(&station_store), &station_id]).json();
    let sentinel = station_store.join("project-sentinel");
    fs::write(&sentinel, b"observatory-owned\n").unwrap();

    let second_source_id = string(
        &invoke(
            &fieldkit,
            &["admit-source", text(&field_store), text(&field_two)],
        )
        .json(),
        "object",
    );
    let two = declare(&fixture, &fieldkit, &second_source_id, "2.0.0", "field-two");
    let second_id = report_id(&build(&fieldkit, &field_store, &two));
    assert_ne!(first_id, second_id);
    let receiver = fixture.path("fieldkit-receiver");
    for (label, object) in [("one", &first_id), ("two", &second_id)] {
        let closure = invoke(&fieldkit, &["closure", text(&field_store), object]).json();
        assert_eq!(closure["objects"].as_array().unwrap().len(), 2);
        assert_eq!(closure["images"], json!([RUNTIME]));
        assert!(
            !closure["objects"]
                .as_array()
                .unwrap()
                .contains(&json!(field_source_id))
        );
        assert!(
            !closure["objects"]
                .as_array()
                .unwrap()
                .contains(&json!(second_source_id))
        );
        let bundle = fixture.path(&format!("fieldkit-{label}.tar"));
        let exported = invoke(
            &fieldkit,
            &["export", text(&field_store), object, text(&bundle)],
        )
        .json();
        let digest = sha256(&bundle);
        assert_eq!(exported["sha256"], digest);
        let imported = invoke(
            &fieldkit,
            &["import", text(&receiver), text(&bundle), &digest],
        )
        .json();
        assert!(
            imported["roots"]
                .as_array()
                .unwrap()
                .contains(&json!(object))
        );
    }
    fs::rename(&field_store, fixture.path("fieldkit-producer-unavailable")).unwrap();
    fs::remove_dir_all(field_one).unwrap();
    fs::remove_dir_all(field_two).unwrap();
    invoke(&fieldkit, &["switch", text(&receiver), &first_id]).json();
    expect_output(
        &fieldkit,
        &["profile-run", text(&receiver)],
        "fieldkit-v1: total=43\n",
    );
    invoke(&fieldkit, &["switch", text(&receiver), &second_id]).json();
    expect_output(
        &fieldkit,
        &["profile-run", text(&receiver)],
        "fieldkit-v2: total=57\n",
    );
    let rolled_back = invoke(&fieldkit, &["rollback", text(&receiver)]).json();
    assert_eq!(rolled_back["object"], first_id);
    assert_eq!(rolled_back["generations"].as_array().unwrap().len(), 2);
    expect_output(
        &fieldkit,
        &["profile-run", text(&receiver)],
        "fieldkit-v1: total=43\n",
    );

    let orphan_source = fixture.path("fieldkit-orphan");
    fs::create_dir(&orphan_source).unwrap();
    fs::write(orphan_source.join("reading"), b"unselected observation\n").unwrap();
    let orphan = string(
        &invoke(
            &fieldkit,
            &["admit-source", text(&receiver), text(&orphan_source)],
        )
        .json(),
        "object",
    );
    for object in [&orphan, &first_id, &second_id] {
        invoke(&fieldkit, &["unpin", text(&receiver), object]).success();
    }
    invoke(&fieldkit, &["unpin-image", text(&receiver), RUNTIME]).success();
    let collected = invoke(&fieldkit, &["gc", text(&receiver)]).json();
    assert_eq!(collected["objects"], json!([orphan]));
    assert!(!receiver.join("objects").join(&orphan).exists());
    expect_output(
        &fieldkit,
        &["profile-run", text(&receiver)],
        "fieldkit-v1: total=43\n",
    );
    expect_output(
        &fieldkit,
        &["run", text(&receiver), &second_id],
        "fieldkit-v2: total=57\n",
    );
    assert_eq!(
        invoke(&observatory, &["profile", text(&station_store)]).json(),
        station_profile
    );
    assert_eq!(fs::read(&profile_index).unwrap(), station_index);
    assert_eq!(fs::read(&sentinel).unwrap(), b"observatory-owned\n");
    assert_eq!(
        invoke(&observatory, &["verify", text(&station_store), &station_id]).json(),
        station_receipt
    );
    expect_output(
        &observatory,
        &["profile-run", text(&station_store)],
        "observatory-v1: total=20\n",
    );
    println!(
        "external projects passed: independent compilation, catalog policy, native calculations, transfer, profiles, rollback and GC isolation"
    );
}

struct CacheSigner {
    private: PathBuf,
    public: PathBuf,
    fingerprint: String,
}

fn cache_signer(fixture: &Fixture, label: &str) -> CacheSigner {
    let private = fixture.path(&format!("{label}-private.pem"));
    let public = fixture.path(&format!("{label}-public.pem"));
    let der = fixture.path(&format!("{label}-public.der"));
    run(Command::new("openssl")
        .args([
            "genpkey",
            "-algorithm",
            "EC",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-out",
        ])
        .arg(&private))
    .success();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o600)).unwrap();
    run(Command::new("openssl")
        .args(["pkey", "-in"])
        .arg(&private)
        .args(["-pubout", "-out"])
        .arg(&public))
    .success();
    run(Command::new("openssl")
        .args(["pkey", "-pubin", "-in"])
        .arg(&public)
        .args(["-outform", "DER", "-out"])
        .arg(&der))
    .success();
    CacheSigner {
        private,
        public,
        fingerprint: sha256(&der),
    }
}

fn sign_cache(fixture: &Fixture, signer: &CacheSigner, receipt: &Value) -> (PathBuf, PathBuf) {
    let receipt = fixture.write_json("external-cache-receipt.json", receipt);
    let der = fixture.path("external-cache-signature.der");
    let signature = fixture.path("external-cache-signature.txt");
    run(Command::new("openssl")
        .args(["dgst", "-sha256", "-sign"])
        .arg(&signer.private)
        .arg("-out")
        .arg(&der)
        .arg(&receipt))
    .success();
    run(Command::new("openssl")
        .args(["base64", "-A", "-in"])
        .arg(&der)
        .arg("-out")
        .arg(&signature))
    .success();
    (receipt, signature)
}

struct CacheSelection<'a> {
    consumer: &'a Path,
    store: &'a Path,
    catalog: &'a Path,
    catalog_policy: &'a Path,
    package: &'a str,
    cache_policy: &'a Path,
    bundle: &'a Path,
}

fn substitute_cache(
    fixture: &Fixture,
    selection: &CacheSelection<'_>,
    signer: &CacheSigner,
    receipt: &Value,
) -> Capture {
    let (receipt, signature) = sign_cache(fixture, signer, receipt);
    invoke(
        selection.consumer,
        &[
            "substitute",
            text(selection.store),
            text(selection.catalog),
            text(selection.catalog_policy),
            selection.package,
            text(selection.cache_policy),
            text(&receipt),
            text(&signature),
            text(&signer.public),
            text(selection.bundle),
        ],
    )
}

struct CacheStoreState {
    object: String,
    receipt: Value,
    roots: Vec<u8>,
    sentinel: PathBuf,
}

fn cache_receiver(fixture: &Fixture, binary: &Path, store: &Path, label: &str) -> CacheStoreState {
    invoke(binary, &["init", text(store)]).json();
    let source = fixture.path(&format!("{label}-sentinel-source"));
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("reading"),
        b"preexisting immutable receiver data\n",
    )
    .unwrap();
    let object = string(
        &invoke(binary, &["admit-source", text(store), text(&source)]).json(),
        "object",
    );
    let sentinel = store.join("project-sentinel");
    fs::write(&sentinel, b"receiver-owned sentinel\n").unwrap();
    CacheStoreState {
        receipt: invoke(binary, &["verify", text(store), &object]).json(),
        object,
        roots: fs::read(store.join("roots/state.json")).unwrap(),
        sentinel,
    }
}

fn unchanged_cache_store(binary: &Path, store: &Path, state: &CacheStoreState) {
    assert_eq!(
        fs::read(store.join("roots/state.json")).unwrap(),
        state.roots
    );
    assert_eq!(
        fs::read(&state.sentinel).unwrap(),
        b"receiver-owned sentinel\n"
    );
    assert_eq!(
        invoke(binary, &["verify", text(store), &state.object]).json(),
        state.receipt
    );
    let unselected = invoke(binary, &["gc-plan", text(store)]).json();
    assert_eq!(
        unselected["objects"],
        json!([]),
        "refusal admitted an unselected object"
    );
    assert_eq!(
        unselected["images"],
        json!([]),
        "refusal admitted an unselected image"
    );
    assert_eq!(unselected["deleted"], false);
}

#[test]
#[ignore = "requires native ARM Docker, retained images and OpenSSL; runs independently compiled consumers with signed cache policy"]
fn independent_consumers_authenticate_cache_before_store_admission() {
    let fixture = Fixture::new();
    let fieldkit = consumer(&fixture, "fieldkit", "bin/field-report");
    let observatory = consumer(&fixture, "observatory", "bin/station-report");
    let producer = fixture.path("signed-cache-producer");
    let source = source(&fixture, "fieldkit", "v1", 2, 1, "5\n7\n8\n");
    let source_id = string(
        &invoke(
            &fieldkit,
            &["admit", text(&producer), text(&source), BUILDER, RUNTIME],
        )
        .json(),
        "object",
    );
    let declaration = declare(&fixture, &fieldkit, &source_id, "1.0.0", "signed-fieldkit");
    let output = report_id(&build(&fieldkit, &producer, &declaration));
    let root_receipt = invoke(&fieldkit, &["verify", text(&producer), &output]).json();
    let bundle = fixture.path("signed-fieldkit.tar");
    let exported = invoke(
        &fieldkit,
        &["export", text(&producer), &output, text(&bundle)],
    )
    .json();
    let selected = cache_signer(&fixture, "selected-cache-authority");
    let unauthorized = cache_signer(&fixture, "unauthorized-cache-authority");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let policy = json!({"schema":1, "scope":"fieldkit/report", "revision":3,
        "valid_from":now-60, "valid_until":now+7200, "trusted_keys":[selected.fingerprint]});
    let policy_path = fixture.write_json("fieldkit-cache-policy.json", &policy);
    let receipt = json!({"schema":1, "purpose":"sysroot-engine-binary-cache-v1",
        "scope":"fieldkit/report", "policy_revision":3, "valid_from":now-30,
        "valid_until":now+3600, "bundle_sha256":exported["sha256"],
        "bundle_bytes":exported["bytes"], "root":root_receipt});
    let receiver = fixture.path("fieldkit-cache-receiver");
    let station = fixture.path("observatory-cache-receiver");
    let field_before = cache_receiver(&fixture, &fieldkit, &receiver, "fieldkit");
    let station_before = cache_receiver(&fixture, &observatory, &station, "observatory");
    let normal = CacheSelection {
        consumer: &fieldkit,
        store: &receiver,
        catalog: &declaration.catalog,
        catalog_policy: &declaration.policy,
        package: "report",
        cache_policy: &policy_path,
        bundle: &bundle,
    };

    // Identical recipes deliberately appear under another namespace/package.
    // A resolved output identity alone cannot authorize either name.
    let mut other_catalog = declaration.document["catalog"].clone();
    other_catalog["namespace"] = json!("observatory");
    let mut other_policy = declaration.document["policy"].clone();
    other_policy["namespace"] = json!("observatory");
    let other_catalog = fixture.write_json("same-recipe-other-namespace.json", &other_catalog);
    let other_policy = fixture.write_json("same-recipe-other-policy.json", &other_policy);
    let other_resolved = invoke(
        &observatory,
        &[
            "resolve",
            text(&other_catalog),
            text(&other_policy),
            "report",
        ],
    )
    .json();
    assert_eq!(other_resolved["plan"]["outputs"]["report"], output);
    let wrong_namespace = CacheSelection {
        consumer: &observatory,
        store: &station,
        catalog: &other_catalog,
        catalog_policy: &other_policy,
        ..normal
    };
    substitute_cache(&fixture, &wrong_namespace, &selected, &receipt)
        .refused("cache scope differs from selected catalog namespace/package");
    unchanged_cache_store(&observatory, &station, &station_before);
    let absent = fixture.path("must-not-create-on-scope-refusal");
    let absent_namespace = CacheSelection {
        store: &absent,
        ..wrong_namespace
    };
    substitute_cache(&fixture, &absent_namespace, &selected, &receipt)
        .refused("cache scope differs from selected catalog namespace/package");
    assert!(!absent.exists());

    let mut alias_catalog = declaration.document["catalog"].clone();
    alias_catalog["packages"]["alternate"] = alias_catalog["packages"]["report"].clone();
    let mut alias_policy = declaration.document["policy"].clone();
    alias_policy["packages"] = json!(["alternate"]);
    let alias_catalog = fixture.write_json("same-recipe-other-package.json", &alias_catalog);
    let alias_policy = fixture.write_json("same-recipe-other-package-policy.json", &alias_policy);
    let alias_resolved = invoke(
        &fieldkit,
        &[
            "resolve",
            text(&alias_catalog),
            text(&alias_policy),
            "alternate",
        ],
    )
    .json();
    assert_eq!(alias_resolved["plan"]["outputs"]["report"], output);
    let wrong_package = CacheSelection {
        catalog: &alias_catalog,
        catalog_policy: &alias_policy,
        package: "alternate",
        ..normal
    };
    substitute_cache(&fixture, &wrong_package, &selected, &receipt)
        .refused("cache scope differs from selected catalog namespace/package");
    unchanged_cache_store(&fieldkit, &receiver, &field_before);

    substitute_cache(&fixture, &normal, &unauthorized, &receipt).refused("not authorized");
    unchanged_cache_store(&fieldkit, &receiver, &field_before);
    let absent_key_store = fixture.path("must-not-open-on-key-refusal");
    let absent_key = CacheSelection {
        store: &absent_key_store,
        ..normal
    };
    substitute_cache(&fixture, &absent_key, &unauthorized, &receipt).refused("not authorized");
    assert!(!absent_key_store.exists());
    for (field, value) in [
        ("scope", json!("observatory/report")),
        ("scope", json!("fieldkit/alternate")),
        ("policy_revision", json!(2)),
    ] {
        let mut changed = receipt.clone();
        changed[field] = value;
        substitute_cache(&fixture, &normal, &selected, &changed).refused("receipt scope");
        unchanged_cache_store(&fieldkit, &receiver, &field_before);
    }
    let mut wrong_recipe = receipt.clone();
    wrong_recipe["root"]["derivation"]["platform"] = json!("x86_64-linux");
    substitute_cache(&fixture, &normal, &selected, &wrong_recipe)
        .refused("independently resolved recipe");
    unchanged_cache_store(&fieldkit, &receiver, &field_before);
    let absent_recipe_store = fixture.path("must-not-open-on-recipe-refusal");
    let absent_recipe = CacheSelection {
        store: &absent_recipe_store,
        ..normal
    };
    substitute_cache(&fixture, &absent_recipe, &selected, &wrong_recipe)
        .refused("independently resolved recipe");
    assert!(!absent_recipe_store.exists());
    let mut wrong_references = receipt.clone();
    wrong_references["root"]["references"]
        .as_array_mut()
        .unwrap()
        .push(json!(format!("src-{}", "9".repeat(64))));
    substitute_cache(&fixture, &normal, &selected, &wrong_references)
        .refused("staged closure differs");
    unchanged_cache_store(&fieldkit, &receiver, &field_before);

    let imported = substitute_cache(&fixture, &normal, &selected, &receipt).json();
    assert_eq!(imported["namespace"], "fieldkit");
    assert_eq!(imported["package"], "report");
    assert_eq!(imported["scope"], "fieldkit/report");
    assert_eq!(imported["roots"], json!([output]));
    assert_eq!(imported["signer_fingerprint"], selected.fingerprint);
    fs::rename(&producer, fixture.path("signed-cache-producer-unavailable")).unwrap();
    fs::remove_dir_all(&source).unwrap();
    expect_output(
        &fieldkit,
        &["run", text(&receiver), &output],
        "fieldkit-v1: total=43\n",
    );
    let reused = build(&fieldkit, &receiver, &declaration);
    assert_eq!(reused["built"], json!([]));
    assert_eq!(reused["reused"].as_array().unwrap().len(), 2);
    unchanged_cache_store(&observatory, &station, &station_before);
    println!(
        "external signed cache: own namespace/package/key/recipe bound before store admission; producer-absent result=43; other project preserved"
    );
}
