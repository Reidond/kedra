//! Post-install machine setup by the ordinary owner. Preflight is unprivileged and
//! read-only; changes run fixed system tools through sudo with explicit arguments.
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
    /// Re-seal an existing TPM enrollment after PCR 7 changed; the old slot is wiped only after
    /// the new one succeeds. An unchanged PCR 7 without a PIN keeps the old slot, so after a TPM
    /// clear use --remove and then enroll again.
    #[arg(long, conflicts_with = "remove")]
    replace: bool,
    /// Remove the TPM enrollment; a passphrase slot must remain. Needs neither Secure Boot,
    /// the TPM nor the passphrase.
    #[arg(long)]
    remove: bool,
    /// Run the unprivileged checks and print the plan and exact commands; change nothing.
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
    use serde::Deserialize;
    use std::fmt;
    use std::fs::File;
    use std::io::{IsTerminal, Read};
    use std::os::unix::fs::FileTypeExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use sysroot_helper::firmware;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    const SUDO: &str = "/usr/bin/sudo";
    const CRYPTENROLL: &str = "/usr/bin/systemd-cryptenroll";
    const TIMEOUT: &str = "/usr/bin/timeout";
    const LSBLK: &str = "/usr/bin/lsblk";
    /// Mounts that must all depend on the enrolled volume (bootc: physical root and state).
    const BACKED: [&str; 3] = ["/", "/sysroot", "/var"];
    /// Slot types printed by systemd-cryptenroll 259 (src/cryptenroll/cryptenroll-list.c).
    const SLOT_TYPES: [&str; 7] = [
        "password", "recovery", "pkcs11", "fido2", "tpm2", "other", "conflict",
    ];

    #[derive(Debug)]
    enum Refusal {
        Privileged,
        Terminal,
        SecureBoot(String),
        Tpm(String),
        Volume(String),
        BootUnlock(String),
        /// A tool failed before anything could change.
        Preflight(&'static str, String),
        Slots(String),
        /// The change command failed; the detail says what to inspect.
        Change(String),
        /// The change ran; the key slots afterwards are unreadable or not as expected.
        Verification(String),
        /// `--replace` without a PIN matched the existing PCR 7 policy, so nothing was re-sealed.
        Kept(u8),
    }
    impl fmt::Display for Refusal {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Privileged => f.write_str(
                    "run setup tpm-unlock as the ordinary owner, without sudo; it uses sudo only for systemd-cryptenroll. Nothing was changed.",
                ),
                Self::Terminal => f.write_str(
                    "run setup tpm-unlock from an interactive terminal, where sudo and systemd-cryptenroll can ask for your password and the current disk passphrase (--dry-run only prints the plan). Nothing was changed.",
                ),
                Self::SecureBoot(observed) => write!(
                    f,
                    "TPM unlock requires UEFI Secure Boot because the TPM policy is bound to the Secure Boot state (PCR 7): {observed}. {} Nothing was changed.",
                    firmware::GUIDANCE
                ),
                Self::Tpm(detail) => write!(
                    f,
                    "TPM unlock requires exactly one TPM 2.0 device with a measured PCR 7: {detail}. Nothing was changed."
                ),
                Self::Volume(detail) => write!(
                    f,
                    "cannot identify exactly one LUKS2 volume holding the root and /var filesystems: {detail}. Nothing was changed."
                ),
                Self::BootUnlock(detail) => write!(
                    f,
                    "the kernel command line does not match the boot unlock this command supports: {detail}. Nothing was changed."
                ),
                Self::Preflight(tool, detail) => {
                    write!(f, "{tool} failed: {detail}. Nothing was changed.")
                }
                Self::Slots(detail) => f.write_str(detail),
                Self::Change(detail) => write!(f, "sudo systemd-cryptenroll failed: {detail}"),
                Self::Verification(detail) => {
                    write!(f, "systemd-cryptenroll reported success, but {detail}")
                }
                Self::Kept(slot) => write!(
                    f,
                    "systemd-cryptenroll kept TPM key slot {slot} instead of re-sealing it, because it already holds an enrollment for the current PCR 7 value without a PIN. If boot asked for the passphrase although PCR 7 is unchanged, the TPM was cleared or its state replaced (UTM: Data/tpmdata) and this slot can no longer be unsealed: run sysroot setup tpm-unlock --remove, then sysroot setup tpm-unlock."
                ),
            }
        }
    }
    impl std::error::Error for Refusal {}

    struct Tpm {
        device: PathBuf,
    }

    /// What the unprivileged preflight established for the selected operation.
    enum Checked {
        /// Enrolling or replacing seals a key, so the TPM, Secure Boot and boot unlock matter.
        Enroll {
            secure_boot: String,
            tpm: Tpm,
            other_volumes: Vec<String>,
        },
        /// Wiping TPM slots needs neither the TPM, the volume key nor the Secure Boot state
        /// (systemd v259 cryptenroll.c `run()`), so revocation works after either is gone.
        Remove,
    }

    struct Volume {
        uuid: String,
        container: String,
        mapping: String,
        mounts: Vec<String>,
        path: PathBuf,
    }

    #[derive(Deserialize)]
    struct Devices {
        blockdevices: Vec<Device>,
    }
    #[derive(Deserialize)]
    struct Device {
        name: String,
        #[serde(rename = "type")]
        kind: String,
        fstype: Option<String>,
        fsver: Option<String>,
        uuid: Option<String>,
        #[serde(default)]
        mountpoints: Vec<Option<String>>,
        #[serde(default)]
        children: Vec<Device>,
    }
    struct Backing<'a> {
        mountpoint: &'a str,
        chain: Vec<&'a Device>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Slot {
        index: u8,
        kind: &'static str,
    }

    fn read_small(path: &Path, limit: usize) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "content exceeds its size bound",
            ));
        }
        String::from_utf8(bytes).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "content is not UTF-8")
        })
    }

    /// Read a bounded stdout; stderr stays visible, and the child is reaped on every path.
    fn capture(command: &mut Command, limit: usize) -> Result<Vec<u8>> {
        let mut child = command.stdout(Stdio::piped()).spawn()?;
        let result = (|| -> Result<Vec<u8>> {
            let mut bytes = Vec::new();
            child
                .stdout
                .take()
                .ok_or("output is unavailable")?
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                return Err("output exceeded its bound".into());
            }
            let status = child.wait()?;
            if !status.success() {
                return Err(format!("exited with {status}").into());
            }
            Ok(bytes)
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }

    fn secure_boot() -> std::result::Result<String, Refusal> {
        match firmware::secure_boot() {
            Ok(state) if state.enforced() => Ok(state.to_string()),
            Ok(state) => Err(Refusal::SecureBoot(state.to_string())),
            Err(error) => Err(Refusal::SecureBoot(error.to_string())),
        }
    }

    fn tpm() -> std::result::Result<Tpm, Refusal> {
        let class = Path::new("/sys/class/tpm");
        let entries = std::fs::read_dir(class).map_err(|e| {
            Refusal::Tpm(if e.kind() == std::io::ErrorKind::NotFound {
                "no TPM is visible in /sys/class/tpm".into()
            } else {
                format!("/sys/class/tpm is unreadable: {e}")
            })
        })?;
        let mut tpm2 = Vec::new();
        for (count, entry) in entries.enumerate() {
            if count >= 16 {
                return Err(Refusal::Tpm("more than 16 TPM entries are listed".into()));
            }
            let entry =
                entry.map_err(|e| Refusal::Tpm(format!("/sys/class/tpm is unreadable: {e}")))?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let numbered = name.strip_prefix("tpm").is_some_and(|number| {
                (1..=3).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit())
            });
            // TPM 1.2 chips, and chips in firmware-upgrade mode, have no version 2 attribute.
            if numbered
                && read_small(&class.join(&name).join("tpm_version_major"), 8)
                    .is_ok_and(|major| major.trim_end() == "2")
            {
                tpm2.push(name);
            }
        }
        let name = match tpm2.as_slice() {
            [name] => name.clone(),
            [] => return Err(Refusal::Tpm("no TPM 2.0 device is present".into())),
            more => {
                return Err(Refusal::Tpm(format!(
                    "{} TPM 2.0 devices are present ({}); this command does not choose one",
                    more.len(),
                    more.join(", ")
                )));
            }
        };
        let device = PathBuf::from(format!("/dev/{}", name.replacen("tpm", "tpmrm", 1)));
        if !std::fs::metadata(&device).is_ok_and(|m| m.file_type().is_char_device()) {
            return Err(Refusal::Tpm(format!(
                "the resource-manager device {} is missing",
                device.display()
            )));
        }
        // drivers/char/tpm/tpm-sysfs.c: 0444 per allocated bank, uppercase hexadecimal.
        let pcr = read_small(&class.join(&name).join("pcr-sha256/7"), 128).map_err(|e| {
            Refusal::Tpm(format!(
                "PCR 7 of the SHA-256 bank is unreadable ({e}); the TPM may have no SHA-256 bank"
            ))
        })?;
        let pcr = pcr.trim_end_matches('\n').to_ascii_uppercase();
        if pcr.len() != 64 || !pcr.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Refusal::Tpm("the PCR 7 value is malformed".into()));
        }
        // All zero: never extended since reset; all one: the reset value of some PCRs.
        if pcr.bytes().all(|b| b == b'0') || pcr.bytes().all(|b| b == b'F') {
            return Err(Refusal::Tpm(
                "PCR 7 (SHA-256) is unset, so the firmware did not measure the Secure Boot state and a PCR 7 policy would not protect the key".into(),
            ));
        }
        Ok(Tpm { device })
    }

    /// Split like the kernel: whitespace separates words except inside double quotes.
    fn kernel_words(text: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut word = String::new();
        let mut quoted = false;
        for character in text.chars() {
            match character {
                '"' => quoted = !quoted,
                c if c.is_ascii_whitespace() && !quoted => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
                c => word.push(c),
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
        words
    }

    /// LUKS UUIDs the image-built initramfs unlocks from the kernel command line.
    fn boot_unlock() -> std::result::Result<Vec<String>, Refusal> {
        let text = read_small(Path::new("/proc/cmdline"), 65_536)
            .map_err(|e| Refusal::BootUnlock(format!("/proc/cmdline is unreadable: {e}")))?;
        let mut uuids = Vec::new();
        let mut overrides = Vec::new();
        for word in kernel_words(&text) {
            let (key, value) = word.split_once('=').unwrap_or((word.as_str(), ""));
            match key {
                "rd.luks.uuid" | "luks.uuid" => uuids.push(
                    value
                        .strip_prefix("luks-")
                        .unwrap_or(value)
                        .to_ascii_lowercase(),
                ),
                "rd.luks.name" | "luks.name" => uuids.push(
                    value
                        .split_once('=')
                        .map_or(value, |(uuid, _)| uuid)
                        .to_ascii_lowercase(),
                ),
                "rd.luks.key" | "luks.key" | "rd.luks.options" | "luks.options" => {
                    overrides.push(key.to_owned())
                }
                _ => (),
            }
        }
        if !overrides.is_empty() {
            overrides.sort();
            overrides.dedup();
            return Err(Refusal::BootUnlock(format!(
                "it sets {}; a key file or explicit options can stop systemd-cryptsetup from trying TPM tokens, so review them and enroll manually",
                overrides.join(", ")
            )));
        }
        uuids.sort();
        uuids.dedup();
        Ok(uuids)
    }

    fn backing<'a>(device: &'a Device, chain: &mut Vec<&'a Device>, out: &mut Vec<Backing<'a>>) {
        chain.push(device);
        for mountpoint in device.mountpoints.iter().flatten() {
            if BACKED.contains(&mountpoint.as_str()) {
                out.push(Backing {
                    mountpoint,
                    chain: chain.clone(),
                });
            }
        }
        for child in &device.children {
            backing(child, chain, out);
        }
        chain.pop();
    }

    fn valid_uuid(value: &str) -> bool {
        value.len() == 36
            && value.bytes().enumerate().all(|(index, b)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
    }

    fn volume() -> std::result::Result<Volume, Refusal> {
        let bytes = capture(
            Command::new(TIMEOUT)
                .args(["--kill-after=2s", "10s", LSBLK])
                .args(["--json", "--paths", "--output"])
                .arg("NAME,TYPE,FSTYPE,FSVER,UUID,MOUNTPOINTS")
                .stdin(Stdio::null()),
            4 << 20,
        )
        .map_err(|e| Refusal::Preflight("lsblk", e.to_string()))?;
        let devices: Devices = serde_json::from_slice(&bytes)
            .map_err(|_| Refusal::Preflight("lsblk", "its JSON output is malformed".into()))?;
        let mut found = Vec::new();
        for device in &devices.blockdevices {
            backing(device, &mut Vec::new(), &mut found);
        }
        if !found
            .iter()
            .any(|b| matches!(b.mountpoint, "/" | "/sysroot"))
        {
            return Err(Refusal::Volume(
                "no block device holds / or /sysroot".into(),
            ));
        }
        let mut volumes: Vec<Volume> = Vec::new();
        for item in found {
            let holder = item.chain[item.chain.len() - 1];
            let Some(crypt) = item.chain.iter().rposition(|d| d.kind == "crypt") else {
                return Err(Refusal::Volume(format!(
                    "{} is on {}, which is not an encrypted volume",
                    item.mountpoint, holder.name
                )));
            };
            let Some(container) = crypt.checked_sub(1).map(|index| item.chain[index]) else {
                return Err(Refusal::Volume(format!(
                    "{} has no visible LUKS container",
                    item.chain[crypt].name
                )));
            };
            if container.fstype.as_deref() != Some("crypto_LUKS") {
                return Err(Refusal::Volume(format!(
                    "{} is on {}, which is not LUKS",
                    item.mountpoint, container.name
                )));
            }
            if container.fsver.as_deref() != Some("2") {
                return Err(Refusal::Volume(format!(
                    "{} is LUKS version {}; TPM tokens need LUKS2",
                    container.name,
                    container.fsver.as_deref().unwrap_or("unknown")
                )));
            }
            let uuid = container
                .uuid
                .as_deref()
                .filter(|uuid| valid_uuid(uuid))
                .ok_or_else(|| {
                    Refusal::Volume(format!(
                        "{} has no valid lowercase LUKS UUID",
                        container.name
                    ))
                })?;
            match volumes.iter_mut().find(|v| v.uuid == uuid) {
                Some(volume) => volume.mounts.push(item.mountpoint.to_owned()),
                None => volumes.push(Volume {
                    uuid: uuid.to_owned(),
                    container: container.name.clone(),
                    mapping: item.chain[crypt].name.clone(),
                    mounts: vec![item.mountpoint.to_owned()],
                    path: PathBuf::from(format!("/dev/disk/by-uuid/{uuid}")),
                }),
            }
        }
        let mut volume = match volumes.len() {
            1 => volumes.remove(0),
            count => {
                return Err(Refusal::Volume(format!(
                    "they depend on {count} LUKS volumes ({}); this command enrolls exactly one and does not choose",
                    volumes
                        .iter()
                        .map(|v| v.container.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        };
        volume.mounts.sort();
        volume.mounts.dedup();
        let resolved = std::fs::canonicalize(&volume.path).map_err(|e| {
            Refusal::Volume(format!("{} is unavailable: {e}", volume.path.display()))
        })?;
        let container = std::fs::canonicalize(&volume.container)
            .map_err(|e| Refusal::Volume(format!("{} is unavailable: {e}", volume.container)))?;
        if resolved != container {
            return Err(Refusal::Volume(format!(
                "{} resolves to {}, not {}",
                volume.path.display(),
                resolved.display(),
                container.display()
            )));
        }
        Ok(volume)
    }

    fn enroll_arguments(tpm: &Tpm, volume: &Volume, options: &TpmUnlock) -> Vec<String> {
        let mut arguments = vec![
            format!("--tpm2-device={}", tpm.device.display()),
            "--tpm2-pcrs=7:sha256".to_owned(),
            // Empty values stop automatic use of a signed-policy key or pcrlock file.
            "--tpm2-public-key=".to_owned(),
            "--tpm2-pcrlock=".to_owned(),
        ];
        if options.with_pin {
            arguments.push("--tpm2-with-pin=yes".to_owned());
        }
        if options.replace {
            arguments.push("--wipe-slot=tpm2".to_owned());
        }
        arguments.push(volume.path.display().to_string());
        arguments
    }

    fn change_arguments(checked: &Checked, volume: &Volume, options: &TpmUnlock) -> Vec<String> {
        match checked {
            Checked::Enroll { tpm, .. } => enroll_arguments(tpm, volume, options),
            Checked::Remove => vec![
                "--wipe-slot=tpm2".to_owned(),
                volume.path.display().to_string(),
            ],
        }
    }

    /// A removal that cannot identify the volume names the manual commands instead.
    fn by_hand(refusal: Refusal) -> Refusal {
        let path = "/dev/disk/by-uuid/<UUID>".to_owned();
        let hint = format!(
            "; to remove TPM key slots by hand, take the UUID of the root's crypto_LUKS container from lsblk -f, list its key slots with {}, confirm that a password slot remains, then run {}",
            shown(std::slice::from_ref(&path)),
            shown(&["--wipe-slot=tpm2".to_owned(), path])
        );
        match refusal {
            Refusal::Volume(detail) => Refusal::Volume(detail + &hint),
            Refusal::Preflight(tool, detail) => Refusal::Preflight(tool, detail + &hint),
            other => other,
        }
    }

    fn sudo(arguments: &[String]) -> Command {
        let mut command = Command::new(SUDO);
        command
            .env_remove("BW_SESSION")
            .args(["--", CRYPTENROLL])
            .args(arguments);
        command
    }

    fn shown(arguments: &[String]) -> String {
        let mut line = format!("{SUDO} -- {CRYPTENROLL}");
        for argument in arguments {
            line.push(' ');
            line.push_str(argument);
        }
        line
    }

    /// Errors are details; the caller knows whether a change preceded the listing.
    fn parse_slots(text: &str) -> std::result::Result<Vec<Slot>, String> {
        let malformed = || "its key slot list is malformed".to_owned();
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        // "No slots found." goes to stderr; an empty list has no header.
        let Some(header) = lines.next() else {
            return Ok(Vec::new());
        };
        if header.split_whitespace().ne(["SLOT", "TYPE"]) {
            return Err(malformed());
        }
        let mut slots: Vec<Slot> = Vec::new();
        for line in lines {
            let mut fields = line.split_whitespace();
            let (Some(index), Some(kind), None) = (fields.next(), fields.next(), fields.next())
            else {
                return Err(malformed());
            };
            let index: u8 = index.parse().map_err(|_| malformed())?;
            let kind = SLOT_TYPES
                .iter()
                .find(|known| **known == kind)
                .copied()
                .ok_or_else(malformed)?;
            if index >= 32 || slots.iter().any(|slot| slot.index == index) {
                return Err(malformed());
            }
            slots.push(Slot { index, kind });
        }
        Ok(slots)
    }

    fn slots(volume: &Volume) -> std::result::Result<Vec<Slot>, String> {
        let bytes = capture(
            sudo(&[volume.path.display().to_string()]).stdin(Stdio::inherit()),
            65_536,
        )
        .map_err(|e| e.to_string())?;
        let text =
            String::from_utf8(bytes).map_err(|_| "its key slot list is not UTF-8".to_owned())?;
        parse_slots(&text)
    }

    fn describe(slots: &[Slot]) -> String {
        if slots.is_empty() {
            return "none".into();
        }
        slots
            .iter()
            .map(|slot| format!("{} {}", slot.index, slot.kind))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn of_kind(slots: &[Slot], kind: &str) -> Vec<u8> {
        slots
            .iter()
            .filter(|slot| slot.kind == kind)
            .map(|slot| slot.index)
            .collect()
    }

    fn numbers(values: &[u8]) -> String {
        values
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn print_plan(options: &TpmUnlock, checked: &Checked, volume: &Volume) {
        let list = shown(&[volume.path.display().to_string()]);
        let change = change_arguments(checked, volume, options);
        println!(
            "Kedra TPM disk unlock{}",
            if options.dry_run {
                " (dry run: nothing is changed and administrator access is not requested)"
            } else {
                ""
            }
        );
        println!();
        println!("Checked without administrator access:");
        if let Checked::Enroll {
            secure_boot, tpm, ..
        } = checked
        {
            println!("  Secure Boot:  {secure_boot}");
            println!(
                "  TPM:          {} (TPM 2.0; PCR 7 in the SHA-256 bank is measured)",
                tpm.device.display()
            );
        }
        println!(
            "  Volume:       {} = {} (LUKS2), opened as {} for {}",
            volume.path.display(),
            volume.container,
            volume.mapping,
            volume.mounts.join(", ")
        );
        match checked {
            Checked::Enroll { other_volumes, .. } => {
                println!(
                    "  Boot unlock:  rd.luks.uuid on the kernel command line; the initramfs tries enrolled TPM tokens before asking for the passphrase"
                );
                if !other_volumes.is_empty() {
                    println!(
                        "  Not changed:  other volumes unlocked at boot ({}) keep asking for their passphrase",
                        other_volumes.join(", ")
                    );
                }
            }
            Checked::Remove => println!(
                "  Not checked:  Secure Boot, the TPM and the kernel command line; wiping TPM slots needs none of them"
            ),
        }
        println!();
        println!("Plan:");
        if options.remove {
            println!(
                "  1. Read the volume's key slots (administrator access). Stop unless a passphrase slot exists."
            );
            println!("  2. Wipe every TPM key slot. This does not ask for the passphrase.");
            println!(
                "  3. Read the key slots again and confirm that no TPM slot remains and the other slots are unchanged."
            );
        } else {
            println!(
                "  1. Read the volume's key slots (administrator access). Stop unless a passphrase slot exists{}.",
                if options.replace {
                    ""
                } else {
                    ", or if a TPM slot already exists (use --replace)"
                }
            );
            println!(
                "  2. systemd-cryptenroll asks for the current disk passphrase{}, seals a new random key in the TPM bound to PCR 7{} and adds it as a new LUKS2 key slot{}.",
                if options.with_pin {
                    " and a new TPM PIN (twice)"
                } else {
                    ""
                },
                if options.with_pin { " and the PIN" } else { "" },
                if options.replace {
                    "; only then does it wipe the previous TPM slot"
                } else {
                    ""
                }
            );
            if options.replace && !options.with_pin {
                println!(
                    "     If the existing TPM slot already matches the current PCR 7 value without a PIN, systemd-cryptenroll keeps it and seals nothing; sysroot then reports that the slot was kept."
                );
            }
            println!(
                "  3. Read the key slots again and confirm exactly one new TPM slot and unchanged other slots."
            );
        }
        println!();
        println!("Commands:");
        println!("  {list}");
        println!("  {}", shown(&change));
        println!("  {list}");
        if !options.remove {
            println!(
                "  The empty --tpm2-public-key= and --tpm2-pcrlock= stop systemd-cryptenroll from adding a signed or pcrlock policy found on disk."
            );
        }
        println!();
        if options.remove {
            println!("After removal, every boot asks for the disk passphrase again.");
        } else {
            println!(
                "Your disk passphrase stays enrolled. Boot asks for it whenever the TPM refuses. After Secure Boot, firmware key (db/dbx) or shim SBAT changes, log in and run: sysroot setup tpm-unlock --replace"
            );
            println!(
                "After a TPM clear (UTM: a new Data/tpmdata), PCR 7 is unchanged and --replace keeps the old slot; run sysroot setup tpm-unlock --remove, then sysroot setup tpm-unlock."
            );
            if options.with_pin {
                println!(
                    "At boot, enter the TPM PIN at the \"LUKS2 token PIN\" prompt. A wrong PIN asks again and counts toward the TPM's dictionary-attack lockout."
                );
            }
            println!(
                "Undo: sysroot setup tpm-unlock --remove (it works even when the TPM or Secure Boot is off)"
            );
            println!();
            println!("Security scope:");
            println!(
                "  - PCR 7 records the Secure Boot state, the firmware key databases (PK, KEK, db, dbx) and the certificates that approved shim, GRUB and the kernel. Kedra image updates replace the kernel and initramfs, which PCR 7 does not cover, so they are not expected to require enrolling again."
            );
            println!(
                "  - The initramfs and kernel command line are not signed or part of PCR 7, and GRUB has no password. Someone at this machine's console can boot an edited command line and get a root shell once the TPM unlocks the disk."
            );
            if options.with_pin {
                println!(
                    "  - The PIN is required at every boot, so the TPM alone does not unlock the disk."
                );
            } else {
                println!(
                    "  - Without --with-pin, this protects the disk when it is separated from this machine, not a stolen machine."
                );
            }
            println!(
                "  - In a virtual machine the host keeps the TPM state (UTM: Data/tpmdata in the VM bundle). Whoever can read it can recover the key without the passphrase, the PCR state or a PIN."
            );
        }
    }

    /// Enrollment preflight: Secure Boot, the TPM, the volume and its boot unlock.
    fn check_enroll() -> std::result::Result<(Checked, Volume), Refusal> {
        let secure_boot = secure_boot()?;
        let tpm = tpm()?;
        let volume = volume()?;
        let unlocked = boot_unlock()?;
        if !unlocked.contains(&volume.uuid) {
            return Err(Refusal::BootUnlock(format!(
                "it has no rd.luks.uuid={} (the image-built initramfs has no /etc/crypttab, so that argument drives the boot unlock)",
                volume.uuid
            )));
        }
        let other_volumes = unlocked
            .into_iter()
            .filter(|uuid| *uuid != volume.uuid)
            .collect();
        Ok((
            Checked::Enroll {
                secure_boot,
                tpm,
                other_volumes,
            },
            volume,
        ))
    }

    pub(super) fn tpm_unlock(options: &TpmUnlock) -> Result<()> {
        if rustix::process::getuid().as_raw() == 0
            || rustix::process::geteuid() != rustix::process::getuid()
        {
            return Err(Refusal::Privileged.into());
        }
        if !options.dry_run && !std::io::stdin().is_terminal() {
            return Err(Refusal::Terminal.into());
        }
        let (checked, volume) = if options.remove {
            (Checked::Remove, volume().map_err(by_hand)?)
        } else {
            check_enroll()?
        };
        print_plan(options, &checked, &volume);
        if options.dry_run {
            return Ok(());
        }

        let list = shown(&[volume.path.display().to_string()]);
        println!();
        eprintln!("sysroot: reading the key slots through sudo");
        let before = slots(&volume).map_err(|detail| {
            Refusal::Preflight("sudo systemd-cryptenroll (listing key slots)", detail)
        })?;
        println!("Key slots before: {}", describe(&before));
        let tpm_before = of_kind(&before, "tpm2");
        let passphrases = of_kind(&before, "password");
        if options.remove && tpm_before.is_empty() {
            println!("No TPM key slot is enrolled; nothing was changed.");
            return Ok(());
        }
        if passphrases.is_empty() {
            return Err(Refusal::Slots(format!(
                "the volume has no passphrase slot, so {}; add a passphrase first (sudo systemd-cryptenroll --password <volume>). Nothing was changed.",
                if options.remove {
                    "removing the TPM slots could leave no way to unlock it"
                } else {
                    "there would be no fallback when the TPM refuses"
                }
            ))
            .into());
        }
        if !options.remove && !options.replace && !tpm_before.is_empty() {
            return Err(Refusal::Slots(format!(
                "key slot {} already holds a TPM enrollment; use --replace to re-seal it or --remove to delete it. Nothing was changed.",
                numbers(&tpm_before)
            ))
            .into());
        }
        let change = change_arguments(&checked, &volume, options);
        if options.remove {
            eprintln!("sysroot: wiping the TPM key slots through sudo");
        } else {
            eprintln!(
                "sysroot: systemd-cryptenroll now asks for the current disk passphrase{}",
                if options.with_pin {
                    ", then the new TPM PIN twice"
                } else {
                    ""
                }
            );
        }
        // A spawn failure means neither sudo nor systemd-cryptenroll ran.
        let status = sudo(&change)
            .status()
            .map_err(|e| Refusal::Preflight("sudo systemd-cryptenroll", e.to_string()))?;
        if !status.success() {
            return Err(Refusal::Change(format!(
                "exited with {status}{}. Inspect the key slots with: {list}",
                if options.remove {
                    "; it wipes slot by slot, so some TPM slots may already be gone"
                } else if options.replace {
                    "; it wipes old TPM slots only after a successful enrollment"
                } else {
                    ""
                }
            ))
            .into());
        }
        eprintln!("sysroot: reading the key slots again through sudo");
        let after = slots(&volume).map_err(|detail| {
            Refusal::Verification(format!(
                "the key slots could not be read again ({detail}), so the change is applied but not verified. Inspect them with: {list}"
            ))
        })?;
        println!("Key slots after: {}", describe(&after));
        let others = |slots: &[Slot]| -> Vec<Slot> {
            slots
                .iter()
                .filter(|slot| slot.kind != "tpm2")
                .copied()
                .collect()
        };
        let inspect = format!(". Inspect them with: {list}");
        if others(&before) != others(&after) {
            return Err(Refusal::Verification(format!(
                "non-TPM slots changed from [{}] to [{}]{inspect}",
                describe(&others(&before)),
                describe(&others(&after))
            ))
            .into());
        }
        let tpm_after = of_kind(&after, "tpm2");
        if options.remove {
            if !tpm_after.is_empty() {
                return Err(Refusal::Verification(format!(
                    "TPM slot {} remains{inspect}",
                    numbers(&tpm_after)
                ))
                .into());
            }
            println!(
                "TPM unlock is removed from {}. Passphrase slot {} is unchanged; every boot asks for it again.",
                volume.path.display(),
                numbers(&passphrases)
            );
            return Ok(());
        }
        let [slot] = tpm_after.as_slice() else {
            return Err(Refusal::Verification(format!(
                "the key slots show TPM slots [{}] instead of exactly one{inspect}",
                numbers(&tpm_after)
            ))
            .into());
        };
        // A new enrollment takes a free slot while the old ones still exist; systemd-cryptenroll
        // 259 returns the existing slot unchanged when its PCR policy matches and no PIN is set.
        if tpm_before.contains(slot) {
            return Err(Refusal::Kept(*slot).into());
        }
        println!(
            "TPM unlock is enrolled in key slot {slot} of {}. Passphrase slot {} is unchanged.",
            volume.path.display(),
            numbers(&passphrases)
        );
        println!(
            "Reboot to use it{}. If the passphrase prompt appears, the TPM refused this boot state; type the passphrase.",
            if options.with_pin {
                ": the boot asks for the TPM PIN instead of the passphrase"
            } else {
                ": the passphrase prompt should not appear"
            }
        );
        println!("Undo: sysroot setup tpm-unlock --remove");
        Ok(())
    }
}
