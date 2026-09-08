use super::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-file-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        std::fs::write(path.join("fixture-marker"), b"synthetic").unwrap();
        Self(path)
    }
    fn directory(&self) -> Directory {
        Directory::open(&self.0).unwrap()
    }
    fn live(&self) -> PathBuf {
        self.0.join("settings.toml")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        assert_eq!(
            std::fs::read(self.0.join("fixture-marker")).unwrap(),
            b"synthetic"
        );
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn token() -> String {
    "a".repeat(32)
}

#[test]
fn prepare_publish_and_finish_keep_metadata_and_only_then_remove_checkpoint() {
    let fixture = Fixture::new();
    std::fs::write(fixture.live(), b"old private value\n").unwrap();
    std::fs::set_permissions(fixture.live(), std::fs::Permissions::from_mode(0o640)).unwrap();
    let directory = fixture.directory();
    let receipt = directory
        .prepare(
            "settings.toml",
            &token(),
            Some(b"old private value\n"),
            b"new private value\n",
        )
        .unwrap();
    assert_eq!(
        std::fs::read(fixture.live()).unwrap(),
        b"old private value\n"
    );
    assert!(
        !String::from_utf8(serde_json::to_vec(&receipt).unwrap())
            .unwrap()
            .contains("private value")
    );
    directory.publish(&receipt).unwrap();
    assert_eq!(
        std::fs::read(fixture.live()).unwrap(),
        b"new private value\n"
    );
    assert_eq!(
        std::fs::metadata(fixture.live()).unwrap().mode() & 0o777,
        0o640
    );
    directory.finish(&receipt).unwrap();
    assert!(
        !fixture
            .0
            .join(Directory::work_name(&token()).unwrap())
            .exists()
    );
}

#[test]
fn abort_restores_the_original_and_rejects_a_later_live_write() {
    let fixture = Fixture::new();
    std::fs::write(fixture.live(), b"old").unwrap();
    let directory = fixture.directory();
    let receipt = directory
        .prepare("settings.toml", &token(), Some(b"old"), b"new")
        .unwrap();
    directory.publish(&receipt).unwrap();
    directory.abort(&receipt).unwrap();
    assert_eq!(std::fs::read(fixture.live()).unwrap(), b"old");
    let receipt = directory
        .prepare("settings.toml", &token(), Some(b"old"), b"new")
        .unwrap();
    directory.publish(&receipt).unwrap();
    std::fs::write(fixture.live(), b"later user change").unwrap();
    assert!(directory.abort(&receipt).is_err());
    assert_eq!(std::fs::read(fixture.live()).unwrap(), b"later user change");
}

#[test]
fn source_rename_and_old_open_writer_are_detected_without_deleting_data() {
    let fixture = Fixture::new();
    std::fs::write(fixture.live(), b"old").unwrap();
    let directory = fixture.directory();
    let receipt = directory
        .prepare("settings.toml", &token(), Some(b"old"), b"new")
        .unwrap();
    std::fs::write(fixture.0.join("other"), b"racing replacement").unwrap();
    std::fs::rename(fixture.0.join("other"), fixture.live()).unwrap();
    assert!(directory.publish(&receipt).is_err());
    assert_eq!(
        std::fs::read(fixture.live()).unwrap(),
        b"racing replacement"
    );

    let receipt = directory
        .prepare(
            "settings.toml",
            &"b".repeat(32),
            Some(b"racing replacement"),
            b"desired",
        )
        .unwrap();
    let mut old_writer = std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.live())
        .unwrap();
    directory.publish(&receipt).unwrap();
    old_writer.set_len(0).unwrap();
    old_writer.write_all(b"late old-inode edit").unwrap();
    old_writer.sync_all().unwrap();
    assert!(directory.finish(&receipt).is_err());
    let checkpoint = fixture
        .0
        .join(Directory::work_name(&receipt.token).unwrap())
        .join(SLOT);
    assert_eq!(std::fs::read(checkpoint).unwrap(), b"late old-inode edit");
    assert_eq!(std::fs::read(fixture.live()).unwrap(), b"desired");
}

#[test]
fn absent_file_uses_no_replace_and_abort_returns_to_absence() {
    let fixture = Fixture::new();
    let directory = fixture.directory();
    let receipt = directory
        .prepare("settings.toml", &token(), None, b"new")
        .unwrap();
    directory.publish(&receipt).unwrap();
    directory.abort(&receipt).unwrap();
    assert!(!fixture.live().exists());
    let receipt = directory
        .prepare("settings.toml", &token(), None, b"new")
        .unwrap();
    std::fs::write(fixture.live(), b"other writer").unwrap();
    assert!(directory.publish(&receipt).is_err());
    assert_eq!(std::fs::read(fixture.live()).unwrap(), b"other writer");
}

#[test]
fn unsafe_paths_links_permissions_and_bounds_refuse_before_publication() {
    let fixture = Fixture::new();
    let directory = fixture.directory();
    assert!(
        directory
            .prepare("../escape", &token(), None, b"x")
            .is_err()
    );
    assert!(
        directory
            .prepare("settings.toml", "../escape", None, b"x")
            .is_err()
    );
    std::fs::write(fixture.0.join("outside"), b"unmanaged").unwrap();
    symlink(fixture.0.join("outside"), fixture.live()).unwrap();
    assert!(directory.read("settings.toml").is_err());
    std::fs::remove_file(fixture.live()).unwrap();
    std::fs::hard_link(fixture.0.join("outside"), fixture.live()).unwrap();
    assert!(directory.read("settings.toml").is_err());
    std::fs::remove_file(fixture.live()).unwrap();
    assert!(
        directory
            .prepare("settings.toml", &token(), None, &vec![0; MAX_FILE + 1])
            .is_err()
    );
    std::fs::write(fixture.live(), b"read only").unwrap();
    std::fs::set_permissions(fixture.live(), std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(
        directory
            .prepare("settings.toml", &token(), Some(b"read only"), b"changed")
            .is_err()
    );
    assert_eq!(
        std::fs::read(fixture.0.join("outside")).unwrap(),
        b"unmanaged"
    );
}
