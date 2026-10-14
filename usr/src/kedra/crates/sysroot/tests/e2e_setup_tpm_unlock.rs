#![cfg(target_os = "linux")]
//! Public `sysroot setup tpm-unlock` refusals and dry runs that need no TPM, LUKS volume or
//! sudo. Enrollment itself is qualified manually in a disposable QEMU VM.
use std::path::Path;
use std::process::{Command, Output, Stdio};

const CRYPTENROLL: &str = "/usr/bin/sudo -- /usr/bin/systemd-cryptenroll";

fn tpm_unlock(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sysroot"))
        .args(["setup", "tpm-unlock"])
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn refused(output: &Output, start: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
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
    stderr
}

/// A dry run prints the exact listing and change commands for the boot volume.
fn planned(output: &Output, change: &str) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains("(dry run: nothing is run or changed)"),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("\n  {CRYPTENROLL} /dev/disk/by-uuid/")),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("\n  {CRYPTENROLL} {change} /dev/disk/by-uuid/")),
        "{stdout}"
    );
    stdout
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

    // A change needs the terminal prompts of sudo and systemd-cryptenroll; without one,
    // nothing runs.
    for arguments in [&[][..], &["--remove"], &["--replace", "--with-pin"]] {
        refused(
            &tpm_unlock(arguments),
            "sysroot: run setup tpm-unlock from an interactive terminal,",
        );
    }

    // A dry run never uses sudo; its read-only preflight decides.
    let enroll = "--tpm2-device=auto --tpm2-pcrs=7:sha256 --tpm2-public-key= --tpm2-pcrlock=";
    let output = tpm_unlock(&["--dry-run"]);
    if !Path::new("/sys/firmware/efi").exists() {
        refused(
            &output,
            "sysroot: TPM unlock requires UEFI Secure Boot because the TPM policy is bound to the Secure Boot state (PCR 7): the system was not booted through UEFI firmware.",
        );
    } else if output.status.success() {
        // Only an installed system that passes every check prints the commands.
        planned(&output, enroll);
        let output = tpm_unlock(&["--replace", "--with-pin", "--dry-run"]);
        planned(
            &output,
            &format!("{enroll} --tpm2-with-pin=yes --wipe-slot=tpm2"),
        );
    } else {
        refused(&output, "sysroot: ");
    }

    // Removal checks only the boot volume, so it works after Secure Boot or the TPM is gone.
    let output = tpm_unlock(&["--remove", "--dry-run"]);
    if output.status.success() {
        let stdout = planned(&output, "--wipe-slot=tpm2");
        assert!(!stdout.contains("--tpm2-device="), "{stdout}");
    } else {
        let stderr = refused(&output, "sysroot: ");
        assert!(
            !stderr.contains("Secure Boot") && !stderr.contains("TPM 2.0"),
            "{stderr}"
        );
        assert!(
            stderr.contains(&format!(
                "then run {CRYPTENROLL} --wipe-slot=tpm2 /dev/disk/by-uuid/<UUID>. Nothing was changed."
            )),
            "{stderr}"
        );
    }
}
