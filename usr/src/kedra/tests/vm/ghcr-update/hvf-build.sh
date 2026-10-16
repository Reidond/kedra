#!/bin/bash
# Runs as root only inside run-hvf.py's disposable privileged controller
# container (Fedora bootc 44 on the macOS Docker engine), never on a host.
# Stages: setup, keys, base, variants, disk. /fixture is <work>/kedra-ghcr,
# /private <work>/kedra-ghcr-private and /evidence <work>/evidence.
set -euo pipefail
test "$(uname -m)" = aarch64
test -f /.dockerenv
repository=ghcr.io/reidond/kedra-qemu-arm64
case "${1:?stage}" in
setup)
    # Nested Podman needs its own cgroup subtree, as in the lab's disk builder.
    test "$(< /proc/self/cgroup)" = '0::/'
    mkdir /sys/fs/cgroup/kedra-init
    for pid in $(cat /sys/fs/cgroup/cgroup.procs); do
        echo "$pid" > /sys/fs/cgroup/kedra-init/cgroup.procs 2>/dev/null || true
    done
    echo $$ > /sys/fs/cgroup/kedra-init/cgroup.procs
    for controller in $(< /sys/fs/cgroup/cgroup.controllers); do
        echo "+$controller" > /sys/fs/cgroup/cgroup.subtree_control
    done
    dnf -y --setopt=install_weak_deps=False install qemu-img > /evidence/controller-packages.log 2>&1
    ;;
keys)
    umask 077
    openssl rand -base64 32 > /private/passphrase
    for key in allowed wrong; do
        skopeo generate-sigstore-key --output-prefix "/private/$key" --passphrase-file /private/passphrase
    done
    openssl req -x509 -newkey rsa:3072 -nodes -days 1 -subj /CN=ghcr.io \
        -addext subjectAltName=DNS:ghcr.io,IP:127.0.0.1 -keyout /private/tls.key -out /fixture/context/tls.crt \
        2> /evidence/tls-generation.log
    chmod 0644 /fixture/context/tls.crt /private/tls.key
    # The controller reaches the registry by its network alias ghcr.io.
    mkdir -p /etc/containers/certs.d/ghcr.io
    cp /fixture/context/tls.crt /etc/containers/certs.d/ghcr.io/ca.crt
    printf 'docker:\n  ghcr.io:\n    use-sigstore-attachments: true\n' > /etc/containers/registries.d/kedra-ghcr-test.yaml
    ;;
base)
    podman build --pull=newer --network=host --build-arg "BASE_IMAGE=${2:?base image}" --tag localhost/kedra-ghcr:base \
        /fixture/context > /evidence/base-build.log 2>&1
    podman run --rm --network=none localhost/kedra-ghcr:base rpm -qa \
        --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
        > /fixture/package-material.txt
    ;;
variants)
    for variant in A B C D E U W R N T X H M; do
        identity=$(cat "/fixture/context/$variant/image-identity.json")
        podman build --pull=never --build-arg "KEDRA_IDENTITY=$identity" --build-arg "VARIANT=$variant" \
            --file /fixture/context/variant.Containerfile --tag "localhost/kedra-ghcr:$variant" /fixture/context \
            > "/evidence/build-$variant.log" 2>&1
        signing=(--sign-by-sigstore-private-key /private/allowed.private --sign-passphrase-file /private/passphrase)
        if test "$variant" = U; then signing=(); fi
        if test "$variant" = W; then
            signing=(--sign-by-sigstore-private-key /private/wrong.private --sign-passphrase-file /private/passphrase)
        fi
        target=$repository
        if test "$variant" = R; then target=ghcr.io/reidond/kedra-other; fi
        skopeo copy --preserve-digests "${signing[@]}" --digestfile "/fixture/$variant.digest" \
            "containers-storage:localhost/kedra-ghcr:$variant" "docker://$target:$variant" > "/evidence/sign-$variant.log" 2>&1
        if test "$variant" = R; then
            skopeo copy --preserve-digests "docker://$target:R" "docker://$repository:R" > /evidence/copy-wrong-repository.log 2>&1
        fi
    done
    # Withdraw only N's signature attachment; its signed image bytes stay.
    missing=$(cat /fixture/N.digest)
    skopeo inspect --raw "docker://$repository:sha256-${missing#sha256:}.sig" > /evidence/removed-signature-manifest.json
    attachment=$(skopeo manifest-digest /evidence/removed-signature-manifest.json)
    curl --silent --show-error --fail --cacert /fixture/context/tls.crt -X DELETE \
        "https://ghcr.io/v2/reidond/kedra-qemu-arm64/manifests/$attachment"
    ;;
disk)
    truncate -s 16M /fixture/cases.raw
    mkfs.ext4 -q -L KEDRA_GHCR_CASES -d /fixture/cases /fixture/cases.raw
    initial="$repository@$(cat /fixture/A.digest)"
    # Signature-verified copy with the fixture key, so containers-storage keeps A's signature.
    jq --arg key /fixture/context/release.pub '.transports[][][].keyPath=$key' /fixture/context/policy.json \
        > /tmp/kedra-ghcr-policy.json
    skopeo --policy /tmp/kedra-ghcr-policy.json copy --preserve-digests "docker://$initial" "containers-storage:$initial" \
        > /evidence/verified-initial.log 2>&1
    # bootc-image-builder installs A with A's own bootc and policy: its
    # enforce-container-sigpolicy install configuration admits only the signed
    # image, and the deployment's origin is the exact signed registry reference.
    builder=${2:?pinned bootc-image-builder}
    podman pull "$builder" > /evidence/builder-pull.log 2>&1
    # On the macOS Docker engine an osbuild stage occasionally fails to reopen a
    # loop partition ("Can't open blockdev") after earlier stages mounted it; the
    # build starts again from nothing, at most three times.
    for attempt in 1 2 3; do
        rm -rf /var/tmp/kedra-ghcr-disk && mkdir -p /var/tmp/kedra-ghcr-disk
        if podman run --rm --privileged --security-opt label=type:unconfined_t \
            -v /var/tmp/kedra-ghcr-disk:/output -v /var/lib/containers/storage:/var/lib/containers/storage \
            "$builder" --type raw --rootfs ext4 --use-librepo=True "$initial" > "/evidence/disk-build-$attempt.log" 2>&1; then
            break
        fi
        test "$attempt" -lt 3
    done
    mapfile -t disks < <(find /var/tmp/kedra-ghcr-disk -name '*.raw' -type f)
    test "${#disks[@]}" -eq 1
    qemu-img convert -f raw -O qcow2 "${disks[0]}" /fixture/disk.qcow2
    qemu-img check /fixture/disk.qcow2 > /evidence/disk-check.log
    rm -rf /var/tmp/kedra-ghcr-disk
    chmod 0644 /fixture/disk.qcow2
    ;;
*)
    echo "unknown stage $1" >&2
    exit 2
    ;;
esac
