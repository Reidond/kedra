# Runs only inside Docker's Linux VM; no host root privileges.
FROM docker.io/library/fedora@sha256:e402cca673711fee025f9ce21c6c08b1bd25ee26b3c482119c8675fbaaedbc85
LABEL dev.kedra.lab.owner=kedra-container-tests
RUN dnf -y --setopt=install_weak_deps=False install podman skopeo qemu-img jq && dnf clean all
COPY build-disk.sh /usr/local/bin/kedra-build-disk
COPY sign-fixture.sh /usr/local/bin/kedra-sign-fixture
RUN chmod 0755 /usr/local/bin/kedra-build-disk /usr/local/bin/kedra-sign-fixture
CMD ["sleep", "infinity"]
