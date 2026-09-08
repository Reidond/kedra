//! Stream downloaded parts into a new ISO, exposing it only after verification.
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use sysroot_core::release::{VerifiedRelease, verify_artifact};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        // Only our fixed temporary member; no recursive cleanup of user paths.
        let _ = std::fs::remove_file(self.0.join("installer.partial"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

fn scratch(parent: &Path) -> Result<Scratch> {
    for _ in 0..128 {
        let path = parent.join(format!(
            ".kedra-assemble-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => return Ok(Scratch(path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err("could not reserve an installer assembly directory".into())
}

fn part(path: &Path) -> Result<File> {
    if !std::fs::symlink_metadata(path)?.is_file() {
        return Err("installer parts must be ordinary files, not links or devices".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err("installer part changed type before opening".into());
    }
    Ok(file)
}

pub fn assemble(
    release: &VerifiedRelease,
    output_dir: &Path,
    parts: &[PathBuf],
) -> Result<PathBuf> {
    if parts.is_empty() || parts.len() > 64 {
        return Err("provide between one and 64 installer parts in release order".into());
    }
    let parent = output_dir.canonicalize()?;
    if !parent.is_dir() {
        return Err("installer output directory must already exist".into());
    }
    let output = parent.join(&release.release().installer.filename);
    if output.try_exists()? {
        return Err("installer output already exists; it will not be replaced".into());
    }
    let scratch = scratch(&parent)?;
    let temporary = scratch.0.join("installer.partial");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    let mut remaining = release.release().installer.size_bytes;
    let mut buffer = [0u8; 131_072];
    for path in parts {
        let mut input = part(path)?;
        if input.metadata()?.len() == 0 {
            return Err("empty installer part refused".into());
        }
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            if count as u64 > remaining {
                return Err("installer parts exceed the signed size".into());
            }
            file.write_all(&buffer[..count])?;
            remaining -= count as u64;
        }
    }
    if remaining != 0 {
        return Err("installer parts are incomplete".into());
    }
    file.sync_all()?;
    file.seek(SeekFrom::Start(0))?;
    verify_artifact(release, &mut file)?;
    // Creating a hard link is atomic and refuses an existing destination on
    // Windows and Linux. A failed assembly never publishes an unverified ISO.
    std::fs::hard_link(&temporary, &output)?;
    #[cfg(unix)]
    File::open(&parent)?.sync_all()?;
    drop(file);
    drop(scratch);
    Ok(output)
}
