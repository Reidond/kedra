# Disposable input-preparation container for full local image builds: Fedora 44
# (rpm and cpio for Bitwarden's official RPM) and the pinned uv from AGENTS.md.
# Runs the same agents/bitwarden preparation scripts as the Actions build.
# Never part of an OS image. Digests are the multi-arch indexes.
FROM quay.io/fedora/fedora:44@sha256:539cadb5d8a43564d8abefd6eafdfcbcd4809070efbb900ec248229903db5911
LABEL dev.kedra.lab.owner=kedra-container-tests
COPY --from=ghcr.io/astral-sh/uv:0.12.19@sha256:04d046b13e60d6bcec73cbc5e1cad25d680dea90c8573340950a0ac2d1aef424 /uv /uvx /usr/local/bin/
RUN dnf -y --setopt=install_weak_deps=False install git-core cpio && dnf clean all
