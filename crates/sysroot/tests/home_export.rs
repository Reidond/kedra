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
        Store::open(&f.store())
            .unwrap()
            .read("noctalia")
            .unwrap()
            .unwrap()
            .revision,
        1
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
