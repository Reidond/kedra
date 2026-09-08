//! Private, owner-checked SQLite state with explicit initialization and CAS writes.
//!
//! The containing directory is a trust boundary: root callers use a fixed,
//! root-controlled path; user callers use their own private state directory.
//! This is not isolation from a malicious process with the same effective UID.
use std::fmt;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, Stat};
use sha2::{Digest, Sha256};

const DATABASE: &str = "state.sqlite";
const MAX_RECORD: usize = 1024 * 1024;
const SCHEMA: i64 = 1;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    UnsafeFile,
    UnsupportedSchema(i64),
    Corrupt,
    Conflict,
    InvalidInput,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "state I/O: {e}"),
            Self::Sqlite(e) => write!(f, "state database: {e}"),
            Self::UnsafeFile => f.write_str(
                "state directory/file ownership, permissions, type or identity is unsafe",
            ),
            Self::UnsupportedSchema(v) => {
                write!(f, "unsupported state database schema {v}; do not reset it")
            }
            Self::Corrupt => f.write_str("state data is corrupt; recovery is required"),
            Self::Conflict => f.write_str("state changed since it was read; retry after review"),
            Self::InvalidInput => f.write_str("state record name, size or revision is invalid"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rustix::io::Errno> for Error {
    fn from(e: rustix::io::Errno) -> Self {
        Self::Io(e.into())
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub revision: u64,
    pub bytes: Vec<u8>,
}
pub struct Store {
    connection: Connection,
    directory: OwnedFd,
    file: File,
    owner: u32,
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}
fn check(stat: &Stat, owner: u32, directory: bool) -> Result<(), Error> {
    let expected = if directory {
        FileType::Directory
    } else {
        FileType::RegularFile
    };
    let mode = if directory { 0o700 } else { 0o600 };
    if stat.st_uid != owner
        || FileType::from_raw_mode(stat.st_mode) != expected
        || stat.st_mode & 0o7777 != mode
        || !directory && stat.st_nlink != 1
    {
        return Err(Error::UnsafeFile);
    }
    Ok(())
}
fn open_file(directory: &OwnedFd, name: &str, owner: u32) -> Result<File, Error> {
    let fd = fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )?;
    check(&fs::fstat(&fd)?, owner, false)?;
    Ok(File::from(fd))
}
fn decode(revision: i64, bytes: Vec<u8>, checksum: String) -> Result<Record, Error> {
    if revision <= 0 || bytes.len() > MAX_RECORD || hash(&bytes) != checksum {
        return Err(Error::Corrupt);
    }
    Ok(Record {
        revision: revision as u64,
        bytes,
    })
}
fn get(connection: &Connection, name: &str) -> Result<Option<Record>, Error> {
    let row = connection
        .query_row(
            "SELECT revision, body, sha256 FROM records WHERE name=?1",
            [name],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    row.map(|(revision, bytes, checksum)| decode(revision, bytes, checksum))
        .transpose()
}

impl Store {
    /// Initialization requires a new directory and at least one initial record.
    /// A pre-existing or incomplete store is never overwritten as a fresh one.
    pub fn create(path: &Path, initial: &[(&str, &[u8])]) -> Result<Self, Error> {
        if initial.is_empty()
            || initial
                .iter()
                .any(|(name, bytes)| !name_valid(name) || bytes.len() > MAX_RECORD)
        {
            return Err(Error::InvalidInput);
        }
        std::fs::DirBuilder::new().mode(0o700).create(path)?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path.join(DATABASE))?;
        file.sync_all()?;
        let mut store = Self::connect(path)?;
        store.configure_writes()?;
        let transaction = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch("CREATE TABLE records(name TEXT PRIMARY KEY, revision INTEGER NOT NULL CHECK(revision>0), body BLOB NOT NULL, sha256 TEXT NOT NULL CHECK(length(sha256)=64)) STRICT;
            CREATE TABLE history(name TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), body BLOB NOT NULL, sha256 TEXT NOT NULL CHECK(length(sha256)=64), PRIMARY KEY(name,revision)) STRICT;
            PRAGMA user_version=1;")?;
        for (name, bytes) in initial {
            transaction.execute(
                "INSERT INTO records VALUES(?1,1,?2,?3)",
                params![name, bytes, hash(bytes)],
            )?;
        }
        transaction.commit()?;
        fs::fsync(&store.directory)?;
        store.validate_schema()?;
        Ok(store)
    }
    /// Open only an existing valid database. No CREATE flag or empty-state fallback.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let store = Self::connect(path)?;
        store.validate_schema()?;
        store.configure_writes()?;
        Ok(store)
    }
    fn connect(path: &Path) -> Result<Self, Error> {
        let owner = rustix::process::geteuid().as_raw();
        let directory = fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )?;
        check(&fs::fstat(&directory)?, owner, true)?;
        let file = open_file(&directory, DATABASE, owner)?;
        // Validate existing SQLite sidecars before SQLite is allowed to open them.
        for name in [
            "state.sqlite-wal",
            "state.sqlite-shm",
            "state.sqlite-journal",
        ] {
            match open_file(&directory, name, owner) {
                Ok(_) => (),
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE;
        let connection = Connection::open_with_flags(path.join(DATABASE), flags)?;
        let store = Self {
            connection,
            directory,
            file,
            owner,
        };
        store.verify_identity()?;
        store.connection.busy_timeout(Duration::ZERO)?;
        store.connection.set_limit(
            rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
            (MAX_RECORD + 8192) as i32,
        )?;
        store
            .connection
            .set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_SQL_LENGTH, 16_384)?;
        store
            .connection
            .set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_ATTACHED, 0)?;
        store
            .connection
            .execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON;")?;
        Ok(store)
    }
    fn configure_writes(&self) -> Result<(), Error> {
        self.connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        Ok(())
    }
    fn verify_identity(&self) -> Result<(), Error> {
        check(&fs::fstat(&self.directory)?, self.owner, true)?;
        let opened = fs::fstat(&self.file)?;
        let named = fs::statat(&self.directory, DATABASE, AtFlags::SYMLINK_NOFOLLOW)?;
        check(&opened, self.owner, false)?;
        check(&named, self.owner, false)?;
        if opened.st_dev != named.st_dev || opened.st_ino != named.st_ino {
            return Err(Error::UnsafeFile);
        }
        Ok(())
    }
    fn validate_schema(&self) -> Result<(), Error> {
        self.verify_identity()?;
        let version: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != SCHEMA {
            return Err(Error::UnsupportedSchema(version));
        }
        let integrity: String = self
            .connection
            .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))?;
        if integrity != "ok" {
            return Err(Error::Corrupt);
        }
        let unexpected: i64 = self.connection.query_row("SELECT count(*) FROM sqlite_schema WHERE type IN ('trigger','view') OR (type='table' AND name NOT IN ('records','history')) OR (type='index' AND name NOT LIKE 'sqlite_autoindex_%')", [], |r| r.get(0))?;
        if unexpected != 0 {
            return Err(Error::Corrupt);
        }
        // Preparation also rejects missing/replaced tables or columns.
        self.connection
            .prepare("SELECT name,revision,body,sha256 FROM records LIMIT 0")?;
        self.connection
            .prepare("SELECT name,revision,body,sha256 FROM history LIMIT 0")?;
        Ok(())
    }
    pub fn read(&self, name: &str) -> Result<Option<Record>, Error> {
        if !name_valid(name) {
            return Err(Error::InvalidInput);
        }
        self.verify_identity()?;
        get(&self.connection, name)
    }
    /// Atomically retain the previous record and replace exactly the read revision.
    /// Contending writers fail or observe a revision conflict instead of overwriting.
    pub fn compare_exchange(
        &mut self,
        name: &str,
        expected: Option<u64>,
        bytes: &[u8],
    ) -> Result<Record, Error> {
        self.compare_exchange_batch(&[(name, expected, bytes)])?
            .pop()
            .ok_or(Error::InvalidInput)
    }
    /// Commit related journal/state records in one SQLite transaction, or none.
    pub fn compare_exchange_batch(
        &mut self,
        changes: &[(&str, Option<u64>, &[u8])],
    ) -> Result<Vec<Record>, Error> {
        let mut names = std::collections::BTreeSet::new();
        if changes.is_empty()
            || changes.len() > 16
            || changes.iter().any(|(name, _, bytes)| {
                !name_valid(name) || bytes.len() > MAX_RECORD || !names.insert(*name)
            })
        {
            return Err(Error::InvalidInput);
        }
        self.verify_identity()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut originals = Vec::new();
        for &(name, expected, _) in changes {
            let current = get(&transaction, name)?;
            if current.as_ref().map(|r| r.revision) != expected {
                return Err(Error::Conflict);
            }
            originals.push(current);
        }
        let mut results = Vec::new();
        for ((name, expected, bytes), current) in changes.iter().copied().zip(originals) {
            let revision = expected
                .unwrap_or(0)
                .checked_add(1)
                .filter(|v| *v <= i64::MAX as u64)
                .ok_or(Error::InvalidInput)?;
            if let Some(current) = current {
                transaction.execute("INSERT INTO history(name,revision,body,sha256) SELECT name,revision,body,sha256 FROM records WHERE name=?1", [name])?;
                let changed = transaction.execute(
                "UPDATE records SET revision=?1,body=?2,sha256=?3 WHERE name=?4 AND revision=?5",
                params![
                    revision as i64,
                    bytes,
                    hash(bytes),
                    name,
                    current.revision as i64
                ],
            )?;
                if changed != 1 {
                    return Err(Error::Conflict);
                }
            } else {
                transaction.execute(
                    "INSERT INTO records VALUES(?1,?2,?3,?4)",
                    params![name, revision as i64, bytes, hash(bytes)],
                )?;
            }
            results.push(Record {
                revision,
                bytes: bytes.to_vec(),
            });
        }
        transaction.commit()?;
        Ok(results)
    }
    /// Coordinate a multi-step user operation; database CAS remains authoritative.
    pub fn coordinate(&self) -> Result<File, Error> {
        self.verify_identity()?;
        let descriptor = fs::openat(
            &self.directory,
            "operation.lock",
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )?;
        check(&fs::fstat(&descriptor)?, self.owner, false)?;
        let file = File::from(descriptor);
        file.try_lock().map_err(|_| Error::Conflict)?;
        Ok(file)
    }
    /// Recovery can inspect history, but opening a store never silently uses it.
    pub fn history(&self, name: &str, revision: u64) -> Result<Option<Record>, Error> {
        if !name_valid(name) || revision > i64::MAX as u64 {
            return Err(Error::InvalidInput);
        }
        self.verify_identity()?;
        let row = self
            .connection
            .query_row(
                "SELECT revision,body,sha256 FROM history WHERE name=?1 AND revision=?2",
                params![name, revision as i64],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(revision, bytes, checksum)| decode(revision, bytes, checksum))
            .transpose()
    }
}
