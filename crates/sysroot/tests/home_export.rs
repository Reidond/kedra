#![cfg(target_os = "linux")]
//! Actual CLI/source/SQLite round trip using only generated state and repositories.
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Output};
use sysroot_core::noctalia::{Baseline, Key, Settings, State, Theme};
use sysroot_helper::storage::Store;

const SOURCE: &str = "home/.config/noctalia/config.toml";
const BASE: &str = "[theme]\nmode='dark'\n[shell]\nbutton_borders=true\ninput_borders=true\n";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-home-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        fs::create_dir(path.join("checkout")).unwrap();
        Self(path)
    }
    fn repo(&self) -> PathBuf {
        self.0.join("checkout")
    }
    fn store(&self) -> PathBuf {
        self.0.join("review")
    }
    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let output = command
            .env("HOME", &self.0)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .arg("-C")
            .arg(self.repo())
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sysroot"))
            .env("HOME", &self.0)
            .args(["home", "--state"])
            .arg(self.store())
            .args(args)
            .output()
            .unwrap()
    }
    fn success(&self, args: &[&str]) -> serde_json::Value {
        let output = self.cli(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn offline_cli_exports_pinned_values_and_records_only_a_real_source_commit() {
    assert_ne!(
        rustix::process::geteuid().as_raw(),
        0,
        "run ordinary-user CLI tests without root"
    );
    let f = Fixture::new();
    f.git(&["init", "--template=", "-b", "fixture"]);
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
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "generated baseline"]);
    let original_commit = f.git(&["rev-parse", "HEAD"]).trim().to_owned();

    let owner = rustix::process::geteuid().as_raw();
    let machine = fs::read("/etc/machine-id").unwrap();
    let hash: String = Sha256::digest([machine, owner.to_le_bytes().to_vec()].concat())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let instance = &hash[..32];
    let baseline = Baseline {
        target: "desktop".into(),
        source_path: SOURCE.into(),
        source_revision: original_commit.clone(),
        settings: Settings {
            theme_mode: Theme::Dark,
            button_borders: true,
            input_borders: true,
        },
    };
    let mut observed = Settings {
        theme_mode: Theme::Light,
        button_borders: false,
        input_borders: false,
    };
    let mut state = State::new(instance.into(), baseline, observed).unwrap();
    state.stage(Key::ThemeMode).unwrap();
    state.ignore_exact(Key::ButtonBorders).unwrap();
    observed.theme_mode = Theme::Auto;
    state.capture(observed).unwrap();
    // Fixture initialization, not enrollment of the runner's home or live application.
    drop(Store::create(&f.store(), &[("noctalia", &state.to_bytes().unwrap())]).unwrap());
    f.write("unrelated", "existing staged edit\n");
    f.git(&["add", "unrelated"]);
    f.write("unrelated", "existing unstaged edit\n");
    let index = fs::read(f.repo().join(".git/index")).unwrap();
    let patch = f.0.join("selection.patch");
    let result = f.success(&[
        "export",
        "--repo",
        f.repo().to_str().unwrap(),
        "--output",
        patch.to_str().unwrap(),
    ]);
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["checkout_changed"], false);
    assert_eq!(result["selection"][0]["after"]["value"], "light");
    assert_eq!(
        fs::metadata(&patch).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read(f.repo().join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read_to_string(f.repo().join("unrelated")).unwrap(),
        "existing unstaged edit\n"
    );
    let cached = f.success(&["status", "--last-capture"]);
    assert_eq!(cached["fields"][0]["live"]["value"], "auto");
    f.git(&["apply", "--3way", patch.to_str().unwrap()]);
    let rejected = f.cli(&[
        "record-source",
        "--repo",
        f.repo().to_str().unwrap(),
        "--commit",
        &original_commit,
    ]);
    assert!(
        !rejected.status.success(),
        "working/index edits are not a commit receipt"
    );
    assert_eq!(
        f.success(&["selection"])["selection"][0]["after"]["value"],
        "light"
    );
    f.git(&["commit", "-m", "selected field only", "--", SOURCE]);
    let commit = f.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let recorded = f.success(&[
        "record-source",
        "--repo",
        f.repo().to_str().unwrap(),
        "--commit",
        &commit,
    ]);
    assert_eq!(recorded["source"]["recorded_commit"], commit);
    assert_eq!(recorded["source"]["deployment_performed"], false);
    assert_eq!(recorded["fields"][0]["committed_pending"]["value"], "light");
    assert_eq!(recorded["fields"][0]["live"]["value"], "auto");
    assert_eq!(recorded["fields"][1]["local_only"], true);
    assert_eq!(
        f.success(&["selection"])["selection"],
        serde_json::json!([])
    );
    assert_eq!(f.git(&["show", ":unrelated"]), "existing staged edit\n");
}

