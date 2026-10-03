//! Actual signed transfer and bounded-filesystem failure through the public CLI.
use super::{
    Capture, Fixture, Running, app_id, assert_version, authored_plan, authoring_consumer, c_source,
    remove_fixture, run, sha256, text,
};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct ProducerKey {
    private: PathBuf,
    public: PathBuf,
    fingerprint: String,
}

fn key(f: &Fixture, name: &str) -> ProducerKey {
    let private = f.path(&format!("{name}.pem"));
    let public = f.path(&format!("{name}.pub"));
    let der = f.path(&format!("{name}.der"));
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
    ProducerKey {
        private,
        public,
        fingerprint: sha256(&der),
    }
}

fn sign(f: &Fixture, key: &ProducerKey, receipt: &Value) -> (PathBuf, PathBuf) {
    let payload = f.write_json("cache-receipt.json", receipt);
    let der = f.path("cache-signature.der");
    let signature = f.path("cache-signature.txt");
    run(Command::new("openssl")
        .args(["dgst", "-sha256", "-sign"])
        .arg(&key.private)
        .arg("-out")
        .arg(&der)
        .arg(&payload))
    .success();
    run(Command::new("openssl")
        .args(["base64", "-A", "-in"])
        .arg(&der)
        .arg("-out")
        .arg(&signature))
    .success();
    (payload, signature)
}

struct Admission<'a> {
    store: &'a Path,
    graph: &'a Path,
    bundle: &'a Path,
    policy: &'a Path,
    scope: &'a str,
}

fn substitute(
    f: &Fixture,
    admission: &Admission<'_>,
    key: &ProducerKey,
    receipt: &Value,
) -> Capture {
    let (payload, signature) = sign(f, key, receipt);
    invoke_substitute(f, admission, key, &payload, &signature)
}

fn invoke_substitute(
    f: &Fixture,
    admission: &Admission<'_>,
    key: &ProducerKey,
    payload: &Path,
    signature: &Path,
) -> Capture {
    run(&mut substitute_command(
        f, admission, key, payload, signature,
    ))
}

fn substitute_command(
    f: &Fixture,
    admission: &Admission<'_>,
    key: &ProducerKey,
    payload: &Path,
    signature: &Path,
) -> Command {
    let mut command = f.command(&["store", "substitute", "--store"]);
    command
        .arg(admission.store)
        .arg("--bundle")
        .arg(admission.bundle)
        .arg("--receipt")
        .arg(payload)
        .arg("--signature")
        .arg(signature)
        .arg("--public-key")
        .arg(&key.public)
        .arg("--cache-policy")
        .arg(admission.policy)
        .args(["--scope", admission.scope, "--root", "app", "--plan"])
        .arg(admission.graph);
    command
}

