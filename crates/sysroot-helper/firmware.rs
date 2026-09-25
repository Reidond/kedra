//! Bounded read-only UEFI Secure Boot observations shared by doctor and installer media.
use std::fmt;
use std::fs::File;
use std::io::Read;

const FIRMWARE: &str = "/sys/firmware/efi";
const SECURE_BOOT: &str =
    "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";
const SETUP_MODE: &str = "/sys/firmware/efi/efivars/SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c";
const LOCKDOWN: &str = "/sys/kernel/security/lockdown";

/// Operator guidance for a missing or disabled Secure Boot state.
pub const GUIDANCE: &str = "Enable UEFI Secure Boot with the Microsoft third-party UEFI CA allowed in the firmware settings (on UTM, enable UEFI boot and TPM); see docs/INSTALL.md.";

#[derive(Debug)]
pub enum Error {
    NotUefi,
    Unavailable(&'static str, std::io::Error),
    Malformed(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUefi => write!(f, "the system was not booted through UEFI firmware"),
            Self::Unavailable(name, e) => write!(f, "UEFI {name} is unavailable: {e}"),
            Self::Malformed(name) => {
                write!(f, "UEFI {name} is malformed or not on efivarfs")
            }
        }
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecureBoot {
    pub enabled: bool,
    pub setup_mode: bool,
}
impl SecureBoot {
    /// Firmware verifies boot images only with Secure Boot on and a platform key enrolled.
    pub fn enforced(self) -> bool {
        self.enabled && !self.setup_mode
    }
}
impl fmt::Display for SecureBoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.setup_mode {
            "UEFI firmware is in Secure Boot setup mode without an enrolled platform key"
        } else if self.enabled {
            "UEFI Secure Boot is enabled"
        } else {
            "UEFI Secure Boot is disabled"
        })
    }
}

fn flag(path: &str, name: &'static str) -> Result<bool, Error> {
    let file = File::open(path).map_err(|e| Error::Unavailable(name, e))?;
    // linux/magic.h EFIVARFS_MAGIC: refuse look-alike files outside efivarfs.
    if rustix::fs::fstatfs(&file)
        .map_err(|e| Error::Unavailable(name, e.into()))?
        .f_type
        != 0xde5e_81e4
    {
        return Err(Error::Malformed(name));
    }
    // Four attribute bytes, then the one-byte value; anything longer is refused.
    let mut bytes = Vec::with_capacity(6);
    file.take(6)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::Unavailable(name, e))?;
    match bytes.as_slice() {
        [_, _, _, _, 0] => Ok(false),
        [_, _, _, _, 1] => Ok(true),
        _ => Err(Error::Malformed(name)),
    }
}

/// Read the global SecureBoot and SetupMode variables. No firmware state is changed.
pub fn secure_boot() -> Result<SecureBoot, Error> {
    match std::fs::metadata(FIRMWARE) {
        Ok(metadata) if metadata.is_dir() => (),
        Ok(_) => return Err(Error::NotUefi),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::NotUefi),
        Err(e) => return Err(Error::Unavailable("firmware interface", e)),
    }
    Ok(SecureBoot {
        enabled: flag(SECURE_BOOT, "SecureBoot variable")?,
        setup_mode: flag(SETUP_MODE, "SetupMode variable")?,
    })
}

/// Informational active kernel lockdown mode, when securityfs exposes it to the caller.
pub fn lockdown() -> Option<String> {
    let mut text = String::new();
    File::open(LOCKDOWN)
        .ok()?
        .take(128)
        .read_to_string(&mut text)
        .ok()?;
    let mode = text.split_once('[')?.1.split_once(']')?.0;
    matches!(mode, "none" | "integrity" | "confidentiality").then(|| mode.to_owned())
}
