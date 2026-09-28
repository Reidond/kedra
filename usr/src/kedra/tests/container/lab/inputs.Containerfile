# Disposable input-preparation container for full local image builds: Fedora 44
# (rpm and cpio for Bitwarden's official RPM) and the pinned uv from AGENTS.md.
# Runs the same agents/bitwarden preparation scripts as the Actions build.
# Never part of an OS image. Digests are the multi-arch indexes.
FROM quay.io/fedora/fedora:44@sha256:1ccd18224d9b302fe62d3cefd0a9e0724c365ea8ac3bfb017c569540fe76433a
LABEL dev.kedra.lab.owner=kedra-container-tests
COPY --from=ghcr.io/astral-sh/uv:0.12.19@sha256:04d046b13e60d6bcec73cbc5e1cad25d680dea90c8573340950a0ac2d1aef424 /uv /uvx /usr/local/bin/
RUN dnf -y --setopt=install_weak_deps=False install git-core cpio && dnf clean all
