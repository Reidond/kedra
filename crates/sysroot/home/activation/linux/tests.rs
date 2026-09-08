use super::*;
use std::cell::Cell;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use sysroot_core::noctalia::{self, Baseline, Key, Theme};
const INSTANCE: &str = "0123456789abcdef0123456789abcdef";
const PRIVATE: &str = "synthetic-private-extra-never-in-review";
struct Fixture {
    root: PathBuf,
    fail_start: Cell<bool>,
    change_on_stop: Cell<bool>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("kedra-activation-{}", token().unwrap()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        std::fs::write(root.join(FILE), format!("[theme]\nmode='auto'\nprivate='{PRIVATE}'\n[shell]\nbutton_borders=true\ninput_borders=true\n")).unwrap();
        Self {
            root,
            fail_start: Cell::new(false),
            change_on_stop: Cell::new(false),
        }
    }
    fn store(&self) -> Store {
        let live = self.observe().unwrap();
        let baseline = Baseline {
            target: "desktop".into(),
            source_path: "home/.config/noctalia/config.toml".into(),
            source_revision: "a".repeat(40),
            settings: Settings {
                theme_mode: Theme::Dark,
                ..live
            },
        };
        let mut state = State::new(INSTANCE.into(), baseline, live).unwrap();
        state.stage(Key::ThemeMode).unwrap();
        state
            .capture(Settings {
                theme_mode: Theme::Light,
                ..live
            })
            .unwrap();
        self.set(Theme::Light);
        Store::create(
            &self.root.join("review"),
            &[(RECORD, &state.to_bytes().unwrap())],
        )
        .unwrap()
    }
    fn set(&self, theme: Theme) {
        let original = std::fs::read(self.root.join(FILE)).unwrap();
        let before = noctalia::project("5.0.1", std::str::from_utf8(&original).unwrap()).unwrap();
        std::fs::write(
            self.root.join(FILE),
            patch(
                &original,
                before,
                Settings {
                    theme_mode: theme,
                    ..before
                },
            )
            .unwrap(),
        )
        .unwrap();
    }
    fn directory(&self) -> Directory {
        Directory::open(&self.root).unwrap()
    }
    fn plan(&self, store: &Store) -> Plan {
        Plan::new(
            &load(store, INSTANCE).unwrap().1,
            Action::Discard {
                key: Key::ThemeMode,
            },
        )
        .unwrap()
    }
    fn audit(&self) {
        for path in [
            self.root.join("review/state.sqlite"),
            self.root.join("review/state.sqlite-wal"),
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                assert!(
                    !bytes
                        .windows(PRIVATE.len())
                        .any(|window| window == PRIVATE.as_bytes())
                );
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.root.parent(), Some(std::env::temp_dir().as_path()));
        assert!(
            self.root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("kedra-activation-")
        );
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
impl Application for Fixture {
    fn observe(&self) -> Result<Settings> {
        Ok(noctalia::project(
            "5.0.1",
            &std::fs::read_to_string(self.root.join(FILE))?,
        )?)
    }
    fn stop(&self) -> Result<()> {
        if self.change_on_stop.replace(false) {
            self.set(Theme::Dark);
        }
        Ok(())
    }
    fn start(&self) -> Result<()> {
        if self.fail_start.get() {
            Err("synthetic restart failure".into())
        } else {
            Ok(())
        }
    }
}
#[test]
fn discard_keeps_pinned_selection_and_private_native_data() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let plan = fixture.plan(&store);
    apply(
        &mut store,
        INSTANCE,
        &fixture.directory(),
        &fixture,
        plan.clone(),
        &plan.id().unwrap(),
    )
    .unwrap();
    let state = load(&store, INSTANCE).unwrap().1;
    assert_eq!(fixture.observe().unwrap().theme_mode, Theme::Auto);
    assert!(state.pending_activation().is_none());
    assert_eq!(
        state.selection().unwrap()[0].after,
        noctalia::Value::Theme(Theme::Auto)
    );
    assert_eq!(state.accepted_baseline().settings.theme_mode, Theme::Dark);
    assert!(
        std::fs::read_to_string(fixture.root.join(FILE))
            .unwrap()
            .contains(PRIVATE)
    );
    fixture.audit();
}
#[test]
fn stale_plan_and_stop_time_edits_refuse_without_reserving_or_overwriting() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let plan = fixture.plan(&store);
    assert!(
        apply(
            &mut store,
            INSTANCE,
            &fixture.directory(),
            &fixture,
            plan.clone(),
            &"0".repeat(64)
        )
        .is_err()
    );
    fixture.change_on_stop.set(true);
    assert!(
        apply(
            &mut store,
            INSTANCE,
            &fixture.directory(),
            &fixture,
            plan.clone(),
            &plan.id().unwrap()
        )
        .is_err()
    );
    assert_eq!(fixture.observe().unwrap().theme_mode, Theme::Dark);
    assert!(
        load(&store, INSTANCE)
            .unwrap()
            .1
            .pending_activation()
            .is_none()
    );
    assert!(store.read(JOURNAL).unwrap().is_none());
}
#[test]
fn failed_restart_preserves_old_baseline_and_resumes_from_durable_journal() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let plan = fixture.plan(&store);
    fixture.fail_start.set(true);
    assert!(
        apply(
            &mut store,
            INSTANCE,
            &fixture.directory(),
            &fixture,
            plan.clone(),
            &plan.id().unwrap()
        )
        .is_err()
    );
    assert!(
        load(&store, INSTANCE)
            .unwrap()
            .1
            .pending_activation()
            .is_some()
    );
    drop(store);
    let mut store = Store::open(&fixture.root.join("review")).unwrap();
    fixture.fail_start.set(false);
    resume(&mut store, INSTANCE, &fixture.directory(), &fixture).unwrap();
    assert!(
        load(&store, INSTANCE)
            .unwrap()
            .1
            .pending_activation()
            .is_none()
    );
    fixture.audit();
}
#[test]
fn abort_restores_original_but_later_edits_require_explicit_keep_current() {
    for changed in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let plan = fixture.plan(&store);
        let before = load(&store, INSTANCE).unwrap().1;
        fixture.fail_start.set(true);
        assert!(
            apply(
                &mut store,
                INSTANCE,
                &fixture.directory(),
                &fixture,
                plan.clone(),
                &plan.id().unwrap()
            )
            .is_err()
        );
        fixture.fail_start.set(false);
        if changed {
            fixture.set(Theme::Dark);
        }
        let aborted = recover(
            &mut store,
            INSTANCE,
            &fixture.directory(),
            &fixture,
            Recovery::Abort,
        );
        if changed {
            assert!(aborted.is_err());
            recover(
                &mut store,
                INSTANCE,
                &fixture.directory(),
                &fixture,
                Recovery::KeepCurrent,
            )
            .unwrap();
            assert_eq!(fixture.observe().unwrap().theme_mode, Theme::Dark);
            assert_eq!(
                load(&store, INSTANCE).unwrap().1.accepted_baseline(),
                before.accepted_baseline()
            );
        } else {
            aborted.unwrap();
            assert_eq!(load(&store, INSTANCE).unwrap().1, before);
            assert_eq!(fixture.observe().unwrap().theme_mode, Theme::Light);
        }
        fixture.audit();
    }
}
#[test]
fn validated_cleanup_can_resume_after_checkpoint_removal() {
    let fixture = Fixture::new();
    let mut store = fixture.store();
    let plan = fixture.plan(&store);
    fixture.fail_start.set(true);
    assert!(
        apply(
            &mut store,
            INSTANCE,
            &fixture.directory(),
            &fixture,
            plan.clone(),
            &plan.id().unwrap()
        )
        .is_err()
    );
    let (_, _, jr, mut journal, _) = pending(&store, INSTANCE).unwrap();
    journal.phase = Phase::Validated;
    save_journal(&mut store, jr, &journal).unwrap();
    fixture
        .directory()
        .finish_validated(&journal.receipt)
        .unwrap();
    fixture.fail_start.set(false);
    resume(&mut store, INSTANCE, &fixture.directory(), &fixture).unwrap();
    assert!(
        load(&store, INSTANCE)
            .unwrap()
            .1
            .pending_activation()
            .is_none()
    );
}
