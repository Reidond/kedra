# Disposable lab builder: the pinned Rust toolchain (rust-toolchain.toml) and Git.
# Builds Linux sysroot binaries and runs `sysroot source archive` on throwaway
# snapshots. Never part of an OS image; digest is the rust:1.98.1 multi-arch index.
FROM docker.io/library/rust:1.98.1@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546
LABEL dev.kedra.lab.owner=kedra-container-tests
# Snapshots and the read-only checkout mount belong to other users.
RUN git config --system --add safe.directory '*'
