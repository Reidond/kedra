use std::process::Command;

#[test]
fn no_privileged_operation_is_implemented() {
    let output = Command::new(env!("CARGO_BIN_EXE_sysroot-helper"))
        .args(["deploy", "--verified", "/tmp/untrusted"])
        .output()
        .expect("run helper");
    assert_eq!(output.status.code(), Some(78));
    assert!(output.stdout.is_empty());
}
