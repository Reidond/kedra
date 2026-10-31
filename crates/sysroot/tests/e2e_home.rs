#![cfg(target_os = "linux")]
//! Public `sysroot home` over a generated image root, home directory and Kedra
//! checkout: status diffs, capture into the source path that provides a file,
//! and update's record, fast-forward, merge, check refusal, reload, conflict,
//! resolution and link-refusal paths with real Git.
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const MAIN: &str = ".config/app/main.conf";
const THEME: &str = ".config/app/theme.json";
const PLAIN: &str = ".config/tool/plain.txt";
/// Counts successful reloads; fails once while `$HOME/fail-reload` exists.
const RELOAD: &str = r#"["/bin/sh", "-c", "if [ -e \"$HOME/fail-reload\" ]; then rm \"$HOME/fail-reload\"; exit 1; fi; echo reload >> \"$HOME/reloads\""]"#;
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o022));
        loop {
            let path = std::env::temp_dir().join(format!(
                "kedra-home-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    let fixture = Self(path);
                    fs::create_dir(fixture.home()).unwrap();
                    fixture.image(
                        "usr/share/kedra/home-files.toml",
                        &format!(
                            "schema_version = 1\n\n[[file]]\npath = \"{MAIN}\"\n\
                             check = [\"/bin/sh\", \"-c\", \"! grep -q INVALID \\\"$1\\\"\", \"check\", \"{{file}}\"]\n\
                             reload = {RELOAD}\n\n[[file]]\npath = \"{THEME}\"\nreload = {RELOAD}\n\n\
                             [[file]]\npath = \"{PLAIN}\"\n"
                        ),
                    );
                    fixture.image(
                        "usr/share/sysroot/source.json",
                        r#"{"schema_version": 1, "target": {"id": "desktop"}, "files": []}"#,
                    );
                    return fixture;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", path.display()),
            }
        }
    }

    fn root(&self) -> PathBuf {
        self.0.join("root")
    }

    fn home(&self) -> PathBuf {
        self.0.join("home")
    }

    fn checkout(&self) -> PathBuf {
        self.0.join("checkout")
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn image(&self, path: &str, text: &str) {
        Self::write(&self.root().join(path), text);
    }

    fn baseline(&self, path: &str, text: &str) {
        self.image(&format!("usr/share/sysroot/home/default/{path}"), text);
    }

    fn live(&self, path: &str, text: &str) {
        Self::write(&self.home().join(path), text);
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.home().join(path)).unwrap()
    }

    fn reloads(&self) -> usize {
        fs::read_to_string(self.home().join("reloads"))
            .map(|text| text.lines().count())
            .unwrap_or(0)
    }

    fn sysroot(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sysroot"))
            .arg("home")
            .arg("--root")
            .arg(self.root())
            .args(arguments)
            .env("HOME", self.home())
            .env_remove("XDG_STATE_HOME")
            .output()
            .unwrap()
    }

    /// JSON from a command, with its expected success.
    fn json(&self, arguments: &[&str], success: bool) -> Value {
        let output = self.sysroot(arguments);
        assert_eq!(
            output.status.success(),
            success,
            "{arguments:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn git(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        command
            .env("HOME", &self.0)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .arg("-C")
            .arg(self.checkout())
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(["-c", "commit.gpgsign=false"])
            .args(arguments)
            .output()
            .unwrap()
    }
}

/// Each file's `result` from an update report, in data-file order.
fn results(report: &Value) -> Vec<&str> {
    report["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["result"].as_str().unwrap())
        .collect()
}

fn file<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["path"] == path)
        .unwrap()
}

