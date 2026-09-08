use super::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
use std::sync::atomic::{AtomicU64, Ordering};
use sysroot_core::noctalia::{Theme, Value};

static NEXT: AtomicU64 = AtomicU64::new(0);
const INSTANCE: &str = "0123456789abcdef0123456789abcdef";
const PRIVATE: &str = "synthetic-private-field-must-not-be-stored";
const BASE: &str = "[theme]\nmode='dark'\n[shell]\nbutton_borders=true\ninput_borders=true\n";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-home-review-{}-{}-{}",
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
        Self(path)
    }
    fn owner(&self) -> u32 {
        rustix::process::geteuid().as_raw()
    }
    fn state(&self) -> PathBuf {
        self.0.join("review")
    }
    fn baseline(&self) -> Baseline {
        Baseline {
            target: "desktop".into(),
            source_path: "home/.config/noctalia/config.toml".into(),
            source_revision: "a".repeat(40),
            settings: noctalia::project(noctalia::APP_VERSION, BASE).unwrap(),
        }
    }
    fn store(&self) -> Store {
        let baseline = self.baseline();
        let state = State::new(INSTANCE.into(), baseline.clone(), baseline.settings).unwrap();
        Store::create(&self.state(), &[(RECORD, &state.to_bytes().unwrap())]).unwrap()
    }
    fn installed(&self) -> PathBuf {
        let root = self.0.join("image");
        let file = root.join(DESTINATION);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, BASE).unwrap();
        let manifest = serde_json::json!({"schema_version": 1, "source_revision": "a".repeat(40),
            "target": {"id": "desktop"}, "files": [{"source_path": "home/.config/noctalia/config.toml",
            "destination": DESTINATION, "sha256": hash(BASE.as_bytes()), "home_baseline": true}]});
        std::fs::write(
            root.join("usr/share/sysroot/source.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        root
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn pinned_selection_survives_reopen_and_subsequent_app_writes() {
    let f = Fixture::new();
    let mut store = f.store();
    let mut observed = f.baseline().settings;
    observed.theme_mode = Theme::Light;
    let staged = change(
        &mut store,
        INSTANCE,
        observed,
        &Command::Stage { key: Key::Theme },
    )
    .unwrap();
    assert_eq!(
        staged.selection().unwrap()[0].after,
        Value::Theme(Theme::Light)
    );
    drop(store);
    let mut store = Store::open(&f.state()).unwrap();
    observed.theme_mode = Theme::Auto;
    let captured = change(&mut store, INSTANCE, observed, &Command::Status).unwrap();
    assert_eq!(
        captured.selection().unwrap()[0].after,
        Value::Theme(Theme::Light)
    );
    assert_eq!(captured.rows().unwrap()[0].live, Value::Theme(Theme::Auto));
    assert_eq!(store.read(RECORD).unwrap().unwrap().revision, 3);
    change(&mut store, INSTANCE, observed, &Command::Status).unwrap();
    assert_eq!(
        store.read(RECORD).unwrap().unwrap().revision,
        3,
        "unchanged captures do not create revisions"
    );
}

#[test]
fn local_policy_is_durable_and_failed_dispositions_do_not_commit_capture() {
    let f = Fixture::new();
    let mut store = f.store();
    let mut observed = f.baseline().settings;
    observed.theme_mode = Theme::Light;
    change(
        &mut store,
        INSTANCE,
        observed,
        &Command::KeepLocal { key: Key::Theme },
    )
    .unwrap();
    let before = store.read(RECORD).unwrap().unwrap();
    assert!(
        change(
            &mut store,
            INSTANCE,
            observed,
            &Command::Stage { key: Key::Theme }
        )
        .is_err()
    );
    assert_eq!(store.read(RECORD).unwrap().unwrap(), before);
    drop(store);
    let mut store = Store::open(&f.state()).unwrap();
    observed.theme_mode = Theme::Auto;
    let state = change(&mut store, INSTANCE, observed, &Command::Status).unwrap();
    assert!(state.rows().unwrap()[0].visible_change);
    change(
        &mut store,
        INSTANCE,
        observed,
        &Command::AppOwn { key: Key::Theme },
    )
    .unwrap();
    observed.theme_mode = Theme::Light;
    let state = change(&mut store, INSTANCE, observed, &Command::Status).unwrap();
    assert!(state.rows().unwrap()[0].app_owned);
    assert!(state.selection().unwrap().is_empty());
}

#[test]
fn full_export_private_fields_never_enter_database_or_history() {
    let f = Fixture::new();
    let mut store = f.store();
    let raw = format!("{BASE}\n[private]\ncredential='{PRIVATE}'\n");
    let observed = noctalia::project(noctalia::APP_VERSION, &raw).unwrap();
    change(&mut store, INSTANCE, observed, &Command::Status).unwrap();
    for entry in std::fs::read_dir(f.state()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(
            !bytes
                .windows(PRIVATE.len())
                .any(|chunk| chunk == PRIVATE.as_bytes())
        );
    }
    assert!(load(&store, "ffffffffffffffffffffffffffffffff").is_err());
}

#[test]
fn installed_baseline_requires_matching_bytes_and_source_provenance() {
    let f = Fixture::new();
    let root = f.installed();
    assert_eq!(installed_baseline(&root, f.owner()).unwrap(), f.baseline());
    std::fs::write(root.join(DESTINATION), BASE.replace("dark", "light")).unwrap();
    assert!(installed_baseline(&root, f.owner()).is_err());
}

#[test]
fn input_reader_refuses_links_special_files_modes_and_oversize() {
    let f = Fixture::new();
    let file = f.0.join("regular");
    std::fs::write(&file, b"safe").unwrap();
    assert_eq!(read_regular(&file, f.owner(), 4).unwrap(), b"safe");
    assert!(read_regular(&file, f.owner(), 3).is_err());
    let link = f.0.join("link");
    symlink(&file, &link).unwrap();
    assert!(read_regular(&link, f.owner(), 4).is_err());
    let hardlink = f.0.join("hardlink");
    std::fs::hard_link(&file, &hardlink).unwrap();
    assert!(read_regular(&file, f.owner(), 4).is_err());
    std::fs::remove_file(hardlink).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();
    assert!(read_regular(&file, f.owner(), 4).is_err());
    assert!(read_regular(&f.0, f.owner(), 4).is_err());
    let socket = f.0.join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert!(read_regular(&socket, f.owner(), 4).is_err());
}

#[test]
fn review_parent_accepts_bootc_style_alias_but_rejects_shared_writes() {
    let f = Fixture::new();
    assert_eq!(private_parent(&f.state(), f.owner()).unwrap(), f.state());
    let alias = f.0.join("home-alias");
    symlink(&f.0, &alias).unwrap();
    assert_eq!(
        private_parent(&alias.join("review"), f.owner()).unwrap(),
        f.state()
    );
    assert!(private_parent(Path::new("relative/review"), f.owner()).is_err());
    std::fs::set_permissions(&f.0, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(private_parent(&f.state(), f.owner()).is_err());
}
