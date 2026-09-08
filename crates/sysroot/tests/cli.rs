use std::process::Command;

#[test]
fn status_distinguishes_source_planning_from_deployment() {
    let output = Command::new(env!("CARGO_BIN_EXE_sysroot"))
        .args(["status", "--json"])
        .output()
        .expect("run sysroot");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 status");
    assert_eq!(stdout.trim(), sysroot_core::STATUS_JSON);
    assert!(output.stderr.is_empty());
}

#[test]
fn operational_commands_fail_closed() {
    for command in ["update", "deploy", "rollback", "home", "setup"] {
        let output = Command::new(env!("CARGO_BIN_EXE_sysroot"))
            .arg(command)
            .output()
            .expect("run sysroot");
        assert_eq!(output.status.code(), Some(78), "{command}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented"));
    }
}

#[test]
fn unknown_arguments_are_not_ignored() {
    let output = Command::new(env!("CARGO_BIN_EXE_sysroot"))
        .args(["status", "--force"])
        .output()
        .expect("run sysroot");
    assert_eq!(output.status.code(), Some(2));
}
