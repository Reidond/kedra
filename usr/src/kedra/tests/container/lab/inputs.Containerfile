# Disposable input-preparation container for full local image builds: Fedora 44
# (rpm and cpio for Bitwarden's official RPM) and the pinned uv from AGENTS.md.
# Runs the same agents/bitwarden preparation scripts as the Actions build.
# Never part of an OS image. Docker Official Images publishes this Fedora 44
# multi-arch index from Fedora's docker-brew repository; it remains available
# after the rolling Quay tag garbage-collected its superseded daily manifest.
FROM docker.io/library/fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80
LABEL dev.kedra.lab.owner=kedra-container-tests
COPY --from=ghcr.io/astral-sh/uv:0.12.19@sha256:04d046b13e60d6bcec73cbc5e1cad25d680dea90c8573340950a0ac2d1aef424 /uv /uvx /usr/local/bin/
RUN dnf -y --setopt=install_weak_deps=False install git-core cpio && dnf clean all
