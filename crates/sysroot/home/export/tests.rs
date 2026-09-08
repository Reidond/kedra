use super::*;
use std::fs;
use std::process::Command as Process;

const INSTANCE: &str = "0123456789abcdef0123456789abcdef";
const SOURCE: &str = "home/.config/noctalia/config.toml";
const BASE: &str = "# retain this header\n[theme]\nmode = 'dark' # owner comment\n[shell]\nbutton_borders = true\ninput_borders = true\n[bar.default]\nposition = 'top'\n";
const PRIVATE: &str = "synthetic-runtime-secret-not-for-source";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-export-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let f = Self(path);
        f.git(&["init", "--template=", "-b", "main"]);
        f.git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/Reidond/kedra.git",
        ]);
        f.write("hosts/desktop/host.toml", "id='desktop'\narchitecture='x86_64'\nimage='ghcr.io/reidond/kedra-desktop'\nfedora_release=44\ncandidate_target=true\nhardware_status='synthetic'\n");
        f.write("packages/common.list", "noctalia\n");
        f.write("packages/remove.list", "# none\n");
        f.write("hosts/desktop/packages.list", "# none\n");
        f.write(SOURCE, BASE);
        f.write("unrelated", "original\n");
        f.commit();
        f
    }
    fn git(&self, args: &[&str]) -> String {
        String::from_utf8(self.git_bytes(args)).unwrap()
    }
    fn git_bytes(&self, args: &[&str]) -> Vec<u8> {
        let mut command = Process::new("git");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let output = command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .arg("-C")
            .arg(&self.0)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
                "-c",
                "gc.auto=0",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "-m", "generated fixture"]);
    }
    fn revision(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().into()
    }
    fn review(&self) -> State {
        let baseline = Baseline {
            target: "desktop".into(),
            source_path: SOURCE.into(),
            source_revision: self.revision(),
            settings: noctalia::project(noctalia::APP_VERSION, BASE).unwrap(),
        };
        let mut live = baseline.settings;
        live.theme_mode = Theme::Light;
        live.button_borders = false;
        live.input_borders = false;
        let mut state = State::new(INSTANCE.into(), baseline, live).unwrap();
        state.stage(Key::ThemeMode).unwrap();
        state.ignore_exact(Key::ButtonBorders).unwrap();
        live.theme_mode = Theme::Auto;
        state.capture(live).unwrap();
        state
    }
    fn scratch(&self) -> PathBuf {
        let path = self.0.join("private-scratch");
        fs::create_dir(&path).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn selected_patch_preserves_dirty_checkout_and_excludes_other_live_values() {
    let f = Fixture::new();
    let state = f.review();
    let before_state = state.to_bytes().unwrap();
    f.write("unrelated", "staged user edit\n");
    f.git(&["add", "unrelated"]);
    f.write("unrelated", "later unstaged edit\n");
    f.write(SOURCE, &BASE.replace("dark", "auto"));
    f.write("untracked", PRIVATE);
    let index = fs::read(f.0.join(".git/index")).unwrap();
    let head = f.revision();
    let working = fs::read(f.0.join(SOURCE)).unwrap();
    let result = prepare(&f.0, &f.scratch(), &state).unwrap();
    let patch = String::from_utf8(result.patch.clone()).unwrap();
    assert_eq!(result.source_revision, head);
    assert_eq!(result.source_path, SOURCE);
    assert!(patch.contains("+mode = \"light\" # owner comment"));
    assert!(!patch.contains("mode = \"auto\""));
    assert!(!patch.contains("button_borders = false"));
    assert!(!patch.contains("input_borders = false"));
    assert!(!patch.contains(PRIVATE));
    assert_eq!(fs::read(f.0.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(f.0.join(SOURCE)).unwrap(), working);
    assert_eq!(
        fs::read_to_string(f.0.join("unrelated")).unwrap(),
        "later unstaged edit\n"
    );
    assert_eq!(f.revision(), head);
    assert_eq!(state.to_bytes().unwrap(), before_state);
    let objects = f.git_bytes(&["cat-file", "--batch-all-objects", "--batch"]);
    assert!(
        !objects
            .windows(PRIVATE.len())
            .any(|bytes| bytes == PRIVATE.as_bytes()),
        "untracked/private bytes must not enter any source object"
    );
    assert!(
        !objects
            .windows(INSTANCE.len())
            .any(|bytes| bytes == INSTANCE.as_bytes()),
        "private review identity must not enter source objects"
    );
    let output = f.0.join("selected.patch");
    result.write_new(&output).unwrap();
    assert!(result.write_new(&output).is_err());
    assert_eq!(fs::read(output).unwrap(), result.patch);
}

#[test]
fn patch_applies_with_git_and_commit_receipt_keeps_newer_live_values() {
    let f = Fixture::new();
    let mut state = f.review();
    let accepted = state.accepted_baseline().clone();
    let prepared = prepare(&f.0, &f.scratch(), &state).unwrap();
    let patch = f.0.join("selected.patch");
    prepared.write_new(&patch).unwrap();
    f.git(&["apply", "--3way", patch.to_str().unwrap()]);
    // Commit only the approved source file; the patch/scratch stay untracked.
    f.git(&["commit", "-m", "selected theme", "--", SOURCE]);
    let commit = f.revision();
    let settings = verify_commit(&f.0, &commit, &state).unwrap();
    assert_eq!(settings.theme_mode, Theme::Light);
    assert!(settings.button_borders && settings.input_borders);
    state.record_source_commit(&commit, settings).unwrap();
    assert!(state.selection().unwrap().is_empty());
    assert_eq!(state.accepted_baseline(), &accepted);
    let rows = state.rows().unwrap();
    assert_eq!(rows[0].live, Value::Theme(Theme::Auto));
    assert_eq!(rows[0].committed_pending, Some(Value::Theme(Theme::Light)));
    assert!(rows[0].visible_change);
    assert!(rows[1].local_only);
}

#[test]
fn newer_nonoverlapping_source_is_preserved_and_same_field_conflict_refuses() {
    let f = Fixture::new();
    let state = f.review();
    f.write(
        SOURCE,
        &BASE.replace("position = 'top'", "position = 'bottom'"),
    );
    f.commit();
    let root = f.scratch();
    let prepared = prepare(&f.0, &root, &state).unwrap();
    assert_eq!(prepared.source_revision, f.revision());
    let patch = f.0.join("chosen.patch");
    prepared.write_new(&patch).unwrap();
    f.git(&["apply", "--3way", patch.to_str().unwrap()]);
    let source_text = fs::read_to_string(f.0.join(SOURCE)).unwrap();
    assert!(source_text.contains("position = 'bottom'"));
    assert!(source_text.contains("# retain this header"));
    // A separate generated source history changes the selected field instead.
    let conflict = Fixture::new();
    let state = conflict.review();
    conflict.write(SOURCE, &BASE.replace("dark", "auto"));
    conflict.commit();
    let index = fs::read(conflict.0.join(".git/index")).unwrap();
    assert!(matches!(
        prepare(&conflict.0, &conflict.scratch(), &state),
        Err(Error::Conflict(Key::ThemeMode))
    ));
    assert_eq!(fs::read(conflict.0.join(".git/index")).unwrap(), index);
}

#[test]
fn already_committed_selection_produces_empty_patch_and_records_existing_commit() {
    let f = Fixture::new();
    let mut state = f.review();
    f.write(SOURCE, &BASE.replace("dark", "light"));
    f.commit();
    let prepared = prepare(&f.0, &f.scratch(), &state).unwrap();
    assert!(prepared.patch.is_empty());
    let settings = verify_commit(&f.0, &f.revision(), &state).unwrap();
    state.record_source_commit(&f.revision(), settings).unwrap();
    assert!(state.selection().unwrap().is_empty());
}

#[test]
fn wrong_origin_override_drift_and_uncommitted_receipts_fail() {
    let f = Fixture::new();
    let state = f.review();
    let root = f.scratch();
    assert!(
        verify_commit(&f.0, &f.revision(), &state).is_err(),
        "HEAD does not contain selected light"
    );
    for value in ["HEAD", "--help", "a", "a/b"] {
        assert!(verify_commit(&f.0, value, &state).is_err());
    }
    f.git(&[
        "remote",
        "set-url",
        "origin",
        "https://example.invalid/other.git",
    ]);
    assert!(prepare(&f.0, &root, &state).is_err());
    f.git(&[
        "remote",
        "set-url",
        "origin",
        "https://github.com/Reidond/kedra.git",
    ]);
    f.write("hosts/desktop/home/.config/noctalia/config.toml", BASE);
    f.commit();
    assert!(
        prepare(&f.0, &root, &state).is_err(),
        "host override must not silently replace shared provenance"
    );
}

#[test]
fn receipt_uses_exact_retained_commit_after_newer_source_advances() {
    let f = Fixture::new();
    let mut state = f.review();
    f.write(SOURCE, &BASE.replace("dark", "light"));
    f.commit();
    let published = f.revision();
    f.write(SOURCE, &BASE.replace("dark", "auto"));
    f.commit();
    let head = f.revision();
    let index = fs::read(f.0.join(".git/index")).unwrap();
    let settings = verify_commit(&f.0, &published, &state).unwrap();
    state.record_source_commit(&published, settings).unwrap();
    assert_eq!(
        state.rows().unwrap()[0].committed_pending,
        Some(Value::Theme(Theme::Light))
    );
    assert_eq!(f.revision(), head);
    assert_eq!(fs::read(f.0.join(".git/index")).unwrap(), index);
}

#[test]
fn missing_baseline_and_receipt_rollback_cannot_fabricate_publication() {
    let f = Fixture::new();
    let initial = f.revision();
    let mut state = f.review();
    let mut unanchored = state.accepted_baseline().clone();
    unanchored.source_revision = "f".repeat(40);
    let mut missing = State::new(
        INSTANCE.into(),
        unanchored,
        Settings {
            theme_mode: Theme::Light,
            button_borders: true,
            input_borders: true,
        },
    )
    .unwrap();
    missing.stage(Key::ThemeMode).unwrap();
    assert!(prepare(&f.0, &f.scratch(), &missing).is_err());
    f.write(SOURCE, &BASE.replace("dark", "light"));
    f.commit();
    let published = f.revision();
    let settings = verify_commit(&f.0, &published, &state).unwrap();
    state.record_source_commit(&published, settings).unwrap();
    let live = Settings {
        theme_mode: Theme::Dark,
        button_borders: false,
        input_borders: false,
    };
    state.capture(live).unwrap();
    state.stage(Key::ThemeMode).unwrap();
    let before = state.to_bytes().unwrap();
    assert!(
        verify_commit(&f.0, &initial, &state).is_err(),
        "old baseline cannot stand in for a new revert commit"
    );
    assert_eq!(state.to_bytes().unwrap(), before);
    f.write(SOURCE, BASE);
    f.commit();
    assert_eq!(
        verify_commit(&f.0, &f.revision(), &state)
            .unwrap()
            .theme_mode,
        Theme::Dark
    );
}

#[test]
fn host_owned_source_stays_in_the_host_namespace() {
    let f = Fixture::new();
    let host_path = "hosts/desktop/home/.config/noctalia/config.toml";
    f.write(host_path, BASE);
    f.commit();
    let baseline = Baseline {
        target: "desktop".into(),
        source_path: host_path.into(),
        source_revision: f.revision(),
        settings: noctalia::project(noctalia::APP_VERSION, BASE).unwrap(),
    };
    let mut live = baseline.settings;
    live.theme_mode = Theme::Light;
    let mut state = State::new(INSTANCE.into(), baseline, live).unwrap();
    state.stage(Key::ThemeMode).unwrap();
    let prepared = prepare(&f.0, &f.scratch(), &state).unwrap();
    assert_eq!(prepared.source_path, host_path);
    let patch = String::from_utf8(prepared.patch).unwrap();
    assert!(patch.contains(&format!("+++ b/{host_path}")));
    assert!(!patch.contains("+++ b/home/"));
    assert_eq!(fs::read_to_string(f.0.join(SOURCE)).unwrap(), BASE);
}
