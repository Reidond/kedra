#!/bin/bash
# Executed as root only in the disposable privileged Docker builder.
set -euo pipefail
test "$(uname -m)" = aarch64
test "$(< /proc/self/cgroup)" = '0::/'
mkdir /sys/fs/cgroup/kedra-init
# Reading cgroup.procs while migrating members can skip entries; take a snapshot.
members=$(cat /sys/fs/cgroup/cgroup.procs)
for pid in $members; do
    echo "$pid" > /sys/fs/cgroup/kedra-init/cgroup.procs 2>/dev/null || true
done
echo $$ > /sys/fs/cgroup/kedra-init/cgroup.procs
for controller in $(< /sys/fs/cgroup/cgroup.controllers); do
    echo "+$controller" > /sys/fs/cgroup/cgroup.subtree_control
done
loaded=$(podman load --quiet)
printf '%s\n' "$loaded"
loaded_id=${loaded#Loaded image: }
[[ "$loaded_id" =~ ^sha256:[a-f0-9]{64}$ ]]
image=${1:?image reference}
builder=${2:?pinned builder}
metadata_sha=${3:?expected image content metadata hash}
# Docker's containerd store and Podman can assign different IDs to the same
# archive. Bind the imported rootfs/config/platform before assigning its tag.
podman image inspect "$loaded_id" > /output/imported-image.json
actual=$(jq -cjS '.[0] | {Architecture, Os, RootFS: .RootFS.Layers, Config: (.Config | {Labels, Env, Cmd, Entrypoint, User, WorkingDir})}' \
    /output/imported-image.json | sha256sum | cut -d ' ' -f 1)
test "$actual" = "$metadata_sha"
podman tag "$loaded_id" "$image"
test "$(podman image inspect "$image" --format '{{.Os}}/{{.Architecture}}')" = linux/arm64
mkdir -p /output
podman pull "$builder"
podman run --rm --privileged --security-opt label=type:unconfined_t \
    -v /output:/output -v /var/lib/containers/storage:/var/lib/containers/storage \
    "$builder" --type raw --rootfs ext4 --use-librepo=True "$image"
mapfile -t disks < <(find /output -name '*.raw' -type f)
test "${#disks[@]}" -eq 1
# bootc-image-builder's qcow2 output enables expensive compression. Local lab
# storage favors preparation/read latency; keep the sparse backing uncompressed.
qemu-img convert -f raw -O qcow2 "${disks[0]}" /output/base.qcow2
qemu-img check /output/base.qcow2
chmod 0644 /output/base.qcow2