#[test]
fn update_merges_baseline_changes_and_leaves_conflicts_beside_live_files() {
    let f = Fixture::new();
    if rustix::process::geteuid().is_root() {
        let output = f.sysroot(&["status"]);
        assert!(String::from_utf8_lossy(&output.stderr).contains("never root"));
        return;
    }
    let v1 = "alpha\nbeta\ngamma\ndelta\nepsilon\n";
    f.baseline(MAIN, v1);
    f.baseline(THEME, "{\"mode\": \"light\"}\n");
    f.baseline(PLAIN, "plain\n");
    // An account created from the image: live files equal the baseline.
    f.live(MAIN, v1);
    f.live(THEME, "{\"mode\": \"light\"}\n");
    f.live(PLAIN, "plain\n");

    let status = f.json(&["status", "--json"], true);
    for row in status["files"].as_array().unwrap() {
        assert_eq!(row["applied"], "none", "{row}");
        assert_eq!(row["live_matches_baseline"], true, "{row}");
        assert!(row.get("diff").is_none(), "{row}");
    }
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["recorded", "recorded", "recorded"]);
    assert_eq!(
        f.reloads(),
        1,
        "an unrecorded file is reloaded once before recording"
    );
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["up_to_date", "up_to_date", "up_to_date"]);

    // A local edit and a baseline change in different places merge; an
    // unchanged file takes the new baseline; one shared reload runs once.
    f.live(MAIN, "alpha\nbeta\ngamma\ndelta\nepsilon local\n");
    f.baseline(MAIN, "alpha image\nbeta\ngamma\ndelta\nepsilon\n");
    f.baseline(THEME, "{\"mode\": \"dark\"}\n");
    let status = f.json(&["status", "--json", MAIN], true);
    let row = file(&status, MAIN);
    assert_eq!(row["applied"], "outdated");
    let diff = row["diff"].as_str().unwrap();
    assert!(
        diff.contains("+alpha\n") && diff.contains("+epsilon local\n"),
        "{diff}"
    );
    assert!(
        diff.contains("--- a/baseline/.config/app/main.conf"),
        "{diff}"
    );
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["merged", "updated", "up_to_date"]);
    assert_eq!(
        f.read(MAIN),
        "alpha image\nbeta\ngamma\ndelta\nepsilon local\n"
    );
    assert_eq!(f.read(THEME), "{\"mode\": \"dark\"}\n");
    assert_eq!(f.reloads(), 2);

    // A candidate that fails its check never replaces the live file.
    f.baseline(MAIN, "alpha image\nbeta\nINVALID\ngamma\ndelta\nepsilon\n");
    let report = f.json(&["update", "--json", MAIN], false);
    assert_eq!(results(&report), ["check_failed"]);
    assert_eq!(
        f.read(MAIN),
        "alpha image\nbeta\ngamma\ndelta\nepsilon local\n"
    );
    assert_eq!(f.reloads(), 2);
    assert_eq!(
        file(&f.json(&["status", "--json"], true), MAIN)["applied"],
        "outdated"
    );
    f.baseline(MAIN, "alpha image\nbeta two\ngamma\ndelta\nepsilon\n");
    let report = f.json(&["update", "--json", MAIN], true);
    assert_eq!(results(&report), ["merged"]);
    assert_eq!(
        f.read(MAIN),
        "alpha image\nbeta two\ngamma\ndelta\nepsilon local\n"
    );
    assert_eq!(f.reloads(), 3);

    // Both sides change one line: the live file stays, the marked merge goes
    // beside it, and deleting that file marks the conflict resolved.
    f.live(
        MAIN,
        "alpha image\nbeta two\ngamma local\ndelta\nepsilon local\n",
    );
    f.baseline(MAIN, "alpha image\nbeta two\ngamma image\ndelta\nepsilon\n");
    let report = f.json(&["update", "--json"], false);
    assert_eq!(results(&report), ["conflict", "up_to_date", "up_to_date"]);
    assert_eq!(
        f.read(MAIN),
        "alpha image\nbeta two\ngamma local\ndelta\nepsilon local\n"
    );
    let conflict = f.read(&format!("{MAIN}.kedra-new"));
    for marker in [
        "<<<<<<< live\n",
        "gamma local\n",
        "||||||| last applied baseline\n",
        "gamma image\n",
        ">>>>>>> new baseline\n",
    ] {
        assert!(conflict.contains(marker), "{conflict}");
    }
    assert_eq!(
        file(&f.json(&["status", "--json"], true), MAIN)["conflict"],
        true
    );
    let report = f.json(&["update", "--json", MAIN], false);
    assert_eq!(results(&report), ["unresolved"]);
    f.live(
        MAIN,
        "alpha image\nbeta two\ngamma both\ndelta\nepsilon local\n",
    );
    fs::remove_file(f.home().join(format!("{MAIN}.kedra-new"))).unwrap();
    let report = f.json(&["update", "--json", MAIN], true);
    assert_eq!(results(&report), ["resolved"]);
    assert_eq!(
        f.read(MAIN),
        "alpha image\nbeta two\ngamma both\ndelta\nepsilon local\n"
    );
    assert_eq!(f.reloads(), 4, "a resolved file is checked and reloaded");
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["up_to_date", "up_to_date", "up_to_date"]);
    assert!(!f.read(MAIN).contains("<<<<<<<"));

    // A failed reload leaves the record outdated; the retry reloads the file
    // that is already in place before recording the baseline.
    fs::write(f.home().join("fail-reload"), "").unwrap();
    f.baseline(THEME, "{\"mode\": \"auto\"}\n");
    let report = f.json(&["update", "--json"], false);
    assert_eq!(
        results(&report),
        ["up_to_date", "reload_failed", "up_to_date"]
    );
    assert_eq!(f.read(THEME), "{\"mode\": \"auto\"}\n");
    assert_eq!(f.reloads(), 4);
    assert_eq!(
        file(&f.json(&["status", "--json"], true), THEME)["applied"],
        "outdated"
    );
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["up_to_date", "recorded", "up_to_date"]);
    assert_eq!(f.reloads(), 5, "the retry ran the reload");
    assert_eq!(
        file(&f.json(&["status", "--json"], true), THEME)["applied"],
        "current"
    );
    let report = f.json(&["update", "--json"], true);
    assert_eq!(results(&report), ["up_to_date", "up_to_date", "up_to_date"]);
    assert_eq!(f.reloads(), 5);

    // Without a recorded baseline a differing file cannot be merged: the new
    // baseline goes beside it. A missing file is created.
    let fresh = f.0.join("fresh-state");
    let fresh = fresh.to_str().unwrap();
    f.baseline(PLAIN, "plain two\n");
    fs::remove_file(f.home().join(THEME)).unwrap();
    let report = f.json(&["--state", fresh, "update", "--json", PLAIN, THEME], false);
    assert_eq!(results(&report), ["created", "conflict"]);
    assert_eq!(f.read(THEME), "{\"mode\": \"auto\"}\n");
    assert_eq!(f.read(PLAIN), "plain\n");
    assert_eq!(f.read(&format!("{PLAIN}.kedra-new")), "plain two\n");
    fs::remove_file(f.home().join(format!("{PLAIN}.kedra-new"))).unwrap();
    let report = f.json(&["--state", fresh, "update", "--json", PLAIN], true);
    assert_eq!(results(&report), ["resolved"]);
    assert_eq!(f.read(PLAIN), "plain\n");

    // Only ordinary files are managed; a link is refused and left alone.
    fs::rename(f.home().join(PLAIN), f.home().join("plain-target")).unwrap();
    std::os::unix::fs::symlink(f.home().join("plain-target"), f.home().join(PLAIN)).unwrap();
    let report = f.json(&["update", "--json", PLAIN], false);
    assert_eq!(results(&report), ["failed"]);
    assert!(
        fs::symlink_metadata(f.home().join(PLAIN))
            .unwrap()
            .is_symlink()
    );
    let output = f.sysroot(&["status", ".config/undeclared"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a managed home file"));
}

#[test]
fn capture_writes_the_providing_source_path_without_staging() {
    let f = Fixture::new();
    if rustix::process::geteuid().is_root() {
        return;
    }
    f.baseline(MAIN, "shared\n");
    f.baseline(PLAIN, "overlay\n");
    f.live(MAIN, "shared\nlocal line\n");
    f.live(PLAIN, "overlay\n");
    fs::create_dir(f.checkout()).unwrap();
    assert!(f.git(&["init", "-q"]).status.success());
    let overlay = format!("usr/src/kedra/image/targets/desktop/etc/skel/{PLAIN}");
    for (path, text) in [
        (
            "usr/src/kedra/image/targets/desktop/target.toml",
            "id = 'desktop'\n",
        ),
        (&format!("etc/skel/{MAIN}"), "shared\n"),
        (&overlay, "older overlay\n"),
    ] {
        Fixture::write(&f.checkout().join(path), text);
    }
    assert!(f.git(&["add", "-A"]).status.success());
    assert!(f.git(&["commit", "-q", "-m", "fixture"]).status.success());
    let head = f.git(&["rev-parse", "HEAD"]).stdout;
    let repo = f.checkout();
    let repo = repo.to_str().unwrap();

    let report = f.json(&["capture", MAIN, "--repo", repo, "--json"], true);
    assert_eq!(report["source_path"], format!("etc/skel/{MAIN}"));
    assert_eq!(report["written"], true);
    assert_eq!(
        fs::read_to_string(f.checkout().join("etc/skel").join(MAIN)).unwrap(),
        "shared\nlocal line\n"
    );
    let report = f.json(&["capture", PLAIN, "--repo", repo, "--json"], true);
    assert_eq!(report["source_path"], overlay);
    assert_eq!(
        fs::read_to_string(f.checkout().join(&overlay)).unwrap(),
        "overlay\n"
    );
    let report = f.json(&["capture", PLAIN, "--repo", repo, "--json"], true);
    assert_eq!(report["written"], false);

    // The owner picks lines: nothing is staged or committed.
    let status = f.git(&["status", "--porcelain"]).stdout;
    let status = String::from_utf8(status).unwrap();
    assert_eq!(
        status,
        format!(" M etc/skel/{MAIN}\n M {overlay}\n"),
        "{status}"
    );
    assert_eq!(f.git(&["rev-parse", "HEAD"]).stdout, head);

    let output = f.sysroot(&["capture", ".config/undeclared", "--repo", repo]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a managed home file"));
    // A repository of its own, so the refusal holds even when the fixture
    // lives inside another Kedra checkout (TMPDIR under its target/).
    let elsewhere = f.0.join("not-a-checkout");
    fs::create_dir(&elsewhere).unwrap();
    let init = Command::new("git")
        .args(["init", "-q", "--template="])
        .arg(&elsewhere)
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");
    let output = f.sysroot(&["capture", MAIN, "--repo", elsewhere.to_str().unwrap()]);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("is not a Kedra checkout"),
        "{output:?}"
    );
}
