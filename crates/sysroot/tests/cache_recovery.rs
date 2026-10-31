//! Bounded-filesystem failure of a real import, recovered and retried through
//! the public CLI. Binary cache substitution is covered by `e2e_rpm_lock`.
#![cfg(target_os = "linux")]
use super::{Fixture, remove_fixture, run, text};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
