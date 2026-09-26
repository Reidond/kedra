#![cfg(target_os = "linux")]
//! Public `sysroot setup tpm-unlock` refusals that need no TPM, LUKS volume or sudo.
//! Enrollment itself is qualified manually in a disposable UTM VM.
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn tpm_unlock(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sysroot"))
        .args(["setup", "tpm-unlock"])
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn refused(output: &Output, start: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(78), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(stderr.starts_with(start), "{stderr}");
    assert!(
        stderr.trim_end().ends_with("Nothing was changed."),
        "{stderr}"
    );
}

#[test]
fn tpm_unlock_refuses_cleanly_before_administrator_access() {
    let output = tpm_unlock(&["--remove", "--with-pin", "--dry-run"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
    let output = tpm_unlock(&["--remove", "--replace"]);
    assert_eq!(output.status.code(), Some(2));

    if rustix::process::geteuid().is_root() {
        for arguments in [&["--dry-run"][..], &[], &["--remove"]] {
            refused(
                &tpm_unlock(arguments),
                "sysroot: run setup tpm-unlock as the ordinary owner, without sudo;",
            );
        }
        return;
    }

    // A change needs systemd-cryptenroll's terminal prompt; without one, nothing runs.
    for arguments in [&[][..], &["--remove"], &["--replace", "--with-pin"]] {
        refused(
            &tpm_unlock(arguments),
            "sysroot: run setup tpm-unlock from an interactive terminal,",
        );
    }

    // A dry run never uses sudo; its read-only preflight decides.
    let output = tpm_unlock(&["--dry-run"]);
    if !Path::new("/sys/firmware/efi").exists() {
        refused(
            &output,
            "sysroot: TPM unlock requires UEFI Secure Boot because the TPM policy is bound to the Secure Boot state (PCR 7): the system was not booted through UEFI firmware.",
        );
    } else if output.status.success() {
        // Only an installed system that passes every check prints the plan.
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("(dry run: nothing is changed"), "{stdout}");
        assert!(
            stdout
                .contains("/usr/bin/sudo -- /usr/bin/systemd-cryptenroll --tpm2-device=/dev/tpmrm"),
            "{stdout}"
        );
        assert!(
            stdout.contains(
                " --tpm2-pcrs=7:sha256 --tpm2-public-key= --tpm2-pcrlock= /dev/disk/by-uuid/"
            ),
            "{stdout}"
        );
    } else {
        refused(&output, "sysroot: ");
    }

    // Removal checks only the volume, so it stays possible after Secure Boot or the TPM is gone.
    let output = tpm_unlock(&["--remove", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        assert!(
            stdout.contains(
                "/usr/bin/sudo -- /usr/bin/systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-uuid/"
            ),
            "{stdout}"
        );
        assert!(!stdout.contains("--tpm2-device="), "{stdout}");
    } else {
        refused(&output, "sysroot: ");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("Secure Boot") && !stderr.contains("TPM 2.0"),
            "{stderr}"
        );
        assert!(
            stderr.contains(
                "then run /usr/bin/sudo -- /usr/bin/systemd-cryptenroll --wipe-slot=tpm2 /dev/disk/by-uuid/<UUID>. Nothing was changed."
            ),
            "{stderr}"
        );
    }
}
