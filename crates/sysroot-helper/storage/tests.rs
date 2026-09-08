use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "kedra-state-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&parent)
            .unwrap();
        std::fs::write(
            parent.join("fixture-marker"),
            "Kedra generated state fixture",
        )
        .unwrap();
        Self(parent)
    }
    fn path(&self) -> PathBuf {
        self.0.join("store")
    }
    fn store(&self) -> Store {
        Store::create(&self.path(), &[("state", b"old")]).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn revisions_history_and_reopen_are_atomic() {
    let f = Fixture::new();
    let mut s = f.store();
    let first = s.read("state").unwrap().unwrap();
    assert_eq!(first.bytes, b"old");
    assert_eq!(
        s.compare_exchange("state", Some(first.revision), b"new")
            .unwrap()
            .revision,
        2
    );
    assert!(matches!(
        s.compare_exchange("state", Some(1), b"stale"),
        Err(Error::Conflict)
    ));
    assert_eq!(s.history("state", 1).unwrap().unwrap().bytes, b"old");
    drop(s);
    assert_eq!(
        Store::open(&f.path())
            .unwrap()
            .read("state")
            .unwrap()
            .unwrap()
            .bytes,
        b"new"
    );
    assert!(Store::create(&f.path(), &[("state", b"reset")]).is_err());
}

#[test]
fn missing_corrupt_and_future_schema_never_reset() {
    let f = Fixture::new();
    assert!(Store::open(&f.path()).is_err());
    assert!(!f.path().exists());
    let s = f.store();
    s.connection
        .pragma_update(None, "user_version", 99)
        .unwrap();
    drop(s);
    assert!(matches!(
        Store::open(&f.path()),
        Err(Error::UnsupportedSchema(99))
    ));
    std::fs::write(f.path().join(DATABASE), b"not sqlite").unwrap();
    assert!(Store::open(&f.path()).is_err());
}

#[test]
fn checksums_detect_valid_sqlite_with_corrupt_record_bytes() {
    let f = Fixture::new();
    let s = f.store();
    s.connection
        .execute(
            "UPDATE records SET body=?1 WHERE name='state'",
            [b"changed".as_slice()],
        )
        .unwrap();
    assert!(matches!(s.read("state"), Err(Error::Corrupt)));
}

#[test]
fn symlinks_hardlinks_permissions_and_inode_replacement_are_refused() {
    let f = Fixture::new();
    let s = f.store();
    symlink(f.path(), f.0.join("alias")).unwrap();
    assert!(Store::open(&f.0.join("alias")).is_err());
    std::fs::hard_link(f.path().join(DATABASE), f.0.join("linked")).unwrap();
    assert!(matches!(Store::open(&f.path()), Err(Error::UnsafeFile)));
    std::fs::remove_file(f.0.join("linked")).unwrap();
    std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(Store::open(&f.path()), Err(Error::UnsafeFile)));
    std::fs::set_permissions(f.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::rename(f.path().join(DATABASE), f.path().join("original")).unwrap();
    std::fs::write(f.path().join(DATABASE), b"replacement").unwrap();
    assert!(s.read("state").is_err());
}

#[test]
fn special_files_and_sidecar_links_fail_without_blocking_or_following() {
    let f = Fixture::new();
    let s = f.store();
    drop(s);
    symlink(f.0.join("outside"), f.path().join("state.sqlite-wal")).unwrap();
    assert!(Store::open(&f.path()).is_err());
    assert!(!f.0.join("outside").exists());
    std::fs::remove_file(f.path().join("state.sqlite-wal")).unwrap();
    std::fs::remove_file(f.path().join(DATABASE)).unwrap();
    fs::mknodat(
        rustix::fs::CWD,
        f.path().join(DATABASE),
        FileType::Fifo,
        Mode::from_raw_mode(0o600),
        0,
    )
    .unwrap();
    assert!(matches!(Store::open(&f.path()), Err(Error::UnsafeFile)));
}

#[test]
fn concurrent_writer_cannot_overwrite_an_uncommitted_transaction() {
    let f = Fixture::new();
    let mut first = f.store();
    let mut second = Store::open(&f.path()).unwrap();
    let transaction = first
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    assert!(
        second
            .compare_exchange("state", Some(1), b"second")
            .is_err()
    );
    drop(transaction);
    second
        .compare_exchange("state", Some(1), b"second")
        .unwrap();
    assert!(matches!(
        first.compare_exchange("state", Some(1), b"lost update"),
        Err(Error::Conflict)
    ));
}

#[test]
fn child_write_then_exit() {
    let Some(path) = std::env::var_os("KEDRA_STATE_CHILD") else {
        return;
    };
    let path = PathBuf::from(path);
    assert_eq!(path.file_name().unwrap(), "store");
    assert!(
        path.parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("kedra-state-")
    );
    assert_eq!(
        std::fs::read_to_string(path.parent().unwrap().join("fixture-marker")).unwrap(),
        "Kedra generated state fixture"
    );
    let mut store = Store::open(&path).unwrap();
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    transaction
        .execute(
            "UPDATE records SET revision=2,body=?1,sha256=?2 WHERE name='state'",
            params![b"child".as_slice(), hash(b"child")],
        )
        .unwrap();
    if std::env::var_os("KEDRA_STATE_COMMIT").is_some() {
        transaction.commit().unwrap();
    }
    std::process::exit(87); // No destructors: model process interruption without core dumps.
}

#[test]
fn process_interruption_retains_old_or_committed_state() {
    let f = Fixture::new();
    let store = f.store();
    for committed in [false, true] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "storage::tests::child_write_then_exit",
                "--nocapture",
            ])
            .env("KEDRA_STATE_CHILD", f.path());
        if committed {
            child.env("KEDRA_STATE_COMMIT", "1");
        } else {
            child.env_remove("KEDRA_STATE_COMMIT");
        }
        assert_eq!(child.status().unwrap().code(), Some(87));
        let expected = if committed {
            b"child".as_slice()
        } else {
            b"old".as_slice()
        };
        assert_eq!(store.read("state").unwrap().unwrap().bytes, expected);
    }
}
