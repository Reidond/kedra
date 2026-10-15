//! Post-install machine setup by the ordinary owner. Preflight is unprivileged and
//! read-only; changes run systemd-cryptenroll through sudo with explicit arguments.
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Unlock the encrypted root at boot with a TPM-sealed key bound to Secure Boot (PCR 7).
    /// The disk passphrase stays enrolled and is asked for whenever the TPM refuses.
    TpmUnlock(TpmUnlock),
}

#[derive(Args)]
struct TpmUnlock {
    /// Also require a PIN at every boot, checked by the TPM.
    #[arg(long, conflicts_with = "remove")]
    with_pin: bool,
    /// Re-seal an existing TPM enrollment after PCR 7 changed; systemd-cryptenroll wipes the
    /// old slot only after the new one succeeds. After a TPM clear use --remove, then enroll.
    #[arg(long, conflicts_with = "remove")]
    replace: bool,
    /// Remove the TPM enrollment; a passphrase slot must remain. Needs neither Secure Boot,
    /// the TPM nor the passphrase.
    #[arg(long)]
    remove: bool,
    /// Run the unprivileged checks and print the exact commands; change nothing.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let Operation::TpmUnlock(tpm_unlock) = options.command;
    #[cfg(target_os = "linux")]
    {
        linux::tpm_unlock(&tpm_unlock)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = tpm_unlock;
        Err("setup tpm-unlock requires an installed Linux Kedra system; nothing was changed".into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::TpmUnlock;
    use std::io::IsTerminal;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use sysroot_linux::firmware;

    type Error = Box<dyn std::error::Error>;

    const SUDO: &str = "/usr/bin/sudo";
    const CRYPTENROLL: &str = "/usr/bin/systemd-cryptenroll";
    /// Policy fixed to the Secure Boot state. The empty values stop systemd-cryptenroll from
    /// also binding a signed PCR 11 key or a pcrlock policy that happens to exist on disk.
    const ENROLL: [&str; 4] = [
        "--tpm2-device=auto",
        "--tpm2-pcrs=7:sha256",
        "--tpm2-public-key=",
        "--tpm2-pcrlock=",
    ];

    fn refusal(detail: impl std::fmt::Display) -> Error {
        format!("{detail}. Nothing was changed.").into()
    }

    fn sudo(arguments: &[&str]) -> Command {
        let mut command = Command::new(SUDO);
        command
            .env_remove("BW_SESSION")
            .args(["--", CRYPTENROLL])
            .args(arguments);
        command
    }

    fn shown(arguments: &[&str]) -> String {
        format!("{SUDO} -- {CRYPTENROLL} {}", arguments.join(" "))
    }

    fn secure_boot() -> Result<String, Error> {
        let observed = match firmware::secure_boot() {
            Ok(state) if state.enforced() => return Ok(state.to_string()),
            Ok(state) => state.to_string(),
            Err(error) => error.to_string(),
        };
        Err(format!(
            "TPM unlock requires UEFI Secure Boot because the TPM policy is bound to the Secure Boot state (PCR 7): {observed}. {} Nothing was changed.",
            firmware::GUIDANCE
        )
        .into())
    }

    /// The TPM `--tpm2-device=auto` uses must show a firmware measurement in SHA-256 PCR 7.
    /// Sealing to a reset value (all zero or all one bits) would bind the key to no Secure Boot
    /// state at all. drivers/char/tpm/tpm-sysfs.c exports the bank 0444, in uppercase hex.
    fn measured_pcr7() -> Result<String, Error> {
        let refuse = |detail: String| {
            refusal(format!(
                "TPM unlock requires one TPM 2.0 whose SHA-256 PCR 7 holds the firmware's Secure Boot measurement: {detail}"
            ))
        };
        let entries = std::fs::read_dir("/sys/class/tpm").map_err(|e| {
            refuse(if e.kind() == std::io::ErrorKind::NotFound {
                "no TPM is visible in /sys/class/tpm".into()
            } else {
                format!("/sys/class/tpm is unreadable ({e})")
            })
        })?;
        let banks: Vec<(String, String)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let value = std::fs::read_to_string(entry.path().join("pcr-sha256/7")).ok()?;
                let name = entry.file_name().to_string_lossy().into_owned();
                Some((name, value.trim().to_ascii_uppercase()))
            })
            .collect();
        let [(name, value)] = banks.as_slice() else {
            return Err(refuse(format!(
                "{} TPMs expose a SHA-256 PCR 7, not exactly one",
                banks.len()
            )));
        };
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(refuse(format!("{name} reports a malformed PCR 7 value")));
        }
        if value.bytes().all(|b| b == b'0') || value.bytes().all(|b| b == b'F') {
            return Err(refuse(format!(
                "PCR 7 of {name} is at its reset value, so the firmware did not measure the Secure Boot state"
            )));
        }
        Ok(name.clone())
    }

    /// The LUKS volume the image's initramfs unlocks at boot. It has no /etc/crypttab, so the
    /// only rd.luks.uuid on the kernel command line names the volume TPM unlock applies to.
    fn boot_volume() -> Result<String, Error> {
        let by_hand = format!(
            "; to remove TPM key slots by hand, take the UUID of the root's crypto_LUKS container from lsblk -f, check with {} that a password slot remains, then run {}",
            shown(&["/dev/disk/by-uuid/<UUID>"]),
            shown(&["--wipe-slot=tpm2", "/dev/disk/by-uuid/<UUID>"])
        );
        let cmdline = std::fs::read_to_string("/proc/cmdline")
            .map_err(|e| refusal(format!("/proc/cmdline is unreadable: {e}")))?;
        let mut uuids: Vec<String> = cmdline
            .split_ascii_whitespace()
            .filter_map(|word| word.strip_prefix("rd.luks.uuid="))
            .map(|uuid| {
                uuid.strip_prefix("luks-")
                    .unwrap_or(uuid)
                    .to_ascii_lowercase()
            })
            .collect();
        uuids.sort();
        uuids.dedup();
        let [uuid] = uuids.as_slice() else {
            return Err(refusal(format!(
                "the kernel command line names {} LUKS volumes (rd.luks.uuid), not exactly one{by_hand}",
                uuids.len()
            )));
        };
        let path = format!("/dev/disk/by-uuid/{uuid}");
        if !uuid.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') || !Path::new(&path).exists() {
            return Err(refusal(format!(
                "rd.luks.uuid={uuid} names no volume at {path}{by_hand}"
            )));
        }
        Ok(path)
    }

    /// Key slot types, from the `SLOT TYPE` table systemd-cryptenroll prints for a volume.
    fn slot_types(volume: &str) -> Result<Vec<String>, Error> {
        let output = sudo(&[volume])
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| refusal(format!("sudo could not run: {e}")))?;
        if !output.status.success() {
            return Err(refusal(format!(
                "listing the key slots failed ({})",
                output.status
            )));
        }
        let malformed = || refusal("the key slot list is malformed");
        let text = String::from_utf8(output.stdout).map_err(|_| malformed())?;
        // An empty list has no header; "No slots found." goes to stderr.
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        if lines
            .next()
            .is_some_and(|header| header.split_whitespace().ne(["SLOT", "TYPE"]))
        {
            return Err(malformed());
        }
        lines
            .map(
                |line| match line.split_whitespace().collect::<Vec<_>>()[..] {
                    [index, kind] if index.parse::<u8>().is_ok() => Ok(kind.to_owned()),
                    _ => Err(malformed()),
                },
            )
            .collect()
    }

    /// Refuse a change that could leave no passphrase or that would add a second TPM slot.
    /// Returns false when there is nothing to remove.
    fn check_slots(options: &TpmUnlock, types: &[String]) -> Result<bool, Error> {
        let count = |kind: &str| types.iter().filter(|t| *t == kind).count();
        println!("Key slots: {}", types.join(", "));
        if count("password") == 0 {
            return Err(refusal(
                "the volume has no passphrase slot, which must remain as the fallback when the TPM refuses; add one first with sudo systemd-cryptenroll --password <volume>",
            ));
        }
        let tpm = count("tpm2");
        if options.remove && tpm == 0 {
            println!("No TPM key slot is enrolled; nothing was changed.");
            return Ok(false);
        }
        if !options.remove && !options.replace && tpm > 0 {
            return Err(refusal(
                "a TPM key slot is already enrolled; use --replace to re-seal it or --remove to delete it",
            ));
        }
        Ok(true)
    }

    fn print_result(options: &TpmUnlock, volume: &str) {
        if options.remove {
            println!(
                "TPM unlock is removed from {volume}; every boot asks for the passphrase again."
            );
            return;
        }
        println!(
            "TPM unlock is enrolled on {volume}; the passphrase stays enrolled. Reboot to use it{}.",
            if options.with_pin {
                ": enter the TPM PIN at the \"LUKS2 token PIN\" prompt"
            } else {
                ""
            }
        );
        if options.replace && !options.with_pin {
            println!(
                "If systemd-cryptenroll said \"This PCR set is already enrolled\", it kept the old slot. If boot still asks for the passphrase, the TPM was cleared: run sysroot setup tpm-unlock --remove, then sysroot setup tpm-unlock."
            );
        }
        println!("Undo: sysroot setup tpm-unlock --remove");
    }

    pub(super) fn tpm_unlock(options: &TpmUnlock) -> Result<(), Error> {
        use rustix::process::{geteuid, getuid};
        if getuid().is_root() || geteuid() != getuid() {
            return Err(refusal(
                "run setup tpm-unlock as the ordinary owner, without sudo; it uses sudo only for systemd-cryptenroll",
            ));
        }
        if !options.dry_run && !std::io::stdin().is_terminal() {
            return Err(refusal(
                "run setup tpm-unlock from an interactive terminal, where sudo and systemd-cryptenroll can ask for your password and the disk passphrase (--dry-run only prints the commands)",
            ));
        }
        // Wiping TPM slots needs neither Secure Boot nor the TPM, so revocation always works.
        let tpm = (!options.remove).then(measured_pcr7).transpose()?;
        let secure_boot = (!options.remove).then(secure_boot).transpose()?;
        let volume = boot_volume()?;
        let mut change: Vec<&str> = if options.remove {
            vec!["--wipe-slot=tpm2"]
        } else {
            ENROLL.to_vec()
        };
        if options.with_pin {
            change.push("--tpm2-with-pin=yes");
        }
        // systemd-cryptenroll wipes old slots only after the new enrollment succeeds.
        if options.replace {
            change.push("--wipe-slot=tpm2");
        }
        change.push(&volume);

        if let (Some(state), Some(tpm)) = (secure_boot, tpm) {
            println!("Secure Boot: {state}");
            println!("TPM:         {tpm}, SHA-256 PCR 7 measured");
        }
        println!("Volume:      {volume} (rd.luks.uuid on the kernel command line)");
        println!(
            "Commands{}:",
            if options.dry_run {
                " (dry run: nothing is run or changed)"
            } else {
                ""
            }
        );
        println!("  {}", shown(&[&volume]));
        println!("  {}", shown(&change));
        if options.dry_run || !check_slots(options, &slot_types(&volume)?)? {
            return Ok(());
        }
        if !options.remove {
            eprintln!(
                "sysroot: systemd-cryptenroll now asks for the current disk passphrase{}",
                if options.with_pin {
                    ", then the new TPM PIN twice"
                } else {
                    ""
                }
            );
        }
        let status = sudo(&change)
            .status()
            .map_err(|e| refusal(format!("sudo could not run: {e}")))?;
        if !status.success() {
            return Err(format!(
                "sudo systemd-cryptenroll failed ({status}). Inspect the key slots with: {}",
                shown(&[&volume])
            )
            .into());
        }
        print_result(options, &volume);
        Ok(())
    }
}
