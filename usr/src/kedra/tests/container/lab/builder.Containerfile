# Disposable lab builder: the pinned Rust toolchain (rust-toolchain.toml), Git,
# bubblewrap, jq, the pinned uv (AGENTS.md) and the Docker CLI with BuildKit
# (buildx). Builds Linux sysroot binaries, runs `sysroot source archive` on
# throwaway snapshots and, for full local builds, the release's catalog steps
# (builder roots, reviewed archives through image/catalog/prepare-sources.py,
# package builds and the contribution with its release material), the engine's
# `sysroot system compose` (which observes the foundation in the engine's
# bubblewrap sandbox) and `sysroot derive` against the lab's own Docker engine.
# Never part of an OS image; digests are multi-arch indexes (rust:1.98.1;
# docker:29.4.0-cli, whose static CLI carries buildx v0.33.0; uv 0.12.19).
FROM docker.io/library/rust:1.98.1@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546
LABEL dev.kedra.lab.owner=kedra-container-tests
COPY --from=ghcr.io/astral-sh/uv:0.12.19@sha256:04d046b13e60d6bcec73cbc5e1cad25d680dea90c8573340950a0ac2d1aef424 /uv /uvx /usr/local/bin/
COPY --from=docker.io/library/docker:29.4.0-cli@sha256:2efe7c8e806b35050def6ba1ba0ddad67b521a0a6d37f4910b3d88f11b495d03 \
    /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker.io/library/docker:29.4.0-cli@sha256:2efe7c8e806b35050def6ba1ba0ddad67b521a0a6d37f4910b3d88f11b495d03 \
    /usr/local/libexec/docker/cli-plugins/docker-buildx /usr/local/libexec/docker/cli-plugins/docker-buildx
# Snapshots and the read-only checkout mount belong to other users.
RUN git config --system --add safe.directory '*' \
    && apt-get update && apt-get install -y --no-install-recommends bubblewrap jq \
    && rm -rf /var/lib/apt/lists/*
