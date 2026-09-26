//! Read an already-owned regular inode without traversing path symlinks.
use rustix::fs::{self, FileType, Mode, OFlags, ResolveFlags};
use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub fn read(path: &Path, owner: u32, limit: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let descriptor = fs::openat2(
        fs::CWD,
        path,
        OFlags::PATH | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?;
    let before = fs::fstat(&descriptor)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_uid != owner
        || before.st_nlink != 1
        || before.st_mode & 0o022 != 0
        || before.st_size < 0
        || before.st_size as u64 > limit as u64
    {
        return Err("unsafe ownership, type, links, size or mode of trusted input".into());
    }
    let mut bytes = Vec::new();
    File::open(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    let after = fs::fstat(&descriptor)?;
    let named = std::fs::symlink_metadata(path)?;
    if bytes.len() > limit
        || bytes.len() as u64 != before.st_size as u64
        || before.st_dev != named.dev()
        || before.st_ino != named.ino()
        || before.st_mode != after.st_mode
        || before.st_uid != after.st_uid
        || before.st_gid != after.st_gid
        || before.st_size != after.st_size
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
        || after.st_nlink != 1
    {
        return Err("trusted input changed while being read".into());
    }
    Ok(bytes)
}
