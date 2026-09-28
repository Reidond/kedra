# Runs only inside Docker's Linux VM; no host root privileges.
FROM quay.io/fedora/fedora@sha256:c61b46d9d6a76b37a765368fb78149e220bccf14019605846cb8558e983dc131
LABEL dev.kedra.lab.owner=kedra-container-tests
RUN dnf -y --setopt=install_weak_deps=False install podman skopeo qemu-img jq && dnf clean all
COPY build-disk.sh /usr/local/bin/kedra-build-disk
RUN chmod 0755 /usr/local/bin/kedra-build-disk
CMD ["sleep", "infinity"]