#[test]
fn ordinary_text_cli_keeps_adjacent_lines_independent_through_git_publication() {
    let f = Fixture::new();
    let path = "home/.config/niri/config.kdl";
    let base = "layout {\n    gaps 12\n    width 2\n    animation 200\n}\n";
    f.git(&["init", "--template=", "-b", "fixture"]);
    f.git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/Reidond/kedra.git",
    ]);
    f.write("hosts/desktop/host.toml", "id='desktop'\narchitecture='x86_64'\nimage='ghcr.io/reidond/kedra-desktop'\nfedora_release=44\ncandidate_target=true\nhardware_status='synthetic'\n");
    f.write("packages/common.list", "niri\n");
    f.write("packages/remove.list", "# none\n");
    f.write("hosts/desktop/packages.list", "# none\n");
    f.write(path, base);
    f.write("unrelated", "original\n");
    f.git(&["add", "."]);
    f.git(&["commit", "-m", "generated niri source baseline"]);
    let initial = f.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let owner = rustix::process::geteuid().as_raw();
    let machine = fs::read("/etc/machine-id").unwrap();
    let instance: String = Sha256::digest([machine, owner.to_le_bytes().to_vec()].concat())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    // Generated offline installed-state fixture. All assertions below use the
    // actual public CLI, ordinary file edits and real source Git operations.
    let state = serde_json::json!({"schema_version":1,"instance":&instance[..32],
        "baseline":{"target":"desktop","source_path":path,"source_revision":initial,"contents":base},
        "reference":base,"selected":[],"ignored":[],"published":[]});
    drop(
        Store::create(
            &f.store(),
            &[("niri-text", &serde_json::to_vec(&state).unwrap())],
        )
        .unwrap(),
    );
    let native = f.0.join(".config/niri/config.kdl");
    fs::create_dir_all(native.parent().unwrap()).unwrap();
    fs::write(
        &native,
        base.replace("gaps 12", "gaps 14")
            .replace("width 2", "width 3")
            .replace("animation 200", "animation 150"),
    )
    .unwrap();
    let displayed = f.success(&["file", "status"]);
    let id = |text: &str| {
        displayed["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["change"]["after"] == text)
            .unwrap()["change"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let selected = id("    width 3\n");
    let local = id("    gaps 14\n");
    f.success(&["file", "stage", &selected]);
    f.success(&["file", "keep-local", &local]);
    assert!(!f.cli(&["file", "stage", &local]).status.success());
    let later = base
        .replace("gaps 12", "gaps 14")
        .replace("width 2", "width 4")
        .replace("animation 200", "animation 160");
    fs::write(&native, &later).unwrap();
    assert_eq!(
        f.success(&["file", "selection"])["selection"][0]["after"],
        "    width 3\n"
    );
    assert!(
        !f.cli(&["file", "stage", &selected]).status.success(),
        "stale live selection must refuse"
    );
    f.write("unrelated", "staged\n");
    f.git(&["add", "unrelated"]);
    f.write("unrelated", "unstaged\n");
    let index = fs::read(f.repo().join(".git/index")).unwrap();
    let patch = f.0.join("niri.patch");
    f.success(&[
        "file",
        "export",
        "--repo",
        f.repo().to_str().unwrap(),
        "--output",
        patch.to_str().unwrap(),
    ]);
    assert_eq!(fs::read(f.repo().join(".git/index")).unwrap(), index);
    assert_eq!(fs::read_to_string(&native).unwrap(), later);
    assert_eq!(
        fs::read_to_string(f.repo().join("unrelated")).unwrap(),
        "unstaged\n"
    );
    let published_patch = fs::read_to_string(&patch).unwrap();
    assert!(
        !published_patch.contains("gaps 14")
            && !published_patch.contains("width 4")
            && !published_patch.contains("animation 160")
    );
    f.git(&["apply", "--3way", patch.to_str().unwrap()]);
    assert!(
        !f.cli(&[
            "file",
            "record-source",
            "--repo",
            f.repo().to_str().unwrap(),
            "--commit",
            &initial
        ])
        .status
        .success()
    );
    f.git(&[
        "commit",
        "-m",
        "publish only the selected niri line",
        "--",
        path,
    ]);
    let committed = f.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let recorded = f.success(&[
        "file",
        "record-source",
        "--repo",
        f.repo().to_str().unwrap(),
        "--commit",
        &committed,
    ]);
    assert_eq!(recorded["publication_count"], 1);
    assert_eq!(recorded["selection"], serde_json::json!([]));
    assert_eq!(recorded["source"]["deployment_performed"], false);
    let next = f.success(&["file", "status"]);
    assert!(
        next["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["change"]["after"] == "    gaps 14\n" && r["local_only"] == true)
    );
    assert!(
        next["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["change"]["before"] == "    width 3\n"
                && r["change"]["after"] == "    width 4\n"
                && r["visible_change"] == true)
    );
    assert_eq!(f.git(&["show", ":unrelated"]), "staged\n");
    assert_eq!(
        f.git(&["show", &format!("HEAD:{path}")]),
        base.replace("width 2", "width 3")
    );
    // Continue the same user workflow with a separate insertion, then deletion.
    let public = base.replace("width 2", "width 3");
    let appended = format!("{public}// selected note\n");
    fs::write(&native, appended.replace("gaps 12", "gaps 14")).unwrap();
    let rows = f.success(&["file", "status"]);
    let insertion = rows["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["change"]["after"] == "// selected note\n")
        .unwrap()["change"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.success(&["file", "stage", &insertion]);
    let insert_patch = f.0.join("insert.patch");
    f.success(&[
        "file",
        "export",
        "--repo",
        f.repo().to_str().unwrap(),
        "--output",
        insert_patch.to_str().unwrap(),
    ]);
    f.git(&["apply", "--3way", insert_patch.to_str().unwrap()]);
    f.git(&["commit", "-m", "selected insertion", "--", path]);
    let inserted = f.git(&["rev-parse", "HEAD"]).trim().to_owned();
    f.success(&[
        "file",
        "record-source",
        "--repo",
        f.repo().to_str().unwrap(),
        "--commit",
        &inserted,
    ]);
    assert_eq!(f.git(&["show", &format!("HEAD:{path}")]), appended);
    fs::write(
        &native,
        appended
            .replace("gaps 12", "gaps 14")
            .replace("    animation 200\n", ""),
    )
    .unwrap();
    let rows = f.success(&["file", "status"]);
    let deletion = rows["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["change"]["before"] == "    animation 200\n" && r["change"]["after"] == "")
        .unwrap()["change"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    f.success(&["file", "stage", &deletion]);
    let delete_patch = f.0.join("delete.patch");
    f.success(&[
        "file",
        "export",
        "--repo",
        f.repo().to_str().unwrap(),
        "--output",
        delete_patch.to_str().unwrap(),
    ]);
    f.git(&["apply", "--3way", delete_patch.to_str().unwrap()]);
    f.git(&["commit", "-m", "selected deletion", "--", path]);
    let deleted = f.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let recorded = f.success(&[
        "file",
        "record-source",
        "--repo",
        f.repo().to_str().unwrap(),
        "--commit",
        &deleted,
    ]);
    assert_eq!(recorded["publication_count"], 3);
    assert_eq!(recorded["accepted_baseline"]["source_revision"], initial);
    assert_eq!(
        f.git(&["show", &format!("HEAD:{path}")]),
        appended.replace("    animation 200\n", "")
    );
    // The command refuses a real symlink substitution instead of capturing it.
    let outside = f.0.join("unadopted");
    fs::write(&outside, "unadopted generated content\n").unwrap();
    fs::remove_file(&native).unwrap();
    std::os::unix::fs::symlink(&outside, &native).unwrap();
    assert!(!f.cli(&["file", "status"]).status.success());
    assert_eq!(
        fs::read_to_string(outside).unwrap(),
        "unadopted generated content\n"
    );
}
