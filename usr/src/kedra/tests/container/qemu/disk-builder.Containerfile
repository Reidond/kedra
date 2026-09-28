# Runs only inside Docker's Linux VM; no host root privileges.
FROM quay.io/fedora/fedora@sha256:a0051694e58c460dc9f23774355f321bfbe6bf5039ef6e66bc4ee6d522e17f2e
LABEL dev.kedra.lab.owner=kedra-container-tests
RUN dnf -y --setopt=install_weak_deps=False install podman skopeo qemu-img jq && dnf clean all
COPY build-disk.sh /usr/local/bin/kedra-build-disk
RUN chmod 0755 /usr/local/bin/kedra-build-disk
CMD ["sleep", "infinity"]