#[test]
#[ignore = "requires native ARM Docker, retained builder/runtime and OpenSSL; executes real signed package transfer"]
fn signed_closure_refuses_untrusted_changes_then_runs_without_producer() {
    let f = Fixture::new();
    let producer = f.path("producer");
    let receiver = f.path("receiver");
    f.images(&producer);
    let author = authoring_consumer(&f);
    let source = c_source(&f, "signed-cache");
    let source_id = f.source(&producer, &source);
    let graph = authored_plan(&f, &author, &source_id, "cache-plan.json");
    let output = app_id(&f.build(&producer, &graph, &[]).json());
    let root = f.store("verify", &producer, &["--object", &output]).json();
    let bundle = f.path("cache.tar");
    let exported = f
        .store(
            "export",
            &producer,
            &["--object", &output, "--output", text(&bundle)],
        )
        .json();
    let authorized = key(&f, "authorized");
    let other = key(&f, "other");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let policy = f.write_json(
        "cache-policy.json",
        &json!({
            "schema":1, "scope":"independent-example/app", "revision":7,
            "valid_from":now-60, "valid_until":now+7200,
            "trusted_keys":[authorized.fingerprint],
        }),
    );
    let receipt = json!({
        "schema":1, "purpose":"sysroot-engine-binary-cache-v1",
        "scope":"independent-example/app", "policy_revision":7,
        "valid_from":now-30, "valid_until":now+3600,
        "bundle_sha256":exported["sha256"], "bundle_bytes":exported["bytes"], "root":root,
    });
    let sentinel_source = f.path("sentinel-source");
    fs::create_dir(&sentinel_source).unwrap();
    fs::write(
        sentinel_source.join("value"),
        b"preserve existing consumer data\n",
    )
    .unwrap();
    let sentinel = f.source(&receiver, &sentinel_source);
    let roots_before = fs::read(receiver.join("roots/state.json")).unwrap();
    let admission = Admission {
        store: &receiver,
        graph: &graph,
        bundle: &bundle,
        policy: &policy,
        scope: "independent-example/app",
    };
    let fifo = f.path("bundle.fifo");
    run(Command::new("mkfifo").arg(&fifo)).success();
    let fifo_admission = Admission {
        bundle: &fifo,
        ..admission
    };
    let (payload, signature) = sign(&f, &authorized, &receipt);
    let mut opening = Running::start(&mut substitute_command(
        &f,
        &fifo_admission,
        &authorized,
        &payload,
        &signature,
    ));
    let started = Instant::now();
    while opening.child.try_wait().unwrap().is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "FIFO bundle blocked the public substitution workflow"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    opening.finish().refused("regular file");
    assert_eq!(
        fs::read(receiver.join("roots/state.json")).unwrap(),
        roots_before
    );
    f.store("verify", &receiver, &["--object", &sentinel])
        .json();
    substitute(&f, &admission, &other, &receipt).refused("not authorized");
    let (payload, signature) = sign(&f, &authorized, &receipt);
    let mut changed_bytes = fs::read(&payload).unwrap();
    changed_bytes.push(b' ');
    fs::write(&payload, changed_bytes).unwrap();
    invoke_substitute(&f, &admission, &authorized, &payload, &signature)
        .refused("signature verification failed");
    for (field, value, reason) in [
        ("purpose", json!("other-protocol"), "receipt scope"),
        ("scope", json!("different-project/app"), "receipt scope"),
        ("policy_revision", json!(6), "receipt scope"),
        ("valid_until", json!(now - 1), "not currently valid"),
    ] {
        let mut changed = receipt.clone();
        changed[field] = value;
        substitute(&f, &admission, &authorized, &changed).refused(reason);
    }
    let mut wrong_recipe = receipt.clone();
    wrong_recipe["root"]["derivation"]["platform"] = json!("x86_64-linux");
    substitute(&f, &admission, &authorized, &wrong_recipe).refused("independently resolved recipe");
    let mut wrong_digest = receipt.clone();
    wrong_digest["bundle_sha256"] = json!("0".repeat(64));
    substitute(&f, &admission, &authorized, &wrong_digest).refused("bundle SHA256 differs");
    // Signature and recipe are valid. Only the final staged root comparison can
    // refuse this producer assertion before any closure admission takes place.
    let mut wrong_reference = receipt.clone();
    wrong_reference["root"]["references"]
        .as_array_mut()
        .unwrap()
        .push(json!(format!("src-{}", "7".repeat(64))));
    substitute(&f, &admission, &authorized, &wrong_reference).refused("staged closure differs");
    assert_eq!(
        fs::read(receiver.join("roots/state.json")).unwrap(),
        roots_before
    );
    f.store("verify", &receiver, &["--object", &sentinel])
        .json();
    assert!(
        !f.store("verify", &receiver, &["--object", &output])
            .status
            .success()
    );
    let imported = substitute(&f, &admission, &authorized, &receipt).json();
    assert_eq!(imported["roots"], json!([output]));
    assert_eq!(imported["signer_fingerprint"], authorized.fingerprint);
    assert_eq!(imported["deployment_authorized"], false);
    remove_fixture(&producer).unwrap();
    remove_fixture(&source).unwrap();
    assert_version(f.program(&receiver, &output, &[]), "signed-cache");
    let reused = f.build(&receiver, &graph, &[]).json();
    assert_eq!(reused["built"], json!([]));
    assert_eq!(reused["reused"].as_array().unwrap().len(), 2);
    let regenerated_source = c_source(&f, "signed-cache");
    assert_eq!(f.source(&receiver, &regenerated_source), source_id);
    f.images(&receiver);
    let reproduced = f.build(&receiver, &graph, &["--rebuild"]).json();
    assert_eq!(reproduced["reproduced"].as_array().unwrap().len(), 2);
    assert_version(f.program(&receiver, &output, &[]), "signed-cache");
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires dedicated <=256 MiB tmpfs KEDRA_CACHE_ENOSPC_ROOT and explicit writable TMPDIR on another device"]
fn full_bounded_store_recovers_and_retries_real_import() {
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;
    let selected = PathBuf::from(
        std::env::var_os("KEDRA_CACHE_ENOSPC_ROOT")
            .expect("set KEDRA_CACHE_ENOSPC_ROOT to the dedicated bounded tmpfs mount"),
    );
    assert!(selected.is_absolute());
    assert!(
        !fs::symlink_metadata(&selected)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let mount = run(Command::new("findmnt").args([
        "--mountpoint",
        text(&selected),
        "--noheadings",
        "--bytes",
        "--output",
        "FSTYPE,SIZE",
    ]))
    .success();
    let description = String::from_utf8(mount.stdout).unwrap();
    let fields: Vec<_> = description.split_whitespace().collect();
    assert_eq!(fields.len(), 2, "must select one exact dedicated mount");
    assert_eq!(fields[0], "tmpfs");
    let capacity: u64 = fields[1].parse().unwrap();
    assert!((16 * 1024 * 1024..=256 * 1024 * 1024).contains(&capacity));
    assert_eq!(
        fs::read_dir(&selected).unwrap().count(),
        0,
        "dedicated mount must be empty"
    );
    let temporary = PathBuf::from(
        std::env::var_os("TMPDIR")
            .filter(|value| !value.is_empty())
            .expect("set explicit writable TMPDIR outside the bounded ENOSPC filesystem"),
    );
    assert!(
        temporary.is_absolute(),
        "TMPDIR must be an absolute directory"
    );
    let temporary_metadata = fs::metadata(&temporary).expect("TMPDIR must already exist");
    assert!(temporary_metadata.is_dir(), "TMPDIR must be a directory");
    let selected_device = fs::metadata(&selected).unwrap().dev();
    assert_ne!(
        temporary_metadata.dev(),
        selected_device,
        "TMPDIR must be on a different device from KEDRA_CACHE_ENOSPC_ROOT"
    );
    let probe_path = temporary.join(format!(
        ".kedra-enospc-write-probe-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut probe = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
        .expect("explicit TMPDIR must permit ordinary file creation");
    assert_eq!(
        probe.metadata().unwrap().dev(),
        temporary_metadata.dev(),
        "TMPDIR device changed during the writability check"
    );
    let writable = probe.write_all(b"w");
    drop(probe);
    fs::remove_file(&probe_path).expect("explicit TMPDIR must permit temporary-file removal");
    writable.expect("explicit TMPDIR must permit ordinary file writes");
    let f = Fixture::new();
    assert_eq!(
        fs::metadata(&f.0).unwrap().dev(),
        temporary_metadata.dev(),
        "fixture temporary directory did not use the selected separate TMPDIR device"
    );
    let producer = f.path("producer");
    let source = f.path("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("payload"), vec![17_u8; 4 * 1024 * 1024]).unwrap();
    let object = f.source(&producer, &source);
    let bundle = f.path("source.tar");
    let exported = f
        .store(
            "export",
            &producer,
            &["--object", &object, "--output", text(&bundle)],
        )
        .json();
    let store = selected.join("store");
    let preserved_source = f.path("preserved-source");
    fs::create_dir(&preserved_source).unwrap();
    fs::write(preserved_source.join("value"), b"retained before ENOSPC\n").unwrap();
    let preserved = f.source(&store, &preserved_source);
    let roots_before = fs::read(store.join("roots/state.json")).unwrap();
    let foreign = f.path("foreign-sentinel");
    fs::write(&foreign, b"outside bounded filesystem\n").unwrap();
    let filler = selected.join("filler");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&filler)
        .unwrap();
    let block = [0_u8; 65536];
    loop {
        match file.write_all(&block) {
            Ok(()) => {}
            Err(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::StorageFull);
                break;
            }
        }
    }
    drop(file);
    f.store(
        "import",
        &store,
        &[
            "--bundle",
            text(&bundle),
            "--expected-sha256",
            &super::string(&exported, "sha256"),
        ],
    )
    .refused("space");
    fs::remove_file(&filler).unwrap();
    f.store("recover", &store, &[]).json();
    assert_eq!(
        fs::read(store.join("roots/state.json")).unwrap(),
        roots_before
    );
    f.store("verify", &store, &["--object", &preserved]).json();
    f.store(
        "import",
        &store,
        &[
            "--bundle",
            text(&bundle),
            "--expected-sha256",
            &super::string(&exported, "sha256"),
        ],
    )
    .json();
    f.store("verify", &store, &["--object", &object]).json();
    assert_eq!(fs::read(&foreign).unwrap(), b"outside bounded filesystem\n");
    remove_fixture(&store).unwrap();
}
