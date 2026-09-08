#![cfg(target_os = "linux")]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn exec_preserves_exit_status_and_holds_checkout_lock_until_agent_exit() {
    let root = std::env::temp_dir().join(format!(
        "kedra-agent-exec-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("hosts/desktop")).unwrap();
    fs::write(
        repo.join("hosts/desktop/host.toml"),
        include_str!("../../../hosts/desktop/host.toml"),
    )
    .unwrap();
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.name", "Fixture"],
        vec!["config", "user.email", "fixture@example.invalid"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["config", "core.hooksPath", "/dev/null"],
        vec![
            "remote",
            "add",
            "origin",
            "https://github.com/Reidond/kedra.git",
        ],
        vec!["add", "."],
        vec!["commit", "--quiet", "-m", "fixture"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let fake = root.join("codex");
    fs::write(&fake, "#!/usr/bin/python3\nimport os,sys,pathlib,time\nif sys.argv[1:] == ['--version']:\n print('fixture-cli 9.2.1')\n sys.exit(0)\nassert 'BW_SESSION' not in os.environ\nassert os.environ['HOME'] == os.environ['EXPECTED_HOME']\nassert pathlib.Path(os.environ['CODEX_HOME']).name == '9.2.1'\nif sys.argv[1:] == ['--hold']:\n pathlib.Path(os.environ['HOME'],'started').write_text('ready')\n time.sleep(60)\nsys.exit(23)\n").unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let launch = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sysroot"));
        command
            .args(["codex", "--runtime", "user", "--executable"])
            .arg(&fake)
            .arg("--repo")
            .arg(&repo)
            .arg("--")
            .env("HOME", &root)
            .env("EXPECTED_HOME", &root)
            .env("XDG_STATE_HOME", root.join("state"))
            .env("BW_SESSION", "synthetic-never-forward")
            .stdin(Stdio::null());
        command
    };
    let mut first = launch()
        .arg("--hold")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let result = || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("started").exists() && Instant::now() < deadline {
            if first.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        assert!(root.join("started").exists(), "agent did not start");
        let second = launch().output().unwrap();
        assert_eq!(second.status.code(), Some(78));
        assert!(String::from_utf8_lossy(&second.stderr).contains("another sysroot agent"));
    };
    // Ensure the generated child cannot survive a failed assertion.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(result));
    first.kill().ok();
    first.wait().unwrap();
    if let Err(error) = outcome {
        fs::remove_dir_all(&root).unwrap();
        std::panic::resume_unwind(error);
    }
    assert_eq!(launch().output().unwrap().status.code(), Some(23));
    assert!(!root.join(".codex").exists());
    let state = root.join("state/sysroot/agents/codex/user/9.2.1/config.toml");
    assert!(fs::read_to_string(state).unwrap().contains("keyring"));
    fs::remove_dir_all(root).unwrap();
}
