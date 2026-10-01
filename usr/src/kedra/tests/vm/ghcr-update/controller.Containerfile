# Disposable local qualification runtime; never a published OS image or host install.
FROM docker.io/library/ubuntu@sha256:008173c23f95b170204355c12626cb5a965d779a7e1283b09e9cffbb1bf33ca3 AS firmware
RUN apt-get update && apt-get install -y --no-install-recommends qemu-efi-aarch64 python3-virt-firmware && \
    mkdir /firmware-evidence && \
    dpkg-query -W -f='${Package} ${Version}\n' qemu-efi-aarch64 python3-virt-firmware > /firmware-evidence/packages.txt && \
    sha256sum /usr/share/AAVMF/AAVMF_CODE.secboot.fd /usr/share/AAVMF/AAVMF_VARS.ms.fd \
      /usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json > /firmware-evidence/sha256.txt && \
    virt-fw-vars --input /usr/share/AAVMF/AAVMF_VARS.ms.fd --print --verbose > /firmware-evidence/variables.txt

FROM docker.io/library/docker@sha256:3f3c01aaaebf7cce837356b688b7c059a4749f10bd7660dec7c58fc454a283f0 AS docker-cli
FROM docker.io/library/rust@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546
ARG FIXTURE_UID=1000
ARG FIXTURE_GID=1000
LABEL dev.kedra.lab.owner=kedra-release-fixture dev.kedra.lab.kind=release-controller
RUN test "$(uname -m)" = aarch64 && \
    apt-get update && apt-get install -y --no-install-recommends sudo git ca-certificates curl jq \
      podman skopeo qemu-system-arm qemu-utils python3-virt-firmware openssl e2fsprogs rpm cpio \
      procps util-linux findutils && rm -rf /var/lib/apt/lists/* && \
    groupadd --force --gid "$FIXTURE_GID" kedra-fixture && \
    useradd --non-unique --uid "$FIXTURE_UID" --gid "$FIXTURE_GID" --create-home --shell /bin/bash kedra-fixture && \
    printf 'kedra-fixture ALL=(root) NOPASSWD: ALL\n' > /etc/sudoers.d/kedra-fixture && \
    chmod 0440 /etc/sudoers.d/kedra-fixture && visudo -cf /etc/sudoers.d/kedra-fixture && \
    mkdir -p /usr/share/kedra-release-fixture && \
    printf 'Kedra disposable ARM release controller v1\n' > /usr/share/kedra-release-fixture/controller
COPY --from=docker-cli /usr/local/bin/docker /usr/bin/docker
COPY --from=docker-cli /usr/local/libexec/docker/cli-plugins/docker-buildx /usr/libexec/docker/cli-plugins/docker-buildx
# Context contains only the root-verified uv 0.12.19 ELF and this fixed entrypoint.
COPY uv /usr/local/bin/uv
RUN printf '%s\n' \
      '55bfa076d51381e0bc28b12d0a6338b74ec5e9e0ca537df04988ea21ef48e42c  /usr/bin/docker' \
      'ba87a4f36e123f77da72c0e1a17e611d0b6b33e4ca0a6a8a4847feb30ac5a79e  /usr/local/bin/uv' | sha256sum --check && \
    chmod 0755 /usr/local/bin/uv && \
    UV_PYTHON_INSTALL_DIR=/opt/kedra-python uv python install 3.12 && chmod -R a+rX /opt/kedra-python
COPY --from=firmware /usr/share/AAVMF/ /usr/share/AAVMF/
COPY --from=firmware /usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json /usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json
COPY --from=firmware /firmware-evidence/ /usr/share/kedra-release-fixture/firmware/
COPY controller-entrypoint.sh /usr/local/bin/kedra-fixture-controller
RUN chmod 0755 /usr/local/bin/kedra-fixture-controller && \
    sha256sum --check /usr/share/kedra-release-fixture/firmware/sha256.txt && \
    dpkg-query -W -f='${Package} ${Version}\n' podman skopeo qemu-system-arm qemu-utils python3-virt-firmware \
      > /usr/share/kedra-release-fixture/tool-packages.txt
ENV HOME=/home/kedra-fixture UV_PYTHON_INSTALL_DIR=/opt/kedra-python UV_PYTHON_DOWNLOADS=never
WORKDIR /repo
ENTRYPOINT ["/usr/local/bin/kedra-fixture-controller"]
CMD ["sleep", "infinity"]
