# ADR 0008: private SQLite state and a separate I/O boundary

Date: 2026-09-08. Status: implemented storage subset; Linux tests pass in
[run 34179189211](https://github.com/Reidond/kedra/actions/runs/34179189211).

R03 now has a serialized disposition model and R08 has replay state. Both require
durable compare-and-replace operations and recovery records. Use SQLite
transactions rather than a new journal/file-commit format. The helper package
adds an explicit flat library target for small Linux I/O primitives, reusable by
the user CLI. The helper remains independent of CLI, Git and agent execution.
The shared core remains pure; source/archive operations live in the CLI package.

Stores use a new private directory for initialization. Opening an existing store
never supplies SQLite's CREATE flag or resets missing/corrupt/new-version data.
Effective-UID ownership, exact 0700 directory/0600 file permissions, regular-file
type, single link count and inode identity are checked with safe rustix APIs.
Symlink database paths and existing sidecars are refused. Root callers must use
a fixed root-controlled location; this is not isolation from another malicious
process with the same UID and permission to edit the directory.

SQLite runs with a versioned strict schema, trusted_schema off, no attached
databases, bounded values/SQL, WAL and synchronous FULL. A CAS transaction checks
the caller's read revision, retains the prior record and writes the new revision
with a SHA-256 checksum. History is available for explicit recovery, never an
automatic fallback that could regress trust high-water marks or drop home policy.
No root deployment or real-home adoption is enabled by adding the storage layer.

Tests use fresh generated temporary directories. They cover reopen/history/CAS,
concurrent writes, corruption/new schemas, ownership/permissions/link/type checks,
inode replacement and child-process termination before/after commit. A child
fixture requires a generated marker. Process-interruption tests are not a physical
power-loss claim; durability depends on the filesystem/device honoring sync.

Dependencies: rustix 1.1.4 and rusqlite 0.40.2 with bundled SQLite, pinned by
Cargo.lock. Bundling avoids a new runtime library dependency but requires tracking
SQLite updates with Cargo/source updates; Fedora RPM refresh does not update the
embedded library. No new package or Cargo check runner was added.

Sources reviewed 2026-09-08:
[rustix openat](https://docs.rs/rustix/1.1.4/rustix/fs/fn.openat.html),
[rusqlite open flags](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.OpenFlags.html),
[SQLite atomic commit](https://www.sqlite.org/atomiccommit.html) and
[SQLite pragmas](https://www.sqlite.org/pragma.html).
